//! QA-only actual Docker/Hermes driver. No producer receipts or admission are fabricated.
use app::{
    AgentProvisioner, FleetRepository, RuntimeSupervisor, container_activation::Phase,
    container_runtime::ContainerLaunch,
};
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
use infra::{FilesystemProvisioner, PostgresFleetRepository, runtime::LocalRuntimeSupervisor};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use shared::{AppConfig, DatabaseConfig, config::ContainerControlConfig};
use std::{
    collections::BTreeMap,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::Path,
    sync::Arc,
};
use tokio::{
    sync::Mutex,
    time::{Duration, sleep, timeout},
};
use uuid::Uuid;

const MODEL_PORT: u16 = 31200;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Proof {
    source_commit: String,
    engine_id: String,
    agents_volume: String,
    model_host: String,
    control: ContainerControlConfig,
}

#[derive(Serialize, Deserialize)]
struct Saved {
    config: AppConfig,
    owner: Uuid,
    agents: Vec<Agent>,
    launches: Vec<ContainerLaunch>,
    unknown_session: Uuid,
    unknown_message: Uuid,
    unknown_request: CreateSessionMessageRequest,
    totals: Value,
    posts: Vec<Value>,
    ack_loss: Vec<Value>,
}

#[derive(Default)]
struct Model {
    calls: Mutex<BTreeMap<String, Vec<String>>>,
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
    if prompt.starts_with("hold-live-") {
        model.release.notified().await;
    }
    if body["stream"] == true {
        let chunk = json!({"id":"chatcmpl-live","object":"chat.completion.chunk","created":1,
            "model":"fleet-container-local-model","choices":[{"index":0,"delta":{"role":"assistant",
            "content":format!("Container answer: {prompt}")},"finish_reason":null}]});
        let end = json!({"id":"chatcmpl-live","object":"chat.completion.chunk","created":1,
            "model":"fleet-container-local-model","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]});
        return (
            [("content-type", "text/event-stream")],
            format!("data: {chunk}\n\ndata: {end}\n\ndata: [DONE]\n\n"),
        )
            .into_response();
    }
    Json(
        json!({"id":"chatcmpl-live","object":"chat.completion","created":1,
        "model":"fleet-container-local-model","choices":[{"index":0,"message":{"role":"assistant",
        "content":format!("Container answer: {prompt}")},"finish_reason":"stop"}],
        "usage":{"prompt_tokens":12,"completion_tokens":5,"total_tokens":17}}),
    )
    .into_response()
}

async fn put(path: &Path, bytes: &[u8]) {
    use tokio::io::AsyncWriteExt;
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true).mode(0o600);
    let mut file = options
        .open(path)
        .await
        .expect("QA private file creation failed");
    file.write_all(bytes).await.unwrap();
    file.sync_all().await.unwrap();
}

async fn evidence(name: &str, value: &Value) {
    put(
        &Path::new("/evidence").join(name),
        &serde_json::to_vec_pretty(value).unwrap(),
    )
    .await;
}

fn configuration(host: &str, soul: &str) -> UpdateAgentConfigRequest {
    UpdateAgentConfigRequest {
        config_json: json!({"model":{"default":"fleet-container-local-model","provider":"custom",
            "api_mode":"chat_completions","base_url":format!("http://{host}:{MODEL_PORT}/v1"),"context_length":131072},
            "platform_toolsets":{"api_server":[]},"mcp_servers":{},
            "security":{"tirith_enabled":false,"allow_lazy_installs":false},
            "memory":{"memory_enabled":false,"user_profile_enabled":false,"provider":""},
            "auxiliary":{"title_generation":{"enabled":false},"background_review":{"enabled":false}},
            "telemetry":{"shared_metrics":{"enabled":false}},"agent":{"max_turns":2}}),
        soul_md: format!("# {soul}\nAnswer without tools.\n"),
        env_json: json!({"OPENAI_API_KEY":{"secret_ref":"LOCAL_MODEL"},"HERMES_DISABLE_LAZY_INSTALLS":"1"}),
    }
}

async fn create(
    repo: &PostgresFleetRepository,
    config: &AppConfig,
    proof: &Proof,
    index: usize,
) -> Agent {
    let a = repo
        .create_agent(
            CreateAgentRequest {
                kind: AgentKind::Hermes,
                product_role: AgentProductRole::Executor,
                role: AgentRole::Developer,
                sdlc_role: None,
                display_name: format!("Native QA {index}"),
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
    FilesystemProvisioner.provision(&a, config).await.unwrap();
    for skill in repo.list_agent_skills(a.id).await.unwrap() {
        repo.update_agent_skill(
            a.id,
            skill.name,
            domain::UpdateSkillRequest {
                state: domain::SkillState::Disabled,
                content: None,
            },
        )
        .await
        .unwrap();
    }
    let root = Path::new(&a.paths.config);
    // Explicit bootstrap fixture only: no effective revision/ACK/launch is inserted into SQL.
    // The original provisioned .env and marker stay byte-identical.
    let env = tokio::fs::read(root.join(".env")).await.unwrap();
    let mut initial = configuration(
        &proof.model_host,
        if index == 0 {
            "LIVE_SOUL_A"
        } else {
            "LIVE_SOUL_B"
        },
    );
    initial.config_json["terminal"] = json!({"cwd":"/workspace"});
    tokio::fs::write(
        root.join("config.yaml"),
        serde_json::to_vec(&initial.config_json).unwrap(),
    )
    .await
    .unwrap();
    tokio::fs::write(root.join("SOUL.md"), initial.soul_md)
        .await
        .unwrap();
    for path in [root.join("config.yaml"), root.join("SOUL.md")] {
        tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .await
            .unwrap();
    }
    assert!(tokio::fs::read(root.join(".env")).await.unwrap() == env);
    for name in ["hermes-container.py", "hermes_fixture.py"] {
        let from = if name == "hermes-container.py" {
            "/base-runtime/deploy/fleet-hermes-container-launch.py"
        } else {
            "/qa/hermes_fixture.py"
        };
        tokio::fs::copy(from, Path::new(&a.paths.runtime).join(name))
            .await
            .unwrap();
    }
    for area in [&a.paths.config, &a.paths.workspace] {
        put(
            &Path::new(area).join("qa-owner"),
            a.id.to_string().as_bytes(),
        )
        .await;
    }
    repo.update_agent_status(a.id, AgentStatus::Ready)
        .await
        .unwrap()
}

async fn launch(repo: &PostgresFleetRepository, agent: Uuid) -> ContainerLaunch {
    repo.get_container_launch(agent).await.unwrap().unwrap()
}

async fn docker(args: &[String]) -> Value {
    let output = tokio::process::Command::new("docker")
        .args(["--context", "default"])
        .args(args)
        .output()
        .await
        .expect("Original Docker command unavailable");
    assert!(output.status.success(), "Original Docker readback failed");
    serde_json::from_slice(&output.stdout).expect("Original Docker JSON unavailable")
}

async fn inspected(l: &ContainerLaunch) -> Value {
    let id = l.prepared.container.registration.container_id.clone();
    docker(&["container".into(), "inspect".into(), id]).await[0].clone()
}

async fn native_posts(a: &Agent) -> Value {
    serde_json::from_slice(
        &tokio::fs::read(Path::new(&a.paths.logs).join("qa-native-posts.json"))
            .await
            .unwrap(),
    )
    .unwrap()
}

async fn native_ack_loss(a: &Agent) -> Value {
    let path = Path::new(&a.paths.logs).join("qa-native-ack-loss.json");
    if !tokio::fs::try_exists(&path).await.unwrap() {
        return json!({});
    }
    serde_json::from_slice(&tokio::fs::read(path).await.unwrap()).unwrap()
}

async fn isolation(
    repo: &PostgresFleetRepository,
    config: &AppConfig,
    proof: &Proof,
    a: &Agent,
) -> Value {
    let l = launch(repo, a.id).await;
    let binding = &l.prepared.container;
    let registration = &binding.registration;
    assert_eq!(registration.contract_version, 3);
    assert_eq!(registration.engine.id, proof.engine_id);
    let mapped = binding.mapped.as_ref().unwrap();
    assert_eq!(mapped.mapping.volume_name, proof.agents_volume);
    assert_eq!(mapped.mapping.local_root, config.fleet.agents_root);
    assert_eq!(
        mapped.mapping.controller,
        serde_json::from_value::<app::container_runtime::MappingController>(
            serde_json::to_value(proof.control.mapping_controller.as_ref().unwrap()).unwrap()
        )
        .unwrap()
    );
    let physical = inspected(&l).await;
    assert_eq!(physical["Id"], registration.container_id);
    assert_eq!(
        physical["Image"],
        proof.control.provisioning.as_ref().unwrap().image_id
    );
    assert_eq!(physical["State"]["Running"], true);
    assert_eq!(
        physical["State"]["Pid"],
        l.snapshot.as_ref().unwrap()["init_pid"]
    );
    assert_eq!(
        physical["State"]["StartedAt"],
        l.snapshot.as_ref().unwrap()["started_at"]
    );
    let mounts = physical["Mounts"].as_array().unwrap();
    assert_eq!(mounts.len(), 4);
    let specifications = physical["HostConfig"]["Mounts"].as_array().unwrap();
    assert_eq!(specifications.len(), 4);
    for area in ["runtime", "config", "workspace", "logs"] {
        let mount = mounts
            .iter()
            .find(|m| m["Destination"] == format!("/{area}"))
            .unwrap();
        assert_eq!(mount["Name"], proof.agents_volume);
        assert_eq!(mount["Type"], "volume");
        assert_eq!(mount["RW"], area != "runtime");
        let matching = specifications
            .iter()
            .filter(|m| m["Target"] == format!("/{area}"))
            .collect::<Vec<_>>();
        assert_eq!(matching.len(), 1);
        let specification = matching[0];
        assert_eq!(specification["Type"], "volume");
        assert_eq!(specification["Source"], proof.agents_volume);
        assert_eq!(specification["Target"], format!("/{area}"));
        let read_only = specification
            .get("ReadOnly")
            .cloned()
            .unwrap_or(json!(false));
        assert_eq!(read_only, area == "runtime");
        assert_eq!(
            specification["VolumeOptions"],
            json!({"NoCopy":true,"Subpath":format!("{}/{area}",a.name)})
        );
    }
    let service = binding.policy["service"].as_str().unwrap().to_owned();
    let probe = "import os,json,pathlib; print(json.dumps(dict(home=os.environ['HOME'],hermes_home=os.environ['HERMES_HOME'],cwd=os.getcwd(),config_owner=pathlib.Path('/config/qa-owner').read_text(),workspace_owner=pathlib.Path('/workspace/qa-owner').read_text(),other=pathlib.Path('/agents').exists())))";
    let view = docker(&[
        "compose".into(),
        "-p".into(),
        proof.control.provisioning.as_ref().unwrap().project.clone(),
        "-f".into(),
        binding.compose.clone(),
        "exec".into(),
        "-T".into(),
        service,
        "/opt/hermes/.venv/bin/python".into(),
        "-B".into(),
        "-c".into(),
        probe.into(),
    ])
    .await;
    assert_eq!(
        view,
        json!({"home":"/config","hermes_home":"/config","cwd":"/workspace",
        "config_owner":a.id.to_string(),"workspace_owner":a.id.to_string(),"other":false})
    );
    let marker = Path::new(&a.paths.runtime)
        .parent()
        .unwrap()
        .join(".fleet-agent.json");
    let metadata = tokio::fs::metadata(&marker).await.unwrap();
    assert_eq!(metadata.uid(), 999);
    assert_eq!(metadata.mode() & 0o777, 0o600);
    let value: Value = serde_json::from_slice(&tokio::fs::read(&marker).await.unwrap()).unwrap();
    assert_eq!(value["id"], a.id.to_string());
    json!({"agent_id":a.id,"container_id":registration.container_id,"generation":registration.generation,
        "pid":physical["State"]["Pid"],"started_at":physical["State"]["StartedAt"],
        "marker_sha256":hex::encode(Sha256::digest(tokio::fs::read(marker).await.unwrap()))})
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
                title: "Native acceptance free chat".into(),
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
    assert_eq!(session.task_bound, Some(false));
    assert!(
        repo.get_task_chat_binding(session.id)
            .await
            .unwrap()
            .is_none()
    );
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
    assert_eq!(
        message.request_payload_hash,
        Some(hex::encode(Sha256::digest(
            serde_json::to_vec(&serde_json::to_value(&request).unwrap()).unwrap()
        )))
    );
    (session.id, message, request)
}

async fn answer(
    repo: &PostgresFleetRepository,
    model: &Model,
    session: Uuid,
    prompt: &str,
    own: &str,
    foreign: &str,
) -> domain::SessionMessage {
    timeout(Duration::from_secs(150), async {
        loop {
            let runs = repo.list_session_agent_runs(session).await.unwrap();
            assert!(runs.len() <= 1);
            if let Some(run) = runs.first() {
                assert!(!matches!(
                    run.state,
                    SessionRunState::Failed | SessionRunState::Cancelled
                ));
                if run.state == SessionRunState::Completed {
                    assert!(run.runtime_run_id.is_some() && run.runtime_session_id.is_some());
                    break;
                }
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("Genuine Hermes completion not observed");
    let messages = repo.list_session_messages(session).await.unwrap();
    let answers = messages
        .iter()
        .filter(|m| m.message_kind == MessageKind::AssistantMessage)
        .collect::<Vec<_>>();
    assert_eq!(answers.len(), 1);
    assert_eq!(answers[0].body, format!("Container answer: {prompt}"));
    let calls = model.calls.lock().await;
    let systems = calls.get(prompt).unwrap();
    assert_eq!(systems.len(), 1);
    assert!(systems[0].contains(own) && !systems[0].contains(foreign));
    (*answers[0]).clone()
}

async fn request(
    repo: &PostgresFleetRepository,
    agent: Uuid,
    owner: Uuid,
    config: UpdateAgentConfigRequest,
) -> i64 {
    assert!(config.input_errors().is_empty());
    let draft = repo
        .create_config_revision(agent, config, owner)
        .await
        .unwrap();
    repo.validate_config_revision(agent, draft.revision, draft.snapshot.input_errors())
        .await
        .unwrap();
    repo.request_config_activation(agent, draft.revision, owner)
        .await
        .unwrap();
    draft.revision
}

async fn settled(
    repo: &PostgresFleetRepository,
    a: Uuid,
    revision: i64,
    phase: Phase,
) -> app::container_activation::Activation {
    timeout(Duration::from_secs(210), async {
        loop {
            if let Some(record) = repo.get_container_activation(a, revision).await.unwrap() {
                if record.phase == phase {
                    return record;
                }
                assert!(
                    !record.phase.terminal(),
                    "Unexpected terminal activation result"
                );
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("Activation did not settle with original proof")
}

async fn files(a: &Agent) -> BTreeMap<String, Vec<u8>> {
    let mut result = BTreeMap::new();
    for name in ["config.yaml", "SOUL.md", ".env"] {
        result.insert(
            name.into(),
            tokio::fs::read(Path::new(&a.paths.config).join(name))
                .await
                .unwrap(),
        );
    }
    result
}

async fn totals(db: &DatabaseConnection) -> Value {
    db.query_one(Statement::from_string(DatabaseBackend::Postgres,
        "SELECT jsonb_build_object('launches',(SELECT count(*) FROM runtime_container_launches),
        'activations',(SELECT count(*) FROM runtime_container_activations),
        'preparations',(SELECT count(*) FROM runtime_container_preparations),
        'runs',(SELECT count(*) FROM session_agent_runs),'journal',(SELECT count(*) FROM hermes_dispatch_journal)) AS value"))
        .await.unwrap().unwrap().try_get("", "value").unwrap()
}

async fn unknown_held(repo: &PostgresFleetRepository, session: Uuid, message: Uuid) {
    let intent = repo
        .get_hermes_dispatch_intent(message)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(intent.state, "submitted");
    assert!(intent.submission_attempted && intent.run.runtime_run_id.is_none());
    assert!(!matches!(
        intent.run.state,
        SessionRunState::Completed | SessionRunState::Failed | SessionRunState::Cancelled
    ));
    assert_eq!(
        repo.list_session_agent_runs(session).await.unwrap().len(),
        1
    );
    assert_eq!(
        repo.list_session_messages(session)
            .await
            .unwrap()
            .iter()
            .filter(|m| m.message_kind == MessageKind::AssistantMessage)
            .count(),
        0
    );
}

async fn initial(
    repo: Arc<PostgresFleetRepository>,
    db: &DatabaseConnection,
    config: Arc<AppConfig>,
    proof: &Proof,
    runtime: &LocalRuntimeSupervisor,
    model: &Model,
    owner: Uuid,
) {
    let mut agents = Vec::new();
    let mut identities = Vec::new();
    for index in 0..2 {
        let a = create(&repo, &config, proof, index).await;
        assert_eq!(
            runtime.start(&a).await.unwrap().status,
            AgentStatus::Running
        );
        assert_eq!(
            runtime
                .health(&repo.get_agent(a.id).await.unwrap())
                .await
                .unwrap()
                .status,
            AgentStatus::Running
        );
        identities.push(isolation(&repo, &config, proof, &a).await);
        agents.push(a);
    }
    assert_ne!(agents[0].paths.config, agents[1].paths.config);
    assert_ne!(identities[0]["container_id"], identities[1]["container_id"]);
    let peer = launch(&repo, agents[1].id).await;
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let denied = client
        .get(format!(
            "{}/v1/capabilities",
            peer.origin.as_deref().unwrap()
        ))
        .bearer_auth(infra::agent_runtime_token(&config, agents[0].id).unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status(), reqwest::StatusCode::UNAUTHORIZED);
    for (index, a) in agents.iter().enumerate() {
        let prompt = format!("isolation-live-{index}-{}", Uuid::new_v4());
        let (session, message, request) = send(&repo, a.id, owner, &prompt).await;
        let (own, other) = if index == 0 {
            ("LIVE_SOUL_A", "LIVE_SOUL_B")
        } else {
            ("LIVE_SOUL_B", "LIVE_SOUL_A")
        };
        let acknowledged = answer(&repo, model, session, &prompt, own, other).await;
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
        let replayed = answer(&repo, model, session, &prompt, own, other).await;
        assert_eq!(replayed.id, acknowledged.id);
        assert_eq!(replayed.body, acknowledged.body);
    }
    let a = &agents[0];
    let original = launch(&repo, a.id).await;
    let before = files(a).await;
    let prompt = format!("hold-live-{}", Uuid::new_v4());
    let (session, _, _) = send(&repo, a.id, owner, &prompt).await;
    timeout(Duration::from_secs(60), async {
        while !model.calls.lock().await.contains_key(&prompt) {
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    let revision = request(
        &repo,
        a.id,
        owner,
        configuration(&proof.model_host, "LIVE_SOUL_A_NEW"),
    )
    .await;
    sleep(Duration::from_secs(4)).await;
    assert!(repo.agent_is_draining(a.id).await.unwrap());
    assert!(files(a).await == before);
    assert_eq!(
        serde_json::to_value(launch(&repo, a.id).await).unwrap(),
        serde_json::to_value(&original).unwrap()
    );
    model.release.notify_one();
    answer(
        &repo,
        model,
        session,
        &prompt,
        "LIVE_SOUL_A",
        "LIVE_SOUL_A_NEW",
    )
    .await;
    let activation = settled(&repo, a.id, revision, Phase::Committed).await;
    assert_eq!(
        activation.previous_stop.as_ref().unwrap()["observation"],
        "namespace_exited"
    );
    assert_eq!(inspected(&original).await["State"]["Running"], false);
    let effective = repo
        .get_effective_config_revision(a.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(effective.revision, revision);
    assert!(!effective.draining);
    FilesystemProvisioner
        .verify_effective_configuration(a, &config, &effective)
        .await
        .unwrap();
    let replacement = launch(&repo, a.id).await;
    assert_ne!(
        replacement.prepared.container.registration.generation,
        original.prepared.container.registration.generation
    );
    assert_ne!(
        replacement.prepared.container.registration.container_id,
        original.prepared.container.registration.container_id
    );
    assert_eq!(
        serde_json::to_value(launch(&repo, agents[1].id).await).unwrap(),
        serde_json::to_value(&peer).unwrap()
    );
    let prompt = format!("new-generation-live-{}", Uuid::new_v4());
    let (session, _, _) = send(&repo, a.id, owner, &prompt).await;
    answer(
        &repo,
        model,
        session,
        &prompt,
        "LIVE_SOUL_A_NEW",
        "LIVE_SOUL_B",
    )
    .await;
    let prior_files = files(a).await;
    let revision = request(
        &repo,
        a.id,
        owner,
        configuration(&proof.model_host, "QA_READINESS_DELAY_18"),
    )
    .await;
    let start = tokio::time::Instant::now();
    let rollback = settled(&repo, a.id, revision, Phase::RolledBack).await;
    assert!(start.elapsed() >= Duration::from_secs(60));
    assert_eq!(
        rollback.previous_stop.as_ref().unwrap()["observation"],
        "namespace_exited"
    );
    assert_eq!(
        rollback.candidate_stop.as_ref().unwrap()["observation"],
        "namespace_exited"
    );
    assert_eq!(inspected(&replacement).await["State"]["Running"], false);
    let candidate = rollback.candidate.as_ref().unwrap();
    assert_eq!(inspected(candidate).await["State"]["Running"], false);
    assert!(files(a).await == prior_files);
    let restored = launch(&repo, a.id).await;
    assert_ne!(
        restored.prepared.container.registration.generation,
        candidate.prepared.container.registration.generation
    );
    assert_ne!(
        restored.prepared.container.registration.generation,
        replacement.prepared.container.registration.generation
    );
    assert_eq!(
        restored.prepared.configuration_revision,
        Some(effective.revision)
    );
    assert_eq!(
        repo.get_config_revision(a.id, revision)
            .await
            .unwrap()
            .state,
        "failed"
    );
    assert!(!repo.agent_is_draining(a.id).await.unwrap());
    FilesystemProvisioner
        .verify_effective_configuration(a, &config, &effective)
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(launch(&repo, agents[1].id).await).unwrap(),
        serde_json::to_value(&peer).unwrap()
    );
    let prompt = format!("after-rollback-live-{}", Uuid::new_v4());
    let (session, _, _) = send(&repo, a.id, owner, &prompt).await;
    answer(
        &repo,
        model,
        session,
        &prompt,
        "LIVE_SOUL_A_NEW",
        "QA_READINESS_DELAY_18",
    )
    .await;
    let prompt = format!("unknown-live-{}", Uuid::new_v4());
    let (session, message, request) = send(&repo, a.id, owner, &prompt).await;
    timeout(Duration::from_secs(60), async {
        loop {
            if repo
                .get_hermes_dispatch_intent(message.id)
                .await
                .unwrap()
                .is_some_and(|i| i.state == "submitted")
                && model.calls.lock().await.contains_key(&prompt)
            {
                break;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("Native lost ACK was not exercised");
    sleep(Duration::from_secs(6)).await;
    unknown_held(&repo, session, message.id).await;
    assert_eq!(model.calls.lock().await.get(&prompt).unwrap().len(), 1);
    assert_eq!(model.calls.lock().await.len(), 6);
    let launches = vec![launch(&repo, a.id).await, launch(&repo, agents[1].id).await];
    for (a, original) in agents.iter().zip(&identities) {
        assert_eq!(
            isolation(&repo, &config, proof, a).await["marker_sha256"],
            original["marker_sha256"]
        );
    }
    let posts = vec![
        native_posts(&agents[0]).await,
        native_posts(&agents[1]).await,
    ];
    assert!(
        posts
            .iter()
            .all(|p| p.as_object().unwrap().values().all(|n| n == 1))
    );
    assert_eq!(
        posts
            .iter()
            .map(|p| p.as_object().unwrap().len())
            .sum::<usize>(),
        6
    );
    let ack_loss = vec![
        native_ack_loss(&agents[0]).await,
        native_ack_loss(&agents[1]).await,
    ];
    assert_eq!(ack_loss[0].as_object().unwrap().len(), 1);
    assert_eq!(
        ack_loss[0].as_object().unwrap().values().next(),
        Some(&json!(1))
    );
    assert!(ack_loss[1].as_object().unwrap().is_empty());
    let saved = Saved {
        config: (*config).clone(),
        owner,
        agents,
        launches,
        unknown_session: session,
        unknown_message: message.id,
        unknown_request: request,
        totals: totals(db).await,
        posts,
        ack_loss,
    };
    put(
        Path::new("/controller/qa-state.json"),
        &serde_json::to_vec(&saved).unwrap(),
    )
    .await;
    evidence("before-restart.json", &json!({"state":"awaiting_physical_controller_restart",
        "source_commit":proof.source_commit,"actual_rust_supervisor":true,"genuine_hermes":true,
        "isolation":identities,"controlled_model_prompts":6,"bootstrap_fixture_not_config_admission":true,
        "matrix":{"two_agent_preparation":"accepted","isolation_health_chat":"accepted",
            "idempotent_transcript":"accepted","drain_new_generation":"accepted","readiness_exact_rollback":"accepted",
            "peer_unchanged":"accepted","unknown_ack":"held"}})).await;
    loop {
        sleep(Duration::from_secs(1)).await;
    }
}

async fn recovered(
    repo: Arc<PostgresFleetRepository>,
    db: &DatabaseConnection,
    config: Arc<AppConfig>,
    proof: &Proof,
    runtime: &LocalRuntimeSupervisor,
    model: &Model,
    saved: Saved,
) {
    let mut report: Value = serde_json::from_slice(
        &tokio::fs::read("/evidence/before-restart.json")
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(report["source_commit"], proof.source_commit);
    // New process, same physical controller CID. Observe over multiple worker/reconcile intervals.
    let mut peer_healthy = false;
    for _ in 0..15 {
        sleep(Duration::from_secs(5)).await;
        let peer = repo.get_agent(saved.agents[1].id).await.unwrap();
        peer_healthy |= runtime
            .health(&peer)
            .await
            .is_ok_and(|r| r.status == AgentStatus::Running);
        unknown_held(&repo, saved.unknown_session, saved.unknown_message).await;
        assert_eq!(totals(db).await, saved.totals);
        assert!(model.calls.lock().await.is_empty());
        for (index, a) in saved.agents.iter().enumerate() {
            let current = launch(&repo, a.id).await;
            assert_eq!(
                serde_json::to_value(&current).unwrap(),
                serde_json::to_value(&saved.launches[index]).unwrap()
            );
            assert_eq!(native_posts(a).await, saved.posts[index]);
            assert_eq!(native_ack_loss(a).await, saved.ack_loss[index]);
            assert_eq!(
                inspected(&current).await["State"]["Pid"],
                current.snapshot.as_ref().unwrap()["init_pid"]
            );
        }
    }
    assert_eq!(
        repo.create_session_message(saved.unknown_session, saved.unknown_request, saved.owner)
            .await
            .unwrap()
            .id,
        saved.unknown_message
    );
    sleep(Duration::from_secs(6)).await;
    unknown_held(&repo, saved.unknown_session, saved.unknown_message).await;
    assert_eq!(totals(db).await, saved.totals);
    assert!(model.calls.lock().await.is_empty());
    for (index, a) in saved.agents.iter().enumerate() {
        assert_eq!(native_posts(a).await, saved.posts[index]);
        assert_eq!(native_ack_loss(a).await, saved.ack_loss[index]);
    }
    report["matrix"]["controller_restart"] = json!(if peer_healthy { "accepted" } else { "held" });
    report["matrix"]["unknown_ack_after_restart"] = json!("held");
    report["no_second_native_post_or_generation"] = json!(true);
    report["runtime_ready"] = json!(false);
    report["sdlc_completion"] = json!(false);
    report["state"] = json!(if peer_healthy {
        "scenario_passed"
    } else {
        "held_controller_recovery"
    });
    evidence("live-report.json", &report).await;
    // Physical stop is checked separately; held unknown delivery is never promoted to completion.
    for a in &saved.agents {
        assert_eq!(
            runtime
                .stop(&repo.get_agent(a.id).await.unwrap())
                .await
                .unwrap()
                .status,
            AgentStatus::Stopped
        );
        assert_eq!(
            inspected(&launch(&repo, a.id).await).await["State"]["Running"],
            false
        );
    }
    evidence(
        "stop-report.json",
        &json!({"state":"original_namespaces_exited","agents":2,"runtime_ready":false}),
    )
    .await;
    let _ = config;
    assert!(
        peer_healthy,
        "Original controller recovery is held, not accepted"
    );
}

#[tokio::main]
async fn main() {
    assert_eq!(std::env::var("QA_NATIVE_EXECUTE").as_deref(), Ok("1"));
    let mode = std::env::args()
        .nth(1)
        .expect("Explicit initial/recover mode required");
    assert!(matches!(mode.as_str(), "initial" | "recover"));
    let proof: Proof =
        serde_json::from_slice(&tokio::fs::read("/proof/controller.json").await.unwrap()).unwrap();
    assert_eq!(
        proof.source_commit,
        std::env::var("QA_SOURCE_COMMIT").unwrap()
    );
    let database = DatabaseConfig {
        url: std::env::var("FLEET_TEST_DATABASE_URL").unwrap(),
        max_connections: 10,
        min_connections: 1,
        connect_timeout_seconds: 10,
        idle_timeout_seconds: 60,
    };
    infra::run_migrations(database.clone()).await.unwrap();
    let db = infra::connect_database(database.clone()).await.unwrap();
    let repo = Arc::new(PostgresFleetRepository::new(
        infra::connect_database(database).await.unwrap(),
    ));
    repo.ensure_runtime_templates().await.unwrap();
    let saved: Option<Saved> = if mode == "recover" {
        let path = Path::new("/controller/qa-state.json");
        let metadata = tokio::fs::symlink_metadata(path).await.unwrap();
        assert!(metadata.is_file() && metadata.mode() & 0o777 == 0o600 && metadata.uid() == 999);
        Some(serde_json::from_slice(&tokio::fs::read(path).await.unwrap()).unwrap())
    } else {
        None
    };
    let owner = saved.as_ref().map(|s| s.owner).unwrap_or_else(Uuid::new_v4);
    if saved.is_none() {
        db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO users(id,email,username,display_name,password_hash,is_system_admin,system_role)
            VALUES($1,$2,$3,'Native QA owner','disabled',false,'user')",
            [owner.into(),format!("{owner}@example.invalid").into(),owner.to_string().into()])).await.unwrap();
    }
    let mut config = saved.as_ref().map(|s| s.config.clone()).unwrap_or_default();
    if saved.is_none() {
        config.fleet.agents_root = "/agents".into();
        config.fleet.runtime_token_secret = Uuid::new_v4().to_string();
        config.fleet.hermes_source = "genuine-bbaf7af-immutable-image".into();
        config.fleet.agent_port_base = 29100;
        config.fleet.agent_port_stride = 5;
        config.fleet.project_workflow_url = None;
        config.fleet.project_workflow_catalog_token = None;
        config.fleet.container_control = Some(proof.control.clone());
    }
    let config = Arc::new(config);
    let model = Arc::new(Model::default());
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", MODEL_PORT))
        .await
        .unwrap();
    let router = Router::new()
        .route("/v1/chat/completions", post(inference))
        .with_state(model.clone());
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let (events, _) = tokio::sync::broadcast::channel(64);
    let runtime = LocalRuntimeSupervisor::new(config.clone(), repo.clone(), events);
    if let Some(saved) = saved {
        recovered(repo, &db, config, &proof, &runtime, &model, saved).await;
    } else {
        initial(repo, &db, config, &proof, &runtime, &model, owner).await;
    }
}
