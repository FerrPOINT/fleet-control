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
use shared::{AppConfig, DatabaseConfig};
use std::os::unix::fs::MetadataExt;
use std::{collections::HashMap, path::Path, sync::Arc};
use tokio::{
    sync::Mutex,
    time::{Duration, sleep, timeout},
};
use uuid::Uuid;

#[derive(Default)]
struct Model {
    requests: Mutex<HashMap<String, Vec<String>>>,
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
        .requests
        .lock()
        .await
        .entry(prompt.clone())
        .or_default()
        .push(system);
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
        let message = repo
            .create_session_message(session.id, request.clone(), owner)
            .await
            .unwrap();
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
    let db = infra::connect_database(database).await.unwrap();
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
    let config = Arc::new(config);
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
        "Managed native gateway cases passed: two homes/SOUL/models, cross-token denial, idempotent messages, terminal mirrors, restart readback, tracked parent stop. No task/PM or process-tree attestation."
    );
}
