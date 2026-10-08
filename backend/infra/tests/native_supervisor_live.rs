//! Opt-in native gateway acceptance; run only through the owned Compose harness.
#![cfg(unix)]
use app::{AgentProvisioner, FleetRepository, RuntimeSupervisor};
use axum::{
    Json, Router,
    extract::State,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use domain::{
    Agent, AgentKind, AgentProductRole, AgentRole, AgentStatus, CreateAgentRequest,
    CreateSessionMessageRequest, CreateSessionRequest, MessageKind, SessionRunState,
    UpdateAgentConfigRequest,
};
use infra::{FilesystemProvisioner, PostgresFleetRepository, runtime::LocalRuntimeSupervisor};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use shared::{AppConfig, DatabaseConfig};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::{collections::HashMap, path::Path, sync::Arc};
use tokio::{
    sync::Mutex,
    time::{Duration, sleep, timeout},
};
use uuid::Uuid;

#[path = "support/native_approvals.rs"]
mod native_approvals;

#[path = "support/native_control_restart.rs"]
mod native_control_restart;

#[path = "support/native_prepared_launch.rs"]
mod native_prepared_launch;

#[path = "support/native_request_observer.rs"]
mod native_request_observer;

fn native_configuration(mut config: AppConfig) -> Arc<AppConfig> {
    let controller_root = Path::new(&config.fleet.agents_root)
        .parent()
        .expect("owned native QA agent root must have a parent")
        .join("controller");
    std::fs::create_dir_all(&controller_root).unwrap();
    std::fs::set_permissions(&controller_root, std::fs::Permissions::from_mode(0o700)).unwrap();
    config.fleet.controller_root = controller_root.to_string_lossy().into_owned();
    Arc::new(config)
}

#[derive(Default)]
struct Model {
    requests: Mutex<HashMap<String, Vec<String>>>,
    wire_requests: Mutex<HashMap<String, Vec<Value>>>,
    control_release: tokio::sync::Notify,
}

async fn inference(State(model): State<Arc<Model>>, Json(body): Json<Value>) -> Response {
    assert_eq!(body["model"], "fleet-managed-local-model");
    let messages = body["messages"].as_array().unwrap();
    let prompt = messages
        .iter()
        .rev()
        .find(|m| m["role"] == "user")
        .and_then(|m| m["content"].as_str())
        .unwrap()
        .to_string();
    let system = messages
        .iter()
        .filter(|m| m["role"] == "system")
        .filter_map(|m| m["content"].as_str())
        .collect::<Vec<_>>()
        .join("\n");
    model
        .wire_requests
        .lock()
        .await
        .entry(prompt.clone())
        .or_default()
        .push(body.clone());
    model
        .requests
        .lock()
        .await
        .entry(prompt.clone())
        .or_default()
        .push(system);
    if prompt.starts_with("managed-control-prompt-") {
        model.control_release.notified().await;
    }
    if body["stream"] == true {
        let chunk = json!({"id":"chatcmpl-managed-local","object":"chat.completion.chunk",
            "created":1,"model":"fleet-managed-local-model","choices":[{"index":0,
            "delta":{"role":"assistant","content":format!("Managed native answer: {prompt}")},
            "finish_reason":null}]});
        let end = json!({"id":"chatcmpl-managed-local","object":"chat.completion.chunk",
            "created":1,"model":"fleet-managed-local-model","choices":[{"index":0,
            "delta":{},"finish_reason":"stop"}]});
        return (
            [("content-type", "text/event-stream")],
            format!("data: {chunk}\n\ndata: {end}\n\ndata: [DONE]\n\n"),
        )
            .into_response();
    }
    Json(json!({"id":"chatcmpl-managed-local","object":"chat.completion","created":1,
        "model":"fleet-managed-local-model","choices":[{"index":0,"message":{
        "role":"assistant","content":format!("Managed native answer: {prompt}")},"finish_reason":"stop"}],
        "usage":{"prompt_tokens":12,"completion_tokens":5,"total_tokens":17}})).into_response()
}

fn configuration(port: u16, soul: &str) -> UpdateAgentConfigRequest {
    UpdateAgentConfigRequest {
        config_json: json!({
            "model":{"default":"fleet-managed-local-model","provider":"custom",
                "api_mode":"chat_completions","base_url":format!("http://127.0.0.1:{port}/v1"),
                "context_length":131072},
            "platform_toolsets":{"api_server":[]},"mcp_servers":{},
            "security":{"tirith_enabled":false,"allow_lazy_installs":false},
            "memory":{"memory_enabled":false,"user_profile_enabled":false,"provider":""},
            "auxiliary":{"title_generation":{"enabled":false},"background_review":{"enabled":false}},
            "telemetry":{"shared_metrics":{"enabled":false}},"agent":{"max_turns":2}
        }),
        soul_md: format!("# {soul}\nAnswer the task without using tools.\n"),
        env_json: json!({"OPENAI_API_KEY":{"secret_ref":"LOCAL_MODEL"},
            "HERMES_HEADLESS":"1","HERMES_DISABLE_LAZY_INSTALLS":"1"}),
    }
}

async fn activate(
    repo: &PostgresFleetRepository,
    id: Uuid,
    owner: Uuid,
    config: UpdateAgentConfigRequest,
) {
    assert!(config.input_errors().is_empty());
    let draft = repo
        .create_config_revision(id, config, owner)
        .await
        .unwrap();
    let errors = draft.snapshot.input_errors();
    assert!(
        errors.is_empty(),
        "native draft validation failed: {errors:?}"
    );
    repo.validate_config_revision(id, draft.revision, errors)
        .await
        .unwrap();
    repo.request_config_activation(id, draft.revision, owner)
        .await
        .unwrap();
    timeout(Duration::from_secs(90), async {
        loop {
            let current = repo.get_config_revision(id, draft.revision).await.unwrap();
            assert_ne!(
                current.state, "failed",
                "native configuration activation failed: {:?}",
                current.last_error
            );
            if current.is_effective && !current.draining {
                break;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("native activation timed out");
}

async fn create_agent(repo: &PostgresFleetRepository, config: &AppConfig, name: &str) -> Agent {
    let agent = repo
        .create_agent(
            CreateAgentRequest {
                kind: AgentKind::Hermes,
                product_role: AgentProductRole::Executor,
                role: AgentRole::Developer,
                sdlc_role: None,
                display_name: name.into(),
                description: None,
                namespace_id: None,
                namespace_name: None,
                workflow_id: None,
                workflow_name: None,
                executor_ids: vec![],
            },
            config,
        )
        .await
        .unwrap();
    FilesystemProvisioner
        .provision(&agent, config)
        .await
        .unwrap();
    for skill in repo.list_agent_skills(agent.id).await.unwrap() {
        if skill.state == domain::SkillState::Enabled {
            repo.update_agent_skill(agent.id, skill.name.clone(), domain::UpdateSkillRequest {
                state: domain::SkillState::Enabled,
                content: Some(format!("---\nname: {}\ndescription: Managed native local-model verification\n---\n# {name}\nAnswer the task without using tools.\n", skill.name)),
            }).await.unwrap();
        }
    }
    repo.update_agent_status(agent.id, AgentStatus::Ready)
        .await
        .unwrap()
}

async fn terminal(repo: &PostgresFleetRepository, session: Uuid) -> domain::SessionAgentRun {
    timeout(Duration::from_secs(120), async {
        loop {
            let runs = repo.list_session_agent_runs(session).await.unwrap();
            for run in &runs {
                assert!(
                    !matches!(
                        run.state,
                        SessionRunState::Failed | SessionRunState::Cancelled
                    ),
                    "managed native run failed: {:?}",
                    run.last_error
                );
            }
            for message in repo.list_session_messages(session).await.unwrap() {
                assert_ne!(
                    message.delivery_state,
                    domain::MessageDeliveryState::Failed,
                    "managed native delivery failed: {:?}",
                    message.delivery_error
                );
            }
            if let Some(run) = runs
                .into_iter()
                .find(|run| run.state == SessionRunState::Completed)
            {
                return run;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("managed native run did not complete")
}

async fn scenario(
    repo: Arc<PostgresFleetRepository>,
    owner: Uuid,
    config: Arc<AppConfig>,
    runtime: &LocalRuntimeSupervisor,
    model: Arc<Model>,
    model_port: u16,
    agents: &mut Vec<Agent>,
) {
    for (name, soul) in [
        ("Managed A", "FLEET_NATIVE_SOUL_A"),
        ("Managed B", "FLEET_NATIVE_SOUL_B"),
    ] {
        let agent = create_agent(&repo, &config, name).await;
        agents.push(agent.clone());
        activate(&repo, agent.id, owner, configuration(model_port, soul)).await;
        let started = runtime.start(&agent).await.unwrap();
        if started.status != AgentStatus::Running {
            let token = infra::agent_runtime_token(&config, agent.id).unwrap();
            for log in repo.list_logs(Some(agent.id), 100).await.unwrap() {
                let safe = log
                    .message
                    .replace(&token, "redacted")
                    .replace(&config.fleet.runtime_token_secret, "redacted")
                    .replace("owned-local-model-fixture", "redacted");
                eprintln!("Native startup {}: {safe}", log.stream);
            }
        }
        assert_eq!(
            started.status,
            AgentStatus::Running,
            "native gateway did not pass readiness"
        );
        let current = repo.get_agent(agent.id).await.unwrap();
        let env_metadata = tokio::fs::metadata(Path::new(&agent.paths.config).join(".env"))
            .await
            .unwrap();
        assert_ne!(
            env_metadata.uid(),
            0,
            "native QA must use the non-root runtime identity"
        );
        assert_eq!(
            env_metadata.mode() & 0o777,
            0o600,
            "runtime dotenv must remain private"
        );
        let pid = current
            .runtime
            .pid
            .expect("native process ownership was not persisted");
        assert_eq!(
            tokio::fs::read_link(format!("/proc/{pid}/cwd"))
                .await
                .unwrap(),
            Path::new(&agent.paths.workspace)
        );
        let environment = tokio::fs::read(format!("/proc/{pid}/environ"))
            .await
            .unwrap();
        let expected = format!("HERMES_HOME={}", agent.paths.config);
        assert!(
            environment
                .split(|b| *b == 0)
                .any(|line| line == expected.as_bytes())
        );
        FilesystemProvisioner
            .verify_effective_configuration(
                &current,
                &config,
                &repo
                    .get_effective_config_revision(agent.id)
                    .await
                    .unwrap()
                    .unwrap(),
            )
            .await
            .unwrap();
    }
    assert_ne!(agents[0].api_port, agents[1].api_port);
    assert_ne!(agents[0].paths.config, agents[1].paths.config);
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let foreign = infra::agent_runtime_token(&config, agents[0].id).unwrap();
    assert_eq!(
        client
            .get(format!(
                "http://127.0.0.1:{}/v1/capabilities",
                agents[1].api_port.unwrap()
            ))
            .bearer_auth(foreign)
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::UNAUTHORIZED
    );

    for (i, agent) in agents.iter().enumerate() {
        let prompt = format!("managed-isolated-prompt-{}-{i}", Uuid::new_v4());
        let session = repo
            .create_session(
                CreateSessionRequest {
                    primary_agent_id: Some(agent.id),
                    agent_id: None,
                    title: "Managed native chat".into(),
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
        let request = CreateSessionMessageRequest {
            body: prompt.clone(),
            author_agent_id: None,
            message_kind: Some(MessageKind::UserPrompt),
            runtime_message_id: None,
            idempotency_key: Some(Uuid::new_v4().to_string()),
        };
        if i == 0 {
            native_prepared_launch::hold_submission(session.id).await;
        }
        let message = repo
            .create_session_message(session.id, request.clone(), owner)
            .await
            .unwrap();
        if i == 0 {
            native_prepared_launch::release_submission(
                repo.clone(),
                config.clone(),
                message.id,
                model.clone(),
                &prompt,
            )
            .await;
        }
        let run = terminal(&repo, session.id).await;
        assert!(run.runtime_run_id.is_some());
        assert!(run.runtime_session_id.is_some());
        let replay = repo
            .create_session_message(session.id, request, owner)
            .await
            .unwrap();
        assert_eq!(replay.id, message.id);
        let before = repo
            .get_hermes_dispatch_intent(message.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(before.state, "accepted");
        let messages = repo.list_session_messages(session.id).await.unwrap();
        let answers = messages
            .iter()
            .filter(|m| m.message_kind == MessageKind::AssistantMessage)
            .collect::<Vec<_>>();
        assert_eq!(answers.len(), 1);
        assert_eq!(answers[0].body, format!("Managed native answer: {prompt}"));
        let seen = model.requests.lock().await;
        let requests = seen
            .get(&prompt)
            .expect("native AIAgent did not use the local model");
        assert_eq!(requests.len(), 1);
        let (own, other) = if i == 0 {
            ("FLEET_NATIVE_SOUL_A", "FLEET_NATIVE_SOUL_B")
        } else {
            ("FLEET_NATIVE_SOUL_B", "FLEET_NATIVE_SOUL_A")
        };
        assert!(
            requests[0].contains(own),
            "native loaded SOUL was not observed by the model"
        );
        assert!(
            !requests[0].contains(other),
            "native SOUL crossed an agent boundary"
        );
        drop(seen);
        assert_eq!(
            runtime.restart(agent).await.unwrap().status,
            AgentStatus::Running
        );
        let token = infra::agent_runtime_token(&config, agent.id).unwrap();
        let status: Value = client
            .get(format!(
                "http://127.0.0.1:{}/v1/runs/{}",
                agent.api_port.unwrap(),
                run.runtime_run_id.as_deref().unwrap()
            ))
            .bearer_auth(token)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(status["run_id"], run.runtime_run_id.as_deref().unwrap());
        assert_eq!(
            status["session_id"],
            run.runtime_session_id.as_deref().unwrap()
        );
        assert_eq!(status["status"], "completed");
        let (events, _) = tokio::sync::broadcast::channel(32);
        let _restarted = LocalRuntimeSupervisor::new(config.clone(), repo.clone(), events);
        sleep(Duration::from_secs(6)).await;
        let after = repo
            .get_hermes_dispatch_intent(message.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(after.run.runtime_run_id, before.run.runtime_run_id);
        assert_eq!(after.request_body, before.request_body);
        assert_eq!(model.requests.lock().await.get(&prompt).unwrap().len(), 1);
        assert_eq!(
            repo.list_session_agent_runs(session.id)
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            repo.list_session_messages(session.id).await.unwrap().len(),
            messages.len()
        );
    }
}

#[tokio::test]
#[ignore = "requires exact native Hermes image and disposable PostgreSQL/agent roots"]
async fn managed_native_gateway_isolates_home_soul_messages_and_restart_history() {
    let db = native_database().await;
    let owner = Uuid::new_v4();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO users(id,email,username,display_name,password_hash,is_system_admin,system_role)
         VALUES($1,$2,$3,'Native QA owner','disabled',false,'user')",
        [owner.into(),format!("{owner}@example.test").into(),owner.to_string().into()])).await.unwrap();
    let repo = Arc::new(PostgresFleetRepository::new(db));
    repo.ensure_runtime_templates().await.unwrap();
    let model = Arc::new(Model::default());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let router = Router::new().route("/v1/chat/completions", post(inference))
        .route("/v1/models", get(|| async { Json(json!({"object":"list","data":[{"id":"fleet-managed-local-model","object":"model"}]})) }))
        .with_state(model.clone());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut config = AppConfig::default();
    config.fleet.agents_root = "/tmp/fleet-native-supervisor/agents".into();
    config.fleet.hermes_source = "/opt/hermes".into();
    config.fleet.hermes_command = "/opt/fleet-hermes/bin/hermes".into();
    config.fleet.runtime_token_secret = format!("native-owned-{}", Uuid::new_v4());
    config.fleet.agent_port_base = 29100;
    config.fleet.agent_port_stride = 5;
    config.fleet.project_workflow_url = None;
    let config = native_configuration(config);
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime = LocalRuntimeSupervisor::new(config.clone(), repo.clone(), events);
    let mut agents = Vec::new();
    let result = std::panic::AssertUnwindSafe(scenario(
        repo.clone(),
        owner,
        config.clone(),
        &runtime,
        model.clone(),
        port,
        &mut agents,
    ));
    use futures_util::FutureExt;
    let result = result.catch_unwind().await;
    let mut clean = true;
    for agent in &agents {
        let stopped = runtime.stop(agent).await;
        clean &= stopped.is_ok_and(|r| r.status == AgentStatus::Stopped);
    }
    server.abort();
    let _ = server.await;
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
    assert!(
        clean,
        "managed parent stop could not be confirmed; Compose cleanup remains mandatory"
    );
    assert_eq!(model.requests.lock().await.len(), 2);
    println!(
        "Managed native gateway cases passed: two homes/SOUL/models, cross-token denial, original prepared-claim recovery with a foreign controller held, idempotent messages, terminal mirrors, restart readback, tracked parent stop. No task/PM or process-tree attestation."
    );
}

async fn native_database() -> sea_orm::DatabaseConnection {
    assert_eq!(
        std::env::var("FLEET_NATIVE_SUPERVISOR_TEST").as_deref(),
        Ok("1")
    );
    let url =
        std::env::var("FLEET_TEST_DATABASE_URL").expect("owned native test PostgreSQL is required");
    let database = DatabaseConfig {
        url,
        max_connections: 10,
        min_connections: 1,
        connect_timeout_seconds: 10,
        idle_timeout_seconds: 60,
    };
    infra::run_migrations(database.clone()).await.unwrap();
    infra::connect_database(database).await.unwrap()
}

fn recovery_configuration(secret: String) -> Arc<AppConfig> {
    let mut config = AppConfig::default();
    config.fleet.agents_root = "/tmp/fleet-native-supervisor/recovery/agents".into();
    config.fleet.hermes_source = "/opt/hermes".into();
    config.fleet.hermes_command = "/opt/fleet-hermes/bin/hermes".into();
    config.fleet.runtime_token_secret = secret;
    config.fleet.agent_port_base = 29200;
    config.fleet.agent_port_stride = 5;
    config.fleet.project_workflow_url = None;
    config.fleet.hermes_recovery_extension_enabled = true;
    native_configuration(config)
}

async fn native_observations(root: &Path) -> Vec<Value> {
    match tokio::fs::read_to_string(root.join("native-events.jsonl")).await {
        Ok(body) => body
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => vec![],
        Err(error) => panic!("native observation failed: {error}"),
    }
}

fn dispatch_snapshot(intent: &app::HermesDispatchIntent) -> Value {
    assert_eq!(
        format!("{:x}", Sha256::digest(intent.request_body.as_bytes())),
        intent.request_hash
    );
    json!({"message_id":intent.message_id, "run_id":intent.run.id,
        "session_id":intent.run.session_id,"agent_id":intent.run.agent_id,
        "key":intent.idempotency_key,"sha256":intent.request_hash,
        "origin":intent.origin,"credential_fingerprint":intent.credential_fingerprint,
        "capabilities":intent.capabilities,"submitted_at":intent.submitted_at,
        "recovery_deadline":intent.recovery_deadline})
}

async fn recovery_driver(phase: &str) {
    let root = std::path::PathBuf::from(std::env::var("FLEET_NATIVE_FAULT_ROOT").unwrap());
    let repo = Arc::new(PostgresFleetRepository::new(native_database().await));
    let config = recovery_configuration(std::env::var("FLEET_NATIVE_RECOVERY_SECRET").unwrap());
    let agent_id = std::env::var("FLEET_NATIVE_RECOVERY_AGENT")
        .unwrap()
        .parse()
        .unwrap();
    let agent = repo.get_agent(agent_id).await.unwrap();
    let (events, _) = tokio::sync::broadcast::channel(32);
    let _runtime = LocalRuntimeSupervisor::new(config.clone(), repo.clone(), events);
    if phase == "dispatch" {
        let owner = std::env::var("FLEET_NATIVE_RECOVERY_OWNER")
            .unwrap()
            .parse()
            .unwrap();
        let port = std::env::var("FLEET_NATIVE_RECOVERY_MODEL_PORT")
            .unwrap()
            .parse()
            .unwrap();
        let prompt = std::env::var("FLEET_NATIVE_RECOVERY_PROMPT").unwrap();
        let mut desired = configuration(port, "FLEET_NATIVE_RECOVERY_SOUL");
        desired.config_json["plugins"] =
            json!({"enabled":["fleet-hermes-recovery","fleet-native-discard-ack"]});
        desired.env_json["FLEET_NATIVE_SUPERVISOR_TEST"] = json!("1");
        desired.env_json["FLEET_NATIVE_FAULT_ROOT"] = json!(root.to_str().unwrap());
        activate(&repo, agent.id, owner, desired).await;
        assert_eq!(
            _runtime.start(&agent).await.unwrap().status,
            AgentStatus::Running
        );
        let session = repo
            .create_session(
                CreateSessionRequest {
                    primary_agent_id: Some(agent.id),
                    agent_id: None,
                    title: "Native lost acknowledgement".into(),
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
        let state = timeout(Duration::from_secs(90), async {
            loop {
                if let Some(intent) = repo.get_hermes_dispatch_intent(message.id).await.unwrap() {
                    let observations = native_observations(&root).await;
                    let accepted = observations
                        .iter()
                        .find(|event| event["kind"] == "accepted");
                    let held = observations
                        .iter()
                        .any(|event| event["kind"] == "lookup" && event["blocked"] == true);
                    if let Some(accepted) = accepted.filter(|_| held) {
                        assert_eq!(intent.state, "submitted");
                        assert!(intent.submission_attempted);
                        assert!(intent.run.runtime_run_id.is_none());
                        assert_eq!(intent.run.state, SessionRunState::Pending);
                        assert_eq!(accepted["key"], intent.idempotency_key);
                        assert_eq!(accepted["sha256"], intent.request_hash);
                        assert_eq!(
                            repo.list_session_agent_runs(session.id)
                                .await
                                .unwrap()
                                .len(),
                            1
                        );
                        let messages = repo.list_session_messages(session.id).await.unwrap();
                        assert_eq!(messages.len(), 2);
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
                                .filter(|m| m.message_kind == MessageKind::SystemEvent)
                                .count(),
                            1
                        );
                        assert!(
                            !messages
                                .iter()
                                .any(|m| m.message_kind == MessageKind::AssistantMessage)
                        );
                        assert_eq!(
                            messages
                                .iter()
                                .find(|m| m.id == message.id)
                                .unwrap()
                                .delivery_state,
                            domain::MessageDeliveryState::Pending
                        );
                        return json!({"dispatch":dispatch_snapshot(&intent),
                            "native_run_id":accepted["run_id"], "pid":std::process::id()});
                    }
                }
                sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("native lost-ACK submission/lookup hold was not observed");
        tokio::fs::write(
            root.join("fleet-state.json"),
            serde_json::to_vec(&state).unwrap(),
        )
        .await
        .unwrap();
    } else {
        assert_eq!(phase, "recover");
        let state: Value = serde_json::from_slice(
            &tokio::fs::read(root.join("fleet-state.json"))
                .await
                .unwrap(),
        )
        .unwrap();
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
        let run = terminal(&repo, session).await;
        assert_eq!(
            run.runtime_run_id.as_deref(),
            state["native_run_id"].as_str()
        );
        sleep(Duration::from_secs(6)).await;
        let intent = repo
            .get_hermes_dispatch_intent(message)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(intent.state, "accepted");
        assert!(intent.submission_attempted);
        assert_eq!(dispatch_snapshot(&intent), state["dispatch"]);
        assert_eq!(
            repo.list_session_agent_runs(session).await.unwrap().len(),
            1
        );
        let answers = repo
            .list_session_messages(session)
            .await
            .unwrap()
            .into_iter()
            .filter(|m| m.message_kind == MessageKind::AssistantMessage)
            .collect::<Vec<_>>();
        assert_eq!(answers.len(), 1);
        assert_eq!(
            answers[0].body,
            format!(
                "Managed native answer: {}",
                std::env::var("FLEET_NATIVE_RECOVERY_PROMPT").unwrap()
            )
        );
        tokio::fs::write(root.join("recovered.json"), serde_json::to_vec(&json!({
            "pid":std::process::id(),"native_run_id":run.runtime_run_id,"dispatch":dispatch_snapshot(&intent)
        })).unwrap()).await.unwrap();
    }
    // Simulate Fleet exit without Rust destructors stopping the owned native gateway.
    // Only the disposable Compose namespace is allowed to reap its orphan descendants.
    std::process::exit(0);
}

async fn fleet_child(
    phase: &str,
    agent: &Agent,
    owner: Uuid,
    port: u16,
    secret: &str,
    prompt: &str,
) {
    let mut child = tokio::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "managed_native_lost_ack_recovers_original_run_across_fleet_processes",
            "--test-threads=1",
            "--nocapture",
        ])
        .env("FLEET_NATIVE_RECOVERY_PHASE", phase)
        .env("FLEET_NATIVE_RECOVERY_AGENT", agent.id.to_string())
        .env("FLEET_NATIVE_RECOVERY_OWNER", owner.to_string())
        .env("FLEET_NATIVE_RECOVERY_MODEL_PORT", port.to_string())
        .env("FLEET_NATIVE_RECOVERY_SECRET", secret)
        .env("FLEET_NATIVE_RECOVERY_PROMPT", prompt)
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let result = timeout(Duration::from_secs(210), child.wait())
        .await
        .expect("Fleet native driver deadline expired")
        .unwrap();
    assert!(result.success(), "Fleet native driver failed: {phase}");
}

#[tokio::test]
#[ignore = "requires exact native Hermes/plugin bytes and disposable owned process namespace"]
async fn managed_native_lost_ack_recovers_original_run_across_fleet_processes() {
    if let Ok(phase) = std::env::var("FLEET_NATIVE_RECOVERY_PHASE") {
        recovery_driver(&phase).await;
        unreachable!();
    }
    let db = native_database().await;
    let owner = Uuid::new_v4();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO users(id,email,username,display_name,password_hash,is_system_admin,system_role)
         VALUES($1,$2,$3,'Native recovery owner','disabled',false,'user')",
        [owner.into(),format!("{owner}@example.test").into(),owner.to_string().into()])).await.unwrap();
    let repo = Arc::new(PostgresFleetRepository::new(db));
    repo.ensure_runtime_templates().await.unwrap();
    let secret = format!("owned-native-recovery-{}", Uuid::new_v4());
    let config = recovery_configuration(secret.clone());
    let agent = create_agent(&repo, &config, "Native recovery agent").await;
    let root = std::path::PathBuf::from(std::env::var("FLEET_NATIVE_FAULT_ROOT").unwrap());
    assert_eq!(
        root,
        Path::new("/tmp/fleet-native-supervisor/recovery-fault")
    );
    tokio::fs::create_dir(&root).await.unwrap();
    tokio::fs::write(root.join("hold-lookup"), b"owned QA hold")
        .await
        .unwrap();
    let plugins = Path::new(&agent.paths.config).join("plugins");
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
    tokio::fs::create_dir(&fault).await.unwrap();
    tokio::fs::write(
        fault.join("__init__.py"),
        include_str!("../../../scripts/native_supervisor_live/discard_ack_plugin.py"),
    )
    .await
    .unwrap();
    tokio::fs::write(fault.join("plugin.yaml"), b"name: fleet-native-discard-ack\nversion: 1.0.0\nkind: platform\nplatforms:\n  - api_server\n").await.unwrap();
    let model = Arc::new(Model::default());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let router = Router::new().route("/v1/chat/completions", post(inference))
        .route("/v1/models", get(|| async { Json(json!({"object":"list","data":[{"id":"fleet-managed-local-model","object":"model"}]})) }))
        .with_state(model.clone());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let prompt = format!("managed-native-lost-ack-{}", Uuid::new_v4());
    fleet_child("dispatch", &agent, owner, port, &secret, &prompt).await;
    let state: Value = serde_json::from_slice(
        &tokio::fs::read(root.join("fleet-state.json"))
            .await
            .unwrap(),
    )
    .unwrap();
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    // Recovery is deliberately released only after native terminal readback, so it
    // must not open an SSE consumer when restoring the original accepted mapping.
    timeout(Duration::from_secs(60), async {
        loop {
            let status: Value = client
                .get(format!(
                    "http://127.0.0.1:{}/v1/runs/{}",
                    agent.api_port.unwrap(),
                    state["native_run_id"].as_str().unwrap()
                ))
                .bearer_auth(infra::agent_runtime_token(&config, agent.id).unwrap())
                .send()
                .await
                .unwrap()
                .error_for_status()
                .unwrap()
                .json()
                .await
                .unwrap();
            if status["status"] == "completed" {
                break;
            }
            assert!(!matches!(
                status["status"].as_str(),
                Some("failed" | "cancelled")
            ));
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("native lost-ACK run did not finish");
    tokio::fs::remove_file(root.join("hold-lookup"))
        .await
        .unwrap();
    fleet_child("recover", &agent, owner, port, &secret, &prompt).await;
    let recovered: Value =
        serde_json::from_slice(&tokio::fs::read(root.join("recovered.json")).await.unwrap())
            .unwrap();
    assert_ne!(state["pid"], recovered["pid"]);
    assert_ne!(state["pid"], std::process::id());
    assert_ne!(recovered["pid"], std::process::id());
    assert_eq!(state["native_run_id"], recovered["native_run_id"]);
    let observations = native_observations(&root).await;
    for kind in ["post", "accepted"] {
        assert_eq!(
            observations
                .iter()
                .filter(|event| event["kind"] == kind)
                .count(),
            1
        );
    }
    assert!(!observations.iter().any(|event| event["kind"] == "events"));
    assert!(
        observations
            .iter()
            .any(|event| event["kind"] == "lookup" && event["blocked"] == false)
    );
    let seen = model.requests.lock().await;
    assert_eq!(seen.len(), 1);
    let inference = seen.get(&prompt).unwrap();
    assert_eq!(inference.len(), 1);
    assert!(inference[0].contains("FLEET_NATIVE_RECOVERY_SOUL"));
    drop(seen);
    server.abort();
    let _ = server.await;
    println!(
        "Managed native lost-ACK recovery passed: Fleet PIDs {} -> {}, native run {}, one POST/inference/assistant, immutable journal, original-key lookup, no SSE or redispatch. Orphan gateway cleanup is Compose-owned, not safe-stop attestation. No task/PM admission.",
        state["pid"], recovered["pid"], state["native_run_id"]
    );
}

#[tokio::test]
#[ignore = "requires exact native Hermes image and disposable owned PostgreSQL/agent roots"]
async fn managed_native_run_steer_and_stop_require_native_ack_and_terminal_readback() {
    native_control_scenario(false).await;
}

#[tokio::test]
#[ignore = "requires committed Base control plugin, pinned Hermes and owned PostgreSQL/process namespace"]
async fn managed_native_original_control_outcomes_recover_lost_http_ack() {
    native_control_scenario(true).await;
}

async fn native_control_scenario(outcomes: bool) {
    let db = native_database().await;
    let owner = Uuid::new_v4();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO users(id,email,username,display_name,password_hash,is_system_admin,system_role)
         VALUES($1,$2,$3,'Native controls owner','disabled',false,'user')",
        [owner.into(),format!("{owner}@example.test").into(),owner.to_string().into()])).await.unwrap();
    let repo = Arc::new(PostgresFleetRepository::new(db));
    repo.ensure_runtime_templates().await.unwrap();
    let model = Arc::new(Model::default());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let router = Router::new().route("/v1/chat/completions",post(inference))
        .route("/v1/models",get(|| async { Json(json!({"object":"list","data":[{"id":"fleet-managed-local-model","object":"model"}]})) }))
        .with_state(model.clone());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut config = AppConfig::default();
    config.fleet.agents_root = "/tmp/fleet-native-supervisor/control-agents".into();
    config.fleet.hermes_source = "/opt/hermes".into();
    config.fleet.hermes_command = "/opt/fleet-hermes/bin/hermes".into();
    config.fleet.runtime_token_secret = format!("owned-native-controls-{}", Uuid::new_v4());
    config.fleet.hermes_control_outcome_enabled = outcomes;
    config.fleet.agent_port_base = 29300;
    config.fleet.agent_port_stride = 5;
    config.fleet.project_workflow_url = None;
    let config = native_configuration(config);
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime = LocalRuntimeSupervisor::new(config.clone(), repo.clone(), events);
    let agent = create_agent(&repo, &config, "Native control agent").await;
    let mut desired = configuration(port, "FLEET_NATIVE_CONTROL_SOUL");
    let fault_root = Path::new(&agent.paths.workspace).join("control-outcome-fault");
    if outcomes {
        tokio::fs::create_dir_all(&fault_root).await.unwrap();
        tokio::fs::write(fault_root.join("hold-lookup"), b"owned QA hold")
            .await
            .unwrap();
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
        tokio::fs::create_dir_all(&fault).await.unwrap();
        tokio::fs::write(
            fault.join("__init__.py"),
            include_str!("../../../scripts/native_supervisor_live/control_fault_plugin.py"),
        )
        .await
        .unwrap();
        tokio::fs::write(fault.join("plugin.yaml"), b"name: fleet-native-discard-control-ack\nversion: 1.0.0\nkind: platform\nplatforms:\n  - api_server\n").await.unwrap();
        desired.config_json["plugins"] =
            json!({"enabled":["fleet-hermes-controls","fleet-native-discard-control-ack"]});
        desired.env_json["FLEET_NATIVE_SUPERVISOR_TEST"] = json!("1");
        desired.env_json["FLEET_NATIVE_CONTROL_FAULT_ROOT"] = json!(fault_root);
    }
    activate(&repo, agent.id, owner, desired).await;
    assert_eq!(
        runtime.start(&agent).await.unwrap().status,
        AgentStatus::Running
    );
    if outcomes {
        let held = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap()
            .get(format!(
                "http://127.0.0.1:{}/fleet/v1/controls/lookup",
                agent.api_port.unwrap()
            ))
            .bearer_auth(infra::agent_runtime_token(&config, agent.id).unwrap())
            .send()
            .await
            .unwrap();
        assert_eq!(held.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            held.json::<Value>().await.unwrap()["error"],
            "owned_qa_control_hold"
        );
    }
    let prompt = format!("managed-control-prompt-{}", Uuid::new_v4());
    let session = repo
        .create_session(
            CreateSessionRequest {
                primary_agent_id: Some(agent.id),
                agent_id: None,
                title: "Native controls".into(),
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
            let runs = repo.list_session_agent_runs(session.id).await.unwrap();
            assert_eq!(runs.len(), 1);
            if runs[0].state == SessionRunState::Running
                && model.requests.lock().await.contains_key(&prompt)
            {
                break runs[0].clone();
            }
            assert!(!matches!(
                runs[0].state,
                SessionRunState::Completed | SessionRunState::Failed | SessionRunState::Cancelled
            ));
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("native model barrier/pinned running run not observed");
    let before = repo
        .get_hermes_dispatch_intent(message.id)
        .await
        .unwrap()
        .unwrap();
    let steer_actor = domain::RuntimeControlActor {
        user_id: owner,
        idempotency_key: Uuid::new_v4().to_string(),
    };
    let mut steered = runtime
        .steer_run(
            &agent,
            &run,
            domain::SteerSessionRunRequest {
                input: "Keep this synthetic QA scope.".into(),
            },
            steer_actor.clone(),
        )
        .await
        .unwrap();
    if outcomes {
        assert!(!steered.accepted);
        let id = steered.command.as_ref().unwrap().id;
        assert!(
            repo.get_runtime_control_outcome(id)
                .await
                .unwrap()
                .is_some()
        );
        tokio::fs::remove_file(fault_root.join("hold-lookup"))
            .await
            .unwrap();
        await_native_control_ack(&repo, session.id, id).await;
        steered = runtime
            .steer_run(
                &agent,
                &run,
                domain::SteerSessionRunRequest {
                    input: "Keep this synthetic QA scope.".into(),
                },
                steer_actor.clone(),
            )
            .await
            .unwrap();
    }
    assert!(steered.accepted);
    assert_eq!(steered.state, SessionRunState::Running);
    let steer_receipt = steered.command.as_ref().unwrap();
    assert_eq!(
        steer_receipt.state,
        domain::RuntimeControlState::Acknowledged
    );
    let replay = runtime
        .steer_run(
            &agent,
            &run,
            domain::SteerSessionRunRequest {
                input: "Keep this synthetic QA scope.".into(),
            },
            steer_actor,
        )
        .await
        .unwrap();
    assert!(replay.accepted);
    assert_eq!(replay.command.unwrap().id, steer_receipt.id);
    assert_eq!(
        repo.list_runtime_controls(session.id, run.id)
            .await
            .unwrap()
            .len(),
        1
    );
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let status: Value = client
        .get(format!(
            "http://127.0.0.1:{}/v1/runs/{}",
            agent.api_port.unwrap(),
            run.runtime_run_id.as_deref().unwrap()
        ))
        .bearer_auth(infra::agent_runtime_token(&config, agent.id).unwrap())
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(status["last_event"], "run.steered");
    assert_eq!(
        status["session_id"],
        run.runtime_session_id.as_deref().unwrap()
    );
    let stop_actor = domain::RuntimeControlActor {
        user_id: owner,
        idempotency_key: Uuid::new_v4().to_string(),
    };
    if outcomes {
        tokio::fs::write(fault_root.join("hold-lookup"), b"owned QA hold")
            .await
            .unwrap();
    }
    let stopped = runtime
        .stop_run(&agent, &run, stop_actor.clone())
        .await
        .unwrap();
    assert_eq!(stopped.accepted, !outcomes);
    assert_ne!(stopped.state, SessionRunState::Completed);
    let stop_receipt = stopped.command.as_ref().unwrap();
    if !outcomes {
        assert_eq!(
            stop_receipt.state,
            domain::RuntimeControlState::Acknowledged
        );
    }
    assert_eq!(
        runtime
            .stop_run(&agent, &run, stop_actor.clone())
            .await
            .unwrap()
            .command
            .unwrap()
            .id,
        stop_receipt.id
    );
    assert!(
        repo.list_session_messages(session.id)
            .await
            .unwrap()
            .iter()
            .all(|m| m.message_kind != MessageKind::AssistantMessage)
    );
    model.control_release.notify_one();
    let finished = timeout(Duration::from_secs(90), async {
        loop {
            let current = repo.get_session_agent_run(run.id).await.unwrap();
            assert_ne!(
                current.state,
                SessionRunState::Completed,
                "interrupted native run cannot complete SDLC"
            );
            if matches!(
                current.state,
                SessionRunState::Failed | SessionRunState::Cancelled
            ) {
                break current;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("native stop terminal readback did not finish");
    assert_eq!(finished.runtime_run_id, run.runtime_run_id);
    if outcomes {
        repo.reconcile_runtime_controls().await.unwrap();
        let original = repo
            .get_runtime_control_outcome(stop_receipt.id)
            .await
            .unwrap()
            .unwrap()
            .context;
        let terminal_receipt = repo
            .get_runtime_control(session.id, stop_receipt.id)
            .await
            .unwrap();
        assert_eq!(
            terminal_receipt.state,
            domain::RuntimeControlState::TerminalObserved
        );
        let pid = repo.get_agent(agent.id).await.unwrap().runtime.pid;
        assert_eq!(
            runtime.restart(&agent).await.unwrap().status,
            AgentStatus::Running
        );
        assert_ne!(repo.get_agent(agent.id).await.unwrap().runtime.pid, pid);
        tokio::fs::remove_file(fault_root.join("hold-lookup"))
            .await
            .unwrap();
        await_native_control_ack(&repo, session.id, stop_receipt.id).await;
        let witnessed = repo
            .get_runtime_control(session.id, stop_receipt.id)
            .await
            .unwrap();
        assert_eq!(witnessed.state, terminal_receipt.state);
        assert_eq!(witnessed.updated_at, terminal_receipt.updated_at);
        assert_eq!(witnessed.acknowledgement.as_deref(), Some("stopping"));
        assert_eq!(
            repo.get_runtime_control_outcome(stop_receipt.id)
                .await
                .unwrap()
                .unwrap()
                .context,
            original
        );
        let observations = tokio::fs::read_to_string(fault_root.join("control-events.jsonl"))
            .await
            .unwrap();
        let rows: Vec<Value> = observations
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        for (operation, receipt) in [("steer", steer_receipt), ("stop", stop_receipt)] {
            let saved = repo
                .get_runtime_control_outcome(receipt.id)
                .await
                .unwrap()
                .unwrap();
            let calls: Vec<_> = rows
                .iter()
                .filter(|row| row["kind"] == "post" && row["operation"] == operation)
                .collect();
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0]["key"], receipt.id.to_string());
            assert_eq!(calls[0]["sha256"], saved.context["request_sha256"]);
            assert_eq!(
                rows.iter()
                    .filter(|row| row["kind"] == "native_ack" && row["operation"] == operation)
                    .count(),
                1
            );
        }
        assert!(
            rows.iter()
                .any(|row| row["kind"] == "lookup" && row["held"] == false)
        );
    }
    assert_eq!(
        runtime
            .stop_run(&agent, &run, stop_actor)
            .await
            .unwrap()
            .command
            .unwrap()
            .id,
        stop_receipt.id
    );
    assert_eq!(
        repo.list_runtime_controls(session.id, run.id)
            .await
            .unwrap()
            .len(),
        2
    );
    let after = repo
        .get_hermes_dispatch_intent(message.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(dispatch_snapshot(&after), dispatch_snapshot(&before));
    assert_eq!(
        repo.list_session_agent_runs(session.id)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(model.requests.lock().await.get(&prompt).unwrap().len(), 1);
    assert!(
        runtime
            .steer_run(
                &agent,
                &run,
                domain::SteerSessionRunRequest {
                    input: "late guidance must fail".into()
                },
                domain::RuntimeControlActor {
                    user_id: owner,
                    idempotency_key: Uuid::new_v4().to_string()
                }
            )
            .await
            .is_err()
    );
    assert_eq!(
        runtime.stop(&agent).await.unwrap().status,
        AgentStatus::Stopped
    );
    server.abort();
    let _ = server.await;
    println!(
        "Managed native controls passed: real AIAgent steer ACK/status, interrupt ACK separated from terminal, same journal/run, one inference, no false completed state, late steer denied. No approval, OS-descendant safe-stop or task/PM admission proof."
    );
    if outcomes {
        println!(
            "Original control outcomes verified: actual native steer/stop ACK transport loss, one POST per UUID, GET-only recovery, gateway PID restart with unchanged store epoch, terminal history preserved. No Fleet OS restart, approval decision recovery or safe descendants claimed."
        );
    }
}

async fn await_native_control_ack(repo: &PostgresFleetRepository, session: Uuid, id: Uuid) {
    timeout(Duration::from_secs(90), async {
        loop {
            if repo
                .get_runtime_control(session, id)
                .await
                .unwrap()
                .acknowledgement
                .is_some()
            {
                break;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("original native control ACK was not recovered");
}
