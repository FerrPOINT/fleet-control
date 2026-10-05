use super::*;

const TEST: &str = "native_approvals::native_approval_recovery::managed_native_waiting_approval_recovers_across_fleet_processes";
const ROOT: &str = "/tmp/fleet-native-supervisor/approval-fault";

fn config(secret: String) -> Arc<AppConfig> {
    let mut config = AppConfig::default();
    config.fleet.agents_root = "/tmp/fleet-native-supervisor/approval-recovery-agents".into();
    config.fleet.hermes_source = "/opt/hermes".into();
    config.fleet.hermes_command = "/opt/fleet-hermes/bin/hermes".into();
    config.fleet.runtime_token_secret = secret;
    config.fleet.agent_port_base = 29600;
    config.fleet.agent_port_stride = 5;
    config.fleet.project_workflow_url = None;
    native_configuration(config)
}

async fn read_state(name: &str) -> Value {
    serde_json::from_slice(&tokio::fs::read(Path::new(ROOT).join(name)).await.unwrap()).unwrap()
}

async fn save_state(name: &str, state: Value) {
    tokio::fs::write(
        Path::new(ROOT).join(name),
        serde_json::to_vec(&state).unwrap(),
    )
    .await
    .unwrap();
}

async fn driver(phase: &str) {
    let repo = Arc::new(PostgresFleetRepository::new(native_database().await));
    let config = config(std::env::var("FLEET_NATIVE_APPROVAL_RECOVERY_SECRET").unwrap());
    let agent_id = std::env::var("FLEET_NATIVE_APPROVAL_RECOVERY_AGENT")
        .unwrap()
        .parse()
        .unwrap();
    let owner = std::env::var("FLEET_NATIVE_APPROVAL_RECOVERY_OWNER")
        .unwrap()
        .parse()
        .unwrap();
    let agent = repo.get_agent(agent_id).await.unwrap();
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime = Arc::new(LocalRuntimeSupervisor::new(
        config.clone(),
        repo.clone(),
        events.clone(),
    ));
    if phase == "dispatch" {
        let port = std::env::var("FLEET_NATIVE_APPROVAL_RECOVERY_MODEL_PORT")
            .unwrap()
            .parse()
            .unwrap();
        let mut desired = configuration(port, "FLEET_NATIVE_APPROVAL_RECOVERY_SOUL");
        desired.soul_md =
            "# Native approval recovery\nUse only the requested owned QA terminal command.\n"
                .into();
        desired.config_json["platform_toolsets"]["api_server"] = json!(["terminal"]);
        desired.config_json["approvals"] = json!({"mode":"manual","timeout":120});
        desired.config_json["plugins"] = json!({"enabled":["fleet-native-approval-observer"]});
        desired.env_json["TERMINAL_ENV"] = json!("local");
        desired.env_json["TERMINAL_CWD"] = json!(agent.paths.workspace);
        desired.env_json["FLEET_NATIVE_SUPERVISOR_TEST"] = json!("1");
        desired.env_json["FLEET_NATIVE_APPROVAL_RECOVERY_TEST"] = json!("1");
        desired.env_json["FLEET_NATIVE_APPROVAL_FAULT_ROOT"] = json!(ROOT);
        activate(&repo, agent.id, owner, desired).await;
        assert_eq!(
            runtime.start(&agent).await.unwrap().status,
            AgentStatus::Running
        );
        let session = repo
            .create_session(
                CreateSessionRequest {
                    primary_agent_id: Some(agent.id),
                    agent_id: None,
                    title: "Native waiting approval recovery".into(),
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
        let message = repo
            .create_session_message(
                session.id,
                CreateSessionMessageRequest {
                    body: std::env::var("FLEET_NATIVE_APPROVAL_RECOVERY_PROMPT").unwrap(),
                    author_agent_id: None,
                    message_kind: Some(MessageKind::UserPrompt),
                    runtime_message_id: None,
                    idempotency_key: Some(Uuid::new_v4().to_string()),
                },
                owner,
            )
            .await
            .unwrap();
        let intent = timeout(Duration::from_secs(90), async {
            loop {
                if let Some(intent) = repo.get_hermes_dispatch_intent(message.id).await.unwrap() {
                    if intent.state == "accepted"
                        && intent.run.runtime_session_id.is_some()
                        && native_observations(Path::new(ROOT))
                            .await
                            .iter()
                            .any(|row| row["kind"] == "events_held")
                    {
                        break intent;
                    }
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("original native run did not pin before the model tool action");
        let state = json!({"dispatch":dispatch_snapshot(&intent),"native_run_id":intent.run.runtime_run_id,
            "native_session_id":intent.run.runtime_session_id,"pid":std::process::id(),
            "gateway_pid":repo.get_agent(agent.id).await.unwrap().runtime.pid});
        save_state("pin-ready.json", state.clone()).await;
        timeout(Duration::from_secs(90), async {
            loop {
                if let Some(row) = native_observations(Path::new(ROOT))
                    .await
                    .into_iter()
                    .find(|row| row["kind"] == "waiting_held")
                {
                    assert_eq!(row["run_id"], state["native_run_id"]);
                    assert_eq!(row["session_id"], state["native_session_id"]);
                    assert!(row["request_id"].as_str().is_some_and(|id| !id.is_empty()));
                    assert!(
                        repo.list_session_approvals(session.id)
                            .await
                            .unwrap()
                            .is_empty()
                    );
                    assert_eq!(
                        repo.list_session_agent_runs(session.id)
                            .await
                            .unwrap()
                            .len(),
                        1
                    );
                    let mut state = state.clone();
                    state["request_id"] = row["request_id"].clone();
                    save_state("fleet-state.json", state).await;
                    break;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("real waiting native snapshot was not withheld before Fleet exit");
    } else {
        assert_eq!(phase, "recover");
        let state = read_state("fleet-state.json").await;
        let session = state["dispatch"]["session_id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let message = state["dispatch"]["message_id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let approval = timeout(Duration::from_secs(90), async {
            loop {
                if let Some(approval) = repo.list_session_approvals(session).await.unwrap().pop() {
                    break approval;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("new Fleet process did not recover the current native approval by GET");
        assert_eq!(
            Some(approval.runtime_run_id.as_str()),
            state["native_run_id"].as_str()
        );
        assert_eq!(
            approval.runtime_approval_id.as_deref(),
            state["request_id"].as_str()
        );
        assert_eq!(
            repo.get_session_agent_run(approval.session_run_id)
                .await
                .unwrap()
                .state,
            SessionRunState::Waiting
        );
        let cursor = repo.session_event_cursor(session).await.unwrap();
        sleep(Duration::from_secs(6)).await;
        assert_eq!(repo.list_session_approvals(session).await.unwrap().len(), 1);
        assert_eq!(repo.session_event_cursor(session).await.unwrap(), cursor);
        let (restart_tx, _) = tokio::sync::mpsc::channel(1);
        let ctx = Arc::new(app::AppContext::new(
            config,
            repo.clone(),
            Arc::new(FilesystemProvisioner),
            runtime,
            events,
            restart_tx,
        ));
        let token = ctx
            .auth
            .issue_tokens(&repo.find_user_by_id(owner).await.unwrap().unwrap())
            .unwrap()
            .response
            .access_token;
        let fleet = Router::new()
            .route(
                "/api/v1/sessions/{session_id}/approvals/{approval_id}/decision",
                get(api::routes::approvals::read).post(api::routes::approvals::decide),
            )
            .route_layer(axum::middleware::from_fn_with_state(
                ctx.clone(),
                api::middleware::require_auth,
            ))
            .with_state(ctx);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!(
            "http://{}/api/v1/sessions/{session}/approvals/{}/decision",
            listener.local_addr().unwrap(),
            approval.id
        );
        let _server = tokio::spawn(async move { axum::serve(listener, fleet).await.unwrap() });
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(20))
            .build()
            .unwrap();
        let command = domain::ApprovalDecisionRequest {
            choice: ApprovalChoice::Once,
            idempotency_key: Uuid::new_v4().to_string(),
        };
        let response = client
            .post(&url)
            .bearer_auth(&token)
            .json(&command)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let decision = response.json::<ApprovalDecision>().await.unwrap();
        assert_eq!(decision.state, ApprovalDecisionState::Delivered);
        let completed = terminal(&repo, session).await;
        assert_eq!(completed.id, approval.session_run_id);
        assert_eq!(
            completed.runtime_run_id.as_deref(),
            state["native_run_id"].as_str()
        );
        assert_eq!(
            completed.runtime_session_id.as_deref(),
            state["native_session_id"].as_str()
        );
        let before =
            serde_json::to_value(repo.list_session_messages(session).await.unwrap()).unwrap();
        let replay = client
            .post(&url)
            .bearer_auth(&token)
            .json(&command)
            .send()
            .await
            .unwrap();
        assert_eq!(replay.status(), reqwest::StatusCode::OK);
        assert_eq!(
            replay.json::<ApprovalDecision>().await.unwrap().id,
            decision.id
        );
        assert_eq!(
            serde_json::to_value(repo.list_session_messages(session).await.unwrap()).unwrap(),
            before
        );
        let intent = repo
            .get_hermes_dispatch_intent(message)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(dispatch_snapshot(&intent), state["dispatch"]);
        save_state("recovered.json",json!({"pid":std::process::id(),"native_run_id":completed.runtime_run_id,
            "native_session_id":completed.runtime_session_id,"gateway_pid":repo.get_agent(agent.id).await.unwrap().runtime.pid})).await;
    }
    // Only the disposable Compose namespace reaps the surviving native gateway.
    std::process::exit(0);
}

fn child(
    phase: &str,
    agent: &Agent,
    owner: Uuid,
    port: u16,
    secret: &str,
    prompt: &str,
) -> tokio::process::Child {
    tokio::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            TEST,
            "--test-threads=1",
            "--nocapture",
        ])
        .env("FLEET_NATIVE_APPROVAL_RECOVERY_PHASE", phase)
        .env("FLEET_NATIVE_APPROVAL_RECOVERY_AGENT", agent.id.to_string())
        .env("FLEET_NATIVE_APPROVAL_RECOVERY_OWNER", owner.to_string())
        .env(
            "FLEET_NATIVE_APPROVAL_RECOVERY_MODEL_PORT",
            port.to_string(),
        )
        .env("FLEET_NATIVE_APPROVAL_RECOVERY_SECRET", secret)
        .env("FLEET_NATIVE_APPROVAL_RECOVERY_PROMPT", prompt)
        .kill_on_drop(true)
        .spawn()
        .unwrap()
}

#[tokio::test]
#[ignore = "requires exact native Hermes and disposable owned process namespace"]
async fn managed_native_waiting_approval_recovers_across_fleet_processes() {
    if let Ok(phase) = std::env::var("FLEET_NATIVE_APPROVAL_RECOVERY_PHASE") {
        driver(&phase).await;
    }
    let db = native_database().await;
    let owner = Uuid::new_v4();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO users(id,email,username,display_name,password_hash,is_system_admin,system_role)
         VALUES($1,$2,$3,'Native recovery owner','disabled',false,'user')",
        [owner.into(),format!("{owner}@example.test").into(),owner.to_string().into()])).await.unwrap();
    let repo = Arc::new(PostgresFleetRepository::new(db));
    repo.ensure_runtime_templates().await.unwrap();
    let secret = format!("owned-native-approval-recovery-{}", Uuid::new_v4());
    let config = config(secret.clone());
    let agent = create_agent(&repo, &config, "Native approval recovery agent").await;
    let root = Path::new(ROOT);
    tokio::fs::create_dir(root).await.unwrap();
    tokio::fs::write(root.join("hold-approval-readback"), b"owned transport hold")
        .await
        .unwrap();
    let plugin = Path::new(&agent.paths.config).join("plugins/fleet-native-approval-observer");
    tokio::fs::create_dir_all(&plugin).await.unwrap();
    tokio::fs::write(
        plugin.join("__init__.py"),
        include_str!("../../../../scripts/native_supervisor_live/approval_fault_plugin.py"),
    )
    .await
    .unwrap();
    tokio::fs::write(plugin.join("plugin.yaml"),b"name: fleet-native-approval-observer\nversion: 1.0.0\nkind: platform\nplatforms:\n  - api_server\n").await.unwrap();
    let file = Path::new(&agent.paths.workspace).join("approval-recovery.txt");
    tokio::fs::write(&file, b"owned native approval recovery target")
        .await
        .unwrap();
    tokio::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600))
        .await
        .unwrap();
    let prompt = format!("managed-native-approval-recovery-{}", Uuid::new_v4());
    let gate = Arc::new(tokio::sync::Notify::new());
    let model = Arc::new(ApprovalModel::default());
    *model.first_gate.lock().await = Some(gate.clone());
    model
        .files
        .lock()
        .await
        .insert(prompt.clone(), "approval-recovery.txt".into());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let router = Router::new().route("/v1/chat/completions",post(approval_inference))
        .route("/v1/models",get(|| async {Json(json!({"object":"list","data":[{"id":"fleet-managed-local-model","object":"model"}]}))}))
        .with_state(model.clone());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut first = child("dispatch", &agent, owner, port, &secret, &prompt);
    timeout(Duration::from_secs(100), async {
        while !root.join("pin-ready.json").is_file() {
            assert!(
                first.try_wait().unwrap().is_none(),
                "Fleet exited before pinning the original run"
            );
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("Fleet did not pin before model release");
    gate.notify_one();
    assert!(
        timeout(Duration::from_secs(120), first.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
    let state = read_state("fleet-state.json").await;
    assert_eq!(
        tokio::fs::metadata(&file)
            .await
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    let session = state["dispatch"]["session_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        repo.list_session_approvals(session)
            .await
            .unwrap()
            .is_empty()
    );
    tokio::fs::remove_file(root.join("hold-approval-readback"))
        .await
        .unwrap();
    let mut second = child("recover", &agent, owner, port, &secret, &prompt);
    assert!(
        timeout(Duration::from_secs(150), second.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
    let recovered = read_state("recovered.json").await;
    assert_ne!(state["pid"], recovered["pid"]);
    assert_eq!(state["gateway_pid"], recovered["gateway_pid"]);
    assert_eq!(state["native_run_id"], recovered["native_run_id"]);
    assert_eq!(state["native_session_id"], recovered["native_session_id"]);
    let rows = native_observations(root).await;
    assert_eq!(
        rows.iter().filter(|row| row["kind"] == "run_post").count(),
        1
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row["kind"] == "events_held")
            .count(),
        1
    );
    let posts: Vec<_> = rows
        .iter()
        .filter(|row| row["kind"] == "approval_post")
        .collect();
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0]["run_id"], state["native_run_id"]);
    assert_eq!(posts[0]["request_id"], state["request_id"]);
    assert_eq!(posts[0]["choice"], "once");
    assert_eq!(posts[0]["resolve_all"], false);
    assert_eq!(model.calls.lock().await.get(&prompt), Some(&2));
    assert_eq!(
        tokio::fs::metadata(&file)
            .await
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o666
    );
    let messages = repo.list_session_messages(session).await.unwrap();
    assert_eq!(
        messages
            .iter()
            .filter(|m| m.message_kind == MessageKind::UserPrompt)
            .count(),
        1
    );
    assert_eq!(
        messages
            .iter()
            .filter(|m| m.message_kind == MessageKind::AssistantMessage)
            .count(),
        1
    );
    assert_eq!(
        repo.list_session_agent_runs(session).await.unwrap().len(),
        1
    );
    assert_eq!(repo.list_session_approvals(session).await.unwrap().len(), 1);
    server.abort();
    println!(
        "Native current approval recovered across two Fleet processes: original run/session, GET-only restore, one owner decision and tool effect, one final answer. No historical approval queue, unknown decision recovery, PM or OS-descendant proof."
    );
}
