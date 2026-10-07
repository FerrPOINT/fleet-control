//! Actual candidate-running/settlement crash; not autonomous SDLC acceptance.
use super::*;
use app::runtime_launch::RuntimeLaunchBinding;
use controller_recovery::{hash, read, save};
use serde::Serialize;

const OLD: &str = "ACTIVATION_LAST_WORKING_SOUL";
const NEW: &str = "ACTIVATION_UNCOMMITTED_SOUL";
const PEER: &str = "ACTIVATION_UNCHANGED_PEER_SOUL";
const TRIGGER: &str = "fleet_qa_activation_pause";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Context {
    owner: Uuid,
    agent: Uuid,
    peer: Uuid,
    effective: i64,
    candidate: i64,
    model_port: u16,
    settlement_backend_pid: i32,
    settlement_backend_start: String,
    original: RuntimeLaunchBinding,
    replacement: RuntimeLaunchBinding,
    peer_launch: RuntimeLaunchBinding,
    peer_files_sha256: String,
    backup_sha256: String,
    journal_sha256: String,
}

async fn file_hash(agent: &Agent) -> String {
    let mut files = Vec::new();
    for name in ["config.yaml", "SOUL.md", ".env"] {
        let bytes = tokio::fs::read(Path::new(&agent.paths.config).join(name))
            .await
            .unwrap();
        files.push((name, hex::encode(Sha256::digest(bytes))));
    }
    hash(&files)
}

async fn model_server(port: u16, model: Arc<Model>) -> (u16, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    let router = Router::new()
        .route("/v1/chat/completions", post(inference))
        .with_state(model);
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    (port, server)
}

async fn prepare(
    repo: Arc<PostgresFleetRepository>,
    db: sea_orm::DatabaseConnection,
    proof: Proof,
) {
    let owner = Uuid::new_v4();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO users(id,email,username,display_name,password_hash,is_system_admin,system_role)
         VALUES($1,$2,$3,'Activation QA owner','disabled',false,'user')",
        [owner.into(),format!("{owner}@example.test").into(),owner.to_string().into()])).await.unwrap();
    repo.ensure_runtime_templates().await.unwrap();
    let model = Arc::new(Model::default());
    let (port, _server) = model_server(0, model.clone()).await;
    let mut config = AppConfig::default();
    config.fleet.agents_root = "/agents".into();
    config.fleet.controller_root = "/controller".into();
    config.fleet.hermes_source = "immutable-pinned-Hermes-image".into();
    config.fleet.runtime_token_secret = format!("activation-owned-{}", Uuid::new_v4());
    config.fleet.agent_port_base = 29100;
    config.fleet.agent_port_stride = 5;
    config.fleet.project_workflow_url = None;
    config.fleet.container_control = Some(proof.control.clone());
    let config = Arc::new(config);
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime = LocalRuntimeSupervisor::new(config.clone(), repo.clone(), events);
    let agent = create_agent(&repo, &config, "Activation interrupted agent").await;
    let peer = create_agent(&repo, &config, "Activation unchanged peer").await;
    let mut effective = 0;
    for (target, soul) in [(&agent, OLD), (&peer, PEER)] {
        let revision = request_activation(
            &repo,
            target.id,
            owner,
            configuration(&proof.model_host, port, soul),
        )
        .await;
        activated(&repo, target.id, revision).await;
        assert_eq!(
            runtime.start(target).await.unwrap().status,
            AgentStatus::Running
        );
        if target.id == agent.id {
            effective = revision;
        }
    }
    let original = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap()
        .binding;
    let peer_launch = repo
        .get_open_runtime_launch(peer.id)
        .await
        .unwrap()
        .unwrap()
        .binding;
    let backup_sha256 = file_hash(&agent).await;
    let peer_files_sha256 = file_hash(&peer).await;
    // Pause the real success transaction; the outer Compose driver kills this process.
    db.execute(Statement::from_string(
        DatabaseBackend::Postgres,
        format!(
            "CREATE FUNCTION {TRIGGER}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
         IF NEW.agent_id='{id}'::uuid AND NEW.state='active' THEN PERFORM pg_sleep(600); END IF;
         RETURN NEW; END $$",
            id = agent.id
        ),
    ))
    .await
    .unwrap();
    db.execute(Statement::from_string(DatabaseBackend::Postgres, format!(
        "CREATE TRIGGER {TRIGGER} BEFORE UPDATE ON agent_config_revisions FOR EACH ROW EXECUTE FUNCTION {TRIGGER}()"))).await.unwrap();
    let candidate = request_activation(
        &repo,
        agent.id,
        owner,
        configuration(&proof.model_host, port, NEW),
    )
    .await;
    let (settlement_backend_pid, settlement_backend_start) = timeout(Duration::from_secs(180), async {
        loop {
            let row = db.query_one(Statement::from_string(DatabaseBackend::Postgres,
                "SELECT pid,backend_start::text AS backend_start FROM pg_stat_activity WHERE datname=current_database() AND wait_event='PgSleep' AND query LIKE 'UPDATE agent_config_revisions%'".to_owned())).await.unwrap();
            if let Some(row) = row {
                break (row.try_get::<i32>("", "pid").unwrap(), row.try_get::<String>("", "backend_start").unwrap());
            }
            sleep(Duration::from_millis(200)).await;
        }
    }).await.expect("actual candidate did not reach paused settlement");
    let replacement = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap()
        .binding;
    assert_ne!(replacement.id, original.id);
    assert_eq!(replacement.phase, "activation");
    assert_eq!(replacement.configuration_revision, Some(candidate));
    let current = repo.get_config_revision(agent.id, candidate).await.unwrap();
    assert_eq!(current.state, "activating");
    assert!(current.draining && !current.is_effective);
    assert_eq!(
        repo.get_effective_config_revision(agent.id)
            .await
            .unwrap()
            .unwrap()
            .revision,
        effective
    );
    let journal = tokio::fs::read(format!("/controller/{}.activation.json", agent.id))
        .await
        .unwrap();
    let signed: Value = serde_json::from_slice(&journal).unwrap();
    assert_eq!(signed["payload"]["contract_version"], 3);
    assert_eq!(
        signed["payload"]["identity"]["original_launch"]["id"],
        json!(original.id)
    );
    assert!(model.calls.lock().await.is_empty());
    save("activation-config.json", config.as_ref()).await;
    save(
        "activation-context.json",
        &Context {
            owner,
            agent: agent.id,
            peer: peer.id,
            effective,
            candidate,
            model_port: port,
            settlement_backend_pid,
            settlement_backend_start,
            original,
            replacement,
            peer_launch,
            peer_files_sha256,
            backup_sha256,
            journal_sha256: hex::encode(Sha256::digest(journal)),
        },
    )
    .await;
    save(
        "activation-ready.json",
        &json!({"state":"ready","candidate_running":true,
        "settlement_paused":true,"actual_model_calls":0}),
    )
    .await;
    std::future::pending::<()>().await;
}

async fn recover(repo: Arc<PostgresFleetRepository>, db: sea_orm::DatabaseConnection) {
    let mut config: AppConfig = read("activation-config.json").await;
    let context: Context = read("activation-context.json").await;
    let journal_path = format!("/controller/{}.activation.json", context.agent);
    assert_eq!(
        hex::encode(Sha256::digest(
            tokio::fs::read(&journal_path).await.unwrap()
        )),
        context.journal_sha256
    );
    // The driver proved physical Fleet death; release only its artificial QA barrier.
    db.query_all(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT pg_cancel_backend(pid) FROM pg_stat_activity
         WHERE datname=current_database() AND pid=$1 AND backend_start::text=$2
         AND wait_event='PgSleep' AND query LIKE 'UPDATE agent_config_revisions%'",
        [
            context.settlement_backend_pid.into(),
            context.settlement_backend_start.clone().into(),
        ],
    ))
    .await
    .unwrap();
    timeout(Duration::from_secs(15), async {
        loop {
            let row = db
                .query_one(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database()
                 AND pid=$1 AND backend_start::text=$2 AND xact_start IS NOT NULL) AS unsettled",
                    [
                        context.settlement_backend_pid.into(),
                        context.settlement_backend_start.clone().into(),
                    ],
                ))
                .await
                .unwrap()
                .unwrap();
            if !row.try_get::<bool>("", "unsettled").unwrap() {
                break;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("killed Fleet settlement transaction did not roll back");
    db.execute(Statement::from_string(
        DatabaseBackend::Postgres,
        format!("DROP TRIGGER {TRIGGER} ON agent_config_revisions"),
    ))
    .await
    .unwrap();
    db.execute(Statement::from_string(
        DatabaseBackend::Postgres,
        format!("DROP FUNCTION {TRIGGER}()"),
    ))
    .await
    .unwrap();
    let model = Arc::new(Model::default());
    let (_, server) = model_server(context.model_port, model.clone()).await;
    config.fleet.controller_recovery_enabled = true;
    config.fleet.configuration_recovery_enabled = true;
    let config = Arc::new(config);
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime = LocalRuntimeSupervisor::new(config.clone(), repo.clone(), events);
    timeout(Duration::from_secs(180), async {
        loop {
            let current = repo
                .get_config_revision(context.agent, context.candidate)
                .await
                .unwrap();
            let launch = repo.get_open_runtime_launch(context.agent).await.unwrap();
            let progress = json!({"candidate_state":current.state,"draining":current.draining,
                "launch_phase":launch.as_ref().map(|v|&v.binding.phase),
                "journal_present":Path::new(&journal_path).exists(),"sdlc_acceptance":false});
            tokio::fs::write(
                "/evidence/activation-progress.json",
                serde_json::to_vec(&progress).unwrap(),
            )
            .await
            .unwrap();
            if current.state == "failed" && !current.draining && !Path::new(&journal_path).exists()
            {
                break;
            }
            sleep(Duration::from_millis(500)).await;
        }
    })
    .await
    .expect("actual interrupted activation remains unreconciled");
    let agent = repo.get_agent(context.agent).await.unwrap();
    let peer = repo.get_agent(context.peer).await.unwrap();
    assert_eq!(file_hash(&agent).await, context.backup_sha256);
    assert_eq!(file_hash(&peer).await, context.peer_files_sha256);
    assert_eq!(
        repo.get_effective_config_revision(agent.id)
            .await
            .unwrap()
            .unwrap()
            .revision,
        context.effective
    );
    let rollback = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap()
        .binding;
    assert_eq!(rollback.phase, "rollback");
    assert_eq!(rollback.configuration_revision, Some(context.effective));
    assert_ne!(rollback.id, context.original.id);
    assert_ne!(rollback.id, context.replacement.id);
    assert_eq!(
        hash(
            &repo
                .get_open_runtime_launch(peer.id)
                .await
                .unwrap()
                .unwrap()
                .binding
        ),
        hash(&context.peer_launch)
    );
    for old in [&context.original, &context.replacement] {
        let saved = repo.get_runtime_launch(old.id).await.unwrap().unwrap();
        assert_eq!(saved.state, "gateway_exited");
        assert!(saved.pid.is_some());
        assert_eq!(hash(&saved.binding), hash(old));
    }
    let stop = repo
        .read_controller_stop(context.replacement.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stop.intent.launch_id, context.replacement.id);
    assert_eq!(
        stop.native_outcome.unwrap()["receipt"]["observation"],
        "namespace_exited"
    );
    let audit = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT count(*) AS count FROM audit_log WHERE entity_id=$1 AND action='agent_config.recovery_rollback'",
        [agent.id.to_string().into()])).await.unwrap().unwrap();
    assert_eq!(audit.try_get::<i64>("", "count").unwrap(), 1);
    let (session, _, _) = send(&repo, agent.id, context.owner, "after-activation-crash").await;
    let run = terminal(&repo, session).await;
    assert_eq!(run.state, SessionRunState::Completed);
    assert_answer(
        &repo,
        &model,
        session,
        "after-activation-crash",
        OLD,
        NEW,
        "LOCAL_MODEL",
    )
    .await;
    assert_eq!(model.calls.lock().await.len(), 1);
    sleep(Duration::from_secs(2)).await;
    assert!(!Path::new(&journal_path).exists());
    assert_eq!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .unwrap()
            .binding
            .id,
        rollback.id
    );
    // Freeze and settle background custody before disposing its observed generations.
    runtime.quiesce_controller_recovery().await.unwrap();
    assert_eq!(
        runtime.stop(&agent).await.unwrap().status,
        AgentStatus::Stopped
    );
    let owner = runtime
        .heartbeat_container_controller(peer.id)
        .await
        .unwrap();
    assert!(owner.lease_valid);
    assert_eq!(
        runtime.stop(&peer).await.unwrap().status,
        AgentStatus::Stopped
    );
    server.abort();
    let _ = server.await;
    tokio::fs::write("/evidence/activation-report.json", serde_json::to_vec(&json!({
        "state":"passed","actual_rust_supervisor":true,"actual_docker_hermes":true,
        "actual_candidate_running_crash":true,"effective_preserved":true,"backup_bytes_restored":true,
        "qa_settlement_barrier_released":true,
        "fresh_rollback_generation":true,"loaded_previous_soul":true,"peer_unchanged":true,
        "original_namespaces_exited":true,"rollback_audit_once":true,"model_prompts":1,
        "sdlc_acceptance":false,"raw_credentials_persisted_in_evidence":false})).unwrap()).await.unwrap();
}

#[tokio::test]
#[ignore = "requires original-Engine Compose controller, mapped volume and real Hermes crash"]
async fn real_candidate_running_crash_restores_effective_configuration() {
    assert_eq!(
        std::env::var("FLEET_CONTAINER_SUPERVISOR_TEST").as_deref(),
        Ok("1")
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
    match std::env::var("FLEET_CONTAINER_ACTIVATION_PHASE")
        .unwrap()
        .as_str()
    {
        "prepare" => {
            let proof = serde_json::from_slice(
                &tokio::fs::read("/qa/controller-proof.json").await.unwrap(),
            )
            .unwrap();
            prepare(repo, db, proof).await;
        }
        "recover" => recover(repo, db).await,
        _ => panic!("unknown activation phase"),
    }
}
