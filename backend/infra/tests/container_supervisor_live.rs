//! Real Docker/Hermes acceptance; only the owned Compose driver may opt in.
#![cfg(target_os = "linux")]

use app::{AgentProvisioner, FleetRepository, RuntimeSupervisor};
use axum::{
    Json, Router,
    extract::State,
    response::{IntoResponse, Response},
    routing::post,
};
use domain::{
    Agent, AgentKind, AgentProductRole, AgentRole, AgentStatus, CreateAgentRequest,
    CreateSessionMessageRequest, CreateSessionRequest, MessageKind, SessionRunState,
    UpdateAgentConfigRequest,
};
use futures_util::FutureExt;
use infra::runtime::container_control::{ContainerControl, ContainerLaunchFiles, ControlSource};
use infra::{FilesystemProvisioner, PostgresFleetRepository, runtime::LocalRuntimeSupervisor};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde::Deserialize;
use serde_json::{Value, json};
use shared::{AppConfig, DatabaseConfig, config::ContainerControlConfig};
use std::{collections::HashMap, os::unix::fs::MetadataExt, path::Path, sync::Arc};
use tokio::{
    sync::Mutex,
    time::{Duration, sleep, timeout},
};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Proof {
    control: ContainerControlConfig,
    model_host: String,
    engine_id: String,
    agents_volume: String,
}

#[derive(Default)]
struct Model {
    calls: Mutex<HashMap<String, Vec<String>>>,
    release: tokio::sync::Notify,
}

async fn inference(State(model): State<Arc<Model>>, Json(body): Json<Value>) -> Response {
    assert_eq!(body["model"], "fleet-container-local-model");
    let messages = body["messages"].as_array().unwrap();
    let prompt = messages
        .iter()
        .rev()
        .find(|m| m["role"] == "user")
        .and_then(|m| m["content"].as_str())
        .unwrap()
        .to_owned();
    let system = messages
        .iter()
        .filter(|m| m["role"] == "system")
        .filter_map(|m| m["content"].as_str())
        .collect::<Vec<_>>()
        .join("\n");
    model
        .calls
        .lock()
        .await
        .entry(prompt.clone())
        .or_default()
        .push(system);
    if prompt.starts_with("hold-container-") {
        model.release.notified().await;
    }
    if body["stream"] == true {
        let chunk = json!({"id":"chatcmpl-container-qa","object":"chat.completion.chunk",
            "created":1,"model":"fleet-container-local-model","choices":[{"index":0,
            "delta":{"role":"assistant","content":format!("Container answer: {prompt}")},"finish_reason":null}]});
        let end = json!({"id":"chatcmpl-container-qa","object":"chat.completion.chunk",
            "created":1,"model":"fleet-container-local-model","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]});
        return (
            [("content-type", "text/event-stream")],
            format!("data: {chunk}\n\ndata: {end}\n\ndata: [DONE]\n\n"),
        )
            .into_response();
    }
    Json(json!({"id":"chatcmpl-container-qa","object":"chat.completion","created":1,
        "model":"fleet-container-local-model","choices":[{"index":0,"message":{
            "role":"assistant","content":format!("Container answer: {prompt}")},"finish_reason":"stop"}],
        "usage":{"prompt_tokens":12,"completion_tokens":5,"total_tokens":17}})).into_response()
}

fn configuration(host: &str, port: u16, soul: &str) -> UpdateAgentConfigRequest {
    UpdateAgentConfigRequest {
        config_json: json!({
            "model":{"default":"fleet-container-local-model","provider":"custom",
                "api_mode":"chat_completions","base_url":format!("http://{host}:{port}/v1"),
                "context_length":131072},
            "platform_toolsets":{"api_server":[]},"mcp_servers":{},
            "security":{"tirith_enabled":false,"allow_lazy_installs":false},
            "memory":{"memory_enabled":false,"user_profile_enabled":false,"provider":""},
            "auxiliary":{"title_generation":{"enabled":false},"background_review":{"enabled":false}},
            "telemetry":{"shared_metrics":{"enabled":false}},"agent":{"max_turns":2}
        }),
        soul_md: format!("# {soul}\nAnswer without tools.\n"),
        env_json: json!({"OPENAI_API_KEY":{"secret_ref":"LOCAL_MODEL"},
            "HERMES_HEADLESS":"1","HERMES_DISABLE_LAZY_INSTALLS":"1"}),
    }
}

async fn request_activation(
    repo: &PostgresFleetRepository,
    agent: Uuid,
    owner: Uuid,
    input: UpdateAgentConfigRequest,
) -> i64 {
    assert!(input.input_errors().is_empty());
    let draft = repo
        .create_config_revision(agent, input, owner)
        .await
        .unwrap();
    let errors = draft.snapshot.input_errors();
    assert!(errors.is_empty());
    repo.validate_config_revision(agent, draft.revision, errors)
        .await
        .unwrap();
    repo.request_config_activation(agent, draft.revision, owner)
        .await
        .unwrap();
    draft.revision
}

async fn activated(repo: &PostgresFleetRepository, agent: Uuid, revision: i64) {
    timeout(Duration::from_secs(150), async {
        loop {
            let current = repo.get_config_revision(agent, revision).await.unwrap();
            assert_ne!(
                current.state, "failed",
                "configuration failed: {:?}",
                current.last_error
            );
            if current.is_effective && !current.draining {
                break;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("container activation timed out");
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
    tokio::fs::copy(
        "/base-control/deploy/fleet-hermes-container-launch.py",
        Path::new(&agent.paths.runtime).join("hermes-container.py"),
    )
    .await
    .unwrap();
    for skill in repo.list_agent_skills(agent.id).await.unwrap() {
        if skill.state == domain::SkillState::Enabled {
            repo.update_agent_skill(agent.id, skill.name.clone(), domain::UpdateSkillRequest {
                state: domain::SkillState::Enabled,
                content: Some(format!("---\nname: {}\ndescription: Container local-model QA\n---\nAnswer without tools.\n", skill.name)),
            }).await.unwrap();
        }
    }
    repo.update_agent_status(agent.id, AgentStatus::Ready)
        .await
        .unwrap()
}

async fn send(
    repo: &PostgresFleetRepository,
    agent: Uuid,
    owner: Uuid,
    prompt: &str,
) -> (Uuid, domain::SessionMessage, CreateSessionMessageRequest) {
    let session = repo
        .create_session(
            CreateSessionRequest {
                primary_agent_id: Some(agent),
                agent_id: None,
                title: "Container acceptance chat".into(),
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
        body: prompt.into(),
        author_agent_id: None,
        message_kind: Some(MessageKind::UserPrompt),
        runtime_message_id: None,
        idempotency_key: Some(Uuid::new_v4().to_string()),
    };
    let message = repo
        .create_session_message(session.id, request.clone(), owner)
        .await
        .unwrap();
    (session.id, message, request)
}

async fn terminal(repo: &PostgresFleetRepository, session: Uuid) -> domain::SessionAgentRun {
    timeout(Duration::from_secs(120), async {
        loop {
            for run in repo.list_session_agent_runs(session).await.unwrap() {
                assert!(
                    !matches!(
                        run.state,
                        SessionRunState::Failed | SessionRunState::Cancelled
                    ),
                    "container run failed: {:?}",
                    run.last_error
                );
                if run.state == SessionRunState::Completed {
                    return run;
                }
            }
            for message in repo.list_session_messages(session).await.unwrap() {
                assert_ne!(
                    message.delivery_state,
                    domain::MessageDeliveryState::Failed,
                    "container delivery failed: {:?}",
                    message.delivery_error
                );
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("real container run did not complete")
}

async fn assert_answer(
    repo: &PostgresFleetRepository,
    model: &Model,
    session: Uuid,
    prompt: &str,
    own: &str,
    other: &str,
) -> domain::SessionAgentRun {
    let run = terminal(repo, session).await;
    assert!(run.runtime_run_id.is_some() && run.runtime_session_id.is_some());
    let messages = repo.list_session_messages(session).await.unwrap();
    let answers = messages
        .iter()
        .filter(|m| m.message_kind == MessageKind::AssistantMessage)
        .collect::<Vec<_>>();
    assert_eq!(answers.len(), 1);
    assert_eq!(answers[0].body, format!("Container answer: {prompt}"));
    let calls = model.calls.lock().await;
    let systems = calls
        .get(prompt)
        .expect("real Hermes did not call the controlled model");
    assert_eq!(systems.len(), 1);
    assert!(systems[0].contains(own));
    assert!(!systems[0].contains(other));
    run
}

async fn scenario(
    repo: &PostgresFleetRepository,
    runtime: &LocalRuntimeSupervisor,
    config: &AppConfig,
    endpoint: (&Proof, u16),
    model: &Model,
    owner: Uuid,
    agents: &mut Vec<Agent>,
) {
    let (proof, port) = endpoint;
    for (name, soul) in [
        ("Container Developer", "CONTAINER_SOUL_A"),
        ("Container Tester", "CONTAINER_SOUL_B"),
    ] {
        let agent = create_agent(repo, config, name).await;
        agents.push(agent.clone());
        let revision = request_activation(
            repo,
            agent.id,
            owner,
            configuration(&proof.model_host, port, soul),
        )
        .await;
        activated(repo, agent.id, revision).await;
        assert!(
            repo.get_open_runtime_launch(agent.id)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            runtime.start(&agent).await.unwrap().status,
            AgentStatus::Running
        );
        let launch = repo
            .get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .unwrap();
        let binding = launch.binding.container.unwrap();
        assert_eq!(binding.policy["contract_version"], 3);
        assert_eq!(binding.registration.engine.id, proof.engine_id);
        assert_eq!(
            binding.mount_mapping.unwrap().volume_name,
            proof.agents_volume
        );
        for area in ["runtime", "config", "workspace", "logs"] {
            let directory = Path::new(&config.fleet.agents_root)
                .join(&agent.name)
                .join(area);
            assert_eq!(tokio::fs::metadata(directory).await.unwrap().uid(), 999);
        }
        let dotenv = tokio::fs::metadata(Path::new(&agent.paths.config).join(".env"))
            .await
            .unwrap();
        assert_eq!(dotenv.uid(), 999);
        assert_eq!(dotenv.mode() & 0o777, 0o600);
        assert_eq!(binding.policy["mounts"].as_array().unwrap().len(), 4);
        assert_eq!(binding.policy["mounts"][0]["read_only"], true);
    }
    assert_eq!(
        agents.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(),
        vec!["agent1", "agent2"]
    );
    assert_ne!(agents[0].paths.config, agents[1].paths.config);
    assert_ne!(agents[0].api_port, agents[1].api_port);
    let peer = repo
        .get_open_runtime_launch(agents[1].id)
        .await
        .unwrap()
        .unwrap()
        .binding
        .container
        .unwrap();
    let control = ContainerControl::new(
        proof.control.python.clone().into(),
        ControlSource {
            root: proof.control.base_root.clone().into(),
            sha256: peer.source_sha256.clone(),
        },
        peer.context.clone(),
    )
    .unwrap();
    let files = ContainerLaunchFiles {
        policy: peer.policy,
        compose: peer.compose.into(),
        journal: peer.journal.into(),
        stop_journal: peer.stop_journal.into(),
        mount_mapping: peer.mount_mapping,
        mapping_file: peer.mapping_file.map(Into::into),
    };
    let endpoint = control.endpoint(&files, &peer.registration).await.unwrap();
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let response = client
        .get(format!(
            "http://{}:{}/v1/capabilities",
            endpoint,
            agents[1].api_port.unwrap()
        ))
        .bearer_auth(infra::agent_runtime_token(config, agents[0].id).unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
    for (i, agent) in agents.iter().enumerate() {
        let prompt = format!("container-isolation-{i}-{}", Uuid::new_v4());
        let (session, message, request) = send(repo, agent.id, owner, &prompt).await;
        let (own, other) = if i == 0 {
            ("CONTAINER_SOUL_A", "CONTAINER_SOUL_B")
        } else {
            ("CONTAINER_SOUL_B", "CONTAINER_SOUL_A")
        };
        assert_answer(repo, model, session, &prompt, own, other).await;
        assert_eq!(
            repo.create_session_message(session, request, owner)
                .await
                .unwrap()
                .id,
            message.id
        );
        sleep(Duration::from_secs(3)).await;
        assert_eq!(model.calls.lock().await.get(&prompt).unwrap().len(), 1);
        assert_eq!(
            repo.list_session_agent_runs(session).await.unwrap().len(),
            1
        );
    }

    let agent = &agents[0];
    let original = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap();
    let peer_original = repo
        .get_open_runtime_launch(agents[1].id)
        .await
        .unwrap()
        .unwrap();
    let prompt = format!("hold-container-{}", Uuid::new_v4());
    let (session, _, _) = send(repo, agent.id, owner, &prompt).await;
    timeout(Duration::from_secs(60), async {
        while !model.calls.lock().await.contains_key(&prompt) {
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    let revision = request_activation(
        repo,
        agent.id,
        owner,
        configuration(&proof.model_host, port, "CONTAINER_SOUL_A_NEW"),
    )
    .await;
    sleep(Duration::from_secs(4)).await;
    assert!(repo.agent_is_draining(agent.id).await.unwrap());
    assert_ne!(
        repo.get_effective_config_revision(agent.id)
            .await
            .unwrap()
            .unwrap()
            .revision,
        revision
    );
    assert_eq!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .unwrap()
            .binding
            .id,
        original.binding.id
    );
    assert!(
        !tokio::fs::read_to_string(Path::new(&agent.paths.config).join("SOUL.md"))
            .await
            .unwrap()
            .contains("CONTAINER_SOUL_A_NEW")
    );
    model.release.notify_one();
    assert_answer(
        repo,
        model,
        session,
        &prompt,
        "CONTAINER_SOUL_A",
        "CONTAINER_SOUL_A_NEW",
    )
    .await;
    activated(repo, agent.id, revision).await;
    let replacement = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap();
    assert_ne!(replacement.binding.id, original.binding.id);
    assert_ne!(
        replacement
            .binding
            .container
            .as_ref()
            .unwrap()
            .registration
            .container_id,
        original
            .binding
            .container
            .as_ref()
            .unwrap()
            .registration
            .container_id
    );
    assert_eq!(replacement.binding.phase, "activation");
    assert_eq!(
        repo.get_open_runtime_launch(agents[1].id)
            .await
            .unwrap()
            .unwrap()
            .binding
            .id,
        peer_original.binding.id
    );
    let prompt = format!("container-new-soul-{}", Uuid::new_v4());
    let (session, _, _) = send(repo, agent.id, owner, &prompt).await;
    assert_answer(
        repo,
        model,
        session,
        &prompt,
        "CONTAINER_SOUL_A_NEW",
        "CONTAINER_SOUL_B",
    )
    .await;
    FilesystemProvisioner
        .verify_effective_configuration(
            &repo.get_agent(agent.id).await.unwrap(),
            config,
            &repo
                .get_effective_config_revision(agent.id)
                .await
                .unwrap()
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        runtime.restart(&agents[1]).await.unwrap().status,
        AgentStatus::Running
    );
    let restarted = repo
        .get_open_runtime_launch(agents[1].id)
        .await
        .unwrap()
        .unwrap();
    assert_ne!(restarted.binding.id, peer_original.binding.id);
    let prompt = format!("container-peer-restart-{}", Uuid::new_v4());
    let (session, _, _) = send(repo, agents[1].id, owner, &prompt).await;
    assert_answer(
        repo,
        model,
        session,
        &prompt,
        "CONTAINER_SOUL_B",
        "CONTAINER_SOUL_A_NEW",
    )
    .await;
}

#[tokio::test]
#[ignore = "requires original-Engine owned Compose controller, mapped volume and real Hermes image"]
async fn real_container_supervisor_isolates_chat_and_drains_configuration_replacement() {
    assert_eq!(
        std::env::var("FLEET_CONTAINER_SUPERVISOR_TEST").as_deref(),
        Ok("1")
    );
    let proof: Proof =
        serde_json::from_slice(&tokio::fs::read("/qa/controller-proof.json").await.unwrap())
            .unwrap();
    let database = DatabaseConfig {
        url: std::env::var("FLEET_TEST_DATABASE_URL").unwrap(),
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
         VALUES($1,$2,$3,'Container QA owner','disabled',false,'user')",
        [owner.into(), format!("{owner}@example.test").into(), owner.to_string().into()])).await.unwrap();
    let repo = Arc::new(PostgresFleetRepository::new(db));
    repo.ensure_runtime_templates().await.unwrap();
    let model = Arc::new(Model::default());
    let listener = tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let router = Router::new()
        .route("/v1/chat/completions", post(inference))
        .with_state(model.clone());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut config = AppConfig::default();
    config.fleet.agents_root = "/agents".into();
    config.fleet.controller_root = "/controller".into();
    config.fleet.hermes_source = "immutable-pinned-Hermes-image".into();
    config.fleet.runtime_token_secret = format!("container-owned-{}", Uuid::new_v4());
    config.fleet.agent_port_base = 29100;
    config.fleet.agent_port_stride = 5;
    config.fleet.project_workflow_url = None;
    config.fleet.container_control = Some(proof.control.clone());
    let config = Arc::new(config);
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime = LocalRuntimeSupervisor::new(config.clone(), repo.clone(), events);
    let mut agents = Vec::new();
    let result = std::panic::AssertUnwindSafe(scenario(
        &repo,
        &runtime,
        &config,
        (&proof, port),
        &model,
        owner,
        &mut agents,
    ))
    .catch_unwind()
    .await;
    let mut clean = true;
    for agent in &agents {
        clean &= runtime
            .stop(agent)
            .await
            .is_ok_and(|r| r.status == AgentStatus::Stopped);
    }
    server.abort();
    let _ = server.await;
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
    assert!(
        clean,
        "original namespace termination unconfirmed; outer Compose cleanup is mandatory"
    );
    assert_eq!(model.calls.lock().await.len(), 5);
    let report = json!({"state":"passed","actual_rust_supervisor":true,"actual_docker_hermes":true,
        "controlled_model":true,"sdlc_acceptance":false,"agents":2,"controller_uid":999,
        "mapped_volume":proof.agents_volume,"engine_id":proof.engine_id,
        "isolated_soul_and_mirror":true,"cross_agent_token_denied":true,
        "idempotent_messages":true,"drain_before_file_effects":true,
        "loaded_replacement_soul":true,"peer_unchanged":true,"fresh_restart_generation":true,
        "confirmed_namespace_stop":true,"model_prompts":5});
    tokio::fs::write(
        "/evidence/live-report.json",
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .await
    .unwrap();
}
