use app::FleetRepository;
use domain::{
    AgentKind, AgentProductRole, AgentRole, AgentStatus, CreateAgentRequest,
    CreateSessionMessageRequest, CreateSessionRequest, MessageKind, SdlcRole, SessionRunRole,
    SessionRunState, UpdateAgentConfigRequest,
};
use infra::{PostgresFleetRepository, connect_database, run_migrations};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, TransactionTrait};
use shared::{AppConfig, DatabaseConfig};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::time::{Duration, sleep};
use uuid::Uuid;

#[path = "support/pm_draft_creation.rs"]
mod pm_draft_creation;

#[path = "support/pm_credential_creation.rs"]
mod pm_credential_creation;

#[path = "support/base_package.rs"]
mod base_package;

#[path = "support/runtime_readiness.rs"]
mod runtime_readiness;

#[path = "support/runtime_acceptance.rs"]
mod runtime_acceptance;

#[path = "support/hermes_dispatch_journal.rs"]
mod hermes_dispatch_journal;

#[path = "support/runtime_acceptance_readback_http.rs"]
mod runtime_acceptance_readback_http;

#[path = "support/hermes_protocol_fixture.rs"]
mod hermes_protocol_fixture;

#[path = "support/runtime_purge.rs"]
mod runtime_purge;

async fn fixture() -> Option<(PostgresFleetRepository, Uuid, Uuid)> {
    let Ok(url) = std::env::var("FLEET_TEST_DATABASE_URL") else {
        eprintln!("FLEET_TEST_DATABASE_URL not configured; PostgreSQL tests skipped");
        return None;
    };
    let config = DatabaseConfig {
        url,
        max_connections: 10,
        min_connections: 1,
        connect_timeout_seconds: 10,
        idle_timeout_seconds: 60,
    };
    run_migrations(config.clone()).await.unwrap();
    let db = connect_database(config).await.unwrap();
    let owner = Uuid::new_v4();
    let stranger = Uuid::new_v4();
    for id in [owner, stranger] {
        db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO users(id,email,username,display_name,password_hash,is_system_admin,system_role)
             VALUES ($1,$2,$3,'SDLC test','disabled',false,'user')",
            [id.into(), format!("{id}@example.test").into(), id.to_string().into()])).await.unwrap();
    }
    Some((PostgresFleetRepository::new(db), owner, stranger))
}

async fn agent(repo: &PostgresFleetRepository) -> Uuid {
    agent_with_config(repo, &AppConfig::default()).await
}

#[tokio::test]
async fn runtime_stop_untracked_never_fabricates_stopped_or_releases_run_capacity() {
    use app::{RuntimeStatePatch, RuntimeSupervisor};
    use domain::DesiredState;
    let Some((repo, owner, _)) = fixture().await else {
        return;
    };
    let id = agent(&repo).await;
    let session = repo
        .create_session(chat(id, "untracked-stop"), owner)
        .await
        .unwrap();
    let run = repo
        .prepare_session_agent_run(
            session.id,
            id,
            SessionRunRole::Primary,
            format!("fleet:{}:{id}", session.id),
        )
        .await
        .unwrap();
    let repo = Arc::new(repo);
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime = infra::runtime::LocalRuntimeSupervisor::new(
        Arc::new(AppConfig::default()),
        repo.clone(),
        events,
    );
    for status in [
        AgentStatus::Running,
        AgentStatus::Starting,
        AgentStatus::Degraded,
    ] {
        let agent = repo
            .update_runtime_state(
                id,
                RuntimeStatePatch {
                    status,
                    desired_state: DesiredState::Running,
                    pid: Some(12345),
                    health_status: Some("untracked".into()),
                    health_detail: Some("test-only untracked process".into()),
                    last_capabilities_json: None,
                    startup_command_redacted: None,
                    started_at: None,
                    stopped_at: None,
                },
            )
            .await
            .unwrap();
        assert!(matches!(
            runtime.stop(&agent).await,
            Err(shared::AppError::Unavailable(_))
        ));
        assert!(matches!(
            runtime.restart(&agent).await,
            Err(shared::AppError::Unavailable(_))
        ));
        let mut stale = agent.clone();
        stale.status = AgentStatus::Ready;
        stale.runtime.pid = None;
        stale.runtime.desired_state = DesiredState::Stopped;
        assert!(matches!(
            runtime.stop(&stale).await,
            Err(shared::AppError::Unavailable(_))
        ));
        assert!(matches!(
            runtime.start(&stale).await,
            Err(shared::AppError::Unavailable(_))
        ));
        let observed = repo.get_agent(id).await.unwrap();
        assert_eq!(observed.status, status);
        assert_eq!(observed.runtime.pid, Some(12345));
        assert_eq!(observed.runtime.desired_state, DesiredState::Running);
        let runs = repo.list_session_agent_runs(session.id).await.unwrap();
        assert!(
            runs.iter()
                .any(|item| item.id == run.id && item.state == SessionRunState::Pending)
        );
        assert!(
            repo.prepare_session_agent_run(
                session.id,
                id,
                SessionRunRole::Primary,
                format!("fleet:{}:{id}", session.id)
            )
            .await
            .is_err()
        );
    }
}

#[tokio::test]
async fn runtime_health_failure_does_not_attest_untracked_process_death() {
    use app::{RuntimeStatePatch, RuntimeSupervisor};
    use domain::DesiredState;
    let Some((repo, _, _)) = fixture().await else {
        return;
    };
    let id = agent(&repo).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            axum::Router::new().route(
                "/health",
                axum::routing::get(|| async { axum::http::StatusCode::SERVICE_UNAVAILABLE }),
            ),
        )
        .await
        .unwrap();
    });
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE agents SET api_port=$2 WHERE id=$1",
        [id.into(), i32::from(port).into()],
    ))
    .await
    .unwrap();
    let before = repo
        .update_runtime_state(
            id,
            RuntimeStatePatch {
                status: AgentStatus::Running,
                desired_state: DesiredState::Running,
                pid: Some(12345),
                health_status: None,
                health_detail: None,
                last_capabilities_json: None,
                startup_command_redacted: None,
                started_at: None,
                stopped_at: None,
            },
        )
        .await
        .unwrap();
    let repo = Arc::new(repo);
    let (events, _) = tokio::sync::broadcast::channel(32);
    let mut config = AppConfig::default();
    config.fleet.runtime_token_secret = "health-proof-test-only".into();
    let runtime =
        infra::runtime::LocalRuntimeSupervisor::new(Arc::new(config), repo.clone(), events);
    assert_eq!(
        runtime.health(&before).await.unwrap().status,
        AgentStatus::Degraded
    );
    let after = repo.get_agent(id).await.unwrap();
    assert_eq!(after.runtime.pid, Some(12345));
    assert_eq!(after.runtime.desired_state, DesiredState::Running);
    assert!(after.runtime.stopped_at.is_none());
    assert!(matches!(
        runtime.start(&after).await,
        Err(shared::AppError::Unavailable(_))
    ));
    assert!(matches!(
        runtime.stop(&after).await,
        Err(shared::AppError::Unavailable(_))
    ));
    server.abort();
    let _ = server.await;
}

async fn agent_with_config(repo: &PostgresFleetRepository, config: &AppConfig) -> Uuid {
    repo.ensure_runtime_templates().await.unwrap();
    let result = repo
        .create_agent(
            CreateAgentRequest {
                kind: AgentKind::Hermes,
                product_role: AgentProductRole::Executor,
                role: AgentRole::Developer,
                sdlc_role: Some(SdlcRole::Developer),
                display_name: "Developer".into(),
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
    assert_eq!(result.sdlc_role, Some(SdlcRole::Developer));
    repo.update_agent_status(result.id, AgentStatus::Running)
        .await
        .unwrap();
    result.id
}

fn chat(agent_id: Uuid, key: &str) -> CreateSessionRequest {
    CreateSessionRequest {
        primary_agent_id: Some(agent_id),
        agent_id: None,
        title: "Private task".into(),
        task_key: None,
        leader_agent_id: None,
        parent_session_id: None,
        namespace_id: None,
        idempotency_key: Some(key.into()),
    }
}

fn prompt(key: &str) -> CreateSessionMessageRequest {
    CreateSessionMessageRequest {
        body: "Implement the confirmed task".into(),
        author_agent_id: None,
        message_kind: Some(MessageKind::UserPrompt),
        runtime_message_id: None,
        idempotency_key: Some(key.into()),
    }
}

async fn approval_fixture(
    repo: &PostgresFleetRepository,
    owner: Uuid,
    key: &str,
) -> (
    domain::AgentSession,
    domain::SessionAgentRun,
    domain::RuntimeApprovalRequest,
) {
    let agent_id = agent(repo).await;
    let session = repo
        .create_session(chat(agent_id, key), owner)
        .await
        .unwrap();
    let run = repo
        .prepare_session_agent_run(
            session.id,
            agent_id,
            SessionRunRole::Primary,
            format!("fleet:{}:{agent_id}", session.id),
        )
        .await
        .unwrap();
    let run = repo
        .update_session_agent_run_dispatch(
            run.id,
            Some("run_approval_test".into()),
            SessionRunState::Waiting,
            None,
        )
        .await
        .unwrap();
    let request = app::RuntimeApprovalCreate {
        session_id: session.id,
        session_run_id: run.id,
        agent_id,
        runtime_run_id: "run_approval_test".into(),
        runtime_approval_id: Some("request_one".into()),
        prompt: "Allow bounded workspace command api_key=private?".into(),
        detail: serde_json::json!({"api_key":"private"}),
    };
    let (a, b) = tokio::join!(
        repo.upsert_runtime_approval_request(request.clone()),
        repo.upsert_runtime_approval_request(request)
    );
    let approval = a.unwrap();
    assert_eq!(approval.id, b.unwrap().id);
    (session, run, approval)
}

#[tokio::test]
async fn targeted_approval_reservation_is_atomic_scoped_and_replay_never_redispatches() {
    let Some((repo, owner, stranger)) = fixture().await else {
        return;
    };
    let (session, run, approval) = approval_fixture(&repo, owner, "approval-atomic").await;
    let req = domain::ApprovalDecisionRequest {
        choice: domain::ApprovalChoice::Once,
        idempotency_key: "approval-command".into(),
    };
    assert!(matches!(
        repo.reserve_approval_decision(session.id, approval.id, stranger, req.clone())
            .await,
        Err(shared::AppError::Forbidden)
    ));
    let (a, b) = tokio::join!(
        repo.reserve_approval_decision(session.id, approval.id, owner, req.clone()),
        repo.reserve_approval_decision(session.id, approval.id, owner, req.clone())
    );
    let a = a.unwrap();
    let b = b.unwrap();
    assert_eq!(a.decision.id, b.decision.id);
    assert_ne!(a.dispatch, b.dispatch);
    assert_eq!(a.decision.state, domain::ApprovalDecisionState::Uncertain);
    let replay = repo
        .reserve_approval_decision(session.id, approval.id, owner, req.clone())
        .await
        .unwrap();
    assert!(!replay.dispatch);
    let mut altered = req.clone();
    altered.choice = domain::ApprovalChoice::Deny;
    assert!(matches!(
        repo.reserve_approval_decision(session.id, approval.id, owner, altered)
            .await,
        Err(shared::AppError::Conflict(_))
    ));
    let listed = repo.list_session_approvals(session.id).await.unwrap();
    assert!(!listed[0].prompt.contains("private"));
    assert_eq!(listed[0].detail["api_key"], "redacted");
    let second = repo
        .upsert_runtime_approval_request(app::RuntimeApprovalCreate {
            session_id: session.id,
            session_run_id: run.id,
            agent_id: run.agent_id,
            runtime_run_id: run.runtime_run_id.clone().unwrap(),
            runtime_approval_id: Some("request_two".into()),
            prompt: "Second action".into(),
            detail: serde_json::json!({}),
        })
        .await
        .unwrap();
    assert!(matches!(
        repo.reserve_approval_decision(session.id, second.id, owner, req.clone())
            .await,
        Err(shared::AppError::Conflict(_))
    ));
    let delivered = repo.deliver_approval_decision(a.decision.id).await.unwrap();
    assert_eq!(delivered.state, domain::ApprovalDecisionState::Delivered);
    assert_eq!(
        repo.deliver_approval_decision(a.decision.id)
            .await
            .unwrap()
            .id,
        a.decision.id
    );
    assert!(
        !repo
            .reserve_approval_decision(session.id, approval.id, owner, req)
            .await
            .unwrap()
            .dispatch
    );
    let pending = repo.list_session_approvals(session.id).await.unwrap();
    assert_eq!(
        pending.iter().find(|x| x.id == approval.id).unwrap().state,
        domain::RuntimeApprovalState::Approved
    );
    assert_eq!(
        pending.iter().find(|x| x.id == second.id).unwrap().state,
        domain::RuntimeApprovalState::Pending
    );
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    assert!(
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE runtime_approval_decisions SET choice='deny' WHERE id=$1",
            [a.decision.id.into()]
        ))
        .await
        .is_err()
    );
    let audits = repo
        .list_audit_log(app::AuditLogFilter {
            entity_id: Some(a.decision.id.to_string()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(audits.len(), 2);
}

#[tokio::test]
async fn targeted_approval_http_requires_human_and_unknown_ack_is_not_repeated() {
    let Some((repo, owner, _)) = fixture().await else {
        return;
    };
    let (session, run, approval) = approval_fixture(&repo, owner, "approval-http").await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE agents SET api_port=$2 WHERE id=$1",
        [run.agent_id.into(), i32::from(port).into()],
    ))
    .await
    .unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let runtime=axum::Router::new().route("/v1/runs/run_approval_test/approval",axum::routing::post(move |headers:axum::http::HeaderMap,axum::Json(body):axum::Json<serde_json::Value>| {let counter=counter.clone();async move {
        assert!(headers.get("authorization").unwrap().to_str().unwrap().starts_with("Bearer fc_"));
        assert_eq!(body["choice"],"once"); assert_eq!(body["resolve_all"],false);
        assert!(matches!(body["request_id"].as_str(),Some("request_one"|"request_two")));
        counter.fetch_add(1,Ordering::SeqCst);
        // Success transport but wrong action identity is unknown, never delivered.
        axum::Json(serde_json::json!({"object":"hermes.run.approval_response","run_id":"run_approval_test","request_id":if body["request_id"]=="request_one" {"foreign"} else {"request_two"},"choice":"once","resolved":1}))
    }}));
    let hermes = tokio::spawn(async move { axum::serve(listener, runtime).await.unwrap() });
    let repo = Arc::new(repo);
    let mut config = AppConfig::default();
    config.fleet.runtime_token_secret = "approval-isolated-test-secret".into();
    let config = Arc::new(config);
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime = Arc::new(infra::runtime::LocalRuntimeSupervisor::new(
        config.clone(),
        repo.clone(),
        events.clone(),
    ));
    let (restart_tx, _) = tokio::sync::mpsc::channel(1);
    let ctx = Arc::new(app::AppContext::new(
        config,
        repo.clone(),
        Arc::new(infra::FilesystemProvisioner),
        runtime,
        events,
        restart_tx,
    ));
    let user = api::middleware::CurrentUser {
        id: owner,
        role: domain::SystemRole::User,
        is_system_admin: false,
    };
    let route = format!(
        "/api/v1/sessions/{}/approvals/{}/decision",
        session.id, approval.id
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let router = axum::Router::new()
        .route(
            "/machine/api/v1/sessions/{session_id}/approvals/{approval_id}/decision",
            axum::routing::post(api::routes::approvals::decide),
        )
        .nest(
            "/human",
            axum::Router::new()
                .route(
                    "/api/v1/sessions/{session_id}/approvals/{approval_id}/decision",
                    axum::routing::get(api::routes::approvals::read)
                        .post(api::routes::approvals::decide),
                )
                .layer(axum::Extension(api::middleware::VerifiedHumanSession)),
        )
        .layer(axum::Extension(user))
        .with_state(ctx);
    let fleet = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let client = reqwest::Client::new();
    let body = serde_json::json!({"choice":"once","idempotency_key":"http-command"});
    assert_eq!(
        client
            .post(format!("{base}/machine{route}"))
            .json(&body)
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::FORBIDDEN
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let url = format!("{base}/human{route}");
    let first = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .unwrap()
        .json::<domain::ApprovalDecision>()
        .await
        .unwrap();
    assert_eq!(first.state, domain::ApprovalDecisionState::Uncertain);
    let repeated = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .unwrap()
        .json::<domain::ApprovalDecision>()
        .await
        .unwrap();
    assert_eq!(first.id, repeated.id);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let read = client
        .get(&url)
        .send()
        .await
        .unwrap()
        .json::<domain::ApprovalDecision>()
        .await
        .unwrap();
    assert_eq!(read.id, first.id);
    assert_eq!(
        repo.list_session_approvals(session.id).await.unwrap()[0].state,
        domain::RuntimeApprovalState::Pending
    );
    assert_eq!(
        client
            .post(&url)
            .json(&serde_json::json!({"choice":"deny","idempotency_key":"http-command"}))
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::CONFLICT
    );
    let second = repo
        .upsert_runtime_approval_request(app::RuntimeApprovalCreate {
            session_id: session.id,
            session_run_id: run.id,
            agent_id: run.agent_id,
            runtime_run_id: run.runtime_run_id.clone().unwrap(),
            runtime_approval_id: Some("request_two".into()),
            prompt: "Exact second action".into(),
            detail: serde_json::json!({}),
        })
        .await
        .unwrap();
    let second_url = format!(
        "{base}/human/api/v1/sessions/{}/approvals/{}/decision",
        session.id, second.id
    );
    let success_body = serde_json::json!({"choice":"once","idempotency_key":"second-http-command"});
    let success = client
        .post(&second_url)
        .json(&success_body)
        .send()
        .await
        .unwrap()
        .json::<domain::ApprovalDecision>()
        .await
        .unwrap();
    assert_eq!(success.state, domain::ApprovalDecisionState::Delivered);
    assert_eq!(
        client
            .post(&second_url)
            .json(&success_body)
            .send()
            .await
            .unwrap()
            .json::<domain::ApprovalDecision>()
            .await
            .unwrap()
            .id,
        success.id
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    let listed = repo.list_session_approvals(session.id).await.unwrap();
    assert_eq!(
        listed.iter().find(|x| x.id == second.id).unwrap().state,
        domain::RuntimeApprovalState::Approved
    );
    assert_eq!(
        listed.iter().find(|x| x.id == approval.id).unwrap().state,
        domain::RuntimeApprovalState::Pending
    );
    hermes.abort();
    fleet.abort();
}

#[tokio::test]
async fn targeted_approval_delivery_fk_does_not_deadlock_concurrent_replay() {
    let Some((repo, owner, _)) = fixture().await else {
        return;
    };
    let (session, _, approval) = approval_fixture(&repo, owner, "approval-lock-order").await;
    let req = domain::ApprovalDecisionRequest {
        choice: domain::ApprovalChoice::Once,
        idempotency_key: "approval-lock-command".into(),
    };
    let first = repo
        .reserve_approval_decision(session.id, approval.id, owner, req.clone())
        .await
        .unwrap();
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let delivery = db.begin().await.unwrap();
    // Hold the delivery prefix: approval row locked, actor FK not yet written.
    delivery
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id FROM runtime_approval_requests WHERE id=$1 FOR UPDATE",
            [approval.id.into()],
        ))
        .await
        .unwrap();
    let repo = Arc::new(repo);
    let replay_repo = repo.clone();
    let replay = tokio::spawn(async move {
        replay_repo
            .reserve_approval_decision(session.id, approval.id, owner, req)
            .await
    });
    let mut user_locked = false;
    for _ in 0..100 {
        if let Err(error) = db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT id FROM users WHERE id=$1 FOR UPDATE NOWAIT",
                [owner.into()],
            ))
            .await
        {
            assert!(
                error.to_string().contains("could not obtain lock"),
                "unexpected probe failure: {error}"
            );
            user_locked = true;
            break;
        }
        sleep(Duration::from_millis(20)).await;
    }
    assert!(
        user_locked,
        "duplicate command did not reach its user serialization lock"
    );
    // The same FK write as production delivery must succeed while the duplicate
    // holds the user lock and waits for this approval. FOR UPDATE would deadlock.
    tokio::time::timeout(
        Duration::from_secs(3),
        delivery.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE runtime_approval_requests SET resolved_by_user_id=$2 WHERE id=$1",
            [approval.id.into(), owner.into()],
        )),
    )
    .await
    .expect("delivery and duplicate formed a lock cycle")
    .unwrap();
    delivery.commit().await.unwrap();
    let duplicate = tokio::time::timeout(Duration::from_secs(3), replay)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(!duplicate.dispatch);
    assert_eq!(duplicate.decision.id, first.decision.id);
    assert_eq!(
        repo.deliver_approval_decision(first.decision.id)
            .await
            .unwrap()
            .state,
        domain::ApprovalDecisionState::Delivered
    );
}

#[tokio::test]
async fn targeted_approval_oidc_provider_tokens_do_not_prove_a_human_session() {
    use base64::Engine;
    use rsa::{pkcs8::EncodePrivateKey, traits::PublicKeyParts};
    let Some((repo, owner, _)) = fixture().await else {
        return;
    };
    assert!(
        std::env::var_os("FLEET_CONTROL_AUTH__CENTRAL_JWKS_URI").is_none(),
        "this test requires isolated direct-provider auth"
    );
    let (session, _, approval) = approval_fixture(&repo, owner, "approval-oidc-machine").await;
    let private = rsa::RsaPrivateKey::new(&mut rand_core::OsRng, 2048).unwrap();
    let public = rsa::RsaPublicKey::from(&private);
    let encoder = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let jwks = serde_json::json!({"keys":[{"kty":"RSA","kid":"isolated-provider","alg":"RS256","use":"sig","n":encoder.encode(public.n().to_bytes_be()),"e":encoder.encode(public.e().to_bytes_be())}]});
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let issuer = format!("http://{}", listener.local_addr().unwrap());
    let provider = axum::Router::new().route(
        "/keys",
        axum::routing::get(move || {
            let jwks = jwks.clone();
            async move { axum::Json(jwks) }
        }),
    );
    let provider_server =
        tokio::spawn(async move { axum::serve(listener, provider).await.unwrap() });
    let pem = private.to_pkcs8_pem(rsa::pkcs8::LineEnding::LF).unwrap();
    let key = jsonwebtoken::EncodingKey::from_rsa_pem(pem.as_bytes()).unwrap();
    let now = chrono::Utc::now().timestamp();
    let mut claims = serde_json::json!({"sub":owner,"email":"oidc-test@example.test","role":"user","iat":now,"exp":now+300,"aud":"fleet-approval-test","iss":issuer});
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some("isolated-provider".into());
    let token = jsonwebtoken::encode(&header, &claims, &key).unwrap();
    let mut config = AppConfig::default();
    config.auth.mode = "oidc".into();
    config.auth.oidc_issuer_url = issuer.clone();
    config.auth.oidc_jwks_url = format!("{issuer}/keys");
    config.auth.oidc_audience = "fleet-approval-test".into();
    config.fleet.runtime_token_secret = "isolated-approval-test-secret".into();
    let config = Arc::new(config);
    let repo = Arc::new(repo);
    let (events, _) = tokio::sync::broadcast::channel(32);
    let (restart_tx, _) = tokio::sync::mpsc::channel(1);
    let runtime = Arc::new(infra::runtime::LocalRuntimeSupervisor::new(
        config.clone(),
        repo.clone(),
        events.clone(),
    ));
    let ctx = Arc::new(app::AppContext::new(
        config,
        repo.clone(),
        Arc::new(infra::FilesystemProvisioner),
        runtime,
        events,
        restart_tx,
    ));
    let router = axum::Router::new()
        .route(
            "/proof",
            axum::routing::get(
                |human: Option<axum::Extension<api::middleware::VerifiedHumanSession>>| async move {
                    axum::Json(serde_json::json!({"human":human.is_some()}))
                },
            ),
        )
        .route(
            "/api/v1/sessions/{session_id}/approvals/{approval_id}/decision",
            axum::routing::post(api::routes::approvals::decide),
        )
        .layer(axum::middleware::from_fn_with_state(
            ctx.clone(),
            api::middleware::require_auth,
        ))
        .with_state(ctx);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let client = reqwest::Client::new();
    let proof = client
        .get(format!("{base}/proof"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(proof.status(), reqwest::StatusCode::OK);
    assert_eq!(
        proof.json::<serde_json::Value>().await.unwrap()["human"],
        false
    );
    for sid in [None, Some("unverified-provider-session")] {
        if let Some(sid) = sid {
            claims["sid"] = serde_json::json!(sid);
        }
        let token = jsonwebtoken::encode(&header, &claims, &key).unwrap();
        let response = client
            .post(format!(
                "{base}/api/v1/sessions/{}/approvals/{}/decision",
                session.id, approval.id
            ))
            .bearer_auth(token)
            .json(&serde_json::json!({"choice":"once","idempotency_key":"oidc-machine-command"}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
    }
    assert!(matches!(
        repo.approval_decision(session.id, approval.id).await,
        Err(shared::AppError::NotFound(_))
    ));
    server.abort();
    provider_server.abort();
}

fn configuration() -> UpdateAgentConfigRequest {
    UpdateAgentConfigRequest {
        config_json: serde_json::json!({"model": "test-model"}),
        soul_md: "Follow the assigned workflow.".into(),
        env_json: serde_json::json!({}),
    }
}

#[tokio::test]
async fn pm_draft_chat_creation_is_atomic_idempotent_and_cannot_queue_a_prompt() {
    let Some((repo, reservation)) = pm_fixture().await else {
        return;
    };
    let original = repo.get_session(reservation.session_id).await.unwrap();
    let mut binding = repo
        .get_task_chat_binding(original.id)
        .await
        .unwrap()
        .unwrap();
    binding.task_id = Uuid::new_v4();
    binding.root_task_id = binding.task_id;
    let command = domain::CreatePmDraftChat {
        binding: binding.clone(),
        title: "PM Draft".into(),
        task_key: "PM-atomic".into(),
        idempotency_key: "atomic-pm-chat".into(),
    };
    let (a, b) = tokio::join!(
        repo.create_pm_draft_chat(command.clone(), original.user_id),
        repo.create_pm_draft_chat(command.clone(), original.user_id)
    );
    let session = a.unwrap();
    assert_eq!(session.id, b.unwrap().id);
    assert_eq!(session.visibility, domain::SessionVisibility::Private);
    assert_eq!(session.leader_agent_id, None);
    assert_eq!(
        repo.get_task_chat_binding(session.id).await.unwrap(),
        Some(binding)
    );
    assert!(
        repo.list_session_agent_runs(session.id)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        repo.list_session_messages(session.id)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(!repo.has_pending_session_dispatch(session.id).await.unwrap());
    assert!(
        repo.create_session_message(session.id, prompt("must-not-dispatch"), original.user_id)
            .await
            .is_err()
    );
    assert_eq!(
        repo.list_session_participants(session.id)
            .await
            .unwrap()
            .len(),
        2
    );
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let restarted = PostgresFleetRepository::new(
        sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
            .await
            .unwrap(),
    );
    assert_eq!(
        restarted
            .create_pm_draft_chat(command.clone(), original.user_id)
            .await
            .unwrap()
            .id,
        session.id
    );
    let mut changed = command.clone();
    changed.title = "Changed payload".into();
    assert!(
        repo.create_pm_draft_chat(changed, original.user_id)
            .await
            .is_err()
    );
    let mut duplicate = command.clone();
    duplicate.idempotency_key = "another-command-same-task".into();
    assert!(
        repo.create_pm_draft_chat(duplicate, original.user_id)
            .await
            .is_err()
    );
    let row = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT (SELECT count(*) FROM agent_sessions WHERE user_id=$1 AND task_key='PM-atomic') AS sessions,
          (SELECT count(*) FROM audit_log WHERE entity_id=$2 AND action='session.pm_draft.create') AS audits,
          (SELECT count(*) FROM session_events WHERE session_id=$3 AND event_type='task.bound') AS events",
        [original.user_id.into(),session.id.to_string().into(),session.id.into()])).await.unwrap().unwrap();
    for column in ["sessions", "audits", "events"] {
        assert_eq!(row.try_get::<i64>("", column).unwrap(), 1);
    }
    let mut left = command.clone();
    left.binding.task_id = Uuid::new_v4();
    left.binding.root_task_id = left.binding.task_id;
    left.task_key = "PM-pair-race".into();
    left.idempotency_key = "pair-race-left".into();
    let mut right = left.clone();
    right.idempotency_key = "pair-race-right".into();
    let (a, b) = tokio::join!(
        repo.create_pm_draft_chat(left, original.user_id),
        repo.create_pm_draft_chat(right, original.user_id)
    );
    assert_ne!(a.is_ok(), b.is_ok());
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT count(*) AS n FROM agent_sessions WHERE user_id=$1 AND task_key='PM-pair-race'",
            [original.user_id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.try_get::<i64>("", "n").unwrap(),
        1,
        "losing transaction must not leave a free chat"
    );
}

#[tokio::test]
async fn pm_draft_chat_rejects_foreign_owner_invalid_identity_and_non_pm_agent_without_rows() {
    let Some((repo, reservation)) = pm_fixture().await else {
        return;
    };
    let original = repo.get_session(reservation.session_id).await.unwrap();
    let mut binding = repo
        .get_task_chat_binding(original.id)
        .await
        .unwrap()
        .unwrap();
    binding.task_id = Uuid::new_v4();
    binding.root_task_id = binding.task_id;
    let command = domain::CreatePmDraftChat {
        binding,
        title: "PM Draft".into(),
        task_key: "PM-rejected".into(),
        idempotency_key: "reject-pm".into(),
    };
    let mut invalid = command.clone();
    invalid.binding.owner_subject = Uuid::new_v4().to_string();
    assert!(
        repo.create_pm_draft_chat(invalid, original.user_id)
            .await
            .is_err()
    );
    let mut invalid = command.clone();
    invalid.binding.tracker_instance_id = " tracker".into();
    assert!(
        repo.create_pm_draft_chat(invalid, original.user_id)
            .await
            .is_err()
    );
    let mut invalid = command.clone();
    invalid.binding.root_task_id = Uuid::new_v4();
    assert!(
        repo.create_pm_draft_chat(invalid, original.user_id)
            .await
            .is_err()
    );
    let mut invalid = command.clone();
    invalid.binding.agent_id = agent(&repo).await;
    assert!(
        repo.create_pm_draft_chat(invalid, original.user_id)
            .await
            .is_err()
    );
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE users SET is_active=false WHERE id=$1",
        [original.user_id.into()],
    ))
    .await
    .unwrap();
    assert!(
        repo.create_pm_draft_chat(command, original.user_id)
            .await
            .is_err()
    );
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT count(*) AS n FROM agent_sessions WHERE user_id=$1 AND task_key='PM-rejected'",
            [original.user_id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "n").unwrap(), 0);
}

async fn pm_fixture() -> Option<(PostgresFleetRepository, domain::PmRunReservation)> {
    let (repo, _, _) = fixture().await?;
    let subject = Uuid::new_v4().to_string();
    let owner = repo
        .find_or_create_central_user(
            &subject,
            &format!("{}@example.test", Uuid::new_v4()),
            "PM owner",
        )
        .await
        .unwrap()
        .id;
    let agent_id = agent(&repo).await;
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE agents SET sdlc_role='project_manager' WHERE id=$1",
        [agent_id.into()],
    ))
    .await
    .unwrap();
    let session = repo
        .create_session(chat(agent_id, "pm-chat"), owner)
        .await
        .unwrap();
    let binding = domain::TaskChatBinding {
        tracker_instance_id: "tracker-pm-test".into(),
        project_id: Uuid::new_v4(),
        task_id: Uuid::new_v4(),
        root_task_id: Uuid::new_v4(),
        agent_id,
        owner_subject: subject,
    };
    repo.bind_task_chat(session.id, binding.clone(), "bind-pm".into())
        .await
        .unwrap();
    Some((
        repo,
        domain::PmRunReservation {
            session_id: session.id,
            session_run_id: Uuid::new_v4(),
            identity: domain::PmExecutionIdentity {
                task: "SDLC-42".into(),
                execution_ref: Uuid::new_v4().to_string(),
                tracker_instance_ref: binding.tracker_instance_id,
                tracker_project_ref: binding.project_id.to_string(),
                task_ref: binding.task_id.to_string(),
                root_ref: binding.root_task_id.to_string(),
                agent_ref: agent_id.to_string(),
                assignment_operation_key: "assign-pm".into(),
                assignment_ref: Uuid::new_v4().to_string(),
                assignment_revision: 1,
            },
            binding_ref: "workflow-binding-42".into(),
            dispatch_operation_key: "dispatch-pm".into(),
            checkpoint_ref: None,
            fence: 1,
        },
    ))
}

fn tracker_page(binding: &domain::TaskChatBinding) -> domain::TrackerOutboxPage {
    domain::TrackerOutboxPage {
        events: [7, 12]
            .into_iter()
            .map(|sequence| domain::TrackerOutboxEvent {
                sequence,
                event_id: Uuid::new_v4(),
                task_id: binding.task_id,
                event_type: "clarification.answered".into(),
                payload: domain::TrackerEventPayload {
                    contract_version: 1,
                    tracker_instance_id: binding.tracker_instance_id.clone(),
                    project_id: binding.project_id,
                    task_id: binding.task_id,
                    root_task_id: binding.root_task_id,
                    owner_subject: binding.owner_subject.clone(),
                    stage: domain::TrackerStage::Draft,
                    requirement_revision: Some(1),
                    result: serde_json::json!({"text":"private-answer-secret","token":"not-for-transcript"}),
                },
                created_at: chrono::Utc::now(),
            })
            .collect(),
    }
}

fn metadata_page(
    binding: &domain::TaskChatBinding,
    after: i64,
    sequences: &[i64],
) -> domain::TrackerMetadataPage {
    use sha2::{Digest, Sha256};
    let events: Vec<serde_json::Value> = sequences.iter().map(|sequence| {
        let mut event = serde_json::json!({"sequence":sequence.to_string(),"event_id":Uuid::new_v4(),
            "task_id":binding.task_id,"event_type":"requirements.published",
            "created_at":"2026-10-02T00:00:00.000000000Z",
            "payload":{"tracker_instance_id":binding.tracker_instance_id,"project_id":binding.project_id,
                "root_task_id":binding.root_task_id,"owner_subject":binding.owner_subject,"stage":"Draft",
                "current_requirement_revision":1,"resource":{"requirement_revision":1,"content_hash":"a".repeat(64)}}});
        let digest = hex::encode(Sha256::digest(serde_json::to_vec(&serde_json::json!({
            "contract_version":1,"projection":"metadata_v1","event":event
        })).unwrap()));
        event["metadata_sha256"] = serde_json::json!(digest);
        event
    }).collect();
    let bytes = serde_json::to_vec(&serde_json::json!({"contract_version":1,"projection":"metadata_v1",
        "after":after.to_string(),"next_after":sequences.last().copied().unwrap_or(after).to_string(),
        "has_more":false,"events":events})).unwrap();
    domain::TrackerMetadataPage::decode(&bytes, binding, after).unwrap()
}

#[tokio::test]
async fn tracker_metadata_poller_authenticates_replays_and_keeps_failed_source_cursors() {
    use axum::{
        Json,
        extract::Query,
        http::{HeaderMap, StatusCode},
        response::IntoResponse,
        routing::get,
    };
    use infra::tracker_event_poller::TrackerEventPoller;
    use std::collections::HashMap;
    let Some((repo, reservation)) = pm_fixture().await else {
        return;
    };
    let session = reservation.session_id;
    let binding = repo.get_task_chat_binding(session).await.unwrap().unwrap();
    let repo = Arc::new(repo);
    let machine = Uuid::new_v4().to_string();
    let pat = format!("sdlc_pat_{}", "test-only-read-credential".repeat(2));
    let expected_header = format!("Bearer {pat}");
    let mode = Arc::new(AtomicUsize::new(0));
    let events_called = Arc::new(AtomicUsize::new(0));
    let leaked = Arc::new(AtomicUsize::new(0));
    let auth_mode = mode.clone();
    let auth_subject = machine.clone();
    let auth_header = expected_header.clone();
    let scope_mode = mode.clone();
    let scope_binding = binding.clone();
    let scope_header = expected_header.clone();
    let event_mode = mode.clone();
    let event_count = events_called.clone();
    let event_binding = binding.clone();
    let event_page = metadata_page(&binding, 0, &[7, 12]);
    let event_header = expected_header;
    let leak_count = leaked.clone();
    let router = axum::Router::new()
        .route("/auth/tokens/introspect", get(move |headers:HeaderMap| {
            let (mode,sub,expected) = (auth_mode.clone(),auth_subject.clone(),auth_header.clone());
            async move {
                assert_eq!(headers["authorization"], expected);
                match mode.load(Ordering::SeqCst) {
                    2 => (StatusCode::UNAUTHORIZED,"denied").into_response(),
                    3 => (StatusCode::SERVICE_UNAVAILABLE,"secret-upstream-error").into_response(),
                    1 => Json(serde_json::json!({"sub":sub,"email":"machine@example.test","scopes":["task-tracker:read","task-tracker:write"]})).into_response(),
                    _ => Json(serde_json::json!({"sub":sub,"email":"machine@example.test","scopes":["task-tracker:read"]})).into_response(),
                }
            }
        }))
        .route("/api/v1/sdlc/project-access",get(move |headers:HeaderMap| {
            let (mode,binding,expected) = (scope_mode.clone(),scope_binding.clone(),scope_header.clone());
            async move {
                assert_eq!(headers["authorization"], expected);
                let projects = if mode.load(Ordering::SeqCst)==5 {vec![]} else {vec![binding.project_id]};
                let instance = if mode.load(Ordering::SeqCst)==4 {"foreign"} else {&binding.tracker_instance_id};
                Json(serde_json::json!({"contract_version":1,"tracker_instance_id":instance,"project_ids":projects}))
            }
        }))
        .route(&format!("/api/v1/issues/{}/sdlc/events",binding.task_id), get(move |headers:HeaderMap,Query(query):Query<HashMap<String,String>>| {
            let (mode,count,binding,page,expected) = (event_mode.clone(),event_count.clone(),event_binding.clone(),event_page.clone(),event_header.clone());
            async move {
                assert_eq!(headers["authorization"], expected);
                assert_eq!(query["projection"], "metadata_v1");
                assert_eq!(query["limit"],"100");
                assert_eq!(query["max_bytes"],"262144");
                count.fetch_add(1,Ordering::SeqCst);
                match mode.load(Ordering::SeqCst) {
                    6 => (StatusCode::FORBIDDEN,"membership revoked").into_response(),
                    7 => (StatusCode::OK,"x".repeat(domain::TRACKER_METADATA_BUDGET+1)).into_response(),
                    8 => {
                        let mut page = page;
                        page.events[1].metadata_sha256="b".repeat(64);
                        Json(page).into_response()
                    }
                    9 => (StatusCode::FOUND,[("location","/leak")],"").into_response(),
                    _ if query["after"]=="0" => Json(page).into_response(),
                    _ => Json(metadata_page(&binding,12,&[])).into_response(),
                }
            }
        }))
        .route("/leak",get(move || { let count=leak_count.clone();async move {count.fetch_add(1,Ordering::SeqCst);"must not follow"} }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut config = shared::TrackerConfig {
        url: url.clone(),
        instance_id: binding.tracker_instance_id.clone(),
        events: shared::TrackerEventsConfig {
            enabled: true,
            auth_url: url,
            machine_subject: machine,
            read_pat: pat,
            poll_interval_seconds: 1,
        },
        ..Default::default()
    };
    let mut disabled = config.clone();
    disabled.events.enabled = false;
    assert!(
        TrackerEventPoller::configured(&disabled, repo.clone())
            .unwrap()
            .is_none()
    );
    let mut poller = TrackerEventPoller::configured(&config, repo.clone())
        .unwrap()
        .unwrap();
    let before = repo.list_session_events(session, 0).await.unwrap().len();
    let runs_before =
        serde_json::to_value(repo.list_session_agent_runs(session).await.unwrap()).unwrap();
    for blocked in 1..=4 {
        mode.store(blocked, Ordering::SeqCst);
        assert!(poller.poll_once().await.is_err());
        assert_eq!(events_called.load(Ordering::SeqCst), 0);
    }
    mode.store(5, Ordering::SeqCst);
    assert_eq!(poller.poll_once().await.unwrap().considered, 0);
    for blocked in 6..=9 {
        mode.store(blocked, Ordering::SeqCst);
        let report = poller.poll_once().await.unwrap();
        assert_eq!(
            (report.considered, report.projected, report.blocked),
            (1, 0, 1)
        );
        assert_eq!(repo.tracker_metadata_cursor(session).await.unwrap(), 0);
        assert_eq!(
            repo.list_session_events(session, 0).await.unwrap().len(),
            before
        );
    }
    assert_eq!(leaked.load(Ordering::SeqCst), 0);
    mode.store(0, Ordering::SeqCst);
    let mut other_poller = TrackerEventPoller::configured(&config, repo.clone())
        .unwrap()
        .unwrap();
    let (a, b) = tokio::join!(poller.poll_once(), other_poller.poll_once());
    assert_eq!(a.unwrap().projected + b.unwrap().projected, 2);
    assert_eq!(repo.tracker_metadata_cursor(session).await.unwrap(), 12);
    assert!(!repo.has_pending_session_dispatch(session).await.unwrap());
    assert_eq!(
        serde_json::to_value(repo.list_session_agent_runs(session).await.unwrap()).unwrap(),
        runs_before
    );
    let count = repo.list_session_events(session, 0).await.unwrap().len();
    let restarted_db =
        sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
            .await
            .unwrap();
    let restarted = Arc::new(PostgresFleetRepository::new(restarted_db));
    let mut poller = TrackerEventPoller::configured(&config, restarted)
        .unwrap()
        .unwrap();
    assert_eq!(poller.poll_once().await.unwrap().projected, 0);
    assert_eq!(
        repo.list_session_events(session, 0).await.unwrap().len(),
        count
    );
    mode.store(6, Ordering::SeqCst);
    assert_eq!(poller.poll_once().await.unwrap().blocked, 1);
    assert_eq!(repo.tracker_metadata_cursor(session).await.unwrap(), 12);
    config.events.machine_subject = Uuid::new_v4().to_string();
    let mut wrong_subject = TrackerEventPoller::configured(&config, repo.clone())
        .unwrap()
        .unwrap();
    assert!(wrong_subject.poll_once().await.is_err());
    server.abort();
}

#[tokio::test]
async fn tracker_projection_target_scan_is_project_scoped_keyset_and_active_owner_only() {
    let Some((repo, reservation)) = pm_fixture().await else {
        return;
    };
    let binding = repo
        .get_task_chat_binding(reservation.session_id)
        .await
        .unwrap()
        .unwrap();
    let scope = [binding.project_id];
    let rows = repo
        .tracker_projection_targets(&binding.tracker_instance_id, &scope, None)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].session_id, reservation.session_id);
    assert!(
        repo.tracker_projection_targets("foreign", &scope, None)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        repo.tracker_projection_targets(&binding.tracker_instance_id, &[], None)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        repo.tracker_projection_targets(&binding.tracker_instance_id, &[Uuid::new_v4()], None)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        repo.tracker_projection_targets(
            &binding.tracker_instance_id,
            &scope,
            Some(reservation.session_id)
        )
        .await
        .unwrap()
        .is_empty()
    );
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let owner = repo
        .get_session(reservation.session_id)
        .await
        .unwrap()
        .user_id;
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE users SET is_active=false WHERE id=$1",
        [owner.into()],
    ))
    .await
    .unwrap();
    assert!(
        repo.tracker_projection_targets(&binding.tracker_instance_id, &scope, None)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn tracker_metadata_empty_page_pins_format_and_replay_is_atomic_after_restart() {
    let Some((repo, reservation)) = pm_fixture().await else {
        return;
    };
    let session = reservation.session_id;
    let binding = repo.get_task_chat_binding(session).await.unwrap().unwrap();
    assert_eq!(repo.tracker_metadata_cursor(session).await.unwrap(), 0);
    let empty = metadata_page(&binding, 0, &[]);
    assert_eq!(
        repo.project_tracker_metadata(session, binding.clone(), 0, empty)
            .await
            .unwrap()
            .projected,
        0
    );
    assert!(repo.tracker_event_cursor(session).await.is_err());
    assert!(
        repo.project_tracker_events(session, binding.clone(), 0, tracker_page(&binding))
            .await
            .is_err()
    );
    let page = metadata_page(&binding, 0, &[7, i64::MAX]);
    let (first, second) = tokio::join!(
        repo.project_tracker_metadata(session, binding.clone(), 0, page.clone()),
        repo.project_tracker_metadata(session, binding.clone(), 0, page.clone())
    );
    assert_eq!(first.unwrap().projected + second.unwrap().projected, 2);
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let restarted = PostgresFleetRepository::new(
        sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
            .await
            .unwrap(),
    );
    assert_eq!(
        restarted.tracker_metadata_cursor(session).await.unwrap(),
        i64::MAX
    );
    assert_eq!(
        restarted
            .project_tracker_metadata(session, binding.clone(), 0, page.clone())
            .await
            .unwrap()
            .projected,
        0
    );
    let count = restarted
        .list_session_events(session, 0)
        .await
        .unwrap()
        .len();
    let mut changed = page.clone();
    changed.events[1].event_id = Uuid::new_v4();
    // A correctly hashed but changed source event must still conflict with its immutable receipt.
    let mut raw = serde_json::to_value(&changed).unwrap();
    raw["events"][1]
        .as_object_mut()
        .unwrap()
        .remove("metadata_sha256");
    use sha2::{Digest, Sha256};
    let digest = hex::encode(Sha256::digest(
        serde_json::to_vec(&serde_json::json!({
            "contract_version":1,"projection":"metadata_v1","event":raw["events"][1]
        }))
        .unwrap(),
    ));
    raw["events"][1]["metadata_sha256"] = serde_json::json!(digest);
    let changed =
        domain::TrackerMetadataPage::decode(&serde_json::to_vec(&raw).unwrap(), &binding, 0)
            .unwrap();
    assert!(
        restarted
            .project_tracker_metadata(session, binding.clone(), 0, changed)
            .await
            .is_err()
    );
    assert_eq!(
        restarted
            .list_session_events(session, 0)
            .await
            .unwrap()
            .len(),
        count
    );
    for sql in [
        "UPDATE tracker_event_cursors SET projection='legacy_full_v1' WHERE session_id=$1",
        "UPDATE tracker_event_cursors SET sequence=0 WHERE session_id=$1",
        "DELETE FROM tracker_event_cursors WHERE session_id=$1",
    ] {
        assert!(
            db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                sql,
                [session.into()]
            ))
            .await
            .is_err()
        );
    }
    let events = restarted.list_session_events(session, 0).await.unwrap();
    let projected: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == "tracker_event")
        .collect();
    assert_eq!(projected.len(), 2);
    assert_eq!(
        projected[1].payload["source_sequence"],
        i64::MAX.to_string()
    );
    assert!(!projected[1].payload.to_string().contains("content_hash"));
}

#[tokio::test]
async fn tracker_metadata_first_page_failure_rolls_back_projection_pin_and_all_mirrors() {
    let Some((repo, reservation)) = pm_fixture().await else {
        return;
    };
    let session = reservation.session_id;
    let binding = repo.get_task_chat_binding(session).await.unwrap().unwrap();
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let page = metadata_page(&binding, 0, &[7, 12]);
    let events_before = repo.list_session_events(session, 0).await.unwrap().len();
    let messages_before = repo.list_session_messages(session).await.unwrap().len();
    let constraint = format!("metadata_qa_{}", session.simple());
    db.execute_unprepared(&format!("ALTER TABLE tracker_event_inbox ADD CONSTRAINT {constraint} CHECK (session_id <> '{session}'::uuid OR source_sequence <> 12) NOT VALID")).await.unwrap();
    let result = repo
        .project_tracker_metadata(session, binding.clone(), 0, page.clone())
        .await;
    db.execute_unprepared(&format!(
        "ALTER TABLE tracker_event_inbox DROP CONSTRAINT {constraint}"
    ))
    .await
    .unwrap();
    assert!(result.is_err());
    assert_eq!(repo.tracker_event_cursor(session).await.unwrap(), 0);
    assert_eq!(repo.tracker_metadata_cursor(session).await.unwrap(), 0);
    assert_eq!(
        repo.list_session_events(session, 0).await.unwrap().len(),
        events_before
    );
    assert_eq!(
        repo.list_session_messages(session).await.unwrap().len(),
        messages_before
    );
    assert_eq!(
        repo.project_tracker_metadata(session, binding.clone(), 0, page)
            .await
            .unwrap()
            .projected,
        2
    );

    let other = repo
        .create_session(
            chat(binding.agent_id, "legacy-pin"),
            repo.get_session(session).await.unwrap().user_id,
        )
        .await
        .unwrap();
    let mut other_binding = binding;
    other_binding.task_id = Uuid::new_v4();
    repo.bind_task_chat(other.id, other_binding.clone(), "legacy-pin-bind".into())
        .await
        .unwrap();
    repo.project_tracker_events(
        other.id,
        other_binding.clone(),
        0,
        domain::TrackerOutboxPage { events: vec![] },
    )
    .await
    .unwrap();
    assert!(
        repo.project_tracker_metadata(
            other.id,
            other_binding.clone(),
            0,
            metadata_page(&other_binding, 0, &[])
        )
        .await
        .is_err()
    );
}

#[tokio::test]
async fn tracker_inbox_projects_concurrent_replay_once_and_preserves_cursor_after_reconnect() {
    let Some((repo, reservation)) = pm_fixture().await else {
        return;
    };
    let session = reservation.session_id;
    let binding = repo.get_task_chat_binding(session).await.unwrap().unwrap();
    let page = tracker_page(&binding);
    assert_eq!(repo.tracker_event_cursor(session).await.unwrap(), 0);
    let (first, second) = tokio::join!(
        repo.project_tracker_events(session, binding.clone(), 0, page.clone()),
        repo.project_tracker_events(session, binding.clone(), 0, page.clone())
    );
    let receipts = [first.unwrap(), second.unwrap()];
    assert_eq!(
        receipts
            .iter()
            .map(|receipt| receipt.projected)
            .sum::<usize>(),
        2
    );
    assert!(receipts.iter().all(|receipt| receipt.cursor == 12));
    let messages = repo.list_session_messages(session).await.unwrap();
    let mirrored: Vec<_> = messages
        .iter()
        .filter(|message| {
            message
                .runtime_message_id
                .as_deref()
                .is_some_and(|id| id.starts_with("tracker:"))
        })
        .collect();
    assert_eq!(mirrored.len(), 2);
    for message in mirrored {
        assert!(message.body.contains("delivery is separate"));
        assert!(!message.body.contains("private-answer-secret"));
        assert!(!message.body.contains("not-for-transcript"));
        assert_eq!(
            message.delivery_state,
            domain::MessageDeliveryState::Mirrored
        );
    }
    let events = repo.list_session_events(session, 0).await.unwrap();
    let projected: Vec<_> = events
        .iter()
        .filter(|event| event.event_type == "tracker_event")
        .collect();
    assert_eq!(projected.len(), 2);
    assert!(
        projected
            .iter()
            .all(|event| !event.payload.to_string().contains("private-answer-secret"))
    );
    assert!(!repo.has_pending_session_dispatch(session).await.unwrap());
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let restarted = PostgresFleetRepository::new(db);
    assert_eq!(restarted.tracker_event_cursor(session).await.unwrap(), 12);
    let replay = restarted
        .project_tracker_events(session, binding, 0, page)
        .await
        .unwrap();
    assert_eq!(replay.projected, 0);
    assert_eq!(replay.cursor, 12);
    assert_eq!(
        restarted
            .list_session_events(session, 0)
            .await
            .unwrap()
            .len(),
        events.len()
    );
}

#[tokio::test]
async fn tracker_inbox_rejects_changed_payload_stale_pages_and_foreign_bindings_atomically() {
    let Some((repo, reservation)) = pm_fixture().await else {
        return;
    };
    let session = reservation.session_id;
    let binding = repo.get_task_chat_binding(session).await.unwrap().unwrap();
    let page = tracker_page(&binding);
    repo.project_tracker_events(session, binding.clone(), 0, page.clone())
        .await
        .unwrap();
    let initial = repo.list_session_events(session, 0).await.unwrap().len();
    let mut changed = page.clone();
    changed.events[1].payload.result = serde_json::json!({"text":"changed-answer"});
    assert!(
        repo.project_tracker_events(session, binding.clone(), 0, changed)
            .await
            .is_err()
    );
    let mut stale = page.clone();
    stale.events[1].sequence = 99;
    stale.events[1].event_id = Uuid::new_v4();
    assert!(
        repo.project_tracker_events(session, binding.clone(), 0, stale.clone())
            .await
            .is_err()
    );
    let mut foreign = binding.clone();
    foreign.project_id = Uuid::new_v4();
    stale.events = vec![stale.events[1].clone()];
    stale.events[0].payload.project_id = foreign.project_id;
    assert!(
        repo.project_tracker_events(session, foreign, 12, stale)
            .await
            .is_err()
    );
    assert_eq!(repo.tracker_event_cursor(session).await.unwrap(), 12);
    assert_eq!(
        repo.list_session_events(session, 0).await.unwrap().len(),
        initial
    );
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    assert!(
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE tracker_event_inbox SET payload_hash=repeat('a',64) WHERE session_id=$1",
            [session.into()]
        ))
        .await
        .is_err()
    );
    assert!(
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "DELETE FROM tracker_event_inbox WHERE session_id=$1",
            [session.into()]
        ))
        .await
        .is_err()
    );
}

#[tokio::test]
async fn tracker_inbox_mid_page_database_failure_rolls_back_messages_events_and_cursor() {
    let Some((repo, reservation)) = pm_fixture().await else {
        return;
    };
    let session = reservation.session_id;
    let binding = repo.get_task_chat_binding(session).await.unwrap().unwrap();
    let page = tracker_page(&binding);
    let events_before = repo.list_session_events(session, 0).await.unwrap().len();
    let messages_before = repo.list_session_messages(session).await.unwrap().len();
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let constraint = format!("inbox_qa_{}", session.simple());
    db.execute_unprepared(&format!(
        "ALTER TABLE tracker_event_inbox ADD CONSTRAINT {constraint} CHECK (session_id <> '{session}'::uuid OR source_sequence <> 12) NOT VALID"
    )).await.unwrap();
    let failed = repo
        .project_tracker_events(session, binding.clone(), 0, page.clone())
        .await;
    db.execute_unprepared(&format!(
        "ALTER TABLE tracker_event_inbox DROP CONSTRAINT {constraint}"
    ))
    .await
    .unwrap();
    assert!(failed.is_err());
    assert_eq!(repo.tracker_event_cursor(session).await.unwrap(), 0);
    assert_eq!(
        repo.list_session_events(session, 0).await.unwrap().len(),
        events_before
    );
    assert_eq!(
        repo.list_session_messages(session).await.unwrap().len(),
        messages_before
    );
    let retried = repo
        .project_tracker_events(session, binding, 0, page)
        .await
        .unwrap();
    assert_eq!(retried.projected, 2);
    assert_eq!(retried.cursor, 12);
}

#[tokio::test]
async fn task_approval_history_survives_reassignment_but_not_project_access_revocation() {
    let Some((repo, reservation)) = pm_fixture().await else {
        return;
    };
    let session = repo.get_session(reservation.session_id).await.unwrap();
    let binding = repo
        .get_task_chat_binding(session.id)
        .await
        .unwrap()
        .unwrap();
    repo.reserve_pm_run(reservation.clone()).await.unwrap();
    repo.accept_pm_run(
        reservation.session_run_id,
        "run_history".into(),
        reservation.runtime_session_id(),
    )
    .await
    .unwrap();
    let make_request = |id: &str| app::RuntimeApprovalCreate {
        session_id: session.id,
        session_run_id: reservation.session_run_id,
        agent_id: session.primary_agent_id,
        runtime_run_id: "run_history".into(),
        runtime_approval_id: Some(id.into()),
        prompt: "Bounded action".into(),
        detail: serde_json::json!({}),
    };
    let old = repo
        .upsert_runtime_approval_request(make_request("old_action"))
        .await
        .unwrap();
    let fresh = repo
        .upsert_runtime_approval_request(make_request("new_action"))
        .await
        .unwrap();
    let command = domain::ApprovalDecisionRequest {
        choice: domain::ApprovalChoice::Once,
        idempotency_key: "history-command".into(),
    };
    let reserved = repo
        .reserve_approval_decision(session.id, old.id, session.user_id, command.clone())
        .await
        .unwrap();
    let decision = repo
        .deliver_approval_decision(reserved.decision.id)
        .await
        .unwrap();

    let context = domain::TrackerTaskContext {
        contract_version: 1,
        tracker_instance_id: binding.tracker_instance_id.clone(),
        project_id: binding.project_id,
        task_id: binding.task_id,
        root_task_id: binding.root_task_id,
        owner_subject: binding.owner_subject.clone(),
        stage: domain::TrackerStage::Draft,
        requirement_revision: None,
        waiting_reason: None,
        permissions: domain::TrackerPermissions {
            can_answer: false,
            can_confirm: false,
        },
        assignment: Some(domain::TrackerPmAssignment {
            assignment_id: Uuid::new_v4(),
            execution_id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            version: 2,
            machine_subject: "replacement-pm".into(),
        }),
    };
    let revoked = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let check = revoked.clone();
    let list_check = revoked.clone();
    let project = binding.project_id;
    let instance = binding.tracker_instance_id.clone();
    let tracker = axum::Router::new()
    .route("/api/v1/sdlc/project-access",axum::routing::get(move || {
        let check=list_check.clone();let instance=instance.clone();
        async move { axum::Json(serde_json::json!({"contract_version":1,"tracker_instance_id":instance,"project_ids":if check.load(Ordering::SeqCst) {vec![]} else {vec![project]}})) }
    }))
    .route(
        "/api/v1/issues/{id}/sdlc/context",
        axum::routing::get(
            move |axum::extract::Path(id): axum::extract::Path<Uuid>,
                  headers: axum::http::HeaderMap| {
                let check = check.clone();
                let context = context.clone();
                async move {
                    assert_eq!(id, context.task_id);
                    assert!(
                        headers
                            .get("authorization")
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .starts_with("Bearer ")
                    );
                    if check.load(Ordering::SeqCst) {
                        (
                            axum::http::StatusCode::FORBIDDEN,
                            axum::Json(serde_json::json!({"error":"project_access_revoked"})),
                        )
                    } else {
                        (
                            axum::http::StatusCode::OK,
                            axum::Json(serde_json::to_value(context).unwrap()),
                        )
                    }
                }
            },
        ),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let tracker_url = format!("http://{}", listener.local_addr().unwrap());
    let tracker_server = tokio::spawn(async move { axum::serve(listener, tracker).await.unwrap() });
    let repo = Arc::new(repo);
    let mut config = AppConfig::default();
    config.tracker.url = tracker_url;
    config.tracker.instance_id = binding.tracker_instance_id;
    let config = Arc::new(config);
    let (events, _) = tokio::sync::broadcast::channel(32);
    // No runtime server exists: all historical commands must replay without dispatch.
    let runtime = Arc::new(infra::runtime::LocalRuntimeSupervisor::new(
        config.clone(),
        repo.clone(),
        events.clone(),
    ));
    let (restart_tx, _) = tokio::sync::mpsc::channel(1);
    let ctx = Arc::new(app::AppContext::new(
        config,
        repo.clone(),
        Arc::new(infra::FilesystemProvisioner),
        runtime,
        events,
        restart_tx,
    ));
    let token = ctx
        .auth
        .issue_tokens(
            &repo
                .find_user_by_id(session.user_id)
                .await
                .unwrap()
                .unwrap(),
        )
        .unwrap()
        .response
        .access_token;
    let router = axum::Router::new()
        .route(
            "/api/v1/sessions",
            axum::routing::get(api::routes::sessions::list_sessions),
        )
        .route(
            "/api/v1/sessions/{session_id}",
            axum::routing::get(api::routes::sessions::get_session),
        )
        .route(
            "/api/v1/sessions/{session_id}/messages",
            axum::routing::get(api::routes::sessions::list_session_messages),
        )
        .route(
            "/api/v1/sessions/{session_id}/participants",
            axum::routing::get(api::routes::sessions::list_session_participants),
        )
        .route(
            "/api/v1/sessions/{session_id}/runs",
            axum::routing::get(api::routes::sessions::list_session_agent_runs),
        )
        .route(
            "/api/v1/sessions/{session_id}/history",
            axum::routing::get(api::routes::task_chats::history),
        )
        .route(
            "/api/v1/sessions/{session_id}/chat-controls",
            axum::routing::get(api::routes::task_chats::controls),
        )
        .route(
            "/api/v1/sessions/{session_id}/stream",
            axum::routing::get(api::routes::sessions::stream_session),
        )
        .route(
            "/api/v1/sessions/{session_id}/runs/{run_id}/stop",
            axum::routing::post(api::routes::sessions::stop_session_run),
        )
        .route(
            "/api/v1/sessions/{session_id}/approvals",
            axum::routing::get(api::routes::approvals::list),
        )
        .route(
            "/api/v1/sessions/{session_id}/approvals/{approval_id}/decision",
            axum::routing::get(api::routes::approvals::read).post(api::routes::approvals::decide),
        )
        .layer(axum::Extension(api::middleware::VerifiedHumanSession))
        .layer(axum::Extension(api::middleware::CurrentUser {
            id: session.user_id,
            role: domain::SystemRole::User,
            is_system_admin: false,
        }))
        .with_state(ctx);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!(
        "http://{}/api/v1/sessions/{}/approvals",
        listener.local_addr().unwrap(),
        session.id
    );
    let fleet_server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let client = reqwest::Client::new();
    let url = format!("{base}/{}/decision", old.id);
    let session_url = base.strip_suffix("/approvals").unwrap();
    let list_url = session_url
        .strip_suffix(&format!("/{}", session.id))
        .unwrap();
    let sessions = client
        .get(list_url)
        .bearer_auth("verified-owner-fixture")
        .send()
        .await
        .unwrap()
        .json::<Vec<domain::AgentSession>>()
        .await
        .unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, session.id);
    for suffix in [
        "",
        "/messages",
        "/participants",
        "/runs",
        "/history",
        "/chat-controls",
    ] {
        assert_eq!(
            client
                .get(format!("{session_url}{suffix}"))
                .bearer_auth("verified-owner-fixture")
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::OK,
            "historical chat read failed after reassignment: {suffix}"
        );
    }
    let read = client
        .get(&url)
        .bearer_auth("verified-owner-fixture")
        .send()
        .await
        .unwrap();
    assert_eq!(read.status(), reqwest::StatusCode::OK);
    assert_eq!(
        read.json::<domain::ApprovalDecision>().await.unwrap().id,
        decision.id
    );
    let replay = client
        .post(&url)
        .bearer_auth("verified-owner-fixture")
        .json(&command)
        .send()
        .await
        .unwrap();
    assert_eq!(replay.status(), reqwest::StatusCode::OK);
    assert_eq!(
        replay.json::<domain::ApprovalDecision>().await.unwrap().id,
        decision.id
    );
    assert_eq!(
        client
            .post(&url)
            .bearer_auth("verified-owner-fixture")
            .json(&serde_json::json!({"choice":"once","idempotency_key":"changed-command"}))
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::CONFLICT
    );
    assert_eq!(
        client
            .post(format!("{base}/{}/decision", fresh.id))
            .bearer_auth("verified-owner-fixture")
            .json(&serde_json::json!({"choice":"once","idempotency_key":"stale-command"}))
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::CONFLICT
    );
    assert!(matches!(
        repo.approval_decision(session.id, fresh.id).await,
        Err(shared::AppError::NotFound(_))
    ));
    let mut stream = client
        .get(format!("{session_url}/stream"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(stream.status(), reqwest::StatusCode::OK);
    let snapshot = tokio::time::timeout(Duration::from_secs(3), stream.chunk())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(std::str::from_utf8(&snapshot).unwrap().contains("snapshot"));
    revoked.store(true, Ordering::SeqCst);
    assert!(
        client
            .get(list_url)
            .bearer_auth("verified-owner-fixture")
            .send()
            .await
            .unwrap()
            .json::<Vec<domain::AgentSession>>()
            .await
            .unwrap()
            .is_empty(),
        "legacy session list leaked revoked task metadata"
    );
    repo.insert_session_message_mirror(
        session.id,
        Some(session.primary_agent_id),
        "Must not leak after revocation".into(),
        MessageKind::AssistantMessage,
        Some("revoked-event".into()),
    )
    .await
    .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(3), stream.chunk())
            .await
            .expect("revoked stream did not terminate")
            .unwrap()
            .is_none(),
        "revoked stream delivered queued data"
    );
    assert_eq!(
        client
            .get(format!("{session_url}/stream"))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::FORBIDDEN
    );
    for suffix in [
        "",
        "/messages",
        "/participants",
        "/runs",
        "/history",
        "/chat-controls",
    ] {
        assert_eq!(
            client
                .get(format!("{session_url}{suffix}"))
                .bearer_auth("verified-owner-fixture")
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::FORBIDDEN,
            "revoked project still exposed chat: {suffix}"
        );
    }
    assert_eq!(
        client
            .post(format!(
                "{session_url}/runs/{}/stop",
                reservation.session_run_id
            ))
            .bearer_auth("verified-owner-fixture")
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::FORBIDDEN
    );
    assert_eq!(
        client
            .get(&url)
            .bearer_auth("verified-owner-fixture")
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::FORBIDDEN
    );
    assert_eq!(
        client
            .get(&base)
            .bearer_auth("verified-owner-fixture")
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::FORBIDDEN
    );
    assert_eq!(
        client
            .post(&url)
            .bearer_auth("verified-owner-fixture")
            .json(&command)
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::FORBIDDEN
    );
    assert_eq!(
        repo.approval_decision(session.id, old.id)
            .await
            .unwrap()
            .state,
        domain::ApprovalDecisionState::Delivered
    );
    fleet_server.abort();
    tracker_server.abort();
}

#[tokio::test]
async fn pm_reservation_is_atomic_idempotent_and_holds_capacity_when_acceptance_is_unknown() {
    let Some((repo, request)) = pm_fixture().await else {
        return;
    };
    let mut wrong = request.clone();
    wrong.identity.root_ref = Uuid::new_v4().to_string();
    assert!(matches!(
        repo.reserve_pm_run(wrong).await,
        Err(shared::AppError::Forbidden)
    ));
    let (first, replay) = tokio::join!(
        repo.reserve_pm_run(request.clone()),
        repo.reserve_pm_run(request.clone())
    );
    let first = first.unwrap();
    assert_eq!(first.reservation, replay.unwrap().reservation);
    assert!(first.hermes_run_ref.is_none());
    assert!(matches!(
        repo.update_session_agent_run_dispatch(
            request.session_run_id,
            None,
            SessionRunState::Completed,
            None
        )
        .await,
        Err(shared::AppError::Conflict(_))
    ));
    assert!(
        repo.observe_pm_run(request.session_run_id, domain::PmRuntimeStatus::Completed)
            .await
            .is_err()
    );
    let mut conflict = request.clone();
    conflict.fence = 2;
    assert!(matches!(
        repo.reserve_pm_run(conflict).await,
        Err(shared::AppError::Conflict(_))
    ));
    let mut concurrent = request.clone();
    concurrent.session_run_id = Uuid::new_v4();
    concurrent.dispatch_operation_key = "next-pm-dispatch".into();
    assert!(matches!(
        repo.reserve_pm_run(concurrent.clone()).await,
        Err(shared::AppError::Conflict(_))
    ));
    repo.accept_pm_run(
        request.session_run_id,
        "run_pm_test".into(),
        request.runtime_session_id(),
    )
    .await
    .unwrap();
    repo.accept_pm_run(
        request.session_run_id,
        "run_pm_test".into(),
        request.runtime_session_id(),
    )
    .await
    .unwrap();
    assert!(matches!(
        repo.accept_pm_run(
            request.session_run_id,
            "run_other".into(),
            request.runtime_session_id()
        )
        .await,
        Err(shared::AppError::Conflict(_))
    ));
    assert!(matches!(
        repo.accept_pm_run(
            request.session_run_id,
            "run_pm_test".into(),
            "foreign-session".into()
        )
        .await,
        Err(shared::AppError::Conflict(_))
    ));
    repo.observe_pm_run(request.session_run_id, domain::PmRuntimeStatus::Running)
        .await
        .unwrap();
    repo.observe_pm_run(request.session_run_id, domain::PmRuntimeStatus::Completed)
        .await
        .unwrap();
    repo.observe_pm_run(request.session_run_id, domain::PmRuntimeStatus::Completed)
        .await
        .unwrap();
    assert!(matches!(
        repo.observe_pm_run(request.session_run_id, domain::PmRuntimeStatus::Running)
            .await,
        Err(shared::AppError::Conflict(_))
    ));
    assert!(matches!(
        repo.observe_pm_run(request.session_run_id, domain::PmRuntimeStatus::Failed)
            .await,
        Err(shared::AppError::Conflict(_))
    ));
    for action in [
        "pm.run.reserved",
        "pm.run.accepted",
        "pm.run.terminal_verified",
    ] {
        let rows = repo
            .list_audit_log(app::AuditLogFilter {
                action: Some(action.into()),
                entity_id: Some(request.session_run_id.to_string()),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(rows.len(), 1, "replay must not duplicate {action}");
        assert!(
            rows[0].actor_user_id.is_none(),
            "system proof must not impersonate the owner"
        );
    }
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    assert!(db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE pm_run_bindings SET reservation=jsonb_set(reservation,'{fence}','2') WHERE session_run_id=$1",
        [request.session_run_id.into()])).await.is_err());
    assert!(
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE pm_run_bindings SET terminal_status=NULL WHERE session_run_id=$1",
            [request.session_run_id.into()]
        ))
        .await
        .is_err()
    );
    assert_eq!(
        repo.get_session_agent_run(request.session_run_id)
            .await
            .unwrap()
            .state,
        SessionRunState::Completed
    );
    // A delayed stream failure must not undo fresh authenticated terminal proof.
    repo.update_session_agent_run_dispatch(
        request.session_run_id,
        None,
        SessionRunState::Waiting,
        Some("late stream EOF".into()),
    )
    .await
    .unwrap();
    assert_eq!(
        repo.get_session_agent_run(request.session_run_id)
            .await
            .unwrap()
            .state,
        SessionRunState::Completed
    );
    assert!(repo.reserve_pm_run(concurrent).await.is_ok());
}

#[tokio::test]
async fn pm_terminal_readback_and_late_stream_updates_serialize_without_reopening_capacity() {
    for status in [
        domain::PmRuntimeStatus::Completed,
        domain::PmRuntimeStatus::Failed,
        domain::PmRuntimeStatus::Cancelled,
        domain::PmRuntimeStatus::Stopped,
    ] {
        let Some((repo, request)) = pm_fixture().await else {
            return;
        };
        repo.reserve_pm_run(request.clone()).await.unwrap();
        repo.accept_pm_run(
            request.session_run_id,
            "run_terminal_race".into(),
            request.runtime_session_id(),
        )
        .await
        .unwrap();
        if status == domain::PmRuntimeStatus::Completed {
            let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
                .await
                .unwrap();
            db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE session_agent_runs SET runtime_run_id='foreign-run' WHERE id=$1",
                [request.session_run_id.into()],
            ))
            .await
            .unwrap();
            assert!(matches!(
                repo.observe_pm_run(request.session_run_id, status).await,
                Err(shared::AppError::Conflict(_))
            ));
            assert!(
                repo.get_pm_run(request.session_run_id)
                    .await
                    .unwrap()
                    .terminal_status
                    .is_none()
            );
            assert_eq!(
                repo.get_session_agent_run(request.session_run_id)
                    .await
                    .unwrap()
                    .state,
                SessionRunState::Running
            );
            db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE session_agent_runs SET runtime_run_id='run_terminal_race' WHERE id=$1",
                [request.session_run_id.into()],
            ))
            .await
            .unwrap();
        }
        let (proof, delayed) = tokio::join!(
            repo.observe_pm_run(request.session_run_id, status),
            repo.update_session_agent_run_dispatch(
                request.session_run_id,
                Some("run_terminal_race".into()),
                SessionRunState::Waiting,
                Some("late EOF".into())
            )
        );
        proof.unwrap();
        delayed.unwrap();
        let expected = match status {
            domain::PmRuntimeStatus::Completed => SessionRunState::Completed,
            domain::PmRuntimeStatus::Failed => SessionRunState::Failed,
            _ => SessionRunState::Cancelled,
        };
        assert_eq!(
            repo.get_session_agent_run(request.session_run_id)
                .await
                .unwrap()
                .state,
            expected
        );
        assert!(matches!(
            repo.update_session_agent_run_dispatch(
                request.session_run_id,
                Some("wrong_runtime".into()),
                SessionRunState::Running,
                None
            )
            .await,
            Err(shared::AppError::Conflict(_))
        ));
        let mut next = request.clone();
        next.session_run_id = Uuid::new_v4();
        next.dispatch_operation_key = "next-after-terminal".into();
        repo.reserve_pm_run(next.clone()).await.unwrap();
        repo.update_session_agent_run_dispatch(
            request.session_run_id,
            None,
            SessionRunState::Running,
            None,
        )
        .await
        .unwrap();
        next.session_run_id = Uuid::new_v4();
        next.dispatch_operation_key = "third-still-held".into();
        assert!(matches!(
            repo.reserve_pm_run(next).await,
            Err(shared::AppError::Conflict(_))
        ));
    }
}

#[tokio::test]
async fn pm_reservation_does_not_deadlock_mirror_agent_foreign_key() {
    let Some((repo, request)) = pm_fixture().await else {
        return;
    };
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let mirror = db.begin().await.unwrap();
    mirror
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id FROM agent_sessions WHERE id=$1 FOR UPDATE",
            [request.session_id.into()],
        ))
        .await
        .unwrap();
    let repo = Arc::new(repo);
    let worker = repo.clone();
    let reservation = request.clone();
    let pending = tokio::spawn(async move { worker.reserve_pm_run(reservation).await });
    let mut locked = false;
    for _ in 0..100 {
        if let Err(error) = db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT id FROM agents WHERE id=$1 FOR UPDATE NOWAIT",
                [request.identity.agent_id().unwrap().into()],
            ))
            .await
        {
            assert!(
                error.to_string().contains("could not obtain lock"),
                "unexpected probe error: {error}"
            );
            locked = true;
            break;
        }
        sleep(Duration::from_millis(20)).await;
    }
    assert!(
        locked,
        "PM reservation never reached its agent capacity lock"
    );
    // Production mirroring holds the session lock before this author-agent FK.
    tokio::time::timeout(Duration::from_secs(3), mirror.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO session_messages(id,session_id,author_type,author_agent_id,body,message_kind,delivery_state,created_at)
         VALUES($1,$2,'agent',$3,'Mirror lock regression','assistant_message','mirrored',now())",
        [Uuid::new_v4().into(),request.session_id.into(),request.identity.agent_id().unwrap().into()]))).await
        .expect("PM capacity reservation and mirror agent FK formed a lock cycle").unwrap();
    mirror.commit().await.unwrap();
    let result = tokio::time::timeout(Duration::from_secs(3), pending)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(result.reservation, request);
    assert_eq!(
        repo.list_session_messages(request.session_id)
            .await
            .unwrap()
            .iter()
            .filter(|message| message.body == "Mirror lock regression")
            .count(),
        1
    );
}

#[tokio::test]
async fn undispatched_approval_failure_is_terminal_and_never_replayed_as_delivery() {
    let Some((repo, owner, _)) = fixture().await else {
        return;
    };
    let (session, _, approval) = approval_fixture(&repo, owner, "undispatched-approval").await;
    let command = domain::ApprovalDecisionRequest {
        choice: domain::ApprovalChoice::Once,
        idempotency_key: "undispatched-command".into(),
    };
    let first = repo
        .reserve_approval_decision(session.id, approval.id, owner, command.clone())
        .await
        .unwrap();
    let failed = repo
        .fail_undispatched_approval_decision(first.decision.id)
        .await
        .unwrap();
    assert_eq!(failed.state, domain::ApprovalDecisionState::Failed);
    assert_eq!(
        repo.fail_undispatched_approval_decision(first.decision.id)
            .await
            .unwrap()
            .state,
        failed.state
    );
    let replay = repo
        .reserve_approval_decision(session.id, approval.id, owner, command)
        .await
        .unwrap();
    assert!(!replay.dispatch);
    assert_eq!(replay.decision.id, failed.id);
    assert_eq!(replay.decision.state, failed.state);
    assert!(
        repo.deliver_approval_decision(first.decision.id)
            .await
            .is_err()
    );
    assert_eq!(
        repo.list_session_approvals(session.id).await.unwrap()[0].state,
        domain::RuntimeApprovalState::Pending
    );
}

#[tokio::test]
async fn task_approval_rechecks_assignment_after_waiting_for_actor_lock() {
    let Some((repo, reservation)) = pm_fixture().await else {
        return;
    };
    let session = repo.get_session(reservation.session_id).await.unwrap();
    let binding = repo
        .get_task_chat_binding(session.id)
        .await
        .unwrap()
        .unwrap();
    repo.reserve_pm_run(reservation.clone()).await.unwrap();
    repo.accept_pm_run(
        reservation.session_run_id,
        "run_race".into(),
        reservation.runtime_session_id(),
    )
    .await
    .unwrap();
    let approval = repo
        .upsert_runtime_approval_request(app::RuntimeApprovalCreate {
            session_id: session.id,
            session_run_id: reservation.session_run_id,
            agent_id: session.primary_agent_id,
            runtime_run_id: "run_race".into(),
            runtime_approval_id: Some("race_action".into()),
            prompt: "Bounded action".into(),
            detail: serde_json::json!({}),
        })
        .await
        .unwrap();
    let mut context = domain::TrackerTaskContext {
        contract_version: 1,
        tracker_instance_id: binding.tracker_instance_id.clone(),
        project_id: binding.project_id,
        task_id: binding.task_id,
        root_task_id: binding.root_task_id,
        owner_subject: binding.owner_subject,
        stage: domain::TrackerStage::Draft,
        requirement_revision: None,
        waiting_reason: None,
        permissions: domain::TrackerPermissions {
            can_answer: false,
            can_confirm: false,
        },
        assignment: Some(domain::TrackerPmAssignment {
            assignment_id: Uuid::parse_str(&reservation.identity.assignment_ref).unwrap(),
            execution_id: Uuid::parse_str(&reservation.identity.execution_ref).unwrap(),
            agent_id: session.primary_agent_id,
            version: reservation.identity.assignment_revision,
            machine_subject: "pm-machine".into(),
        }),
    };
    let initial = context.clone();
    context.assignment.as_mut().unwrap().version += 1;
    let changed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let check = changed.clone();
    let reads = Arc::new(AtomicUsize::new(0));
    let read_count = reads.clone();
    let tracker = axum::Router::new().route(
        "/api/v1/issues/{id}/sdlc/context",
        axum::routing::get(move || {
            let initial = initial.clone();
            let context = context.clone();
            let check = check.clone();
            let read_count = read_count.clone();
            async move {
                read_count.fetch_add(1, Ordering::SeqCst);
                axum::Json(if check.load(Ordering::SeqCst) {
                    context
                } else {
                    initial
                })
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let tracker_url = format!("http://{}", listener.local_addr().unwrap());
    let tracker_server = tokio::spawn(async move { axum::serve(listener, tracker).await.unwrap() });
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let hermes=axum::Router::new().route("/v1/runs/run_race/approval",axum::routing::post(move || {
        let count=count.clone(); async move {count.fetch_add(1,Ordering::SeqCst); axum::Json(serde_json::json!({"object":"hermes.run.approval_response","run_id":"run_race","request_id":"race_action","choice":"once","resolved":1}))}
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let hermes_server = tokio::spawn(async move { axum::serve(listener, hermes).await.unwrap() });
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE agents SET api_port=$2 WHERE id=$1",
        [session.primary_agent_id.into(), i32::from(port).into()],
    ))
    .await
    .unwrap();
    let actor_lock = db.begin().await.unwrap();
    let blocker: i32 = actor_lock
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT pg_backend_pid() AS pid",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap();
    actor_lock
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id FROM users WHERE id=$1 FOR UPDATE",
            [session.user_id.into()],
        ))
        .await
        .unwrap();
    let mut config = AppConfig::default();
    config.tracker.url = tracker_url;
    config.tracker.instance_id = binding.tracker_instance_id;
    config.fleet.runtime_token_secret = "isolated-race-test-secret".into();
    let config = Arc::new(config);
    let repo = Arc::new(repo);
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime = Arc::new(infra::runtime::LocalRuntimeSupervisor::new(
        config.clone(),
        repo.clone(),
        events.clone(),
    ));
    let (restart_tx, _) = tokio::sync::mpsc::channel(1);
    let ctx = Arc::new(app::AppContext::new(
        config,
        repo.clone(),
        Arc::new(infra::FilesystemProvisioner),
        runtime,
        events,
        restart_tx,
    ));
    let router = axum::Router::new()
        .route(
            "/api/v1/sessions/{session_id}/approvals/{approval_id}/decision",
            axum::routing::post(api::routes::approvals::decide),
        )
        .layer(axum::Extension(api::middleware::VerifiedHumanSession))
        .layer(axum::Extension(api::middleware::CurrentUser {
            id: session.user_id,
            role: domain::SystemRole::User,
            is_system_admin: false,
        }))
        .with_state(ctx);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!(
        "http://{}/api/v1/sessions/{}/approvals/{}/decision",
        listener.local_addr().unwrap(),
        session.id,
        approval.id
    );
    let fleet_server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let client = reqwest::Client::new();
    let command = serde_json::json!({"choice":"once","idempotency_key":"race-command"});
    let response_client = client.clone();
    let request_url = url.clone();
    let body = command.clone();
    let pending = tokio::spawn(async move {
        response_client
            .post(request_url)
            .bearer_auth("verified-owner-fixture")
            .json(&body)
            .send()
            .await
            .unwrap()
    });
    let mut waiting = false;
    for _ in 0..100 {
        waiting=db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid))) AS waiting",[blocker.into()])).await.unwrap().unwrap().try_get("","waiting").unwrap();
        if waiting {
            break;
        }
        sleep(Duration::from_millis(20)).await;
    }
    assert!(waiting, "approval never waited for the actor lock");
    assert_eq!(
        reads.load(Ordering::SeqCst),
        1,
        "initial assignment was not checked before reservation"
    );
    changed.store(true, Ordering::SeqCst);
    actor_lock.commit().await.unwrap();
    let response = tokio::time::timeout(Duration::from_secs(5), pending)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
    assert_eq!(reads.load(Ordering::SeqCst), 2);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "stale assignment dispatched an approval"
    );
    let failed = repo
        .approval_decision(session.id, approval.id)
        .await
        .unwrap();
    assert_eq!(failed.state, domain::ApprovalDecisionState::Failed);
    let replay = client
        .post(&url)
        .bearer_auth("verified-owner-fixture")
        .json(&command)
        .send()
        .await
        .unwrap()
        .json::<domain::ApprovalDecision>()
        .await
        .unwrap();
    assert_eq!(replay.id, failed.id);
    assert_eq!(replay.state, domain::ApprovalDecisionState::Failed);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    fleet_server.abort();
    tracker_server.abort();
    hermes_server.abort();
}

#[tokio::test]
async fn pm_callback_requires_machine_auth_and_fresh_authenticated_runtime_proof() {
    let Some((repo, request)) = pm_fixture().await else {
        return;
    };
    repo.reserve_pm_run(request.clone()).await.unwrap();
    let effective_session = Uuid::new_v4().to_string();
    assert_ne!(effective_session, request.runtime_session_id());
    repo.accept_pm_run(
        request.session_run_id,
        "run_pm_readback".into(),
        effective_session.clone(),
    )
    .await
    .unwrap();
    let agent_id = request.identity.agent_id().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE agents SET api_port=$2 WHERE id=$1",
        [agent_id.into(), i32::from(port).into()],
    ))
    .await
    .unwrap();
    let state = Arc::new(AtomicUsize::new(0));
    let reads = Arc::new(AtomicUsize::new(0));
    let runtime_state = state.clone();
    let runtime_reads = reads.clone();
    let expected_session = effective_session;
    let runtime_router = axum::Router::new().route("/v1/runs/run_pm_readback", axum::routing::get(move |headers: axum::http::HeaderMap| {
        let state = runtime_state.clone(); let reads = runtime_reads.clone(); let session = expected_session.clone();
        async move {
            assert!(headers.get("authorization").unwrap().to_str().unwrap().starts_with("Bearer fc_"));
            reads.fetch_add(1, Ordering::SeqCst);
            let phase = state.load(Ordering::SeqCst);
            axum::Json(serde_json::json!({"object":"hermes.run","run_id":if phase == 2 {"run_foreign"} else {"run_pm_readback"},
                "session_id":if phase == 3 { "foreign-session" } else { &session },"status":if phase == 1 || phase == 4 {"completed"} else {"running"},
                "completed":phase == 1 || phase == 4,"partial":phase == 4,"interrupted":false}))
        }
    }));
    let hermes_server =
        tokio::spawn(async move { axum::serve(listener, runtime_router).await.unwrap() });
    let repo = Arc::new(repo);
    let mut config = AppConfig::default();
    config.fleet.runtime_token_secret = "isolated-test-runtime-secret".into();
    config.pm.readback_token = "isolated-readback-test-secret-123456789".into();
    let secret = config.pm.readback_token.clone();
    let config = Arc::new(config);
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime = Arc::new(infra::runtime::LocalRuntimeSupervisor::new(
        config.clone(),
        repo.clone(),
        events.clone(),
    ));
    let (restart_tx, _) = tokio::sync::mpsc::channel(1);
    let ctx = Arc::new(app::AppContext::new(
        config,
        repo.clone(),
        Arc::new(infra::FilesystemProvisioner),
        runtime,
        events,
        restart_tx,
    ));
    let callback = axum::Router::new()
        .route(
            "/internal/runtime/v1/pm/runs/{session_run_id}",
            axum::routing::get(api::routes::pm_runtime::readback),
        )
        .with_state(ctx);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!(
        "http://{}/internal/runtime/v1/pm/runs/{}",
        listener.local_addr().unwrap(),
        request.session_run_id
    );
    let fleet_server = tokio::spawn(async move { axum::serve(listener, callback).await.unwrap() });
    let client = reqwest::Client::new();
    assert_eq!(
        client.get(&url).send().await.unwrap().status(),
        reqwest::StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        client
            .get(&url)
            .bearer_auth("human-or-invalid-machine-token")
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::UNAUTHORIZED
    );
    assert_eq!(reads.load(Ordering::SeqCst), 0);
    let first = client
        .get(&url)
        .bearer_auth(&secret)
        .send()
        .await
        .unwrap()
        .json::<domain::PmRuntimeObservation>()
        .await
        .unwrap();
    assert_eq!(first.status, domain::PmRuntimeStatus::Running);
    assert_eq!(first.identity, request.identity);
    assert_eq!(first.session_run_id, request.session_run_id);
    assert_eq!(first.dispatch_operation_key, request.dispatch_operation_key);
    state.store(2, Ordering::SeqCst);
    assert_eq!(
        client
            .get(&url)
            .bearer_auth(&secret)
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::SERVICE_UNAVAILABLE
    );
    state.store(3, Ordering::SeqCst);
    assert_eq!(
        client
            .get(&url)
            .bearer_auth(&secret)
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::SERVICE_UNAVAILABLE
    );
    state.store(4, Ordering::SeqCst);
    assert_eq!(
        client
            .get(&url)
            .bearer_auth(&secret)
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::SERVICE_UNAVAILABLE
    );
    assert!(
        repo.get_pm_run(request.session_run_id)
            .await
            .unwrap()
            .terminal_status
            .is_none(),
        "partial PM result must not create a terminal proof or release its slot"
    );
    state.store(1, Ordering::SeqCst);
    let completed = client
        .get(&url)
        .bearer_auth(&secret)
        .send()
        .await
        .unwrap()
        .json::<domain::PmRuntimeObservation>()
        .await
        .unwrap();
    assert_eq!(completed.status, domain::PmRuntimeStatus::Completed);
    assert_ne!(first.observation_ref, completed.observation_ref);
    assert_eq!(
        repo.get_pm_run(request.session_run_id)
            .await
            .unwrap()
            .terminal_status,
        Some(domain::PmRuntimeStatus::Completed)
    );
    state.store(0, Ordering::SeqCst);
    assert_eq!(
        client
            .get(&url)
            .bearer_auth(&secret)
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::CONFLICT
    );
    assert!(reads.load(Ordering::SeqCst) >= 4);
    hermes_server.abort();
    assert_eq!(
        client
            .get(&url)
            .bearer_auth(&secret)
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::SERVICE_UNAVAILABLE
    );
    fleet_server.abort();
}

#[tokio::test]
async fn concurrent_idempotency_and_private_message_authorization() {
    let Some((repo, owner, stranger)) = fixture().await else {
        return;
    };
    let agent_id = agent(&repo).await;
    let request = chat(agent_id, "create-once");
    let (a, b) = tokio::join!(
        repo.create_session(request.clone(), owner),
        repo.create_session(request.clone(), owner)
    );
    let first = a.unwrap();
    assert_eq!(first.id, b.unwrap().id);
    assert!(first.leader_agent_id.is_none());
    let mut changed = request;
    changed.title = "Different payload".into();
    assert!(
        repo.create_session(changed, owner)
            .await
            .unwrap_err()
            .to_string()
            .contains("different session payload")
    );
    assert!(
        repo.create_session_message(first.id, prompt("stranger"), stranger)
            .await
            .is_err()
    );
    let (a, b) = tokio::join!(
        repo.create_session_message(first.id, prompt("once"), owner),
        repo.create_session_message(first.id, prompt("once"), owner)
    );
    let a = a.unwrap();
    let b = b.unwrap();
    assert_eq!(a.id, b.id);
    assert_ne!(a.replayed, b.replayed);
    let mut changed = prompt("once");
    changed.body = "Another message".into();
    assert!(
        repo.create_session_message(first.id, changed, owner)
            .await
            .unwrap_err()
            .to_string()
            .contains("different message payload")
    );
    assert_eq!(
        repo.list_session_messages(first.id)
            .await
            .unwrap()
            .iter()
            .filter(|message| message.message_kind == MessageKind::UserPrompt)
            .count(),
        1
    );
    let events = repo.list_session_events(first.id, 0).await.unwrap();
    assert!(events.len() >= 4);
    assert!(
        events
            .windows(2)
            .all(|pair| pair[0].sequence < pair[1].sequence)
    );
    let last = events.last().unwrap().sequence;
    assert_eq!(repo.session_event_cursor(first.id).await.unwrap(), last);
    assert!(
        repo.list_session_events(first.id, last)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn unknown_dispatch_holds_agent_capacity_and_terminal_mirror_is_deduplicated() {
    let Some((repo, owner, _)) = fixture().await else {
        return;
    };
    let agent_id = agent(&repo).await;
    let first = repo
        .create_session(chat(agent_id, "first"), owner)
        .await
        .unwrap();
    let second = repo
        .create_session(chat(agent_id, "second"), owner)
        .await
        .unwrap();
    let message = repo
        .create_session_message(first.id, prompt("run-first"), owner)
        .await
        .unwrap();
    repo.create_session_message(second.id, prompt("run-second"), owner)
        .await
        .unwrap();
    // Other tests may leave pending messages; dispatch only this fixture's agent.
    let claimed = loop {
        let Some(candidate) = repo.claim_message_dispatch().await.unwrap() else {
            panic!("missing dispatch")
        };
        if candidate.id == message.id {
            break candidate;
        }
        repo.finish_message_dispatch(candidate.id, true, Some("test isolation hold".into()))
            .await
            .unwrap();
    };
    repo.finish_message_dispatch(claimed.id, true, Some("unknown runtime acceptance".into()))
        .await
        .unwrap();
    while let Some(candidate) = repo.claim_message_dispatch().await.unwrap() {
        assert_ne!(candidate.session_id, second.id);
        repo.finish_message_dispatch(candidate.id, true, Some("test isolation hold".into()))
            .await
            .unwrap();
    }
    let run = repo
        .prepare_session_agent_run(
            first.id,
            agent_id,
            SessionRunRole::Primary,
            "fleet:test".into(),
        )
        .await
        .unwrap();
    assert!(
        repo.prepare_session_agent_run(
            second.id,
            agent_id,
            SessionRunRole::Primary,
            "fleet:other".into()
        )
        .await
        .is_err()
    );
    repo.update_session_agent_run_dispatch(
        run.id,
        Some("runtime-run".into()),
        SessionRunState::Completed,
        None,
    )
    .await
    .unwrap();
    let a = repo
        .insert_session_message_mirror(
            first.id,
            Some(agent_id),
            "Done".into(),
            MessageKind::AssistantMessage,
            Some("runtime-run".into()),
        )
        .await
        .unwrap();
    let b = repo
        .insert_session_message_mirror(
            first.id,
            Some(agent_id),
            "Done".into(),
            MessageKind::AssistantMessage,
            Some("runtime-run".into()),
        )
        .await
        .unwrap();
    assert_eq!(a.id, b.id);
}

#[tokio::test]
async fn config_revision_readiness_http_uses_exact_heads_without_trusting_database_only_files() {
    let Some((repo, owner, _)) = fixture().await else {
        return;
    };
    let mut config = AppConfig::default();
    config.fleet.runtime_token_secret = "readiness-test-runtime-secret".into();
    // No filesystem installation: a DB active revision must not imply successful readback.
    let root = std::env::temp_dir().join(format!("fleet-readiness-http-{}", Uuid::new_v4()));
    config.fleet.agents_root = root.to_string_lossy().into_owned();
    repo.ensure_runtime_templates().await.unwrap();
    let agent = repo
        .create_agent(
            CreateAgentRequest {
                kind: AgentKind::Hermes,
                product_role: AgentProductRole::Executor,
                role: AgentRole::Developer,
                sdlc_role: Some(SdlcRole::Developer),
                display_name: "Readiness test".into(),
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
    let revision = repo
        .create_config_revision(agent.id, configuration(), owner)
        .await
        .unwrap();
    repo.validate_config_revision(agent.id, revision.revision, vec![])
        .await
        .unwrap();
    repo.request_config_activation(agent.id, revision.revision, owner)
        .await
        .unwrap();
    assert_eq!(
        repo.claim_config_activation()
            .await
            .unwrap()
            .unwrap()
            .agent_id,
        agent.id
    );
    repo.finish_config_activation(agent.id, revision.revision, None, true)
        .await
        .unwrap();
    for skill in repo.list_agent_skills(agent.id).await.unwrap() {
        if skill.state == domain::SkillState::Enabled {
            repo.update_agent_skill(
                agent.id,
                skill.name,
                domain::UpdateSkillRequest {
                    state: skill.state,
                    content: Some("Synthetic config-history regression skill".into()),
                },
            )
            .await
            .unwrap();
        }
    }
    let old_draft = repo
        .create_config_revision(agent.id, configuration(), owner)
        .await
        .unwrap();
    for _ in 0..100 {
        repo.create_config_revision(agent.id, configuration(), owner)
            .await
            .unwrap();
    }
    assert!(
        repo.list_config_revisions(agent.id)
            .await
            .unwrap()
            .iter()
            .all(
                |value| value.revision != revision.revision && value.revision != old_draft.revision
            )
    );
    assert_eq!(
        repo.get_config_revision(agent.id, old_draft.revision)
            .await
            .unwrap()
            .revision,
        old_draft.revision
    );
    let other_agent = self::agent(&repo).await;
    assert!(
        repo.get_config_revision(other_agent, old_draft.revision)
            .await
            .is_err()
    );
    let repo = Arc::new(repo);
    let config = Arc::new(config);
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime = Arc::new(infra::runtime::LocalRuntimeSupervisor::new(
        config.clone(),
        repo.clone(),
        events.clone(),
    ));
    let (restart_tx, _) = tokio::sync::mpsc::channel(1);
    let ctx = Arc::new(app::AppContext::new(
        config,
        repo,
        Arc::new(infra::FilesystemProvisioner),
        runtime,
        events,
        restart_tx,
    ));
    let endpoint = axum::Router::new()
        .route(
            "/agents/{id}/readiness",
            axum::routing::get(api::routes::agents::get_sdlc_readiness),
        )
        .route(
            "/agents/{id}/config/revisions/{revision}/validate",
            axum::routing::post(api::routes::agents::validate_config_revision),
        )
        .route(
            "/agents/{id}/config/revisions/{revision}/activate",
            axum::routing::post(api::routes::agents::activate_config_revision),
        );
    let router = axum::Router::new()
        .nest(
            "/operator",
            endpoint
                .clone()
                .layer(axum::Extension(api::middleware::CurrentUser {
                    id: owner,
                    role: domain::SystemRole::Operator,
                    is_system_admin: false,
                })),
        )
        .nest(
            "/user",
            endpoint.layer(axum::Extension(api::middleware::CurrentUser {
                id: owner,
                role: domain::SystemRole::User,
                is_system_admin: false,
            })),
        )
        .with_state(ctx);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{base}/operator/agents/{}/readiness", agent.id))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["effective_revision"], revision.revision);
    assert_eq!(body["ready_for_sdlc"], false);
    assert!(
        body["blockers"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "effective_configuration_readback_failed"
            ))
    );
    assert!(!body.to_string().contains(root.to_str().unwrap()));
    assert!(!body.to_string().contains("readiness-test-runtime-secret"));
    let revision_url = format!(
        "{base}/operator/agents/{}/config/revisions/{}",
        agent.id, old_draft.revision
    );
    let validated = client
        .post(format!("{revision_url}/validate"))
        .send()
        .await
        .unwrap();
    assert_eq!(validated.status(), reqwest::StatusCode::OK);
    let validated: serde_json::Value = validated.json().await.unwrap();
    assert_eq!(validated["revision"], old_draft.revision);
    assert_eq!(validated["state"], "validated");
    assert_eq!(validated["validation_errors"], serde_json::json!([]));
    assert_eq!(
        client
            .post(format!("{revision_url}/activate"))
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::CONFLICT
    );
    for action in ["validate", "activate"] {
        assert_eq!(
            client
                .post(format!(
                    "{base}/operator/agents/{other_agent}/config/revisions/{}/{action}",
                    old_draft.revision
                ))
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::NOT_FOUND
        );
        assert_eq!(
            client
                .post(format!(
                    "{base}/user/agents/{}/config/revisions/{}/{action}",
                    agent.id, old_draft.revision
                ))
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::FORBIDDEN
        );
    }
    let refreshed: serde_json::Value = client
        .get(format!("{base}/operator/agents/{}/readiness", agent.id))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(refreshed["effective_revision"], revision.revision);
    assert_eq!(refreshed["ready_for_sdlc"], false);
    assert_eq!(
        client
            .get(format!("{base}/user/agents/{}/readiness", agent.id))
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::FORBIDDEN
    );
    assert!(!root.exists());
    server.abort();
}

#[tokio::test]
async fn config_renderer_versions_are_server_selected_without_rewriting_legacy_history() {
    let Some((repo, owner, _)) = fixture().await else {
        return;
    };
    let id = agent(&repo).await;
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let request = || UpdateAgentConfigRequest {
        config_json: serde_json::json!({}),
        soul_md: "# Renderer fixture".into(),
        env_json: serde_json::json!({}),
    };
    let first = repo
        .create_config_revision(id, request(), owner)
        .await
        .unwrap();
    assert_eq!(first.snapshot.renderer_version, 2);
    // Fixture-only historical row, in the wire shape used before renderer versioning.
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE agent_config_revisions SET snapshot=snapshot-'renderer_version' WHERE agent_id=$1 AND revision=$2",
        [id.into(), first.revision.into()])).await.unwrap();
    let legacy = repo.get_config_revision(id, first.revision).await.unwrap();
    assert_eq!(legacy.snapshot.renderer_version, 1);
    assert!(
        serde_json::to_value(&legacy.snapshot)
            .unwrap()
            .get("renderer_version")
            .is_none()
    );
    let before: serde_json::Value = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT to_jsonb(r) AS value FROM agent_config_revisions r WHERE agent_id=$1 AND revision=$2",
        [id.into(), first.revision.into()])).await.unwrap().unwrap().try_get("", "value").unwrap();
    let next = repo
        .create_config_revision(id, request(), owner)
        .await
        .unwrap();
    assert_eq!(next.snapshot.renderer_version, 2);
    let after: serde_json::Value = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT to_jsonb(r) AS value FROM agent_config_revisions r WHERE agent_id=$1 AND revision=$2",
        [id.into(), first.revision.into()])).await.unwrap().unwrap().try_get("", "value").unwrap();
    assert_eq!(before, after);
    let mut invalid = request();
    invalid.config_json = serde_json::json!({"platforms":{"api_server":{"extra":null}}});
    assert!(
        repo.create_config_revision(id, invalid, owner)
            .await
            .is_err()
    );
    assert_eq!(repo.list_config_revisions(id).await.unwrap().len(), 2);
    let java = agent(&repo).await;
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE agents SET kind='java_agent' WHERE id=$1",
        [java.into()],
    ))
    .await
    .unwrap();
    assert_eq!(
        repo.create_config_revision(java, request(), owner)
            .await
            .unwrap()
            .snapshot
            .renderer_version,
        1
    );
}

#[tokio::test]
async fn config_revision_drains_runs_and_failed_rollback_stays_blocked() {
    let Some((repo, owner, _)) = fixture().await else {
        return;
    };
    let agent_id = agent(&repo).await;
    let session = repo
        .create_session(chat(agent_id, "config-drain"), owner)
        .await
        .unwrap();
    let run = repo
        .prepare_session_agent_run(
            session.id,
            agent_id,
            SessionRunRole::Primary,
            "fleet:config".into(),
        )
        .await
        .unwrap();
    let draft = repo
        .create_config_revision(agent_id, configuration(), owner)
        .await
        .unwrap();
    assert_eq!(draft.state, "draft");
    assert!(!draft.is_effective);
    assert!(
        repo.request_config_activation(agent_id, draft.revision, owner)
            .await
            .is_err()
    );
    repo.validate_config_revision(agent_id, draft.revision, vec![])
        .await
        .unwrap();
    repo.request_config_activation(agent_id, draft.revision, owner)
        .await
        .unwrap();
    assert!(repo.agent_is_draining(agent_id).await.unwrap());
    assert!(repo.claim_config_activation().await.unwrap().is_none());
    assert!(
        repo.create_config_revision(agent_id, configuration(), owner)
            .await
            .is_err()
    );
    repo.update_session_agent_run_dispatch(
        run.id,
        Some("config-run".into()),
        SessionRunState::Completed,
        None,
    )
    .await
    .unwrap();
    let claimed = repo.claim_config_activation().await.unwrap().unwrap();
    assert_eq!(claimed.agent_id, agent_id);
    repo.finish_config_activation(agent_id, draft.revision, None, true)
        .await
        .unwrap();
    assert!(!repo.agent_is_draining(agent_id).await.unwrap());
    assert!(repo.list_config_revisions(agent_id).await.unwrap()[0].is_effective);
    let next = repo
        .create_config_revision(agent_id, configuration(), owner)
        .await
        .unwrap();
    repo.validate_config_revision(agent_id, next.revision, vec![])
        .await
        .unwrap();
    repo.request_config_activation(agent_id, next.revision, owner)
        .await
        .unwrap();
    repo.claim_config_activation().await.unwrap().unwrap();
    repo.finish_config_activation(
        agent_id,
        next.revision,
        Some("rollback unverified".into()),
        false,
    )
    .await
    .unwrap();
    let revisions = repo.list_config_revisions(agent_id).await.unwrap();
    assert!(revisions[0].draining);
    assert_eq!(revisions[0].state, "failed");
    assert!(revisions[1].is_effective);
}

#[tokio::test]
async fn config_revision_identity_guard_fences_rebind_active_runs_and_unknown_dispatch() {
    let Some((repo, owner, _)) = fixture().await else {
        return;
    };
    let agent_id = agent(&repo).await;
    let namespace = domain::WorkflowNamespaceCatalogEntry {
        id: "42".into(),
        name: "hermes-developer".into(),
        workflow_id: "17".into(),
    };
    let workflow = domain::WorkflowCatalogEntry {
        id: "17".into(),
        name: "Developer".into(),
    };
    let original = repo
        .rebind_workflow_binding(agent_id, namespace.clone(), workflow.clone())
        .await
        .unwrap();
    let mut replacement = namespace.clone();
    replacement.id = "43".into();
    let patch: domain::UpdateAgentRequest = serde_json::from_value(serde_json::json!({
        "sdlc_role": "tester", "namespace_id": "43", "workflow_id": "18"
    }))
    .unwrap();
    let session = repo
        .create_session(chat(agent_id, "rebind-guard"), owner)
        .await
        .unwrap();
    let run = repo
        .prepare_session_agent_run(
            session.id,
            agent_id,
            SessionRunRole::Primary,
            "fleet:rebind".into(),
        )
        .await
        .unwrap();
    for state in [
        SessionRunState::Pending,
        SessionRunState::Running,
        SessionRunState::Waiting,
        SessionRunState::Stopping,
    ] {
        repo.update_session_agent_run_dispatch(run.id, None, state, None)
            .await
            .unwrap();
        assert!(matches!(
            repo.rebind_workflow_binding(agent_id, replacement.clone(), workflow.clone())
                .await,
            Err(shared::AppError::Conflict(_))
        ));
        assert!(matches!(
            repo.update_agent(agent_id, patch.clone()).await,
            Err(shared::AppError::Conflict(_))
        ));
    }
    let label: domain::UpdateAgentRequest = serde_json::from_value(serde_json::json!({
        "display_name": "New label", "product_role": "executor", "role": "developer",
        "sdlc_role": "developer", "namespace_id": "42", "workflow_id": "17"
    }))
    .unwrap();
    repo.update_agent(agent_id, label).await.unwrap();
    repo.update_session_agent_run_dispatch(run.id, None, SessionRunState::Cancelled, None)
        .await
        .unwrap();
    let message = repo
        .create_session_message(session.id, prompt("rebind-uncertain"), owner)
        .await
        .unwrap();
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    for state in ["pending", "dispatching", "uncertain"] {
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE message_dispatch_outbox SET state=$2 WHERE message_id=$1",
            [message.id.into(), state.into()],
        ))
        .await
        .unwrap();
        assert!(matches!(
            repo.rebind_workflow_binding(agent_id, replacement.clone(), workflow.clone())
                .await,
            Err(shared::AppError::Conflict(_))
        ));
        assert!(matches!(
            repo.update_agent(agent_id, patch.clone()).await,
            Err(shared::AppError::Conflict(_))
        ));
    }
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE message_dispatch_outbox SET state='failed' WHERE message_id=$1",
        [message.id.into()],
    ))
    .await
    .unwrap();
    let draft = repo
        .create_config_revision(agent_id, configuration(), owner)
        .await
        .unwrap();
    repo.validate_config_revision(agent_id, draft.revision, vec![])
        .await
        .unwrap();
    let txn = db.begin().await.unwrap();
    txn.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT id FROM agents WHERE id=$1 FOR UPDATE",
        [agent_id.into()],
    ))
    .await
    .unwrap();
    let repo = Arc::new(repo);
    let worker = repo.clone();
    let next_namespace = replacement.clone();
    let next_workflow = workflow.clone();
    let (started, ready) = tokio::sync::oneshot::channel();
    let pending = tokio::spawn(async move {
        started.send(()).unwrap();
        worker
            .rebind_workflow_binding(agent_id, next_namespace, next_workflow)
            .await
    });
    ready.await.unwrap();
    sleep(Duration::from_millis(50)).await;
    assert!(!pending.is_finished(), "rebind bypassed the agent row lock");
    txn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE agent_config_heads SET draining=true WHERE agent_id=$1",
        [agent_id.into()],
    ))
    .await
    .unwrap();
    txn.commit().await.unwrap();
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(3), pending)
            .await
            .unwrap()
            .unwrap(),
        Err(shared::AppError::Conflict(_))
    ));
    assert!(matches!(
        repo.update_agent(agent_id, patch).await,
        Err(shared::AppError::Conflict(_))
    ));
    let retained = repo
        .list_workflow_bindings()
        .await
        .unwrap()
        .into_iter()
        .find(|value| value.agent_id == agent_id)
        .unwrap();
    assert_eq!(retained.namespace_id, original.namespace_id);
    assert_eq!(retained.workflow_id, original.workflow_id);
    assert_eq!(
        repo.get_agent(agent_id).await.unwrap().namespace_id,
        original.namespace_id
    );
}

#[tokio::test]
async fn runtime_task_protocol_blocks_memory_only_idempotency_before_submission() {
    runtime_task_protocol_case(false).await;
}

#[tokio::test]
async fn runtime_task_protocol_durable_capabilities_do_not_bypass_admission() {
    runtime_task_protocol_case(true).await;
}

async fn runtime_task_protocol_case(durable: bool) {
    let Some((repo, reservation)) = pm_fixture().await else {
        return;
    };
    let session = repo.get_session(reservation.session_id).await.unwrap();
    let agent_id = reservation.identity.agent_id().unwrap();
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let message_id = Uuid::new_v4();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO session_messages(id,session_id,author_type,author_user_id,body,message_kind,delivery_state)
         VALUES($1,$2,'user',$3,'protocol gate fixture','user_prompt','pending')",
        [message_id.into(), session.id.into(), session.user_id.into()])).await.unwrap();
    let message = repo
        .list_session_messages(session.id)
        .await
        .unwrap()
        .into_iter()
        .find(|item| item.id == message_id)
        .unwrap();
    let runs_before =
        serde_json::to_value(repo.list_session_agent_runs(session.id).await.unwrap()).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE agents SET api_port=$2 WHERE id=$1",
        [agent_id.into(), i32::from(port).into()],
    ))
    .await
    .unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let calls = count.clone();
    let mut config = AppConfig::default();
    config.fleet.runtime_token_secret = "protocol-gate-fixture-only".into();
    let bearer = format!(
        "Bearer {}",
        infra::agent_runtime_token(&config, agent_id).unwrap()
    );
    let router = axum::Router::new()
        .route("/health", axum::routing::get(|| async { axum::Json(serde_json::json!({"status":"ok"})) }))
        .route("/v1/capabilities", axum::routing::get(move |headers: axum::http::HeaderMap| {
            assert_eq!(headers["authorization"], bearer);
            async move { axum::Json(serde_json::json!({
                "object":"hermes.api_server.capabilities", "platform":"hermes-agent",
                "auth":{"type":"bearer","required":true},
                "runtime":{"mode":"server_agent","tool_execution":"server","split_runtime":false},
                "features":{"run_submission":true,"run_status":true,"run_events_sse":true,"run_stop":true,
                    "runs_idempotency":{"supported":true,"durable":durable,"retention_seconds":86400}},
                "endpoints":{"runs":{"method":"POST","path":"/v1/runs"},
                    "run_status":{"method":"GET","path":"/v1/runs/{run_id}"},
                    "run_events":{"method":"GET","path":"/v1/runs/{run_id}/events"},
                    "run_stop":{"method":"POST","path":"/v1/runs/{run_id}/stop"}}
            })) }
        }))
        .route("/v1/runs", axum::routing::post(move || {
            calls.fetch_add(1, Ordering::SeqCst);
            async { axum::http::StatusCode::INTERNAL_SERVER_ERROR }
        }));
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let agent = repo.get_agent(agent_id).await.unwrap();
    let repo = Arc::new(repo);
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime =
        infra::runtime::LocalRuntimeSupervisor::new(Arc::new(config), repo.clone(), events);
    let response = app::RuntimeSupervisor::send_message(&runtime, &agent, &session, &message)
        .await
        .unwrap();
    assert_eq!(response.status, AgentStatus::Failed);
    assert!(response.message.contains(if durable {
        "admission is not yet verified"
    } else {
        "durable task protocol"
    }));
    assert_eq!(count.load(Ordering::SeqCst), 0);
    assert_eq!(
        serde_json::to_value(repo.list_session_agent_runs(session.id).await.unwrap()).unwrap(),
        runs_before
    );
    server.abort();
    let _ = server.await;
}

async fn runtime_http_fixture(runtime_status: &'static str) {
    let expected = match runtime_status {
        "completed" => SessionRunState::Completed,
        "interrupted" => SessionRunState::Failed,
        _ => SessionRunState::Waiting,
    };
    runtime_http_scenario(
        serde_json::json!({"object":"hermes.run","run_id":"fixture-run","status":runtime_status,
            "completed":runtime_status == "completed","partial":false,"interrupted":runtime_status == "interrupted",
            "output":"verified response"}),
        "event: message.delta\ndata: {\"delta\":\"partial response\"}\n\n",
        expected,
        runtime_status == "completed",
    ).await;
}

async fn runtime_http_scenario(
    mut readback: serde_json::Value,
    events: &'static str,
    expected: SessionRunState,
    expected_reply: bool,
) {
    let Some((repo, owner, _)) = fixture().await else {
        return;
    };
    let agent_id = agent(&repo).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE agents SET api_port = $2 WHERE id = $1",
        [agent_id.into(), i32::from(port).into()],
    ))
    .await
    .unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let calls = count.clone();
    if readback.get("session_id").is_none() {
        readback["session_id"] = serde_json::json!("native-fixture-session");
    }
    let reads = Arc::new(AtomicUsize::new(0));
    let router = axum::Router::new()
        .route("/v1/runs", axum::routing::post(move |headers: axum::http::HeaderMap, axum::Json(body): axum::Json<serde_json::Value>| {
            let calls = calls.clone();
            async move {
                assert!(headers.get("authorization").unwrap().to_str().unwrap().starts_with("Bearer fc_"));
                assert!(headers.contains_key("idempotency-key"));
                assert!(body["session_id"].as_str().unwrap().starts_with("fleet:"));
                calls.fetch_add(1, Ordering::SeqCst);
                (axum::http::StatusCode::ACCEPTED, axum::Json(serde_json::json!({"run_id": "fixture-run", "status":"started", "replayed":false})))
            }
        }))
        .route("/v1/runs/fixture-run/events", axum::routing::get(move || async move {
            ([(axum::http::header::CONTENT_TYPE, "text/event-stream")], events)
        }))
        .route("/v1/runs/fixture-run", axum::routing::get(move |headers: axum::http::HeaderMap| {
            assert_eq!(headers["accept-encoding"], "identity");
            assert!(headers["authorization"].to_str().unwrap().starts_with("Bearer fc_"));
            let payload = if reads.fetch_add(1, Ordering::SeqCst) == 0 {
                serde_json::json!({"object":"hermes.run","run_id":"fixture-run","session_id":"native-fixture-session","status":"running"})
            } else {
                readback.clone()
            };
            async move { axum::Json(payload) }
        }));
    let router = hermes_protocol_fixture::preflight(router);
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let repo = Arc::new(repo);
    let session = repo
        .create_session(chat(agent_id, "http-chat"), owner)
        .await
        .unwrap();
    let mut config = AppConfig::default();
    config.fleet.runtime_token_secret = "isolated-test-runtime-secret".into();
    let (events, _) = tokio::sync::broadcast::channel(32);
    let _runtime =
        infra::runtime::LocalRuntimeSupervisor::new(Arc::new(config), repo.clone(), events);
    repo.create_session_message(session.id, prompt("http-once"), owner)
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let runs = repo.list_session_agent_runs(session.id).await.unwrap();
            if runs.iter().any(|run| run.state == expected) {
                break;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("runtime event/status reconciliation completed");
    repo.create_session_message(session.id, prompt("http-once"), owner)
        .await
        .unwrap();
    sleep(Duration::from_millis(350)).await;
    assert_eq!(count.load(Ordering::SeqCst), 1);
    let replies = repo
        .list_session_messages(session.id)
        .await
        .unwrap()
        .into_iter()
        .filter(|message| message.message_kind == MessageKind::AssistantMessage)
        .collect::<Vec<_>>();
    if expected_reply {
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0].body, "verified response");
    } else {
        assert!(
            replies.is_empty(),
            "EOF must not fabricate a completed answer"
        );
    }
    if expected == SessionRunState::Waiting {
        assert!(
            matches!(
                repo.prepare_session_agent_run(
                    session.id,
                    agent_id,
                    domain::SessionRunRole::Primary,
                    format!("fleet:{}:{agent_id}", session.id)
                )
                .await,
                Err(shared::AppError::Conflict(_))
            ),
            "unverified terminal evidence must retain capacity"
        );
    }
    server.abort();
    let _ = server.await;
}

#[tokio::test]
async fn runtime_http_eof_without_terminal_status_keeps_run_waiting() {
    runtime_http_fixture("running").await;
}

#[tokio::test]
async fn runtime_http_terminal_readback_persists_one_answer() {
    runtime_http_fixture("completed").await;
}

#[tokio::test]
async fn runtime_http_interrupted_is_failed_without_fabricated_reply() {
    runtime_http_fixture("interrupted").await;
}

#[tokio::test]
async fn runtime_http_foreign_or_inconsistent_terminal_readback_retains_capacity() {
    let good = serde_json::json!({"object":"hermes.run","run_id":"fixture-run","status":"completed",
        "completed":true,"partial":false,"interrupted":false,"output":"verified response"});
    for (field, value) in [
        ("run_id", serde_json::json!("foreign-run")),
        ("object", serde_json::json!("hermes.response")),
        ("status", serde_json::json!("succeeded")),
        ("partial", serde_json::json!(true)),
    ] {
        let mut payload = good.clone();
        payload[field] = value;
        runtime_http_scenario(payload, "", SessionRunState::Waiting, false).await;
    }
}

#[tokio::test]
async fn runtime_http_foreign_or_partial_terminal_sse_retains_capacity() {
    for events in [
        "event: run.completed\ndata: {\"run_id\":\"foreign-run\",\"completed\":true,\"partial\":false,\"interrupted\":false,\"output\":\"foreign\"}\n\n",
        "event: run.completed\ndata: {\"run_id\":\"fixture-run\",\"completed\":true,\"partial\":true,\"interrupted\":false,\"output\":\"partial\"}\n\n",
    ] {
        runtime_http_scenario(
            serde_json::json!({"object":"hermes.run","run_id":"fixture-run","status":"running"}),
            events,
            SessionRunState::Waiting,
            false,
        )
        .await;
    }
}

#[tokio::test]
async fn runtime_http_nested_or_unknown_events_do_not_complete_the_run() {
    for events in [
        "event: subagent.completed\ndata: {\"run_id\":\"fixture-run\",\"completed\":true,\"partial\":false,\"interrupted\":false,\"output\":\"child\"}\n\n",
        "event: run.cancellation_requested\ndata: {\"run_id\":\"fixture-run\"}\n\n",
        "event: response.completed\ndata: {\"run_id\":\"fixture-run\",\"output\":\"not terminal\"}\n\n",
        "data: {\"run_id\":\"fixture-run\",\"completed\":true,\"partial\":false,\"interrupted\":false,\"child\":{\"event\":\"run.completed\"}}\n\n",
    ] {
        runtime_http_scenario(
            serde_json::json!({"object":"hermes.run","run_id":"fixture-run","status":"running"}),
            events,
            SessionRunState::Waiting,
            false,
        )
        .await;
    }
}

#[tokio::test]
async fn runtime_http_exact_terminal_sse_persists_one_answer_and_ignores_late_errors() {
    for events in [
        "event: run.completed\ndata: {\"event\":\"run.completed\",\"run_id\":\"fixture-run\",\"completed\":true,\"partial\":false,\"interrupted\":false,\"output\":\"verified response\"}\n\nevent: run.failed\ndata: {\"run_id\":\"fixture-run\",\"error\":\"late transport diagnostic\"}\n\n",
        "data: {\"event\":\"run.completed\",\"run_id\":\"fixture-run\",\"completed\":true,\"partial\":false,\"interrupted\":false,\"output\":\"verified response\"}\n\ndata: {\"event\":\"run.failed\",\"run_id\":\"fixture-run\",\"error\":\"late transport diagnostic\"}\n\n",
    ] {
        runtime_http_scenario(
            serde_json::json!({"object":"hermes.run","run_id":"fixture-run","status":"running"}),
            events,
            SessionRunState::Completed,
            true,
        )
        .await;
    }
}

#[tokio::test]
async fn task_binding_is_immutable_unique_and_replays_concurrent_requests() {
    let Some((repo, _, _)) = fixture().await else {
        return;
    };
    let subject = format!("pm-owner-{}", Uuid::new_v4());
    let owner = repo
        .find_or_create_central_user(
            &subject,
            &format!("{}@example.test", Uuid::new_v4()),
            "PM owner",
        )
        .await
        .unwrap()
        .id;
    let agent_id = agent(&repo).await;
    let session = repo
        .create_session(chat(agent_id, "binding-chat"), owner)
        .await
        .unwrap();
    let task = Uuid::new_v4();
    let binding = domain::TaskChatBinding {
        tracker_instance_id: "tracker-fixture".into(),
        project_id: Uuid::new_v4(),
        task_id: task,
        root_task_id: task,
        agent_id,
        owner_subject: subject,
    };
    let (first, replay) = tokio::join!(
        repo.bind_task_chat(session.id, binding.clone(), "bind-once".into()),
        repo.bind_task_chat(session.id, binding.clone(), "bind-once".into())
    );
    assert_eq!(first.unwrap(), binding);
    assert_eq!(replay.unwrap(), binding);
    let events = repo.list_session_events(session.id, 0).await.unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|event| event.event_type == "task.bound")
            .count(),
        1
    );
    let audits = repo
        .list_audit_log(app::AuditLogFilter {
            action: Some("session.task.bind".into()),
            entity_id: Some(session.id.to_string()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(audits.len(), 1);
    let mut changed = binding.clone();
    changed.root_task_id = Uuid::new_v4();
    assert!(matches!(
        repo.bind_task_chat(session.id, changed, "bind-once".into())
            .await,
        Err(shared::AppError::Conflict(_))
    ));
    let second = repo
        .create_session(chat(agent_id, "binding-second"), owner)
        .await
        .unwrap();
    assert!(matches!(
        repo.bind_task_chat(second.id, binding.clone(), "second-bind".into())
            .await,
        Err(shared::AppError::Conflict(_))
    ));
    assert_eq!(
        repo.get_task_chat_binding(session.id).await.unwrap(),
        Some(binding)
    );
}

#[tokio::test]
async fn task_binding_rejects_foreign_subject_and_legacy_transcript() {
    let Some((repo, _, _)) = fixture().await else {
        return;
    };
    let subject = format!("pm-owner-{}", Uuid::new_v4());
    let owner = repo
        .find_or_create_central_user(
            &subject,
            &format!("{}@example.test", Uuid::new_v4()),
            "PM owner",
        )
        .await
        .unwrap()
        .id;
    let agent_id = agent(&repo).await;
    let session = repo
        .create_session(chat(agent_id, "foreign-subject"), owner)
        .await
        .unwrap();
    let task = Uuid::new_v4();
    let mut binding = domain::TaskChatBinding {
        tracker_instance_id: "tracker-fixture".into(),
        project_id: Uuid::new_v4(),
        task_id: task,
        root_task_id: task,
        agent_id,
        owner_subject: "other-subject".into(),
    };
    assert!(matches!(
        repo.bind_task_chat(session.id, binding.clone(), "bind".into())
            .await,
        Err(shared::AppError::Forbidden)
    ));
    binding.owner_subject = subject;
    repo.insert_session_message_mirror(
        session.id,
        Some(agent_id),
        "Existing legacy history".into(),
        MessageKind::AssistantMessage,
        None,
    )
    .await
    .unwrap();
    assert!(matches!(
        repo.bind_task_chat(session.id, binding, "bind".into())
            .await,
        Err(shared::AppError::Conflict(_))
    ));
}

#[tokio::test]
async fn task_binding_and_generic_prompt_are_mutually_exclusive_under_concurrency() {
    let Some((repo, _, _)) = fixture().await else {
        return;
    };
    let subject = format!("pm-race-{}", Uuid::new_v4());
    let owner = repo
        .find_or_create_central_user(
            &subject,
            &format!("{}@example.test", Uuid::new_v4()),
            "Race owner",
        )
        .await
        .unwrap()
        .id;
    let agent_id = agent(&repo).await;
    for attempt in 0..4 {
        let session = repo
            .create_session(chat(agent_id, &format!("race-{attempt}")), owner)
            .await
            .unwrap();
        let task = Uuid::new_v4();
        let binding = domain::TaskChatBinding {
            tracker_instance_id: "race-tracker".into(),
            project_id: Uuid::new_v4(),
            task_id: task,
            root_task_id: task,
            agent_id,
            owner_subject: subject.clone(),
        };
        let (bound, sent) = tokio::join!(
            repo.bind_task_chat(session.id, binding, "bind".into()),
            repo.create_session_message(session.id, prompt("race-prompt"), owner)
        );
        assert_ne!(
            bound.is_ok(),
            sent.is_ok(),
            "bind and prompt must not both commit"
        );
        if bound.is_ok() {
            assert!(matches!(
                repo.create_session_message(session.id, prompt("later-prompt"), owner)
                    .await,
                Err(shared::AppError::Conflict(_))
            ));
            assert!(!repo.has_pending_session_dispatch(session.id).await.unwrap());
        }
    }
}

#[tokio::test]
async fn long_history_creation_replay_dispatch_and_terminal_mirror_return_exact_message() {
    let Some((repo, owner, _)) = fixture().await else {
        return;
    };
    let agent_id = agent(&repo).await;
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let session = repo
        .create_session(chat(agent_id, "long-history"), owner)
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO session_messages(id,session_id,author_type,body,message_kind)
            SELECT gen_random_uuid(),$1,'system','Historical ' || i,'system_event' FROM generate_series(1,505) i",
        [session.id.into()])).await.unwrap();
    let message = repo
        .create_session_message(session.id, prompt("after-500"), owner)
        .await
        .unwrap();
    let replay = repo
        .create_session_message(session.id, prompt("after-500"), owner)
        .await
        .unwrap();
    assert_eq!(message.id, replay.id);
    assert!(replay.replayed);
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE message_dispatch_outbox SET created_at='2000-01-01' WHERE message_id=$1",
        [message.id.into()],
    ))
    .await
    .unwrap();
    assert_eq!(
        repo.claim_message_dispatch().await.unwrap().unwrap().id,
        message.id
    );
    let reply = repo
        .insert_session_message_mirror(
            session.id,
            Some(agent_id),
            "Final result".into(),
            MessageKind::AssistantMessage,
            Some("long-history-final".into()),
        )
        .await
        .unwrap();
    let duplicate = repo
        .insert_session_message_mirror(
            session.id,
            Some(agent_id),
            "Final result".into(),
            MessageKind::AssistantMessage,
            Some("long-history-final".into()),
        )
        .await
        .unwrap();
    assert_eq!(reply.id, duplicate.id);
}

#[tokio::test]
async fn message_history_returns_latest_page_and_scopes_cursor() {
    let Some((repo, owner, _)) = fixture().await else {
        return;
    };
    let agent_id = agent(&repo).await;
    let session = repo
        .create_session(chat(agent_id, "paged-history"), owner)
        .await
        .unwrap();
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE session_messages SET created_at='2020-01-01T00:00:00Z' WHERE session_id=$1",
        [session.id.into()],
    ))
    .await
    .unwrap();
    for index in 0..8 {
        let message = repo
            .insert_session_message_mirror(
                session.id,
                Some(agent_id),
                format!("Message {index}"),
                MessageKind::AssistantMessage,
                None,
            )
            .await
            .unwrap();
        // Force clock rollback; history must follow database append order instead.
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE session_messages SET created_at='2020-01-01T00:00:00Z'::timestamptz + $2::integer * interval '1 second' WHERE id=$1",
            [message.id.into(), (8 - index).into()],
        ))
        .await
        .unwrap();
    }
    let latest = repo
        .session_message_history(session.id, None, 3)
        .await
        .unwrap();
    assert_eq!(latest.items.len(), 3);
    assert_eq!(latest.items.last().unwrap().body, "Message 7");
    let all = repo.list_session_messages(session.id).await.unwrap();
    assert_eq!(all.last().unwrap().body, "Message 7");
    assert!(
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE session_messages SET append_sequence=append_sequence+100000 WHERE id=$1",
            [latest.items[0].id.into()],
        ))
        .await
        .is_err()
    );
    let older = repo
        .session_message_history(session.id, latest.next_before, 3)
        .await
        .unwrap();
    assert_eq!(older.items.len(), 3);
    assert!(
        older
            .items
            .iter()
            .all(|item| latest.items.iter().all(|previous| previous.id != item.id))
    );
    let last = repo
        .session_message_history(session.id, older.next_before, 3)
        .await
        .unwrap();
    assert_eq!(last.items.len(), 3);
    assert!(last.next_before.is_none());
    let other = repo
        .create_session(chat(agent_id, "other-history"), owner)
        .await
        .unwrap();
    assert!(
        repo.session_message_history(other.id, Some(latest.items[0].id), 3)
            .await
            .is_err()
    );
    assert!(
        repo.session_message_history(session.id, None, 101)
            .await
            .is_err()
    );
}
