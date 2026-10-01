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
            &AppConfig::default(),
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

async fn pm_fixture() -> Option<(PostgresFleetRepository, domain::PmRunReservation)> {
    let (repo, _, _) = fixture().await?;
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
    let tracker = axum::Router::new().route(
        "/api/v1/issues/{id}/sdlc/context",
        axum::routing::get(
            move |axum::extract::Path(id): axum::extract::Path<Uuid>,
                  headers: axum::http::HeaderMap| {
                let check = check.clone();
                let context = context.clone();
                async move {
                    assert_eq!(id, context.task_id);
                    assert_eq!(
                        headers.get("authorization").unwrap(),
                        "Bearer verified-owner-fixture"
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
    let router = axum::Router::new()
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
    revoked.store(true, Ordering::SeqCst);
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
    // Runtime event processing owns the visible run state; proof alone cannot free capacity.
    assert!(repo.reserve_pm_run(concurrent.clone()).await.is_err());
    repo.update_session_agent_run_dispatch(
        request.session_run_id,
        None,
        SessionRunState::Completed,
        None,
    )
    .await
    .unwrap();
    assert!(repo.reserve_pm_run(concurrent).await.is_ok());
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
                "session_id":if phase == 3 { "foreign-session" } else { &session },"status":if phase == 1 {"completed"} else {"running"}}))
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

async fn runtime_http_fixture(runtime_status: &'static str) {
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
    let router = axum::Router::new()
        .route("/v1/runs", axum::routing::post(move |headers: axum::http::HeaderMap, axum::Json(body): axum::Json<serde_json::Value>| {
            let calls = calls.clone();
            async move {
                assert!(headers.get("authorization").unwrap().to_str().unwrap().starts_with("Bearer fc_"));
                assert!(headers.contains_key("idempotency-key"));
                assert!(body["session_id"].as_str().unwrap().starts_with("fleet:"));
                calls.fetch_add(1, Ordering::SeqCst);
                axum::Json(serde_json::json!({"run_id": "fixture-run"}))
            }
        }))
        .route("/v1/runs/fixture-run/events", axum::routing::get(|| async {
            ([(axum::http::header::CONTENT_TYPE, "text/event-stream")], "event: response.delta\ndata: {\"delta\":\"partial response\"}\n\n")
        }))
        .route("/v1/runs/fixture-run", axum::routing::get(move || async move {
            axum::Json(serde_json::json!({"status": runtime_status, "final_response": "verified response"}))
        }));
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
    let expected = match runtime_status {
        "completed" => SessionRunState::Completed,
        "interrupted" => SessionRunState::Failed,
        _ => SessionRunState::Waiting,
    };
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
    if runtime_status == "completed" {
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0].body, "verified response");
    } else {
        assert!(
            replies.is_empty(),
            "EOF must not fabricate a completed answer"
        );
    }
    server.abort();
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
    for index in 0..8 {
        repo.insert_session_message_mirror(
            session.id,
            Some(agent_id),
            format!("Message {index}"),
            MessageKind::AssistantMessage,
            None,
        )
        .await
        .unwrap();
    }
    let latest = repo
        .session_message_history(session.id, None, 3)
        .await
        .unwrap();
    assert_eq!(latest.items.len(), 3);
    assert_eq!(latest.items.last().unwrap().body, "Message 7");
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
