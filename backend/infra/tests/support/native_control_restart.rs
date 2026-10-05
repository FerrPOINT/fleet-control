use super::*;

const TEST: &str =
    "native_control_restart::managed_native_control_outcomes_survive_fleet_process_death";
const COMBINED_TEST: &str = "native_control_restart::managed_native_combined_run_and_control_outcomes_survive_fleet_process_death";
const ROOT: &str = "/tmp/fleet-native-supervisor/control-process-fault";
const DISPATCH_ROOT: &str = "/tmp/fleet-native-supervisor/control-process-fault/dispatch";
const GUIDANCE: &str = "Stay within the owned synthetic native restart task.";

fn config(secret: String, combined: bool) -> Arc<AppConfig> {
    let mut config = AppConfig::default();
    config.fleet.agents_root = "/tmp/fleet-native-supervisor/control-process-agents".into();
    config.fleet.hermes_source = "/opt/hermes".into();
    config.fleet.hermes_command = "/opt/fleet-hermes/bin/hermes".into();
    config.fleet.runtime_token_secret = secret;
    config.fleet.hermes_control_outcome_enabled = true;
    config.fleet.hermes_recovery_extension_enabled = combined;
    config.fleet.agent_port_base = 29700;
    config.fleet.agent_port_stride = 5;
    config.fleet.project_workflow_url = None;
    Arc::new(config)
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

async fn hold() {
    tokio::fs::write(Path::new(ROOT).join("hold-lookup"), b"owned QA hold")
        .await
        .unwrap();
}

async fn driver(phase: &str, combined: bool) {
    let repo = Arc::new(PostgresFleetRepository::new(native_database().await));
    let config = config(
        std::env::var("FLEET_NATIVE_CONTROL_PROCESS_SECRET").unwrap(),
        combined,
    );
    let agent_id = std::env::var("FLEET_NATIVE_CONTROL_PROCESS_AGENT")
        .unwrap()
        .parse()
        .unwrap();
    let owner = std::env::var("FLEET_NATIVE_CONTROL_PROCESS_OWNER")
        .unwrap()
        .parse()
        .unwrap();
    let agent = repo.get_agent(agent_id).await.unwrap();
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime = LocalRuntimeSupervisor::new(config.clone(), repo.clone(), events);
    if phase == "steer" {
        let port = std::env::var("FLEET_NATIVE_CONTROL_PROCESS_MODEL_PORT")
            .unwrap()
            .parse()
            .unwrap();
        let mut desired = configuration(port, "FLEET_NATIVE_CONTROL_PROCESS_SOUL");
        desired.config_json["plugins"] =
            json!({"enabled":["fleet-hermes-controls","fleet-native-discard-control-ack"]});
        desired.env_json["FLEET_NATIVE_SUPERVISOR_TEST"] = json!("1");
        desired.env_json["FLEET_NATIVE_CONTROL_FAULT_ROOT"] = json!(ROOT);
        if combined {
            desired.config_json["plugins"] = json!({"enabled":["fleet-hermes-recovery","fleet-hermes-controls",
                "fleet-native-discard-ack","fleet-native-discard-control-ack"]});
            desired.env_json["FLEET_NATIVE_FAULT_ROOT"] = json!(DISPATCH_ROOT);
        }
        activate(&repo, agent.id, owner, desired).await;
        assert_eq!(
            runtime.start(&agent).await.unwrap().status,
            AgentStatus::Running
        );
        let held = reqwest::Client::builder()
            .no_proxy()
            .build()
            .unwrap()
            .get(format!(
                "http://127.0.0.1:{}/fleet/v1/controls/lookup",
                agent.api_port.unwrap()
            ))
            .bearer_auth(infra::agent_runtime_token(&config, agent.id).unwrap())
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .unwrap();
        assert_eq!(held.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            held.json::<Value>().await.unwrap()["error"],
            "owned_qa_control_hold"
        );
        let session = repo
            .create_session(
                CreateSessionRequest {
                    primary_agent_id: Some(agent.id),
                    agent_id: None,
                    title: "Native controls across Fleet process death".into(),
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
        let prompt = std::env::var("FLEET_NATIVE_CONTROL_PROCESS_PROMPT").unwrap();
        let message = repo
            .create_session_message(
                session.id,
                CreateSessionMessageRequest {
                    body: prompt,
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
                        && intent.run.state == SessionRunState::Running
                    {
                        break intent;
                    }
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("native run did not pin before steer");
        save("run-ready.json", json!({"run_id":intent.run.id})).await;
        timeout(Duration::from_secs(90), async {
            while !Path::new(ROOT).join("model-observed").is_file() {
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("parent did not observe the actual model barrier before steer");
        let actor = domain::RuntimeControlActor {
            user_id: owner,
            idempotency_key: Uuid::new_v4().to_string(),
        };
        let result = runtime
            .steer_run(
                &agent,
                &intent.run,
                domain::SteerSessionRunRequest {
                    input: GUIDANCE.into(),
                },
                actor.clone(),
            )
            .await
            .unwrap();
        assert!(!result.accepted);
        let receipt = result.command.unwrap();
        let original = repo
            .get_runtime_control_outcome(receipt.id)
            .await
            .unwrap()
            .unwrap();
        save(
            "steer-ready.json",
            json!({"pid":std::process::id(),
            "gateway_pid":repo.get_agent(agent.id).await.unwrap().runtime.pid,
            "dispatch":dispatch_snapshot(&intent),"session_id":session.id,"message_id":message.id,
            "run_id":intent.run.id,"steer_id":receipt.id,"steer_key":actor.idempotency_key,
            "steer_context":original.context}),
        )
        .await;
    } else if phase == "stop" {
        let state = read("steer-ready.json").await;
        let session = state["session_id"].as_str().unwrap().parse().unwrap();
        let id = state["steer_id"].as_str().unwrap().parse().unwrap();
        let run = repo
            .get_session_agent_run(state["run_id"].as_str().unwrap().parse().unwrap())
            .await
            .unwrap();
        let actor = domain::RuntimeControlActor {
            user_id: owner,
            idempotency_key: state["steer_key"].as_str().unwrap().into(),
        };
        // A fresh supervisor still cannot resend an uncertain saved command.
        let replay = runtime
            .steer_run(
                &agent,
                &run,
                domain::SteerSessionRunRequest {
                    input: GUIDANCE.into(),
                },
                actor.clone(),
            )
            .await
            .unwrap();
        assert!(!replay.accepted);
        assert_eq!(replay.command.unwrap().id, id);
        save(
            "steer-recovery-ready.json",
            json!({"pid":std::process::id()}),
        )
        .await;
        await_native_control_ack(&repo, session, id).await;
        let replay = runtime
            .steer_run(
                &agent,
                &run,
                domain::SteerSessionRunRequest {
                    input: GUIDANCE.into(),
                },
                actor,
            )
            .await
            .unwrap();
        assert!(replay.accepted);
        assert_eq!(replay.command.unwrap().id, id);
        assert_eq!(
            repo.get_runtime_control_outcome(id)
                .await
                .unwrap()
                .unwrap()
                .context,
            state["steer_context"]
        );
        hold().await;
        let actor = domain::RuntimeControlActor {
            user_id: owner,
            idempotency_key: Uuid::new_v4().to_string(),
        };
        let result = runtime.stop_run(&agent, &run, actor.clone()).await.unwrap();
        assert!(!result.accepted);
        let receipt = result.command.unwrap();
        let original = repo
            .get_runtime_control_outcome(receipt.id)
            .await
            .unwrap()
            .unwrap();
        save(
            "stop-ready.json",
            json!({"pid":std::process::id(),
            "gateway_pid":repo.get_agent(agent.id).await.unwrap().runtime.pid,
            "stop_id":receipt.id,"stop_key":actor.idempotency_key,"stop_context":original.context}),
        )
        .await;
    } else {
        assert_eq!(phase, "recover");
        let steer = read("steer-ready.json").await;
        let stop = read("stop-ready.json").await;
        let session = steer["session_id"].as_str().unwrap().parse().unwrap();
        let id = stop["stop_id"].as_str().unwrap().parse().unwrap();
        let run_id = steer["run_id"].as_str().unwrap().parse().unwrap();
        timeout(Duration::from_secs(90), async {
            loop {
                let run = repo.get_session_agent_run(run_id).await.unwrap();
                assert_ne!(run.state, SessionRunState::Completed);
                if matches!(
                    run.state,
                    SessionRunState::Failed | SessionRunState::Cancelled
                ) {
                    break;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("fresh Fleet did not read the real interrupted terminal run");
        repo.reconcile_runtime_controls().await.unwrap();
        let terminal = repo.get_runtime_control(session, id).await.unwrap();
        assert_eq!(
            terminal.state,
            domain::RuntimeControlState::TerminalObserved
        );
        assert!(terminal.acknowledgement.is_none());
        save(
            "terminal-ready.json",
            json!({"pid":std::process::id(),"receipt":terminal}),
        )
        .await;
        await_native_control_ack(&repo, session, id).await;
        let witnessed = repo.get_runtime_control(session, id).await.unwrap();
        assert_eq!(witnessed.state, terminal.state);
        assert_eq!(witnessed.updated_at, terminal.updated_at);
        assert_eq!(witnessed.acknowledgement.as_deref(), Some("stopping"));
        let run = repo.get_session_agent_run(run_id).await.unwrap();
        let replay = runtime
            .stop_run(
                &agent,
                &run,
                domain::RuntimeControlActor {
                    user_id: owner,
                    idempotency_key: stop["stop_key"].as_str().unwrap().into(),
                },
            )
            .await
            .unwrap();
        assert!(replay.accepted);
        assert_eq!(replay.command.unwrap().id, id);
        assert_eq!(
            repo.get_runtime_control_outcome(id)
                .await
                .unwrap()
                .unwrap()
                .context,
            stop["stop_context"]
        );
        let message = steer["message_id"].as_str().unwrap().parse().unwrap();
        let intent = repo
            .get_hermes_dispatch_intent(message)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(dispatch_snapshot(&intent), steer["dispatch"]);
        assert_eq!(
            repo.list_session_agent_runs(session).await.unwrap().len(),
            1
        );
        assert!(
            repo.list_session_messages(session)
                .await
                .unwrap()
                .iter()
                .all(|m| m.message_kind != MessageKind::AssistantMessage)
        );
        save(
            "recovered.json",
            json!({"pid":std::process::id(),
            "gateway_pid":repo.get_agent(agent.id).await.unwrap().runtime.pid}),
        )
        .await;
        std::process::exit(0);
    }
    // The parent SIGKILLs this process after the durable original context is observed.
    // No destructor or graceful runtime shutdown participates in recovery.
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
        .env("FLEET_NATIVE_CONTROL_PROCESS_PHASE", phase)
        .env("FLEET_NATIVE_CONTROL_PROCESS_AGENT", agent.id.to_string())
        .env("FLEET_NATIVE_CONTROL_PROCESS_OWNER", owner.to_string())
        .env("FLEET_NATIVE_CONTROL_PROCESS_MODEL_PORT", port.to_string())
        .env("FLEET_NATIVE_CONTROL_PROCESS_SECRET", secret)
        .env("FLEET_NATIVE_CONTROL_PROCESS_PROMPT", prompt)
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
async fn managed_native_control_outcomes_survive_fleet_process_death() {
    scenario(false).await;
}

#[tokio::test]
#[ignore = "requires both committed Base plugins, exact native Hermes and owned process namespace"]
async fn managed_native_combined_run_and_control_outcomes_survive_fleet_process_death() {
    scenario(true).await;
}

async fn scenario(combined: bool) {
    if let Ok(phase) = std::env::var("FLEET_NATIVE_CONTROL_PROCESS_PHASE") {
        driver(&phase, combined).await;
    }
    let db = native_database().await;
    let owner = Uuid::new_v4();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO users(id,email,username,display_name,password_hash,is_system_admin,system_role)
         VALUES($1,$2,$3,'Native control restart owner','disabled',false,'user')",
        [owner.into(),format!("{owner}@example.test").into(),owner.to_string().into()])).await.unwrap();
    let audit_db = native_database().await;
    let repo = Arc::new(PostgresFleetRepository::new(db));
    repo.ensure_runtime_templates().await.unwrap();
    let secret = format!("owned-native-control-process-{}", Uuid::new_v4());
    let config = config(secret.clone(), combined);
    let agent = create_agent(&repo, &config, "Native control process recovery").await;
    tokio::fs::create_dir_all(ROOT).await.unwrap();
    hold().await;
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
    let fault = plugins.join("fleet-native-discard-control-ack");
    if combined {
        // Run recovery remains available while the independent control lookup is held.
        tokio::fs::create_dir_all(DISPATCH_ROOT).await.unwrap();
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
        let discard = plugins.join("fleet-native-discard-ack");
        tokio::fs::create_dir_all(&discard).await.unwrap();
        tokio::fs::write(
            discard.join("__init__.py"),
            include_str!("../../../../scripts/native_supervisor_live/discard_ack_plugin.py"),
        )
        .await
        .unwrap();
        tokio::fs::write(discard.join("plugin.yaml"), b"name: fleet-native-discard-ack\nversion: 1.0.0\nkind: platform\nplatforms:\n  - api_server\n").await.unwrap();
    }
    tokio::fs::create_dir_all(&fault).await.unwrap();
    tokio::fs::write(
        fault.join("__init__.py"),
        include_str!("../../../../scripts/native_supervisor_live/control_fault_plugin.py"),
    )
    .await
    .unwrap();
    tokio::fs::write(fault.join("plugin.yaml"), b"name: fleet-native-discard-control-ack\nversion: 1.0.0\nkind: platform\nplatforms:\n  - api_server\n").await.unwrap();
    let model = Arc::new(Model::default());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let router = Router::new().route("/v1/chat/completions",post(inference))
        .route("/v1/models",get(|| async {Json(json!({"object":"list","data":[{"id":"fleet-managed-local-model","object":"model"}]}))}))
        .with_state(model.clone());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let prompt = format!("managed-control-prompt-{}", Uuid::new_v4());
    let mut first = child("steer", &agent, owner, port, &secret, &prompt, combined);
    ready(&mut first, "run-ready.json").await;
    timeout(Duration::from_secs(90), async {
        while !model.requests.lock().await.contains_key(&prompt) {
            assert!(first.try_wait().unwrap().is_none());
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("native model request did not reach the owned barrier");
    tokio::fs::write(
        Path::new(ROOT).join("model-observed"),
        b"actual inference observed",
    )
    .await
    .unwrap();
    ready(&mut first, "steer-ready.json").await;
    kill(&mut first).await;
    let steer = read("steer-ready.json").await;
    assert_eq!(model.requests.lock().await.get(&prompt).unwrap().len(), 1);
    let mut second = child("stop", &agent, owner, port, &secret, &prompt, combined);
    ready(&mut second, "steer-recovery-ready.json").await;
    tokio::fs::remove_file(Path::new(ROOT).join("hold-lookup"))
        .await
        .unwrap();
    ready(&mut second, "stop-ready.json").await;
    kill(&mut second).await;
    let stop = read("stop-ready.json").await;
    assert_ne!(steer["pid"], stop["pid"]);
    assert_eq!(steer["gateway_pid"], stop["gateway_pid"]);
    let session = steer["session_id"].as_str().unwrap().parse().unwrap();
    let stop_id = stop["stop_id"].as_str().unwrap().parse().unwrap();
    assert!(
        repo.get_runtime_control(session, stop_id)
            .await
            .unwrap()
            .acknowledgement
            .is_none()
    );
    model.control_release.notify_one();
    let mut third = child("recover", &agent, owner, port, &secret, &prompt, combined);
    ready(&mut third, "terminal-ready.json").await;
    tokio::fs::remove_file(Path::new(ROOT).join("hold-lookup"))
        .await
        .unwrap();
    assert!(
        timeout(Duration::from_secs(120), third.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
    let recovered = read("recovered.json").await;
    assert_ne!(recovered["pid"], steer["pid"]);
    assert_ne!(recovered["pid"], stop["pid"]);
    assert_eq!(recovered["gateway_pid"], steer["gateway_pid"]);
    assert_eq!(model.requests.lock().await.get(&prompt).unwrap().len(), 1);
    let observations = tokio::fs::read_to_string(Path::new(ROOT).join("control-events.jsonl"))
        .await
        .unwrap();
    let rows: Vec<Value> = observations
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    if combined {
        assert!(steer["dispatch"]["capabilities"]["fleet_recovery"].is_object());
        let native = native_observations(Path::new(DISPATCH_ROOT)).await;
        for kind in ["post", "accepted"] {
            assert_eq!(native.iter().filter(|row| row["kind"] == kind).count(), 1);
        }
        let submitted = native.iter().find(|row| row["kind"] == "post").unwrap();
        assert_eq!(submitted["key"], steer["dispatch"]["key"]);
        assert_eq!(submitted["sha256"], steer["dispatch"]["sha256"]);
        let accepted = native.iter().find(|row| row["kind"] == "accepted").unwrap();
        assert_eq!(accepted["key"], steer["dispatch"]["key"]);
        assert_eq!(accepted["run_id"], steer["steer_context"]["run_id"]);
        assert!(
            native
                .iter()
                .any(|row| row["kind"] == "lookup" && row["blocked"] == false)
        );
    }
    for (operation, field, context) in [
        ("steer", "steer_id", &steer["steer_context"]),
        ("stop", "stop_id", &stop["stop_context"]),
    ] {
        let state = if operation == "steer" { &steer } else { &stop };
        let posts: Vec<_> = rows
            .iter()
            .filter(|row| row["kind"] == "post" && row["operation"] == operation)
            .collect();
        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0]["key"], state[field]);
        assert_eq!(posts[0]["sha256"], context["request_sha256"]);
        assert_eq!(
            rows.iter()
                .filter(|row| row["kind"] == "native_ack" && row["operation"] == operation)
                .count(),
            1
        );
        assert!(rows.iter().any(|row| row["kind"] == "lookup"
            && row["held"] == false
            && row["key"] == state[field]));
        let id: Uuid = state[field].as_str().unwrap().parse().unwrap();
        let audit = audit_db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT count(*)::bigint AS count FROM audit_log WHERE entity_id=$1 AND action='runtime_control.acknowledged'",
            [id.to_string().into()])).await.unwrap().unwrap();
        assert_eq!(audit.try_get::<i64>("", "count").unwrap(), 1);
    }
    assert_eq!(
        repo.list_runtime_controls(session, steer["run_id"].as_str().unwrap().parse().unwrap())
            .await
            .unwrap()
            .len(),
        2
    );
    server.abort();
    if combined {
        println!(
            "Combined native run/steer/stop recovery passed with both committed plugins: lost real initial202 and control ACKs, original read-only POST run lookup and GET control recovery, two Fleet SIGKILLs/three PIDs, one gateway/run/steer/stop POST/native ACK/audit, immutable contexts/dispatch/terminal history and one inference. No approval decision, safe descendants, task/PM or installed acceptance claimed."
        );
    } else {
        println!(
            "Actual native control outcomes survived two Fleet SIGKILLs and three distinct PIDs: one steer/stop POST each, original-key GET recovery, same gateway/context/journal, late stop ACK preserves terminal history. No approval decision or OS-descendant safe-stop proof."
        );
    }
}
