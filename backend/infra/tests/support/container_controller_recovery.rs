//! Actual custody across two original Compose controller restarts; execution remains fenced.
use super::*;
use app::runtime_launch::{
    ControllerRecoveryCommand, ControllerRecoveryRecord, RuntimeLaunchBinding,
};
use serde::Serialize;
use std::os::unix::fs::PermissionsExt;

const ROOT: &str = "/controller";
const TARGET: &str = "custody-approval.txt";
const PROMPT: &str = "Await custody approval, do not repeat the command";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Context {
    owner: Uuid,
    agents: Vec<Uuid>,
    launches: Vec<RuntimeLaunchBinding>,
    pids: Vec<i32>,
    session: Uuid,
    run: Uuid,
    message: Uuid,
    message_key: String,
    native_run: String,
    native_session: String,
    endpoint: String,
    approval: Uuid,
    transcript_sha256: String,
    transcript_content_sha256: String,
    dispatch_sha256: String,
}

fn command(record: &ControllerRecoveryRecord) -> ControllerRecoveryCommand {
    ControllerRecoveryCommand {
        request: record.request.clone(),
        epoch: record.epoch,
        lease_version: record.lease_version,
        lease_expires_at: record.lease_expires_at.clone(),
    }
}

async fn save(name: &str, value: &impl Serialize) {
    use tokio::io::AsyncWriteExt;
    let path = Path::new(ROOT).join(name);
    let bytes = serde_json::to_vec(value).unwrap();
    assert!(bytes.len() < 1024 * 1024);
    let mut file = tokio::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&path)
        .await
        .unwrap();
    file.write_all(&bytes).await.unwrap();
    file.sync_all().await.unwrap();
    tokio::fs::File::open(ROOT)
        .await
        .unwrap()
        .sync_all()
        .await
        .unwrap();
}

async fn read<T: serde::de::DeserializeOwned>(name: &str) -> T {
    let path = Path::new(ROOT).join(name);
    let meta = tokio::fs::symlink_metadata(&path).await.unwrap();
    assert!(meta.is_file() && meta.uid() == 999 && meta.mode() & 0o777 == 0o600);
    assert!(meta.len() < 1024 * 1024);
    serde_json::from_slice(&tokio::fs::read(path).await.unwrap()).unwrap()
}

fn hash(value: &impl Serialize) -> String {
    let mut sorted = serde_json::to_value(value).unwrap();
    sorted.sort_all_objects();
    hex::encode(Sha256::digest(serde_json::to_vec(&sorted).unwrap()))
}

async fn dispatch_hash(repo: &PostgresFleetRepository, message: Uuid) -> String {
    let intent = repo
        .get_hermes_dispatch_intent(message)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(hashlib(&intent.request_body), intent.request_hash);
    hash(
        &json!({"message_id":intent.message_id,"run_id":intent.run.id,
        "session_id":intent.run.session_id,"agent_id":intent.run.agent_id,
        "key":intent.idempotency_key,"sha256":intent.request_hash,"origin":intent.origin,
        "credential_fingerprint":intent.credential_fingerprint,"capabilities":intent.capabilities,
        "submitted_at":intent.submitted_at,"recovery_deadline":intent.recovery_deadline}),
    )
}

fn hashlib(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

async fn approval_model(
    State(calls): State<Arc<Mutex<usize>>>,
    Json(body): Json<Value>,
) -> Response {
    let mut count = calls.lock().await;
    *count += 1;
    assert_eq!(
        *count, 1,
        "custody fixture must not receive a second inference"
    );
    assert_eq!(body["model"], "fleet-container-local-model");
    assert!(
        body["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["function"]["name"] == "terminal")
    );
    let arguments = json!({"command":format!("chmod 666 ./{TARGET}"),"timeout":5}).to_string();
    let mut message = json!({"role":"assistant","content":null,"tool_calls":[{
        "id":"call_custody_approval","type":"function","function":{"name":"terminal","arguments":arguments}}]});
    if body["stream"] == true {
        message["tool_calls"][0]["index"] = json!(0);
        let chunk = json!({"id":"chatcmpl-custody","object":"chat.completion.chunk","created":1,
            "model":"fleet-container-local-model","choices":[{"index":0,"delta":message,"finish_reason":null}]});
        let end = json!({"id":"chatcmpl-custody","object":"chat.completion.chunk","created":1,
            "model":"fleet-container-local-model","choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]});
        return (
            [("content-type", "text/event-stream")],
            format!("data: {chunk}\n\ndata: {end}\n\ndata: [DONE]\n\n"),
        )
            .into_response();
    }
    Json(json!({"id":"chatcmpl-custody","object":"chat.completion","created":1,
        "model":"fleet-container-local-model","choices":[{"index":0,"message":message,"finish_reason":"tool_calls"}],
        "usage":{"prompt_tokens":12,"completion_tokens":5,"total_tokens":17}})).into_response()
}

fn control(
    config: &AppConfig,
    launch: &RuntimeLaunchBinding,
) -> (ContainerControl, ContainerLaunchFiles) {
    let binding = launch.container.as_ref().unwrap();
    let settings = config.fleet.container_control.as_ref().unwrap();
    (
        ContainerControl::new(
            settings.python.clone().into(),
            ControlSource {
                root: settings.base_root.clone().into(),
                sha256: binding.source_sha256.clone(),
            },
            binding.context.clone(),
        )
        .unwrap(),
        ContainerLaunchFiles {
            policy: binding.policy.clone(),
            compose: binding.compose.clone().into(),
            journal: binding.journal.clone().into(),
            stop_journal: binding.stop_journal.clone().into(),
            mount_mapping: binding.mount_mapping.clone(),
            mapping_file: binding.mapping_file.clone().map(Into::into),
        },
    )
}

fn transcript_content_hash(messages: &[domain::SessionMessage]) -> String {
    let mut value = json!(messages);
    for message in value.as_array_mut().unwrap() {
        let object = message.as_object_mut().unwrap();
        for field in ["delivery_state", "delivery_error", "replayed"] {
            object.remove(field);
        }
    }
    hash(&value)
}

async fn session_state(repo: &PostgresFleetRepository, context: &Context, stopped: bool) {
    assert_eq!(context.agents.len(), 2);
    assert_eq!(context.launches.len(), 2);
    assert_eq!(context.pids.len(), 2);
    assert_eq!(
        repo.list_session_agent_runs(context.session)
            .await
            .unwrap()
            .len(),
        1
    );
    let run = repo
        .list_session_agent_runs(context.session)
        .await
        .unwrap()
        .remove(0);
    assert_eq!(run.id, context.run);
    assert_eq!(
        run.runtime_run_id.as_deref(),
        Some(context.native_run.as_str())
    );
    if stopped {
        assert_eq!(run.state, SessionRunState::Cancelled);
        assert_eq!(
            run.last_error.as_deref(),
            Some("Original runtime namespace exit confirmed")
        );
    } else {
        assert!(matches!(
            run.state,
            SessionRunState::Running | SessionRunState::Waiting
        ));
    }
    let approvals = repo.list_session_approvals(context.session).await.unwrap();
    assert_eq!(approvals.len(), 1);
    assert_eq!(approvals[0].id, context.approval);
    let messages = repo.list_session_messages(context.session).await.unwrap();
    if stopped {
        assert_eq!(approvals[0].state, domain::RuntimeApprovalState::Cancelled);
        assert!(approvals[0].resolved_by_user_id.is_none());
        assert!(approvals[0].resolved_at.is_some());
        assert_eq!(
            messages
                .iter()
                .find(|message| message.id == context.message)
                .unwrap()
                .delivery_state,
            domain::MessageDeliveryState::Completed
        );
        assert_eq!(
            transcript_content_hash(&messages),
            context.transcript_content_sha256
        );
    } else {
        assert_eq!(approvals[0].state, domain::RuntimeApprovalState::Pending);
        assert_eq!(hash(&messages), context.transcript_sha256);
    }
    assert_eq!(
        dispatch_hash(repo, context.message).await,
        context.dispatch_sha256
    );
    let target = Path::new(
        &repo
            .get_agent(context.agents[0])
            .await
            .unwrap()
            .paths
            .workspace,
    )
    .join(TARGET);
    assert_eq!(
        tokio::fs::metadata(target).await.unwrap().mode() & 0o777,
        0o600
    );
}

async fn state(repo: &PostgresFleetRepository, config: &AppConfig, context: &Context) {
    session_state(repo, context, false).await;
    for ((id, binding), pid) in context
        .agents
        .iter()
        .zip(&context.launches)
        .zip(&context.pids)
    {
        let current = repo.get_open_runtime_launch(*id).await.unwrap().unwrap();
        assert_eq!(hash(&current.binding), hash(binding));
        assert_eq!(current.pid, Some(*pid));
        assert_eq!(current.state, "gateway_started");
    }
    let native: Value = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap()
        .get(format!(
            "{}/v1/runs/{}",
            context.endpoint, context.native_run
        ))
        .bearer_auth(infra::agent_runtime_token(config, context.agents[0]).unwrap())
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(native["run_id"], context.native_run);
    assert_eq!(native["status"], "waiting_for_approval");
    assert_eq!(native["session_id"], context.native_session);
}

async fn prepare(
    repo: Arc<PostgresFleetRepository>,
    db: &sea_orm::DatabaseConnection,
    proof: &Proof,
) {
    let owner = Uuid::new_v4();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO users(id,email,username,display_name,password_hash,is_system_admin,system_role)
         VALUES($1,$2,$3,'Custody QA owner','disabled',false,'user')",
        [owner.into(), format!("{owner}@example.test").into(), owner.to_string().into()])).await.unwrap();
    repo.ensure_runtime_templates().await.unwrap();
    let calls = Arc::new(Mutex::new(0));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let router = Router::new()
        .route("/v1/chat/completions", post(approval_model))
        .with_state(calls.clone());
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut config = AppConfig::default();
    config.fleet.agents_root = "/agents".into();
    config.fleet.controller_root = ROOT.into();
    config.fleet.hermes_source = "immutable-pinned-Hermes-image".into();
    config.fleet.runtime_token_secret = format!("owned-custody-{}", Uuid::new_v4());
    config.fleet.agent_port_base = 29100;
    config.fleet.agent_port_stride = 5;
    config.fleet.project_workflow_url = None;
    config.fleet.container_control = Some(proof.control.clone());
    let config = Arc::new(config);
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime = LocalRuntimeSupervisor::new(config.clone(), repo.clone(), events);
    let mut agents = Vec::new();
    let mut launches = Vec::new();
    let mut pids = Vec::new();
    for ordinal in 1..=2 {
        let agent = create_agent(&repo, &config, &format!("Custody agent {ordinal}")).await;
        let mut desired =
            configuration(&proof.model_host, port, &format!("CUSTODY_SOUL_{ordinal}"));
        desired.soul_md = format!(
            "# CUSTODY_SOUL_{ordinal}\nUse only the requested owned QA terminal command.\n"
        );
        desired.config_json["platform_toolsets"]["api_server"] = json!(["terminal"]);
        desired.config_json["approvals"] = json!({"mode":"manual","timeout":600});
        desired.env_json["TERMINAL_ENV"] = json!("local");
        desired.env_json["TERMINAL_CWD"] = json!("/workspace");
        let revision = request_activation(&repo, agent.id, owner, desired).await;
        activated(&repo, agent.id, revision).await;
        assert_eq!(
            runtime.start(&agent).await.unwrap().status,
            AgentStatus::Running
        );
        let launch = repo
            .get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .unwrap();
        pids.push(launch.pid.unwrap());
        launches.push(launch.binding);
        agents.push(agent.id);
    }
    let agent = repo.get_agent(agents[0]).await.unwrap();
    let target = Path::new(&agent.paths.workspace).join(TARGET);
    tokio::fs::write(&target, b"owned custody approval target")
        .await
        .unwrap();
    tokio::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600))
        .await
        .unwrap();
    let (session, message, request) = send(&repo, agent.id, owner, PROMPT).await;
    let (native, files) = control(&config, &launches[0]);
    let endpoint = native
        .endpoint(
            &files,
            &launches[0].container.as_ref().unwrap().registration,
        )
        .await
        .unwrap();
    let endpoint = format!("http://{endpoint}:{}", agent.api_port.unwrap());
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let approval = timeout(Duration::from_secs(120), async {
        loop {
            let values = repo.list_session_approvals(session).await.unwrap();
            if let Some(approval) = values.into_iter().next() {
                break approval;
            }
            let runs = repo.list_session_agent_runs(session).await.unwrap();
            let mut diagnostic = json!({"actual_model_calls":*calls.lock().await,
                "native_status":"unavailable","native_approval_present":false,
                "native_error_present":false,"fleet_approval_present":false,
                "raw_output_persisted":false,"sdlc_acceptance":false});
            let messages = repo.list_session_messages(session).await.unwrap();
            if let Some(run) = runs.first() {
                diagnostic["fleet_native_session_pinned"] = json!(run.runtime_session_id.is_some());
                diagnostic["message_native_id_matches"] = json!(
                    messages
                        .iter()
                        .find(|stored| stored.id == message.id)
                        .is_some_and(|stored| stored.runtime_message_id.is_some()
                            && stored.runtime_message_id == run.runtime_run_id)
                );
            }
            if let Some(id) = runs.first().and_then(|run| run.runtime_run_id.as_deref()) {
                if let Ok(response) = client
                    .get(format!("{endpoint}/v1/runs/{id}"))
                    .bearer_auth(infra::agent_runtime_token(&config, agent.id).unwrap())
                    .send()
                    .await
                {
                    if response.status().is_success() {
                        if let Ok(payload) = response.json::<Value>().await {
                            let status = payload["status"].as_str().unwrap_or("invalid");
                            diagnostic["native_status"] = json!(match status {
                                "queued"
                                | "started"
                                | "running"
                                | "waiting_for_approval"
                                | "completed"
                                | "failed"
                                | "cancelled"
                                | "interrupted"
                                | "stopping"
                                | "stopped" => status,
                                _ => "invalid",
                            });
                            diagnostic["native_approval_present"] =
                                json!(payload["approval"].is_object());
                            diagnostic["native_error_present"] = json!(!payload["error"].is_null());
                            let run = runs.first().unwrap();
                            let approval = &payload["approval"];
                            diagnostic["native_session_matches"] = json!(
                                payload["session_id"].as_str() == run.runtime_session_id.as_deref()
                            );
                            diagnostic["approval_session_matches"] = json!(
                                approval
                                    .get("session_id")
                                    .is_none_or(|session| session.as_str()
                                        == run.runtime_session_id.as_deref())
                            );
                            diagnostic["approval_run_matches"] =
                                json!(approval["run_id"] == payload["run_id"]);
                            diagnostic["approval_event_matches"] =
                                json!(approval["event"] == "approval.request");
                            diagnostic["approval_request_id_valid"] = json!(
                                approval["request_id"]
                                    .as_str()
                                    .is_some_and(|id| domain::valid_ref(id, 256))
                            );
                            diagnostic["approval_choices_valid"] =
                                json!(approval["choices"].as_array().is_some_and(|choices| {
                                    choices.iter().any(|choice| choice == "once")
                                        && choices.iter().any(|choice| choice == "deny")
                                }));
                            diagnostic["approval_prompt_valid"] = json!(
                                ["prompt", "description", "command", "message"]
                                    .into_iter()
                                    .find_map(|key| approval[key]
                                        .as_str()
                                        .filter(|value| !value.trim().is_empty()))
                                    .is_some_and(|value| value.len() <= 16384)
                            );
                        }
                    }
                }
            }
            tokio::fs::write(
                "/evidence/custody-prepare-readback.json",
                serde_json::to_vec(&diagnostic).unwrap(),
            )
            .await
            .unwrap();
            sleep(Duration::from_secs(1)).await;
        }
    })
    .await
    .unwrap();
    sleep(Duration::from_secs(1)).await;
    let run = repo
        .list_session_agent_runs(session)
        .await
        .unwrap()
        .remove(0);
    let context = Context {
        owner,
        agents,
        launches,
        pids,
        session,
        run: run.id,
        message: message.id,
        message_key: request.idempotency_key.unwrap(),
        native_session: run.runtime_session_id.unwrap(),
        native_run: run.runtime_run_id.unwrap(),
        endpoint,
        approval: approval.id,
        transcript_sha256: hash(&repo.list_session_messages(session).await.unwrap()),
        transcript_content_sha256: transcript_content_hash(
            &repo.list_session_messages(session).await.unwrap(),
        ),
        dispatch_sha256: dispatch_hash(&repo, message.id).await,
    };
    state(&repo, &config, &context).await;
    assert_eq!(*calls.lock().await, 1);
    save("custody-config.json", config.as_ref()).await;
    save("custody-context.json", &context).await;
    save(
        "custody-ready.json",
        &json!({"state":"ready","actual_model_calls":1,
        "prepare_pid":std::process::id(),"native_waiting_for_approval":true}),
    )
    .await;
    // Only an actual Compose container restart may terminate this first Fleet process.
    std::future::pending::<()>().await;
}

async fn recover(
    repo: Arc<PostgresFleetRepository>,
    mut config: AppConfig,
    context: Context,
    epoch: i64,
) {
    config.fleet.controller_recovery_enabled = true;
    let config = Arc::new(config);
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime = LocalRuntimeSupervisor::new(config.clone(), repo.clone(), events);
    let records = timeout(Duration::from_secs(90), async {
        loop {
            let mut records = Vec::new();
            let mut observed = Vec::new();
            for binding in &context.launches {
                if let Some(record) = repo.current_controller_recovery(binding.id).await.unwrap() {
                    let delivery = repo.read_controller_recovery_delivery(record.request.id).await.unwrap();
                    observed.push(json!({"state":record.state,"epoch":record.epoch,
                        "lease_version":record.lease_version,"lease_valid":record.lease_valid,
                        "native_receipt_present":record.native_receipt_sha256.is_some(),
                        "dispatch_claimed":delivery.as_ref().is_some_and(|value| value.dispatch_claimed)}));
                    if record.state == "acknowledged"
                        && record.lease_valid
                        && record.epoch == epoch
                        && record.lease_version >= 4
                    {
                        records.push(record);
                    }
                } else {
                    observed.push(json!({"state":"absent"}));
                }
            }
            tokio::fs::write(format!("/evidence/custody-recover-{epoch}-progress.json"),
                serde_json::to_vec(&json!({"agents":observed,"raw_receipts_persisted":false,
                    "sdlc_acceptance":false})).unwrap()).await.unwrap();
            if records.len() == context.agents.len() {
                break records;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("actual default-off startup worker did not maintain both leases");
    let mut proofs = Vec::new();
    for (id, record) in context.agents.iter().zip(&records) {
        let renewed = runtime.heartbeat_container_controller(*id).await.unwrap();
        assert!(renewed.lease_version > record.lease_version && renewed.lease_valid);
        let delivery = repo
            .read_controller_recovery_delivery(renewed.request.id)
            .await
            .unwrap()
            .unwrap();
        assert!(delivery.dispatch_claimed && delivery.native_receipt.is_some());
        assert_eq!(delivery.command.lease_version, 1);
        let original_receipt_hash = hash(delivery.native_receipt.as_ref().unwrap());
        assert_eq!(
            renewed.native_receipt_sha256.as_deref(),
            Some(original_receipt_hash.as_str())
        );
        assert!(
            repo.read_controller_stop(renewed.request.launch_id)
                .await
                .unwrap()
                .is_none(),
            "custody heartbeat cannot dispatch namespace stop"
        );
        proofs.push(
            json!({"initial":delivery.command,"current":command(&renewed),
            "receipt_sha256":original_receipt_hash}),
        );
    }
    let mut competitor_config = (*config).clone();
    competitor_config.fleet.controller_recovery_enabled = false;
    let (events, _) = tokio::sync::broadcast::channel(32);
    let competitor = LocalRuntimeSupervisor::new(Arc::new(competitor_config), repo.clone(), events);
    for id in &context.agents {
        assert!(
            competitor
                .stop(&repo.get_agent(*id).await.unwrap())
                .await
                .is_err()
        );
        assert!(competitor.recover_container_controller(*id).await.is_err());
        assert!(
            competitor
                .heartbeat_container_controller(*id)
                .await
                .is_err()
        );
    }
    let replay = repo
        .create_session_message(
            context.session,
            CreateSessionMessageRequest {
                body: PROMPT.into(),
                author_agent_id: None,
                message_kind: Some(MessageKind::UserPrompt),
                runtime_message_id: None,
                idempotency_key: Some(context.message_key.clone()),
            },
            context.owner,
        )
        .await
        .unwrap();
    assert_eq!(replay.id, context.message);
    state(&repo, &config, &context).await;
    if epoch == 3 {
        stop_original_namespaces(&runtime, &repo, &config, &context).await;
        runtime.quiesce_controller_recovery().await.unwrap();
        return;
    }
    runtime.quiesce_controller_recovery().await.unwrap();
    save(&format!("custody-epoch{epoch}.json"), &proofs).await;
    tokio::fs::write(
        format!("/evidence/custody-epoch{epoch}.json"),
        serde_json::to_vec(&json!({
        "state":"passed","epoch":epoch,"actual_startup_worker":true,"minimum_lease_version":4,
        "native_live_observation":true,"agents":2,"original_launches_unchanged":true,
        "native_run_waiting_for_approval":true,"new_owner_execution_held":true,
        "competing_logical_controller_denied":true,"message_replay_did_not_dispatch":true,
        "sdlc_acceptance":false,"resumed_execution":false}))
        .unwrap(),
    )
    .await
    .unwrap();
}

async fn stop_original_namespaces(
    runtime: &LocalRuntimeSupervisor,
    repo: &PostgresFleetRepository,
    config: &AppConfig,
    context: &Context,
) {
    let mut database = config.database.clone();
    database.url = std::env::var("FLEET_TEST_DATABASE_URL").unwrap();
    let db = infra::connect_database(database).await.unwrap();
    let mut proofs = Vec::new();
    let mut uncertain_readbacks = 0;
    for ((id, binding), pid) in context
        .agents
        .iter()
        .zip(&context.launches)
        .zip(&context.pids)
    {
        // Renewal is the real owner protocol, not an extended test-only deadline.
        let renewed = runtime.heartbeat_container_controller(*id).await.unwrap();
        assert!(renewed.epoch == 3 && renewed.lease_valid);
        assert!(
            repo.read_controller_stop(binding.id)
                .await
                .unwrap()
                .is_none()
        );
        let agent = repo.get_agent(*id).await.unwrap();
        let stopped = match runtime.stop(&agent).await {
            Ok(stopped) => stopped,
            Err(shared::AppError::Unavailable(_)) => {
                let retained = repo.read_controller_stop(binding.id).await.unwrap();
                let owner = repo
                    .current_controller_recovery(binding.id)
                    .await
                    .unwrap()
                    .unwrap();
                tokio::fs::write("/evidence/custody-stop-readback.json", serde_json::to_vec(&json!({
                    "stop_intent_retained":retained.is_some(),
                    "stop_claim_retained":retained.as_ref().is_some_and(|value| value.dispatch_command.is_some()),
                    "stop_outcome_retained":retained.as_ref().is_some_and(|value| value.native_outcome.is_some()),
                    "owner_lease_valid":owner.lease_valid,"sdlc_acceptance":false})).unwrap()).await.unwrap();
                let retained = retained.expect("unknown stop did not retain an original intent");
                assert!(
                    retained.dispatch_command.is_some(),
                    "unclaimed failure cannot become a stop retry"
                );
                uncertain_readbacks += 1;
                // The production claimed-command path only observes; it cannot resend stop.
                runtime
                    .stop(&agent)
                    .await
                    .expect("original namespace exit was not confirmed")
            }
            Err(_) => panic!("non-reconcilable runtime stop failure"),
        };
        assert_eq!(stopped.status, AgentStatus::Stopped);
        assert!(repo.get_open_runtime_launch(*id).await.unwrap().is_none());
        let saved = repo
            .read_controller_stop(binding.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(saved.intent.launch_sha256, hash(binding));
        assert_eq!(saved.intent.pid, *pid);
        assert_eq!(saved.intent.operation_id, binding.id);
        let admitted = saved.dispatch_command.as_ref().unwrap();
        assert_eq!(admitted.epoch, 3);
        assert!(admitted.request == renewed.request);
        let outcome = saved.native_outcome.as_ref().unwrap();
        assert!(outcome["kind"] == "stop" || outcome["kind"] == "observe");
        assert_eq!(outcome["receipt"]["observation"], "namespace_exited");
        assert_eq!(
            repo.get_agent(*id).await.unwrap().status,
            AgentStatus::Stopped
        );
        let row = db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT binding,state,pid FROM runtime_launches WHERE id=$1",
                [binding.id.into()],
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            hash(&row.try_get::<Value>("", "binding").unwrap()),
            hash(binding)
        );
        assert_eq!(
            row.try_get::<String>("", "state").unwrap(),
            "gateway_exited"
        );
        assert_eq!(row.try_get::<i32>("", "pid").unwrap(), *pid);
        let row = db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT pid,desired_state,health_status FROM agent_runtime WHERE agent_id=$1",
                [(*id).into()],
            ))
            .await
            .unwrap()
            .unwrap();
        assert!(row.try_get::<Option<i32>>("", "pid").unwrap().is_none());
        assert_eq!(
            row.try_get::<String>("", "desired_state").unwrap(),
            "stopped"
        );
        assert_eq!(
            row.try_get::<String>("", "health_status").unwrap(),
            "gateway_exited"
        );
        // Exact replay is historical settlement, never another physical stop.
        let cursor = repo.session_event_cursor(context.session).await.unwrap();
        repo.settle_controller_stop(binding.id, outcome)
            .await
            .unwrap();
        assert_eq!(
            cursor,
            repo.session_event_cursor(context.session).await.unwrap()
        );
        let row = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT count(*) AS count FROM audit_log WHERE action='runtime.controller_stop.outcome' AND entity_id=$1",
            [binding.id.to_string().into()])).await.unwrap().unwrap();
        assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
        proofs.push(json!({"intent":saved.intent,"command":admitted,"outcome":outcome}));
    }
    session_state(repo, context, true).await;
    let row = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT count(*) AS count FROM audit_log WHERE action='runtime.namespace_exit.run_cancelled' AND entity_id=$1",
        [context.run.to_string().into()])).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
    save("custody-stops.json", &proofs).await;
    tokio::fs::write(
        "/evidence/custody-stop.json",
        serde_json::to_vec(&json!({
        "state":"passed","agents":2,"epoch":3,"actual_startup_worker":true,
        "current_owner_namespace_stop":true,"competing_logical_controller_denied":true,
        "original_launch_identity_retained":true,"immutable_stop_delivery":true,
        "single_outcome_audit":true,"original_dispatch_transcript_content_unchanged":true,
        "accepted_run_cancelled":true,"pending_approval_cancelled_without_grant":true,
        "terminal_events_once":true,
        "approval_target_unchanged":true,"resumed_execution":false,
        "uncertain_stop_readbacks":uncertain_readbacks,
        "sdlc_acceptance":false,"raw_receipts_persisted":false}))
        .unwrap(),
    )
    .await
    .unwrap();
}

async fn expired(repo: Arc<PostgresFleetRepository>, config: AppConfig, context: Context) {
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime = LocalRuntimeSupervisor::new(Arc::new(config.clone()), repo.clone(), events);
    assert!(!config.fleet.controller_recovery_enabled);
    for binding in &context.launches {
        let record = repo
            .current_controller_recovery(binding.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(record.state, "acknowledged");
        assert!(!record.lease_valid);
        assert!(
            runtime
                .recover_container_controller(binding.agent_id)
                .await
                .is_err()
        );
        assert!(
            runtime
                .heartbeat_container_controller(binding.agent_id)
                .await
                .is_err()
        );
        let after = repo
            .current_controller_recovery(binding.id)
            .await
            .unwrap()
            .unwrap();
        assert!(after.request == record.request);
        assert_eq!(after.lease_version, record.lease_version);
        assert_eq!(after.lease_expires_at, record.lease_expires_at);
    }
    state(&repo, &config, &context).await;
    tokio::fs::write(
        "/evidence/custody-expired.json",
        serde_json::to_vec(&json!({"state":"passed",
        "same_physical_start_cannot_take_over":true,"expired_db_lease_not_revived":true,
        "native_run_waiting_for_approval":true,"sdlc_acceptance":false}))
        .unwrap(),
    )
    .await
    .unwrap();
}

async fn freeze(repo: Arc<PostgresFleetRepository>, context: Context, epoch: i64) {
    let saved: Vec<Value> = read(&format!("custody-epoch{epoch}.json")).await;
    assert_eq!(saved.len(), context.launches.len());
    let mut proofs = Vec::new();
    for (binding, previous) in context.launches.iter().zip(saved) {
        let record = repo
            .current_controller_recovery(binding.id)
            .await
            .unwrap()
            .unwrap();
        let delivery = repo
            .read_controller_recovery_delivery(record.request.id)
            .await
            .unwrap()
            .unwrap();
        assert!(record.state == "acknowledged" && record.lease_valid && record.epoch == epoch);
        assert_eq!(json!(delivery.command), previous["initial"]);
        assert!(record.lease_version >= previous["current"]["lease_version"].as_i64().unwrap());
        let digest = hash(delivery.native_receipt.as_ref().unwrap());
        assert_eq!(json!(digest), previous["receipt_sha256"]);
        assert_eq!(
            record.native_receipt_sha256.as_deref(),
            Some(digest.as_str())
        );
        proofs.push(
            json!({"initial":delivery.command,"current":command(&record),
            "receipt_sha256":digest}),
        );
    }
    // No supervisor exists in this process: the preceding worker has exited.
    save(&format!("custody-epoch{epoch}-frozen.json"), &proofs).await;
}

#[tokio::test]
#[ignore = "requires owned Compose phases, actual controller restarts and real waiting Hermes run"]
async fn real_controller_startup_maintains_custody_without_granting_execution() {
    shared::telemetry::init_tracing("fleet-custody-qa");
    assert_eq!(
        std::env::var("FLEET_CONTAINER_SUPERVISOR_TEST").as_deref(),
        Ok("1")
    );
    let phase = std::env::var("FLEET_CONTAINER_RECOVERY_PHASE").unwrap();
    let database = DatabaseConfig {
        url: std::env::var("FLEET_TEST_DATABASE_URL").unwrap(),
        max_connections: 10,
        min_connections: 1,
        connect_timeout_seconds: 10,
        idle_timeout_seconds: 60,
    };
    infra::run_migrations(database.clone()).await.unwrap();
    let db = infra::connect_database(database.clone()).await.unwrap();
    let repo = Arc::new(PostgresFleetRepository::new(db));
    if phase == "prepare" {
        let proof: Proof =
            serde_json::from_slice(&tokio::fs::read("/qa/controller-proof.json").await.unwrap())
                .unwrap();
        let owner_db = infra::connect_database(database).await.unwrap();
        prepare(repo, &owner_db, &proof).await;
    } else {
        let config: AppConfig = read("custody-config.json").await;
        let context: Context = read("custody-context.json").await;
        if phase == "freeze-1" || phase == "freeze-2" {
            freeze(repo, context, if phase == "freeze-1" { 1 } else { 2 }).await;
            return;
        }
        match phase.as_str() {
            "recover-1" => recover(repo, config, context, 1).await,
            "recover-2" => recover(repo, config, context, 2).await,
            "stop" => recover(repo, config, context, 3).await,
            "expired" => expired(repo, config, context).await,
            _ => panic!("unknown own custody phase"),
        }
    }
}
