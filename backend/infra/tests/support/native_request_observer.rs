//! Queued actual Fleet/native acceptance, not a component fixture or admission proof.
use super::*;
use hmac::{Hmac, Mac};

fn sorted(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut names: Vec<_> = object.keys().collect();
            names.sort();
            Value::Object(
                names
                    .into_iter()
                    .map(|name| (name.clone(), sorted(&object[name])))
                    .collect(),
            )
        }
        Value::Array(values) => Value::Array(values.iter().map(sorted).collect()),
        value => value.clone(),
    }
}

fn digest(token: &str, value: &Value) -> String {
    let mut domain = Hmac::<Sha256>::new_from_slice(token.as_bytes()).unwrap();
    domain.update(b"fleet-request-observation/v1");
    let key = domain.finalize().into_bytes();
    let mut mac = Hmac::<Sha256>::new_from_slice(&key).unwrap();
    mac.update(&serde_json::to_vec(&sorted(value)).unwrap());
    hex::encode(mac.finalize().into_bytes())
}

const PLUGIN_FILES: [&str; 4] = ["__init__.py", "plugin.py", "store.py", "plugin.yaml"];

async fn plugin_bytes(agent: &Agent) -> Vec<Option<Vec<u8>>> {
    let mut bytes = Vec::new();
    for name in PLUGIN_FILES {
        let path = Path::new(&agent.paths.config)
            .join("plugins/fleet-hermes-request-observer")
            .join(name);
        bytes.push(match tokio::fs::read(path).await {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => panic!("own observer file read failed: {error}"),
        });
    }
    bytes
}

async fn failed_activation(
    repo: &PostgresFleetRepository,
    config: &AppConfig,
    agent: &Agent,
    owner: Uuid,
    request: UpdateAgentConfigRequest,
) {
    let before = plugin_bytes(agent).await;
    let previous = repo
        .get_effective_config_revision(agent.id)
        .await
        .unwrap()
        .unwrap();
    let previous_snapshot = serde_json::to_vec(&previous.snapshot).unwrap();
    let launch = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap();
    let request = FilesystemProvisioner
        .prepare_request_observer_configuration(config, request)
        .await
        .unwrap();
    let draft = repo
        .create_config_revision(agent.id, request, owner)
        .await
        .unwrap();
    repo.validate_config_revision(agent.id, draft.revision, draft.snapshot.input_errors())
        .await
        .unwrap();
    tokio::fs::write(
        Path::new(&agent.paths.config).join(".qa-fail-next-launch"),
        b"owned startup fault",
    )
    .await
    .unwrap();
    repo.request_config_activation(agent.id, draft.revision, owner)
        .await
        .unwrap();
    timeout(Duration::from_secs(90), async {
        loop {
            let current = repo
                .get_config_revision(agent.id, draft.revision)
                .await
                .unwrap();
            assert_ne!(
                current.state, "active",
                "startup fault cannot record success"
            );
            if current.state == "failed" && !current.draining {
                break;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("signed rollback did not settle under its existing deadline");
    assert_eq!(plugin_bytes(agent).await, before);
    assert_eq!(
        repo.get_effective_config_revision(agent.id)
            .await
            .unwrap()
            .unwrap()
            .revision,
        previous.revision
    );
    assert_eq!(
        serde_json::to_vec(
            &repo
                .get_config_revision(agent.id, previous.revision)
                .await
                .unwrap()
                .snapshot
        )
        .unwrap(),
        previous_snapshot
    );
    let restored = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap();
    assert_ne!(restored.binding.id, launch.binding.id);
    assert_eq!(
        restored.binding.configuration_revision,
        Some(previous.revision)
    );
    FilesystemProvisioner
        .verify_effective_configuration(agent, config, &previous)
        .await
        .unwrap();
}

struct ApiServer(tokio::task::JoinHandle<()>);
impl Drop for ApiServer {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn hold_ack(session: Uuid) {
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db.execute_unprepared(&format!(
        "CREATE FUNCTION qa_hold_observer_ack() RETURNS trigger AS $$ BEGIN
        IF NEW.session_id='{session}'::uuid AND OLD.state='submitted' AND NEW.state='accepted' THEN
            RAISE EXCEPTION 'owned observer QA accepted-ACK commit fault';
        END IF; RETURN NEW; END $$ LANGUAGE plpgsql;
        CREATE TRIGGER qa_hold_observer_ack BEFORE UPDATE ON hermes_dispatch_journal
        FOR EACH ROW EXECUTE FUNCTION qa_hold_observer_ack()"
    ))
    .await
    .unwrap();
}

async fn release_prepared(
    repo: &PostgresFleetRepository,
    message: Uuid,
    model: &Model,
    prompt: &str,
) {
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let original = timeout(Duration::from_secs(30), async {
        loop {
            if let Some(intent) = repo.get_hermes_dispatch_intent(message).await.unwrap() {
                let row = db
                    .query_one(Statement::from_sql_and_values(
                        DatabaseBackend::Postgres,
                        "SELECT state FROM message_dispatch_outbox WHERE message_id=$1",
                        [message.into()],
                    ))
                    .await
                    .unwrap()
                    .unwrap();
                if row.try_get::<String>("", "state").unwrap() == "uncertain" {
                    assert_eq!(intent.state, "prepared");
                    assert!(!intent.submission_attempted);
                    assert!(intent.capabilities.get("fleet_request_observer").is_some());
                    assert!(intent.capabilities.get("fleet_recovery").is_some());
                    break intent;
                }
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("owned pre-submission fault did not retain original observer facts");
    assert!(!model.wire_requests.lock().await.contains_key(prompt));
    db.execute_unprepared("DROP TRIGGER qa_hold_prepared_claim ON hermes_dispatch_journal; DROP FUNCTION qa_hold_prepared_claim()").await.unwrap();
    let accepted = timeout(Duration::from_secs(60), async {
        loop {
            let current = repo
                .get_hermes_dispatch_intent(message)
                .await
                .unwrap()
                .unwrap();
            if current.state == "accepted" {
                break current;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("prepared dispatch did not retain observer incarnation on submission");
    assert_eq!(accepted.capabilities, original.capabilities);
    assert_eq!(accepted.request_body, original.request_body);
    assert_eq!(accepted.recovery_deadline, original.recovery_deadline);
}

async fn release_ack(
    repo: &PostgresFleetRepository,
    runtime: &LocalRuntimeSupervisor,
    agent: &Agent,
    message: Uuid,
    model: &Model,
    prompt: &str,
) {
    let original = timeout(Duration::from_secs(30), async {
        loop {
            if let Some(intent) = repo.get_hermes_dispatch_intent(message).await.unwrap() {
                if intent.state == "submitted"
                    && intent.run.runtime_run_id.is_none()
                    && model.wire_requests.lock().await.contains_key(prompt)
                {
                    break intent;
                }
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("actual inference did not precede the held ACK");
    assert!(
        runtime
            .read_request_observation(agent, &original.run)
            .await
            .is_err()
    );
    assert_eq!(model.wire_requests.lock().await[prompt].len(), 1);
    assert!(
        original
            .capabilities
            .get("fleet_request_observer")
            .is_some()
    );
    assert!(original.capabilities.get("fleet_recovery").is_some());
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db.execute_unprepared("DROP TRIGGER qa_hold_observer_ack ON hermes_dispatch_journal; DROP FUNCTION qa_hold_observer_ack()").await.unwrap();
    let accepted = timeout(Duration::from_secs(60), async {
        loop {
            let current = repo
                .get_hermes_dispatch_intent(message)
                .await
                .unwrap()
                .unwrap();
            if current.state == "accepted" {
                break current;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("original non-dispatch recovery did not restore ACK");
    assert_eq!(accepted.capabilities, original.capabilities);
    assert_eq!(accepted.request_body, original.request_body);
    assert_eq!(accepted.recovery_deadline, original.recovery_deadline);
    assert_eq!(model.wire_requests.lock().await[prompt].len(), 1);
}

async fn api_owner_readback(
    repo: Arc<PostgresFleetRepository>,
    config: Arc<AppConfig>,
    runtime: &LocalRuntimeSupervisor,
    owner: Uuid,
    run: &domain::SessionAgentRun,
    receipt: &Value,
    fault: (&Path, &Model),
) {
    use axum::http::StatusCode;
    let (events, _) = tokio::sync::broadcast::channel(16);
    let (restart_tx, _) = tokio::sync::mpsc::channel(1);
    let ctx = Arc::new(app::AppContext::new(
        config,
        repo.clone(),
        Arc::new(FilesystemProvisioner),
        Arc::new(runtime.clone()),
        events,
        restart_tx,
    ));
    let owner_token = ctx
        .auth
        .issue_tokens(&repo.find_user_by_id(owner).await.unwrap().unwrap())
        .unwrap()
        .response
        .access_token;
    let stranger = Uuid::new_v4();
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO users(id,email,username,display_name,password_hash,is_system_admin,system_role)
         VALUES($1,$2,$3,'Observer stranger','disabled',false,'user')",
        [stranger.into(),format!("{stranger}@example.test").into(),stranger.to_string().into()])).await.unwrap();
    let foreign_token = ctx
        .auth
        .issue_tokens(&repo.find_user_by_id(stranger).await.unwrap().unwrap())
        .unwrap()
        .response
        .access_token;
    let router = Router::new()
        .route(
            "/api/v1/sessions/{session_id}/runs/{run_id}/request-observation",
            get(api::routes::sessions::read_request_observation),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            ctx.clone(),
            api::middleware::require_auth,
        ))
        .with_state(ctx);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!(
        "http://{}/api/v1/sessions/{}/runs/{}/request-observation",
        listener.local_addr().unwrap(),
        run.session_id,
        run.id
    );
    let _server = ApiServer(tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    }));
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(20))
        .build()
        .unwrap();
    let response = client
        .get(&url)
        .bearer_auth(&owner_token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(&response.json::<Value>().await.unwrap(), receipt);
    let response = client
        .get(&url)
        .bearer_auth(&foreign_token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let (fault_root, model) = fault;
    let plugin = Path::new(&repo.get_agent(run.agent_id).await.unwrap().paths.config)
        .join("plugins/fleet-hermes-request-observer/store.py");
    let original_bytes = tokio::fs::read(&plugin).await.unwrap();
    let original_intent = repo
        .get_hermes_dispatch_intent_for_run(run.id)
        .await
        .unwrap()
        .unwrap();
    let original_launch = serde_json::to_value(
        repo.get_open_runtime_launch(run.agent_id)
            .await
            .unwrap()
            .unwrap()
            .binding,
    )
    .unwrap();
    let original_calls = model.wire_requests.lock().await.clone();
    tokio::fs::write(
        fault_root.join("armed-run"),
        run.runtime_run_id.as_deref().unwrap(),
    )
    .await
    .unwrap();
    let pending = tokio::spawn({
        let client = client.clone();
        let url = url.clone();
        let token = owner_token.clone();
        async move { client.get(url).bearer_auth(token).send().await }
    });
    let native_caps = timeout(Duration::from_secs(15), async {
        loop {
            if tokio::fs::try_exists(fault_root.join("ready.json"))
                .await
                .unwrap()
            {
                break serde_json::from_slice::<Value>(
                    &tokio::fs::read(fault_root.join("ready.json"))
                        .await
                        .unwrap(),
                )
                .unwrap();
            }
            sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("real final capabilities GET did not reach its owned pause");
    assert!(!pending.is_finished());
    assert_eq!(
        native_caps,
        original_intent.capabilities["fleet_request_observer"]
    );
    assert_eq!(
        native_caps["incarnation"],
        receipt["observation"]["incarnation"]
    );
    tokio::fs::write(&plugin, b"owned final-GET source mutation")
        .await
        .unwrap();
    tokio::fs::write(fault_root.join("release"), b"owned release")
        .await
        .unwrap();
    let result = pending.await;
    let returned = tokio::fs::read(fault_root.join("returned")).await;
    tokio::fs::write(&plugin, &original_bytes).await.unwrap();
    for name in ["armed-run", "ready.json", "release", "returned"] {
        tokio::fs::remove_file(fault_root.join(name)).await.unwrap();
    }
    assert_eq!(returned.unwrap(), b"genuine-200");
    let response = result.unwrap().unwrap();
    assert_eq!(
        response.status(),
        StatusCode::SERVICE_UNAVAILABLE,
        "old code returned 200 after the genuine final GET despite changed source"
    );
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(*model.wire_requests.lock().await, original_calls);
    let current_intent = repo
        .get_hermes_dispatch_intent_for_run(run.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(current_intent.capabilities, original_intent.capabilities);
    assert_eq!(current_intent.request_body, original_intent.request_body);
    assert_eq!(current_intent.request_hash, original_intent.request_hash);
    assert_eq!(
        current_intent.recovery_deadline,
        original_intent.recovery_deadline
    );
    assert_eq!(
        current_intent.run.runtime_run_id,
        original_intent.run.runtime_run_id
    );
    assert_eq!(
        current_intent.run.runtime_session_id,
        original_intent.run.runtime_session_id
    );
    assert_eq!(
        serde_json::to_value(
            repo.get_open_runtime_launch(run.agent_id)
                .await
                .unwrap()
                .unwrap()
                .binding
        )
        .unwrap(),
        original_launch
    );
    let response = client
        .get(&url)
        .bearer_auth(&owner_token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(&response.json::<Value>().await.unwrap(), receipt);
    assert_eq!(*model.wire_requests.lock().await, original_calls);
    // Only the owned QA database gains this negative fixture; no Tracker request is needed.
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO task_chat_bindings(session_id,tracker_instance_id,project_id,task_id,root_task_id,agent_id,owner_subject,idempotency_key)
         VALUES($1,'observer-qa',$2,$3,$3,$4,$5,$6)",
        [run.session_id.into(),Uuid::new_v4().into(),Uuid::new_v4().into(),run.agent_id.into(),owner.to_string().into(),Uuid::new_v4().to_string().into()])).await.unwrap();
    let response = client
        .get(&url)
        .bearer_auth(&owner_token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(response.headers()["cache-control"], "no-store");
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "DELETE FROM task_chat_bindings WHERE session_id=$1 AND tracker_instance_id='observer-qa'",
        [run.session_id.into()],
    ))
    .await
    .unwrap();
}

async fn combined_unknown_controls(
    repo: &PostgresFleetRepository,
    runtime: &LocalRuntimeSupervisor,
    agent: &Agent,
    owner: Uuid,
    model: &Model,
    fault_root: &Path,
) {
    let session = repo
        .create_session(
            CreateSessionRequest {
                primary_agent_id: Some(agent.id),
                agent_id: None,
                title: "Observer/control coexistence".into(),
                task_key: None,
                leader_agent_id: None,
                parent_session_id: None,
                namespace_id: None,
                idempotency_key: Some(Uuid::new_v4().to_string()),
            },
            owner,
        )
        .await
        .unwrap();
    let prompt = format!("managed-control-prompt-observer-{}", Uuid::new_v4());
    let message = repo
        .create_session_message(
            session.id,
            CreateSessionMessageRequest {
                body: prompt.clone(),
                author_agent_id: None,
                message_kind: Some(MessageKind::UserPrompt),
                runtime_message_id: None,
                idempotency_key: Some(Uuid::new_v4().to_string()),
            },
            owner,
        )
        .await
        .unwrap();
    let run = timeout(Duration::from_secs(90), async {
        loop {
            let intent = repo.get_hermes_dispatch_intent(message.id).await.unwrap();
            if let Some(intent) = intent {
                if intent.state == "accepted"
                    && intent.run.state == SessionRunState::Running
                    && intent.run.runtime_session_id.is_some()
                    && model.wire_requests.lock().await.contains_key(&prompt)
                {
                    break intent.run;
                }
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("actual control model barrier and original run were not pinned");
    let original = repo
        .get_hermes_dispatch_intent(message.id)
        .await
        .unwrap()
        .unwrap();
    let observation = runtime.read_request_observation(agent, &run).await.unwrap();
    tokio::fs::write(fault_root.join("hold-lookup"), b"owned control lookup hold")
        .await
        .unwrap();
    let actor = domain::RuntimeControlActor {
        user_id: owner,
        idempotency_key: Uuid::new_v4().to_string(),
    };
    let request = domain::SteerSessionRunRequest {
        input: "Stay in this owned synthetic task.".into(),
    };
    let result = runtime
        .steer_run(agent, &run, request.clone(), actor.clone())
        .await
        .unwrap();
    assert!(!result.accepted);
    let command = result.command.unwrap();
    let context = repo
        .get_runtime_control_outcome(command.id)
        .await
        .unwrap()
        .unwrap()
        .context;
    assert_eq!(
        runtime.read_request_observation(agent, &run).await.unwrap(),
        observation
    );
    assert_eq!(model.wire_requests.lock().await[&prompt].len(), 1);
    tokio::fs::remove_file(fault_root.join("hold-lookup"))
        .await
        .unwrap();
    await_native_control_ack(repo, session.id, command.id).await;
    let replay = runtime
        .steer_run(agent, &run, request, actor)
        .await
        .unwrap();
    assert!(replay.accepted);
    assert_eq!(replay.command.unwrap().id, command.id);
    assert_eq!(
        repo.get_runtime_control_outcome(command.id)
            .await
            .unwrap()
            .unwrap()
            .context,
        context
    );
    assert_eq!(
        runtime.read_request_observation(agent, &run).await.unwrap(),
        observation
    );
    let stop_actor = domain::RuntimeControlActor {
        user_id: owner,
        idempotency_key: Uuid::new_v4().to_string(),
    };
    tokio::fs::write(fault_root.join("hold-lookup"), b"owned stop lookup hold")
        .await
        .unwrap();
    let stopped = runtime
        .stop_run(agent, &run, stop_actor.clone())
        .await
        .unwrap();
    assert!(!stopped.accepted);
    let stop = stopped.command.unwrap();
    tokio::fs::remove_file(fault_root.join("hold-lookup"))
        .await
        .unwrap();
    await_native_control_ack(repo, session.id, stop.id).await;
    assert_eq!(
        runtime
            .stop_run(agent, &run, stop_actor)
            .await
            .unwrap()
            .command
            .unwrap()
            .id,
        stop.id
    );
    model.control_release.notify_one();
    timeout(Duration::from_secs(90), async {
        loop {
            let current = repo.get_session_agent_run(run.id).await.unwrap();
            assert_ne!(current.state, SessionRunState::Completed);
            if matches!(
                current.state,
                SessionRunState::Failed | SessionRunState::Cancelled
            ) {
                break;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("controlled native run did not retain interruption after stop");
    let current = repo
        .get_hermes_dispatch_intent(message.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(current.capabilities, original.capabilities);
    assert_eq!(current.request_body, original.request_body);
    assert!(current.capabilities.get("fleet_request_observer").is_some());
    assert!(current.capabilities.get("fleet_recovery").is_some());
    let records = tokio::fs::read_to_string(fault_root.join("control-events.jsonl"))
        .await
        .unwrap();
    let records: Vec<Value> = records
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    for receipt in [command, stop] {
        let key = repo
            .get_runtime_control_outcome(receipt.id)
            .await
            .unwrap()
            .unwrap()
            .context["command_id"]
            .as_str()
            .unwrap()
            .to_owned();
        assert_eq!(
            records
                .iter()
                .filter(|event| event["kind"] == "post" && event["key"] == key)
                .count(),
            1
        );
        assert!(records.iter().any(|event| event["kind"] == "lookup"
            && event["key"] == key
            && event["held"] == false));
    }
    assert_eq!(model.wire_requests.lock().await[&prompt].len(), 1);
}

#[tokio::test]
#[ignore = "requires owned Compose, published Base observer Git objects and both committed native recovery/control plugins"]
async fn managed_native_observer_activation_reads_two_original_runs_with_existing_extensions() {
    use futures_util::FutureExt;
    let db = native_database().await;
    let owner = Uuid::new_v4();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO users(id,email,username,display_name,password_hash,is_system_admin,system_role)
         VALUES($1,$2,$3,'Native observer owner','disabled',false,'user')",
        [owner.into(),format!("{owner}@example.test").into(),owner.to_string().into()])).await.unwrap();
    let repo = Arc::new(PostgresFleetRepository::new(db));
    repo.ensure_runtime_templates().await.unwrap();
    let model = Arc::new(Model::default());
    let socket = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = socket.local_addr().unwrap().port();
    let app = Router::new().route("/v1/chat/completions", post(inference))
        .route("/v1/models", get(|| async { Json(json!({"object":"list","data":[{"id":"fleet-managed-local-model","object":"model"}]})) }))
        .with_state(model.clone());
    let server = tokio::spawn(async move {
        axum::serve(socket, app).await.unwrap();
    });
    let mut config = AppConfig::default();
    config.fleet.agents_root = "/tmp/fleet-native-supervisor/observer-agents".into();
    config.fleet.hermes_source = "/opt/hermes".into();
    config.fleet.hermes_command = "/opt/fleet-hermes/bin/observer-hermes".into();
    config.fleet.runtime_token_secret = format!("owned-native-observer-{}", Uuid::new_v4());
    config.fleet.base_package_checkout = std::env::var("FLEET_OBSERVER_BASE_CHECKOUT")
        .expect("read-only Base Git object cache with observer commit is required");
    config.fleet.hermes_recovery_extension_enabled = true;
    config.fleet.hermes_control_outcome_enabled = true;
    config.fleet.agent_port_base = 29700;
    config.fleet.agent_port_stride = 5;
    config.fleet.project_workflow_url = None;
    let config = native_configuration(config);
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime = LocalRuntimeSupervisor::new(config.clone(), repo.clone(), events);
    let mut agents = Vec::new();
    let outcome = std::panic::AssertUnwindSafe(async {
        let mut observations = Vec::new();
        for index in 0..2 {
            let agent = create_agent(&repo, &config, &format!("Observer {index}")).await;
            agents.push(agent.clone());
            // Existing extensions belong to the native QA fixture. Observer is installed only
            // by the production revision plan, never by copying it alongside the runtime.
            for (plugin, source) in [
                ("fleet-hermes-recovery", "/qa/recovery-plugin"),
                ("fleet-hermes-controls", "/qa/control-plugin"),
            ] {
                let destination = Path::new(&agent.paths.config).join("plugins").join(plugin);
                tokio::fs::create_dir_all(&destination).await.unwrap();
                for name in ["__init__.py", "plugin.py", "store.py", "plugin.yaml"] {
                    tokio::fs::copy(Path::new(source).join(name), destination.join(name))
                        .await
                        .unwrap();
                }
            }
            let mut desired = configuration(port, &format!("FLEET_OBSERVER_SOUL_{index}"));
            desired.config_json["plugins"] =
                json!({"enabled":["fleet-hermes-recovery","fleet-hermes-controls"]});
            let fault_root = Path::new(&agent.paths.workspace).join("observer-control-fault");
            tokio::fs::create_dir_all(&fault_root).await.unwrap();
            let fault_plugin = Path::new(&agent.paths.config).join("plugins/fleet-native-discard-control-ack");
            tokio::fs::create_dir_all(&fault_plugin).await.unwrap();
            tokio::fs::write(fault_plugin.join("__init__.py"), include_str!("../../../../scripts/native_supervisor_live/control_fault_plugin.py")).await.unwrap();
            tokio::fs::write(fault_plugin.join("plugin.yaml"), b"name: fleet-native-discard-control-ack\nversion: 1.0.0\nkind: platform\nplatforms:\n  - api_server\n").await.unwrap();
            desired.config_json["plugins"]["enabled"].as_array_mut().unwrap().push(json!("fleet-native-discard-control-ack"));
            desired.env_json["FLEET_NATIVE_SUPERVISOR_TEST"] = json!("1");
            desired.env_json["FLEET_NATIVE_CONTROL_FAULT_ROOT"] = json!(fault_root);
            let observer_fault_root = Path::new(&agent.paths.workspace).join("observer-read-fault");
            tokio::fs::create_dir_all(&observer_fault_root).await.unwrap();
            let observer_fault_plugin = Path::new(&agent.paths.config).join("plugins/fleet-native-observer-read-fault");
            tokio::fs::create_dir_all(&observer_fault_plugin).await.unwrap();
            tokio::fs::write(observer_fault_plugin.join("__init__.py"), include_str!("../../../../scripts/native_supervisor_live/observer_fault_plugin.py")).await.unwrap();
            tokio::fs::write(observer_fault_plugin.join("plugin.yaml"), b"name: fleet-native-observer-read-fault\nversion: 1.0.0\nkind: platform\nplatforms:\n  - api_server\n").await.unwrap();
            desired.config_json["plugins"]["enabled"].as_array_mut().unwrap().push(json!("fleet-native-observer-read-fault"));
            desired.env_json["FLEET_NATIVE_OBSERVER_FAULT_ROOT"] = json!(observer_fault_root);
            let mut unmanaged = desired.clone();
            unmanaged.config_json["plugins"]["enabled"]
                .as_array_mut()
                .unwrap()
                .push(json!("fleet-hermes-request-observer"));
            assert!(
                repo.create_config_revision(agent.id, unmanaged, owner)
                    .await
                    .is_err()
            );
            activate(&repo, agent.id, owner, desired.clone()).await;
            let legacy = repo
                .get_effective_config_revision(agent.id)
                .await
                .unwrap()
                .unwrap();
            let legacy_bytes = serde_json::to_vec(&legacy.snapshot).unwrap();
            assert_eq!(legacy.snapshot.renderer_version, 2);
            let started = runtime.start(&agent).await.unwrap();
            if started.status != AgentStatus::Running {
                let token = infra::agent_runtime_token(&config, agent.id).unwrap();
                for log in repo.list_logs(Some(agent.id), 100).await.unwrap() {
                    let safe = log.message.replace(&token, "redacted")
                        .replace(&config.fleet.runtime_token_secret, "redacted")
                        .replace("owned-local-model-fixture", "redacted");
                    eprintln!("Native observer startup {}: {safe}", log.stream);
                }
            }
            assert_eq!(started.status, AgentStatus::Running);
            assert!(plugin_bytes(&agent).await.iter().all(Option::is_none));
            desired.config_json["fleet_request_observer"] = json!({"enabled":true});
            if index == 0 {
                failed_activation(&repo, &config, &agent, owner, desired.clone()).await;
            }
            let prepared = FilesystemProvisioner
                .prepare_request_observer_configuration(&config, desired)
                .await
                .unwrap();
            let draft = repo
                .create_config_revision(agent.id, prepared.clone(), owner)
                .await
                .unwrap();
            assert_eq!(draft.snapshot.renderer_version, 3);
            let mut desired_removal = prepared.clone();
            desired_removal.config_json["fleet_request_observer"]["enabled"] = json!(false);
            let desired_removal = FilesystemProvisioner
                .prepare_request_observer_configuration(&config, desired_removal)
                .await
                .unwrap();
            let removal_draft = repo
                .create_config_revision(agent.id, desired_removal, owner)
                .await
                .unwrap();
            assert_eq!(removal_draft.snapshot.renderer_version, 3);
            assert_eq!(
                removal_draft.snapshot.config.config_json["fleet_request_observer"]["enabled"],
                false
            );
            assert_eq!(
                repo.get_effective_config_revision(agent.id)
                    .await
                    .unwrap()
                    .unwrap()
                    .revision,
                legacy.revision
            );
            assert!(plugin_bytes(&agent).await.iter().all(Option::is_none));
            assert!(
                repo.create_config_revision(
                    agent.id,
                    configuration(port, "implicit-removal"),
                    owner
                )
                .await
                .is_err()
            );
            activate(&repo, agent.id, owner, prepared).await;
            assert_eq!(
                serde_json::to_vec(
                    &repo
                        .get_config_revision(agent.id, legacy.revision)
                        .await
                        .unwrap()
                        .snapshot
                )
                .unwrap(),
                legacy_bytes
            );
            assert_eq!(
                repo.get_effective_config_revision(agent.id)
                    .await
                    .unwrap()
                    .unwrap()
                    .snapshot
                    .renderer_version,
                3
            );
            assert_eq!(
                runtime.start(&agent).await.unwrap().status,
                AgentStatus::Running
            );
            let session = repo
                .create_session(
                    CreateSessionRequest {
                        primary_agent_id: Some(agent.id),
                        agent_id: None,
                        title: "Observer native chat".into(),
                        task_key: None,
                        leader_agent_id: None,
                        parent_session_id: None,
                        namespace_id: None,
                        idempotency_key: Some(Uuid::new_v4().to_string()),
                    },
                    owner,
                )
                .await
                .unwrap();
            let prompt = format!("managed-observer-prompt-{}-{index}", Uuid::new_v4());
            if index == 0 {
                native_prepared_launch::hold_submission(session.id).await;
            } else {
                hold_ack(session.id).await;
            }
            let message = repo
                .create_session_message(
                    session.id,
                    CreateSessionMessageRequest {
                        idempotency_key: Some(Uuid::new_v4().to_string()),
                        body: prompt.clone(),
                        author_agent_id: None,
                        message_kind: Some(MessageKind::UserPrompt),
                        runtime_message_id: None,
                    },
                    owner,
                )
                .await
                .unwrap();
            if index == 0 {
                release_prepared(&repo, message.id, &model, &prompt).await;
            } else {
                release_ack(&repo, &runtime, &agent, message.id, &model, &prompt).await;
            }
            let run = terminal(&repo, session.id).await;
            let before = repo
                .get_hermes_dispatch_intent_for_run(run.id)
                .await
                .unwrap()
                .unwrap();
            let receipt = runtime
                .read_request_observation(&agent, &run)
                .await
                .unwrap();
            assert_eq!(
                runtime
                    .read_request_observation(&agent, &run)
                    .await
                    .unwrap(),
                receipt
            );
            assert_eq!(receipt["complete"], false);
            assert_eq!(receipt["runtime_ready"], false);
            assert_eq!(
                receipt["blockers"],
                json!([
                    "runtime_skill_inventory_not_verified",
                    "workflow_assignment_protocol_not_verified"
                ])
            );
            let native = &receipt["observation"];
            let token = infra::agent_runtime_token(&config, agent.id).unwrap();
            let wire = model.wire_requests.lock().await;
            let calls = wire.get(&prompt).unwrap();
            assert_eq!(calls.len(), 1, "observer GET must not start inference");
            let body = &calls[0];
            let system = Value::Array(
                body["messages"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|message| message["role"] == "system")
                    .cloned()
                    .collect(),
            );
            assert_eq!(native["system_hmac_sha256"], digest(&token, &system));
            assert_eq!(
                native["tools_hmac_sha256"],
                digest(&token, body.get("tools").unwrap_or(&json!([])))
            );
            assert_eq!(native["model_hmac_sha256"], digest(&token, &body["model"]));
            assert_eq!(
                native["home_hmac_sha256"],
                digest(&token, &json!(agent.paths.config))
            );
            assert_eq!(
                native["cwd_hmac_sha256"],
                digest(&token, &json!(agent.paths.workspace))
            );
            drop(wire);
            let after = repo
                .get_hermes_dispatch_intent_for_run(run.id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(before.capabilities, after.capabilities);
            assert_eq!(before.request_body, after.request_body);
            assert!(before.capabilities.get("fleet_request_observer").is_some());
            assert!(before.capabilities.get("fleet_recovery").is_some());
            let plugin = Path::new(&agent.paths.config)
                .join("plugins/fleet-hermes-request-observer/store.py");
            let original_plugin = tokio::fs::read(&plugin).await.unwrap();
            tokio::fs::write(&plugin, b"owned tampered-source negative")
                .await
                .unwrap();
            assert!(
                runtime
                    .read_request_observation(&agent, &run)
                    .await
                    .is_err()
            );
            tokio::fs::write(&plugin, original_plugin).await.unwrap();
            assert_eq!(
                runtime
                    .read_request_observation(&agent, &run)
                    .await
                    .unwrap(),
                receipt
            );
            assert!(
                repo.create_config_revision(
                    agent.id,
                    configuration(port, "implicit-effective-removal"),
                    owner
                )
                .await
                .is_err()
            );
            api_owner_readback(
                repo.clone(),
                config.clone(),
                &runtime,
                owner,
                &run,
                &receipt,
                (&observer_fault_root, model.as_ref()),
            )
            .await;
            if index == 0 { combined_unknown_controls(&repo, &runtime, &agent, owner, &model, &fault_root).await; }
            observations.push((run, receipt));
        }
        assert!(
            runtime
                .read_request_observation(&agents[1], &observations[0].0)
                .await
                .is_err()
        );
        assert_ne!(
            observations[0].1["observation"]["incarnation"],
            observations[1].1["observation"]["incarnation"]
        );
        assert_eq!(
            runtime.restart(&agents[0]).await.unwrap().status,
            AgentStatus::Running
        );
        assert!(
            runtime
                .read_request_observation(&agents[0], &observations[0].0)
                .await
                .is_err()
        );
        let mut removal = configuration(port, "FLEET_OBSERVER_REMOVED");
        removal.config_json["plugins"] =
            json!({"enabled":["fleet-hermes-recovery","fleet-hermes-controls"]});
        removal.config_json["fleet_request_observer"] = json!({"enabled":false});
        failed_activation(&repo, &config, &agents[1], owner, removal.clone()).await;
        let token = infra::agent_runtime_token(&config, agents[1].id).unwrap();
        let live: Value = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap()
            .get(format!(
                "http://127.0.0.1:{}/fleet/v1/request-observations/capabilities",
                agents[1].api_port.unwrap()
            ))
            .bearer_auth(token)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_ne!(
            live["incarnation"],
            observations[1].1["observation"]["incarnation"]
        );
        assert!(
            runtime
                .read_request_observation(&agents[1], &observations[1].0)
                .await
                .is_err()
        );
        let removal = FilesystemProvisioner
            .prepare_request_observer_configuration(&config, removal)
            .await
            .unwrap();
        activate(&repo, agents[1].id, owner, removal).await;
        for name in ["__init__.py", "plugin.py", "store.py", "plugin.yaml"] {
            assert!(
                !tokio::fs::try_exists(
                    Path::new(&agents[1].paths.config)
                        .join("plugins/fleet-hermes-request-observer")
                        .join(name)
                )
                .await
                .unwrap()
            );
        }
        assert!(
            runtime
                .read_request_observation(&agents[1], &observations[1].0)
                .await
                .is_err()
        );
        assert_eq!(
            model
                .wire_requests
                .lock()
                .await
                .values()
                .map(Vec::len)
                .sum::<usize>(),
                3
        );
    })
    .catch_unwind()
    .await;
    let mut clean = true;
    for agent in &agents {
        clean &= runtime
            .stop(agent)
            .await
            .is_ok_and(|result| result.status == AgentStatus::Stopped);
    }
    server.abort();
    let _ = server.await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
    assert!(
        clean,
        "own Compose cleanup remains mandatory after failed native stop"
    );
}
