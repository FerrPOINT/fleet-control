//! Real repository/CAS tests with a controlled native HTTP/process boundary.
//! No Docker, model, Workflow admission or controller-restart claim.
use super::*;
use crate::PostgresFleetRepository;
use app::container_runtime::ContainerPreparationClaim;
use domain::{AgentRole, CreateAgentRequest, CreateSessionRequest, PmRunReservation};
use migration::MigratorTrait;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use std::{net::Ipv4Addr, os::unix::fs::PermissionsExt, path::PathBuf};

struct Resources {
    root: PathBuf,
    server: Option<tokio::task::JoinHandle<()>>,
}

impl Drop for Resources {
    fn drop(&mut self) {
        if let Some(server) = &self.server {
            server.abort();
        }
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

struct Fixture {
    supervisor: LocalRuntimeSupervisor,
    repo: Arc<PostgresFleetRepository>,
    agent: Agent,
    intent: PmDispatchIntent,
    calls: Arc<AtomicUsize>,
    effects: Arc<AtomicUsize>,
    _resources: Resources,
}

async fn repository() -> Arc<PostgresFleetRepository> {
    let url = std::env::var("FLEET_TEST_DATABASE_URL")
        .expect("isolated PostgreSQL required for PM replay production-path tests");
    let parsed = reqwest::Url::parse(&url).unwrap();
    assert!(parsed.path().starts_with("/fleet_") && parsed.path().ends_with("_test"));
    Arc::new(PostgresFleetRepository::new(
        crate::connect_database(shared::DatabaseConfig {
            url,
            max_connections: 10,
            min_connections: 1,
            connect_timeout_seconds: 10,
            idle_timeout_seconds: 60,
        })
        .await
        .unwrap(),
    ))
}

// Construct the real supervisor without unrelated background recovery workers.
fn supervisor(
    config: Arc<shared::AppConfig>,
    repo: Arc<PostgresFleetRepository>,
    controller_id: Uuid,
) -> LocalRuntimeSupervisor {
    LocalRuntimeSupervisor {
        config,
        repo: repo.clone(),
        children: Arc::new(Mutex::new(HashMap::new())),
        controller_id,
        container_operations: Arc::new(
            super::super::super::container_workers::ContainerOperations::default(),
        ),
        client: reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(Duration::from_secs(5))
            .build()
            .unwrap(),
        events: broadcast::channel(16).0,
        alerts: Arc::new(app::RepositoryAlertService { repository: repo }),
    }
}

async fn fixture(lost_ack: bool, concurrent: bool) -> Fixture {
    eprintln!("\nFLEET_PM_RECOVERY_PHASE=fixture_entered");
    let repo = repository().await;
    migration::Migrator::up(&repo.db, None).await.unwrap();
    repo.ensure_runtime_templates().await.unwrap();
    eprintln!("\nFLEET_PM_RECOVERY_PHASE=fixture_database_ready");
    let root = std::env::temp_dir().join(format!("fleet-pm-replay-{}", Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    let mut resources = Resources { root, server: None };
    let agents = resources.root.join("agents");
    let controller = resources.root.join("controller");
    std::fs::create_dir(&agents).unwrap();
    std::fs::create_dir(&controller).unwrap();
    std::fs::set_permissions(&controller, std::fs::Permissions::from_mode(0o700)).unwrap();
    let utility = PathBuf::from(
        std::env::var("FLEET_TEST_BASE_UTILITY_CHECKOUT")
            .expect("canonical utility9b files required"),
    );
    assert!(utility.is_absolute());
    for (name, expected) in [
        "runtime_boundary.py",
        "runtime_bootstrap.py",
        "runtime_control.py",
        "runtime_replacement.py",
    ]
    .into_iter()
    .zip(super::super::super::container_lifecycle::CONTROL_SHA256)
    {
        assert!(
            hex::encode(Sha256::digest(
                std::fs::read(utility.join("scripts").join(name)).unwrap()
            )) == expected
        );
    }
    let host: Ipv4Addr = std::env::var("FLEET_PM_RECOVERY_TEST_HOST")
        .expect("owned local private IPv4 required; container guard rejects loopback")
        .parse()
        .unwrap();
    assert!(host.is_private() && !host.is_loopback() && !host.is_link_local());
    let listener = tokio::net::TcpListener::bind((host, 0)).await.unwrap();
    let port = i32::from(listener.local_addr().unwrap().port());
    assert!(port >= 1024);
    let base = format!("http://{host}:{port}");
    let python = controller.join("controlled-python");
    let mut config = shared::AppConfig::default();
    config.fleet.agents_root = agents.to_str().unwrap().into();
    config.fleet.runtime_token_secret = "pm-replay-pg-test-only-secret".into();
    config.fleet.container_control = Some(serde_json::from_value(json!({
        "controller_root":controller,"base_root":utility,"python":python,"context":"desktop-linux"
    })).unwrap());
    let created = repo
        .create_agent(
            CreateAgentRequest {
                kind: AgentKind::Hermes,
                product_role: AgentProductRole::Executor,
                role: AgentRole::Developer,
                sdlc_role: Some(SdlcRole::Developer),
                display_name: "PM replay fixture".into(),
                description: None,
                namespace_id: None,
                namespace_name: None,
                workflow_id: None,
                workflow_name: None,
                executor_ids: vec![],
            },
            &config,
        )
        .await
        .unwrap();
    repo.db
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE agents SET sdlc_role='project_manager',api_port=$2 WHERE id=$1",
            [created.id.into(), port.into()],
        ))
        .await
        .unwrap();
    repo.update_agent_status(created.id, AgentStatus::Ready)
        .await
        .unwrap();
    let agent = repo.get_agent(created.id).await.unwrap();
    let home = crate::safe_agent_root(&agents, &agent.name).unwrap();
    for (area, path) in [
        ("runtime", &agent.paths.runtime),
        ("config", &agent.paths.config),
        ("workspace", &agent.paths.workspace),
        ("logs", &agent.paths.logs),
    ] {
        assert!(std::path::Path::new(path) == home.join(area));
        std::fs::create_dir_all(path).unwrap();
    }
    std::fs::write(
        home.join(".fleet-agent.json"),
        serde_json::to_vec(&json!({
            "id":agent.id,"name":agent.name
        }))
        .unwrap(),
    )
    .unwrap();
    let token = crate::agent_runtime_token(&config, agent.id).unwrap();
    let (mut intent, mut launch) = super::fixture(&base);
    let generation = launch.prepared.container.registration.generation;
    let policy = json!({"contract_version":2,"resource_id":agent.id.to_string(),
        "generation":generation.to_string(),"project":format!("sdlc-qa-pm-replay-{generation}"),
        "service":"hermes","mounts":[
            {"type":"bind","source":agent.paths.runtime,"destination":"/runtime","read_only":true},
            {"type":"bind","source":agent.paths.config,"destination":"/config","read_only":false},
            {"type":"bind","source":agent.paths.workspace,"destination":"/workspace","read_only":false},
            {"type":"bind","source":agent.paths.logs,"destination":"/logs","read_only":false}]});
    let compose = json!({"name":policy["project"],"services":{"hermes":{
        "working_dir":"/workspace","command":["serve","--host","0.0.0.0","--port",port.to_string()],
        "environment":{"HOME":"/config","HERMES_HOME":"/config","API_SERVER_ENABLED":"true",
            "API_SERVER_HOST":"0.0.0.0","HERMES_SERVE_HEADLESS":"1","API_SERVER_KEY":token,
            "API_SERVER_PORT":port.to_string()}}}});
    let compose_path = controller.join("compose.json");
    std::fs::write(&compose_path, serde_json::to_vec(&compose).unwrap()).unwrap();
    std::fs::set_permissions(&compose_path, std::fs::Permissions::from_mode(0o600)).unwrap();
    launch.prepared.agent_id = agent.id;
    launch.prepared.paths = agent.paths.clone();
    launch.prepared.api_port = Some(port);
    launch.prepared.configuration_revision = None;
    launch.prepared.configuration_sha256 = None;
    launch.prepared.container.registration.container_id = generation.simple().to_string().repeat(2);
    launch.prepared.container.registration.resource_id = agent.id;
    launch.prepared.container.registration.network_sha256 = Some("f".repeat(64));
    launch.prepared.container.registration.policy_sha256 =
        container_control::canonical_hash(&policy).unwrap();
    launch.prepared.container.registration.compose_sha256 =
        container_control::canonical_hash(&compose).unwrap();
    launch.prepared.container.policy = policy;
    launch.prepared.container.compose = compose_path.to_str().unwrap().into();
    launch.prepared.container.journal = controller.join("start.sqlite").to_str().unwrap().into();
    launch.prepared.container.stop_journal =
        controller.join("stop.sqlite").to_str().unwrap().into();
    launch.prepared.container.context = "desktop-linux".into();
    launch.prepared.container.source_sha256 =
        super::super::super::container_lifecycle::UTILITY_SHA256.map(str::to_owned);
    let r = &launch.prepared.container.registration;
    container_control::validate_registration(r).unwrap();
    let snapshot = json!({"contract_version":2,"container_id":r.container_id,"engine":r.engine,
        "policy_sha256":r.policy_sha256,"inventory_sha256":r.running_inventory_sha256,
        "network_sha256":r.network_sha256,
        "started_at":"2026-10-10T00:00:00Z","init_pid":42});
    let receipt = json!({"contract_version":2,"operation_id":r.operation_id,"container_id":r.container_id,
        "resource_id":r.resource_id,"generation":generation,"registration_sha256":container_control::canonical_hash(r).unwrap(),
        "state":"observed","observation":"running","snapshot":snapshot});
    container_control::validate_receipt(
        &serde_json::from_value(receipt.clone()).unwrap(),
        r,
        0,
        "observe",
    )
    .unwrap();
    // The executable is a controlled process boundary, not a Docker/Base emulator.
    // Canonical utility bytes are still verified by production ContainerControl.
    std::fs::write(&python, format!(
        "#!/usr/bin/python3\nimport json,sys\np=json.load(sys.stdin)['request']\nassert p['protocol_version']==1\nassert p['action'] in ('observe','endpoint')\nr=json.loads({:?})\nif p['action']=='endpoint': r={{'host':{:?},'receipt':r}}\nprint(json.dumps({{'protocol_version':1,'action':p['action'],'result':r}}))\n",
        receipt.to_string(), host.to_string())).unwrap();
    std::fs::set_permissions(&python, std::fs::Permissions::from_mode(0o700)).unwrap();
    let preparation = ContainerPreparationClaim {
        agent_id: agent.id,
        generation,
        operation_id: r.operation_id,
        paths: agent.paths.clone(),
        api_port: Some(port),
        configuration_revision: None,
        configuration_sha256: None,
        intent_sha256: "0".repeat(64),
    };
    repo.claim_container_preparation(&preparation)
        .await
        .unwrap();
    repo.claim_container_preparation_delivery(&preparation)
        .await
        .unwrap();
    repo.acknowledge_container_preparation(&preparation, &launch.prepared)
        .await
        .unwrap();
    launch.state = "claimed".into();
    launch.snapshot = None;
    launch.origin = None;
    repo.claim_container_launch(&launch).await.unwrap();
    repo.advance_container_launch(&launch, "running", Some(snapshot), Some(base.clone()))
        .await
        .unwrap();
    repo.update_agent_status(agent.id, AgentStatus::Running)
        .await
        .unwrap();
    let agent = repo.get_agent(agent.id).await.unwrap();
    let subject = Uuid::new_v4().to_string();
    let owner = repo
        .find_or_create_central_user(
            &subject,
            &format!("{subject}@example.test"),
            "PM replay owner",
        )
        .await
        .unwrap();
    let session = repo
        .create_session(
            CreateSessionRequest {
                primary_agent_id: Some(agent.id),
                agent_id: None,
                title: "PM replay fixture".into(),
                task_key: None,
                leader_agent_id: None,
                parent_session_id: None,
                namespace_id: None,
                idempotency_key: Some(subject.clone()),
            },
            owner.id,
        )
        .await
        .unwrap();
    let binding = domain::TaskChatBinding {
        tracker_instance_id: "tracker-pm-test".into(),
        project_id: Uuid::new_v4(),
        task_id: Uuid::new_v4(),
        root_task_id: Uuid::new_v4(),
        agent_id: agent.id,
        owner_subject: subject,
    };
    repo.bind_task_chat(session.id, binding.clone(), "bind-pm".into())
        .await
        .unwrap();
    let reservation = PmRunReservation {
        session_id: session.id,
        session_run_id: intent.session_run_id,
        identity: domain::PmExecutionIdentity {
            task: "SDLC-42".into(),
            execution_ref: Uuid::new_v4().to_string(),
            tracker_instance_ref: binding.tracker_instance_id,
            tracker_project_ref: binding.project_id.to_string(),
            task_ref: binding.task_id.to_string(),
            root_ref: binding.root_task_id.to_string(),
            agent_ref: agent.id.to_string(),
            assignment_operation_key: "assign-pm".into(),
            assignment_ref: Uuid::new_v4().to_string(),
            assignment_revision: 1,
        },
        binding_ref: "workflow-binding-42".into(),
        dispatch_operation_key: "dispatch-pm".into(),
        checkpoint_ref: None,
        fence: 1,
    };
    repo.reserve_pm_run(reservation.clone()).await.unwrap();
    intent.submitted = false;
    intent.credential_fingerprint = hermes_wire::credential_fingerprint(&token);
    intent.request_body = serde_json::to_string(
        &json!({"input":"original fixture","session_id":reservation.runtime_session_id()}),
    )
    .unwrap();
    intent.runtime_context = json!({"fleet_container_generation":generation});
    let calls = Arc::new(AtomicUsize::new(0));
    let effects = Arc::new(AtomicUsize::new(0));
    resources.server = Some(peer(
        listener,
        token,
        &intent,
        calls.clone(),
        effects.clone(),
        lost_ack,
        concurrent,
    ));
    let supervisor = supervisor(Arc::new(config), repo.clone(), launch.controller_id);
    // Capture the proof through the real prepare path, never a fabricated proof.
    eprintln!("\nFLEET_PM_RECOVERY_PHASE=fixture_context_ready");
    let intent = prepare(&supervisor, &agent, intent).await.unwrap();
    eprintln!("\nFLEET_PM_RECOVERY_PHASE=fixture_prepared");
    Fixture {
        supervisor,
        repo,
        agent,
        intent,
        calls,
        effects,
        _resources: resources,
    }
}

fn peer(
    listener: tokio::net::TcpListener,
    token: String,
    intent: &PmDispatchIntent,
    calls: Arc<AtomicUsize>,
    effects: Arc<AtomicUsize>,
    lost_ack: bool,
    concurrent: bool,
) -> tokio::task::JoinHandle<()> {
    let expected_key = intent.session_run_id.to_string();
    let expected_body = intent.request_body.clone();
    let ledger = Arc::new(Mutex::new(None::<(String, String)>));
    let barrier = concurrent.then(|| Arc::new(tokio::sync::Barrier::new(2)));
    let router = axum::Router::new()
        .route(
            "/health",
            axum::routing::get(|| async { axum::Json(json!({"status":"ok"})) }),
        )
        .route(
            "/v1/capabilities",
            axum::routing::get(|| async { axum::Json(capabilities()) }),
        )
        .route(
            "/v1/runs",
            axum::routing::post(move |headers: axum::http::HeaderMap, body: String| {
                let token = token.clone();
                let calls = calls.clone();
                let effects = effects.clone();
                let expected_key = expected_key.clone();
                let expected_body = expected_body.clone();
                let ledger = ledger.clone();
                let barrier = barrier.clone();
                async move {
                    use axum::response::IntoResponse;
                    assert!(
                        headers
                            .get("authorization")
                            .is_some_and(|value| value == format!("Bearer {token}").as_str())
                    );
                    let key = headers
                        .get("idempotency-key")
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_owned();
                    assert!(key == expected_key && body == expected_body);
                    let call = calls.fetch_add(1, Ordering::SeqCst);
                    let mut saved = ledger.lock().await;
                    let replay = saved.is_some();
                    if let Some((old_key, old_body)) = &*saved {
                        assert!(*old_key == key && *old_body == body);
                    } else {
                        *saved = Some((key, body));
                        effects.fetch_add(1, Ordering::SeqCst);
                    }
                    drop(saved);
                    if let Some(barrier) = barrier {
                        barrier.wait().await;
                    }
                    if lost_ack && call == 0 {
                        return reqwest::StatusCode::SERVICE_UNAVAILABLE.into_response();
                    }
                    (
                        reqwest::StatusCode::ACCEPTED,
                        axum::Json(
                            json!({"run_id":"run_original","status":"started","replayed":replay}),
                        ),
                    )
                        .into_response()
                }
            }),
        );
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    })
}

async fn bounded_submit(
    f: &Fixture,
    supervisor: &LocalRuntimeSupervisor,
    intent: &PmDispatchIntent,
) -> Result<String, AppError> {
    eprintln!("\nFLEET_PM_RECOVERY_PHASE=submit_entered");
    // Task admission is outside this helper-level regression; production callers
    // supply verify_dispatch/machine_context. Context and repository guards are real.
    tokio::time::timeout(
        Duration::from_secs(20),
        submit(supervisor, &f.agent, intent, async { Ok(()) }),
    )
    .await
    .expect("bounded PM production-path test")
}

#[test]
#[ignore = "reports future layout alongside isolated PostgreSQL production-path tests"]
fn production_aaa_future_layout_without_constructing_or_polling_runtime() {
    fn fixture_bytes<F, Fut>(_: F) -> usize
    where
        F: FnOnce(bool, bool) -> Fut,
        Fut: std::future::Future,
    {
        std::mem::size_of::<Fut>()
    }
    fn submit_bytes<F, Fut>(_: F) -> usize
    where
        F: FnOnce(
            &'static Fixture,
            &'static LocalRuntimeSupervisor,
            &'static PmDispatchIntent,
        ) -> Fut,
        Fut: std::future::Future,
    {
        std::mem::size_of::<Fut>()
    }
    // Function items are not called; no future, reference, process or DB is created.
    eprintln!(
        "\nFLEET_PM_RECOVERY_LAYOUT={},{}",
        fixture_bytes(fixture),
        submit_bytes(bounded_submit),
    );
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL, canonical utility9b and owned private HTTP bind"]
async fn production_lost_ack_reload_reuses_original_journal_and_persists_same_native_ack() {
    eprintln!("\nFLEET_PM_RECOVERY_PHASE=lost_ack_entered");
    let f = Box::pin(fixture(true, false)).await;
    assert!(bounded_submit(&f, &f.supervisor, &f.intent).await.is_err());
    let unknown = f
        .repo
        .get_pm_dispatch(f.intent.session_run_id)
        .await
        .unwrap()
        .unwrap();
    assert!(unknown.submitted && unknown.hermes_run_ref.is_none());
    let mut expected_unknown = f.intent.clone();
    expected_unknown.submitted = true;
    assert!(unknown == expected_unknown);
    assert_eq!(f.calls.load(Ordering::SeqCst), 1);
    assert_eq!(f.effects.load(Ordering::SeqCst), 1);
    let reloaded_repo = repository().await;
    // Repository/intent reload retains the original live controller, not a
    // simulated physical controller restart with invented recovery authority.
    let reloaded = supervisor(
        f.supervisor.config.clone(),
        reloaded_repo.clone(),
        f.supervisor.controller_id,
    );
    let original = prepare(&reloaded, &f.agent, f.intent.clone())
        .await
        .unwrap();
    assert!(original == unknown);
    assert_eq!(
        bounded_submit(&f, &reloaded, &original).await.unwrap(),
        "run_original"
    );
    let mut expected = unknown;
    expected.hermes_run_ref = Some("run_original".into());
    assert!(
        reloaded_repo
            .get_pm_dispatch(expected.session_run_id)
            .await
            .unwrap()
            .unwrap()
            == expected
    );
    // Even a stale unknown intent must read the saved ACK without another POST.
    assert_eq!(
        bounded_submit(&f, &reloaded, &original).await.unwrap(),
        "run_original"
    );
    let mut changed = f.intent.clone();
    changed.request_body.push(' ');
    assert!(prepare(&reloaded, &f.agent, changed).await.is_err());
    assert_eq!(f.calls.load(Ordering::SeqCst), 2);
    assert_eq!(f.effects.load(Ordering::SeqCst), 1);
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL, canonical utility9b and owned private HTTP bind"]
async fn production_concurrent_submission_cas_and_ack_keep_one_native_effect() {
    eprintln!("\nFLEET_PM_RECOVERY_PHASE=concurrent_entered");
    let f = Box::pin(fixture(false, true)).await;
    let other_repo = repository().await;
    let other = supervisor(
        f.supervisor.config.clone(),
        other_repo.clone(),
        f.supervisor.controller_id,
    );
    let authorizations = Arc::new(tokio::sync::Barrier::new(2));
    let authorize = || {
        let barrier = authorizations.clone();
        async move {
            barrier.wait().await;
            Ok(())
        }
    };
    eprintln!("\nFLEET_PM_RECOVERY_PHASE=concurrent_submit_entered");
    let (first, second) = tokio::time::timeout(Duration::from_secs(20), async {
        tokio::join!(
            submit(&f.supervisor, &f.agent, &f.intent, authorize()),
            submit(&other, &f.agent, &f.intent, authorize())
        )
    })
    .await
    .expect("bounded concurrent PM production path");
    assert_eq!(first.unwrap(), "run_original");
    assert_eq!(second.unwrap(), "run_original");
    let saved = other_repo
        .get_pm_dispatch(f.intent.session_run_id)
        .await
        .unwrap()
        .unwrap();
    let mut expected = f.intent.clone();
    expected.submitted = true;
    expected.hermes_run_ref = Some("run_original".into());
    assert!(saved == expected);
    assert!(
        !other_repo
            .claim_pm_submission(saved.session_run_id)
            .await
            .unwrap()
    );
    assert!(
        other_repo
            .record_pm_submission(saved.session_run_id, "run_foreign".into())
            .await
            .is_err()
    );
    assert!(
        other_repo
            .get_pm_dispatch(saved.session_run_id)
            .await
            .unwrap()
            .unwrap()
            == saved
    );
    assert_eq!(
        bounded_submit(&f, &other, &f.intent).await.unwrap(),
        "run_original"
    );
    assert_eq!(f.calls.load(Ordering::SeqCst), 2);
    assert_eq!(f.effects.load(Ordering::SeqCst), 1);
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL, canonical utility9b and owned private HTTP bind"]
async fn production_unknown_replay_rejects_current_token_context_revocation_before_post() {
    eprintln!("\nFLEET_PM_RECOVERY_PHASE=revocation_entered");
    let f = Box::pin(fixture(true, false)).await;
    assert!(bounded_submit(&f, &f.supervisor, &f.intent).await.is_err());
    let unknown = f
        .repo
        .get_pm_dispatch(f.intent.session_run_id)
        .await
        .unwrap()
        .unwrap();
    let mut config = (*f.supervisor.config).clone();
    config.fleet.runtime_token_secret = "revoked-pm-replay-pg-secret".into();
    let revoked = supervisor(
        Arc::new(config),
        repository().await,
        f.supervisor.controller_id,
    );
    assert!(
        hermes_wire::credential_fingerprint(
            &crate::agent_runtime_token(&revoked.config, f.agent.id).unwrap()
        ) != unknown.credential_fingerprint
    );
    assert!(bounded_submit(&f, &revoked, &unknown).await.is_err());
    assert!(
        f.repo
            .get_pm_dispatch(unknown.session_run_id)
            .await
            .unwrap()
            .unwrap()
            == unknown
    );
    assert_eq!(f.calls.load(Ordering::SeqCst), 1);
    assert_eq!(f.effects.load(Ordering::SeqCst), 1);
}
