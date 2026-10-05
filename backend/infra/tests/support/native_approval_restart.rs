use super::*;

const TEST: &str = "native_approvals::native_approval_restart::managed_native_approval_outcomes_survive_fleet_process_death";
const COMBINED_TEST: &str = "native_approvals::native_approval_restart::managed_native_combined_run_and_approval_outcomes_survive_fleet_process_death";
const ROOT: &str = "/tmp/fleet-native-supervisor/approval-fault";

fn config(secret: String, combined: bool) -> Arc<AppConfig> {
    let mut config = AppConfig::default();
    config.fleet.agents_root = "/tmp/fleet-native-supervisor/approval-process-agents".into();
    config.fleet.hermes_source = "/opt/hermes".into();
    config.fleet.hermes_command = "/opt/fleet-hermes/bin/hermes".into();
    config.fleet.runtime_token_secret = secret;
    config.fleet.hermes_control_outcome_enabled = true;
    config.fleet.hermes_recovery_extension_enabled = combined;
    config.fleet.agent_port_base = 29800;
    config.fleet.agent_port_stride = 5;
    config.fleet.project_workflow_url = None;
    native_configuration(config)
}

async fn save(name: &str, value: Value) {
    let path = Path::new(ROOT).join(name);
    let temporary = path.with_extension("tmp");
    tokio::fs::write(&temporary, serde_json::to_vec(&value).unwrap())
        .await
        .unwrap();
    tokio::fs::rename(temporary, path).await.unwrap();
}

async fn read(name: &str) -> Value {
    serde_json::from_slice(&tokio::fs::read(Path::new(ROOT).join(name)).await.unwrap()).unwrap()
}

async fn owner_api(
    config: Arc<AppConfig>,
    repo: Arc<PostgresFleetRepository>,
    runtime: Arc<LocalRuntimeSupervisor>,
    events: tokio::sync::broadcast::Sender<shared::FleetEvent>,
    owner: Uuid,
    session: Uuid,
    approval: Uuid,
) -> (reqwest::Client, String, String) {
    let (restart, _) = tokio::sync::mpsc::channel(1);
    let ctx = Arc::new(app::AppContext::new(
        config,
        repo.clone(),
        Arc::new(FilesystemProvisioner),
        runtime,
        events,
        restart,
    ));
    let token = ctx
        .auth
        .issue_tokens(&repo.find_user_by_id(owner).await.unwrap().unwrap())
        .unwrap()
        .response
        .access_token;
    let router = Router::new()
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
        "http://{}/api/v1/sessions/{session}/approvals/{approval}/decision",
        listener.local_addr().unwrap()
    );
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(20))
        .build()
        .unwrap();
    (client, token, url)
}

async fn driver(phase: &str, combined: bool) {
    let repo = Arc::new(PostgresFleetRepository::new(native_database().await));
    let config = config(
        std::env::var("FLEET_NATIVE_APPROVAL_PROCESS_SECRET").unwrap(),
        combined,
    );
    let agent_id = std::env::var("FLEET_NATIVE_APPROVAL_PROCESS_AGENT")
        .unwrap()
        .parse()
        .unwrap();
    let owner = std::env::var("FLEET_NATIVE_APPROVAL_PROCESS_OWNER")
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
        let port = std::env::var("FLEET_NATIVE_APPROVAL_PROCESS_MODEL_PORT")
            .unwrap()
            .parse()
            .unwrap();
        let mut desired = configuration(port, "FLEET_NATIVE_APPROVAL_PROCESS_SOUL");
        desired.soul_md = "# Native approval process recovery\nUse only the requested owned QA terminal command.\n".into();
        desired.config_json["platform_toolsets"]["api_server"] = json!(["terminal"]);
        desired.config_json["approvals"] = json!({"mode":"manual","timeout":120});
        desired.config_json["plugins"] =
            json!({"enabled":["fleet-hermes-controls","fleet-native-approval-observer"]});
        desired.env_json["TERMINAL_ENV"] = json!("local");
        desired.env_json["TERMINAL_CWD"] = json!(agent.paths.workspace);
        desired.env_json["FLEET_NATIVE_SUPERVISOR_TEST"] = json!("1");
        desired.env_json["FLEET_NATIVE_APPROVAL_RECOVERY_TEST"] = json!("1");
        desired.env_json["FLEET_NATIVE_APPROVAL_FAULT_ROOT"] = json!(ROOT);
        if combined {
            desired.config_json["plugins"] = json!({"enabled":["fleet-hermes-recovery","fleet-hermes-controls",
                "fleet-native-discard-ack","fleet-native-approval-observer"]});
            desired.env_json["FLEET_NATIVE_FAULT_ROOT"] = json!(ROOT);
        }
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
                    title: "Native approval across Fleet SIGKILL".into(),
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
                    body: std::env::var("FLEET_NATIVE_APPROVAL_PROCESS_PROMPT").unwrap(),
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
        .expect("original native dispatch did not pin before tool release");
        save(
            "pin-ready.json",
            json!({"pid":std::process::id(),"run_id":intent.run.id}),
        )
        .await;
        let approval = timeout(Duration::from_secs(90), async {
            loop {
                if let Some(approval) = repo.list_session_approvals(session.id).await.unwrap().pop()
                {
                    break approval;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("real native waiting approval was not recovered");
        assert_eq!(approval.session_run_id, intent.run.id);
        let command = domain::ApprovalDecisionRequest {
            choice: ApprovalChoice::Once,
            idempotency_key: Uuid::new_v4().to_string(),
        };
        let (client, token, url) = owner_api(
            config,
            repo.clone(),
            runtime,
            events,
            owner,
            session.id,
            approval.id,
        )
        .await;
        let response = client
            .post(&url)
            .bearer_auth(token)
            .json(&command)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let decision = response.json::<ApprovalDecision>().await.unwrap();
        assert_eq!(decision.state, ApprovalDecisionState::Uncertain);
        let saved = repo
            .get_approval_outcome(decision.id)
            .await
            .unwrap()
            .unwrap();
        save("decision-ready.json", json!({"pid":std::process::id(),"session_id":session.id,"message_id":message.id,
            "approval_id":approval.id,"decision_id":decision.id,"command":command,"context":saved.context,
            "run_id":intent.run.id,"native_run_id":approval.runtime_run_id,"request_id":approval.runtime_approval_id,
            "dispatch":dispatch_snapshot(&intent),"gateway_pid":repo.get_agent(agent.id).await.unwrap().runtime.pid})).await;
    } else {
        let state = read("decision-ready.json").await;
        let session = state["session_id"].as_str().unwrap().parse().unwrap();
        let approval = state["approval_id"].as_str().unwrap().parse().unwrap();
        let id: Uuid = state["decision_id"].as_str().unwrap().parse().unwrap();
        let run_id = state["run_id"].as_str().unwrap().parse().unwrap();
        let command: domain::ApprovalDecisionRequest =
            serde_json::from_value(state["command"].clone()).unwrap();
        let (client, token, url) = owner_api(
            config,
            repo.clone(),
            runtime,
            events,
            owner,
            session,
            approval,
        )
        .await;
        if phase == "replay" {
            let baseline = read("replay-baseline.json").await;
            let response = client
                .post(&url)
                .bearer_auth(&token)
                .json(&command)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), reqwest::StatusCode::OK);
            let replay = response.json::<ApprovalDecision>().await.unwrap();
            assert_eq!(replay.id, id);
            assert_eq!(replay.state, ApprovalDecisionState::Uncertain);
            timeout(Duration::from_secs(20), async {
                loop {
                    if native_observations(Path::new(ROOT))
                        .await
                        .iter()
                        .filter(|row| {
                            row["kind"] == "approval_outcome_lookup"
                                && row["key"] == id.to_string()
                                && row["held"] == true
                        })
                        .count() as u64
                        > baseline["held_lookups"].as_u64().unwrap()
                    {
                        break;
                    }
                    sleep(Duration::from_millis(50)).await;
                }
            })
            .await
            .expect("original decision GET hold was not observed");
            assert_eq!(
                repo.get_approval_outcome(id)
                    .await
                    .unwrap()
                    .unwrap()
                    .context,
                state["context"]
            );
            assert!(
                repo.list_session_messages(session)
                    .await
                    .unwrap()
                    .iter()
                    .all(|m| m.message_kind != MessageKind::AssistantMessage)
            );
            save("replay-ready.json", json!({"pid":std::process::id(),"gateway_pid":repo.get_agent(agent.id).await.unwrap().runtime.pid})).await;
        } else {
            assert_eq!(phase, "recover");
            let completed = terminal(&repo, session).await;
            assert_eq!(completed.id, run_id);
            assert_eq!(
                repo.approval_decision(session, approval)
                    .await
                    .unwrap()
                    .state,
                ApprovalDecisionState::Uncertain
            );
            let before =
                serde_json::to_value(repo.list_session_messages(session).await.unwrap()).unwrap();
            save(
                "terminal-ready.json",
                json!({"pid":std::process::id(),"run":completed}),
            )
            .await;
            timeout(Duration::from_secs(20), async {
                loop {
                    if repo
                        .approval_decision(session, approval)
                        .await
                        .unwrap()
                        .state
                        == ApprovalDecisionState::Delivered
                    {
                        break;
                    }
                    sleep(Duration::from_millis(50)).await;
                }
            })
            .await
            .expect("original decision ACK was not recovered after Fleet death");
            let after = repo.get_approval_outcome(id).await.unwrap().unwrap();
            assert_eq!(after.context, state["context"]);
            assert_eq!(
                serde_json::to_value(repo.get_session_agent_run(run_id).await.unwrap()).unwrap(),
                serde_json::to_value(&completed).unwrap()
            );
            for _ in 0..2 {
                let response = client
                    .post(&url)
                    .bearer_auth(&token)
                    .json(&command)
                    .send()
                    .await
                    .unwrap();
                assert_eq!(response.status(), reqwest::StatusCode::OK);
                let replay = response.json::<ApprovalDecision>().await.unwrap();
                assert_eq!(replay.id, id);
                assert_eq!(replay.state, ApprovalDecisionState::Delivered);
            }
            let readback = client.get(&url).bearer_auth(&token).send().await.unwrap();
            assert_eq!(readback.status(), reqwest::StatusCode::OK);
            assert_eq!(
                readback.json::<ApprovalDecision>().await.unwrap().state,
                ApprovalDecisionState::Delivered
            );
            let changed = domain::ApprovalDecisionRequest {
                choice: ApprovalChoice::Deny,
                ..command
            };
            assert_eq!(
                client
                    .post(&url)
                    .bearer_auth(&token)
                    .json(&changed)
                    .send()
                    .await
                    .unwrap()
                    .status(),
                reqwest::StatusCode::CONFLICT
            );
            assert_eq!(
                serde_json::to_value(repo.list_session_messages(session).await.unwrap()).unwrap(),
                before
            );
            let intent = repo
                .get_hermes_dispatch_intent(state["message_id"].as_str().unwrap().parse().unwrap())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(dispatch_snapshot(&intent), state["dispatch"]);
            save("recovered.json", json!({"pid":std::process::id(),"gateway_pid":repo.get_agent(agent.id).await.unwrap().runtime.pid})).await;
            std::process::exit(0);
        }
    }
    // The parent verifies a real SIGKILL; no destructor or supervisor clone replaces OS death.
    std::future::pending::<()>().await;
}

fn child(
    phase: &str,
    agent: &Agent,
    owner: Uuid,
    port: u16,
    secret: &str,
    prompt: &str,
    combined: bool,
) -> tokio::process::Child {
    tokio::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            if combined { COMBINED_TEST } else { TEST },
            "--test-threads=1",
            "--nocapture",
        ])
        .env("FLEET_NATIVE_APPROVAL_PROCESS_PHASE", phase)
        .env("FLEET_NATIVE_APPROVAL_PROCESS_AGENT", agent.id.to_string())
        .env("FLEET_NATIVE_APPROVAL_PROCESS_OWNER", owner.to_string())
        .env("FLEET_NATIVE_APPROVAL_PROCESS_MODEL_PORT", port.to_string())
        .env("FLEET_NATIVE_APPROVAL_PROCESS_SECRET", secret)
        .env("FLEET_NATIVE_APPROVAL_PROCESS_PROMPT", prompt)
        .kill_on_drop(true)
        .spawn()
        .unwrap()
}

async fn ready(child: &mut tokio::process::Child, name: &str) {
    timeout(Duration::from_secs(120), async {
        while !Path::new(ROOT).join(name).is_file() {
            assert!(
                child.try_wait().unwrap().is_none(),
                "Fleet exited before {name}"
            );
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("Fleet did not persist {name}"));
}

async fn kill(child: &mut tokio::process::Child) {
    use std::os::unix::process::ExitStatusExt;
    child.start_kill().unwrap();
    let status = timeout(Duration::from_secs(10), child.wait())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(status.signal(), Some(9));
}

#[tokio::test]
#[ignore = "requires committed Base control plugin, exact native Hermes and owned process namespace"]
async fn managed_native_approval_outcomes_survive_fleet_process_death() {
    scenario(false).await;
}

#[tokio::test]
#[ignore = "requires both committed Base plugins, exact native Hermes and owned process namespace"]
async fn managed_native_combined_run_and_approval_outcomes_survive_fleet_process_death() {
    scenario(true).await;
}

async fn scenario(combined: bool) {
    if let Ok(phase) = std::env::var("FLEET_NATIVE_APPROVAL_PROCESS_PHASE") {
        driver(&phase, combined).await;
    }
    let db = native_database().await;
    let owner = Uuid::new_v4();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO users(id,email,username,display_name,password_hash,is_system_admin,system_role)
         VALUES($1,$2,$3,'Native approval restart owner','disabled',false,'user')",
        [owner.into(),format!("{owner}@example.test").into(),owner.to_string().into()])).await.unwrap();
    let audit_db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let repo = Arc::new(PostgresFleetRepository::new(db));
    repo.ensure_runtime_templates().await.unwrap();
    let secret = format!("owned-native-approval-process-{}", Uuid::new_v4());
    let config = config(secret.clone(), combined);
    let agent = create_agent(&repo, &config, "Native approval process recovery").await;
    let root = Path::new(ROOT);
    tokio::fs::create_dir(root).await.unwrap();
    for name in ["hold-outcome-lookup", "drop-next-approval"] {
        tokio::fs::write(root.join(name), b"owned QA fault")
            .await
            .unwrap();
    }
    let plugins = Path::new(&agent.paths.config).join("plugins");
    let controls = plugins.join("fleet-hermes-controls");
    tokio::fs::create_dir_all(&controls).await.unwrap();
    for name in ["__init__.py", "plugin.py", "store.py", "plugin.yaml"] {
        tokio::fs::copy(
            Path::new("/qa/control-plugin").join(name),
            controls.join(name),
        )
        .await
        .unwrap();
    }
    if combined {
        let recovery = plugins.join("fleet-hermes-recovery");
        tokio::fs::create_dir_all(&recovery).await.unwrap();
        for name in ["__init__.py", "plugin.py", "store.py", "plugin.yaml"] {
            tokio::fs::copy(
                Path::new("/qa/recovery-plugin").join(name),
                recovery.join(name),
            )
            .await
            .unwrap();
        }
        let fault = plugins.join("fleet-native-discard-ack");
        tokio::fs::create_dir_all(&fault).await.unwrap();
        tokio::fs::write(
            fault.join("__init__.py"),
            include_str!("../../../../scripts/native_supervisor_live/discard_ack_plugin.py"),
        )
        .await
        .unwrap();
        tokio::fs::write(fault.join("plugin.yaml"), b"name: fleet-native-discard-ack\nversion: 1.0.0\nkind: platform\nplatforms:\n  - api_server\n").await.unwrap();
    }
    let observer = plugins.join("fleet-native-approval-observer");
    tokio::fs::create_dir_all(&observer).await.unwrap();
    tokio::fs::write(
        observer.join("__init__.py"),
        include_str!("../../../../scripts/native_supervisor_live/approval_fault_plugin.py"),
    )
    .await
    .unwrap();
    tokio::fs::write(observer.join("plugin.yaml"), b"name: fleet-native-approval-observer\nversion: 1.0.0\nkind: platform\nplatforms:\n  - api_server\n").await.unwrap();
    let filename = "approval-process.txt";
    let file = Path::new(&agent.paths.workspace).join(filename);
    tokio::fs::write(&file, b"owned native approval process target")
        .await
        .unwrap();
    tokio::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600))
        .await
        .unwrap();
    let prompt = format!("managed-native-approval-process-{}", Uuid::new_v4());
    let first_gate = Arc::new(tokio::sync::Notify::new());
    let tool_gate = Arc::new(tokio::sync::Notify::new());
    let model = Arc::new(ApprovalModel::default());
    *model.first_gate.lock().await = Some(first_gate.clone());
    *model.tool_gate.lock().await = Some(tool_gate.clone());
    model
        .files
        .lock()
        .await
        .insert(prompt.clone(), filename.into());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let router = Router::new().route("/v1/chat/completions", post(approval_inference))
        .route("/v1/models", get(|| async { Json(json!({"object":"list","data":[{"id":"fleet-managed-local-model","object":"model"}]})) }))
        .with_state(model.clone());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut first = child("dispatch", &agent, owner, port, &secret, &prompt, combined);
    ready(&mut first, "pin-ready.json").await;
    first_gate.notify_one();
    ready(&mut first, "decision-ready.json").await;
    let state = read("decision-ready.json").await;
    assert_eq!(state["pid"], first.id().unwrap());
    timeout(Duration::from_secs(20), async {
        loop {
            if tokio::fs::metadata(&file)
                .await
                .unwrap()
                .permissions()
                .mode()
                & 0o777
                == 0o666
                && model.calls.lock().await.get(&prompt) == Some(&2)
            {
                break;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("real approved terminal effect did not reach the paused model");
    let session = state["session_id"].as_str().unwrap().parse().unwrap();
    assert!(
        repo.list_session_messages(session)
            .await
            .unwrap()
            .iter()
            .all(|m| m.message_kind != MessageKind::AssistantMessage)
    );
    kill(&mut first).await;
    let held_lookups = native_observations(root)
        .await
        .iter()
        .filter(|row| {
            row["kind"] == "approval_outcome_lookup"
                && row["key"] == state["decision_id"]
                && row["held"] == true
        })
        .count();
    save("replay-baseline.json", json!({"held_lookups":held_lookups})).await;
    let mut second = child("replay", &agent, owner, port, &secret, &prompt, combined);
    ready(&mut second, "replay-ready.json").await;
    let replayed = read("replay-ready.json").await;
    assert_eq!(replayed["pid"], second.id().unwrap());
    assert_ne!(state["pid"], replayed["pid"]);
    assert_eq!(state["gateway_pid"], replayed["gateway_pid"]);
    kill(&mut second).await;
    tool_gate.notify_one();
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    timeout(Duration::from_secs(90), async {
        loop {
            let response = client
                .get(format!(
                    "http://127.0.0.1:{}/v1/runs/{}",
                    agent.api_port.unwrap(),
                    state["native_run_id"].as_str().unwrap()
                ))
                .bearer_auth(infra::agent_runtime_token(&config, agent.id).unwrap())
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), reqwest::StatusCode::OK);
            let body = response.json::<Value>().await.unwrap();
            assert_eq!(body["run_id"], state["native_run_id"]);
            if body["status"] == "completed" {
                break;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("native tool response did not reach terminal after Fleet deaths");
    let mut third = child("recover", &agent, owner, port, &secret, &prompt, combined);
    ready(&mut third, "terminal-ready.json").await;
    tokio::fs::remove_file(root.join("hold-outcome-lookup"))
        .await
        .unwrap();
    assert!(
        timeout(Duration::from_secs(45), third.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
    let recovered = read("recovered.json").await;
    assert_ne!(recovered["pid"], state["pid"]);
    assert_ne!(recovered["pid"], replayed["pid"]);
    assert_eq!(recovered["gateway_pid"], state["gateway_pid"]);
    let rows = native_observations(root).await;
    if combined {
        assert!(state["dispatch"]["capabilities"]["fleet_recovery"].is_object());
        for kind in ["post", "accepted"] {
            assert_eq!(rows.iter().filter(|row| row["kind"] == kind).count(), 1);
        }
        let submitted = rows.iter().find(|row| row["kind"] == "post").unwrap();
        assert_eq!(submitted["key"], state["dispatch"]["key"]);
        assert_eq!(submitted["sha256"], state["dispatch"]["sha256"]);
        let accepted = rows.iter().find(|row| row["kind"] == "accepted").unwrap();
        assert_eq!(accepted["run_id"], state["native_run_id"]);
        assert!(
            rows.iter()
                .any(|row| row["kind"] == "lookup" && row["blocked"] == false)
        );
    }
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
    for kind in ["approval_post", "approval_ack", "approval_ack_dropped"] {
        assert_eq!(rows.iter().filter(|row| row["kind"] == kind).count(), 1);
    }
    let post = rows
        .iter()
        .find(|row| row["kind"] == "approval_post")
        .unwrap();
    assert_eq!(post["key"], state["decision_id"]);
    assert_eq!(
        post["store_id"],
        state["context"]["capabilities"]["store_id"]
    );
    assert_eq!(post["sha256"], state["context"]["request_sha256"]);
    assert_eq!(post["run_id"], state["native_run_id"]);
    assert_eq!(post["request_id"], state["request_id"]);
    assert_eq!(post["choice"], "once");
    assert_eq!(post["resolve_all"], false);
    assert!(
        rows.iter()
            .any(|row| row["kind"] == "approval_outcome_lookup"
                && row["key"] == state["decision_id"]
                && row["held"] == false)
    );
    let audit = audit_db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT count(*) AS count FROM audit_log WHERE entity_id=$1 AND action='approval.decision_delivered'",
        [state["decision_id"].as_str().unwrap().to_string().into()])).await.unwrap().unwrap();
    assert_eq!(audit.try_get::<i64>("", "count").unwrap(), 1);
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
    assert_eq!(model.calls.lock().await.get(&prompt), Some(&2));
    server.abort();
    if combined {
        println!(
            "Combined native run/approval recovery passed with both committed plugins: lost real initial202 and decision ACKs, original read-only POST run lookup and GET approval recovery, two Fleet SIGKILLs/three PIDs, one gateway/run/decision POST/native ACK/audit, immutable dispatch/decision context/terminal history and one assistant. No safe descendants, task/PM or installed acceptance claimed."
        );
    } else {
        println!(
            "Actual native approval outcome survived two Fleet SIGKILLs and three PIDs: one decision POST/native ACK/audit, original GET-only recovery, same gateway/context/dispatch, one tool effect and assistant, late ACK preserves terminal history. No combined extensions, task/PM or safe descendants claimed."
        );
    }
}
