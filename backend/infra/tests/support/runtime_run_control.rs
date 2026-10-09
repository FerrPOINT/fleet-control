use super::*;
use app::RuntimeSupervisor;
use axum::{
    Json, Router,
    body::Body,
    http::{HeaderMap, Response, StatusCode},
    routing::{get, post},
};
use domain::{Agent, SessionAgentRun, SteerSessionRunRequest};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[path = "runtime_steer_transcript.rs"]
mod steer_transcript;

async fn required_fixture() -> Option<(PostgresFleetRepository, Uuid, Uuid)> {
    std::env::var("FLEET_TEST_DATABASE_URL")
        .expect("isolated PostgreSQL is required for runtime control tests");
    super::fixture().await
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn runtime_control_fresh_agent_origin_and_rotated_credential_deny_before_post() {
    for drift in ["origin", "credential"] {
        let f = ledger_fixture(stop_ack()).await.unwrap();
        let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
            .await
            .unwrap();
        let before = f
            .repo
            .get_hermes_dispatch_intent_for_run(f.run.id)
            .await
            .unwrap()
            .unwrap();
        let runtime = if drift == "origin" {
            let old_port = f.agent.api_port.unwrap();
            let port = if old_port == 65535 { 65534 } else { 65535 };
            db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE agents SET api_port=$2 WHERE id=$1",
                [f.agent.id.into(), port.into()],
            ))
            .await
            .unwrap();
            f.restarted()
        } else {
            let config = AppConfig {
                fleet: shared::FleetConfig {
                    runtime_token_secret: "rotated-control-fixture".into(),
                    ..Default::default()
                },
                ..Default::default()
            };
            let (events, _) = tokio::sync::broadcast::channel(32);
            infra::runtime::LocalRuntimeSupervisor::new(Arc::new(config), f.repo.clone(), events)
        };
        assert!(runtime.stop_run(&f.agent, &f.run, f.actor()).await.is_err());
        assert_eq!(f.calls.load(Ordering::SeqCst), 0);
        let receipts = f
            .repo
            .list_runtime_controls(f.run.session_id, f.run.id)
            .await
            .unwrap();
        if drift == "origin" {
            assert!(receipts.is_empty());
        } else {
            assert_eq!(receipts.len(), 1);
            assert_eq!(receipts[0].state, domain::RuntimeControlState::Rejected);
        }
        let after = f
            .repo
            .get_hermes_dispatch_intent_for_run(f.run.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(after.request_body, before.request_body);
        assert_eq!(after.request_hash, before.request_hash);
        assert_eq!(after.idempotency_key, before.idempotency_key);
        assert_eq!(after.origin, before.origin);
        assert_eq!(after.credential_fingerprint, before.credential_fingerprint);
        assert_eq!(after.run.state, SessionRunState::Running);
    }
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn runtime_control_task_and_pm_binding_deny_without_reservation_or_post() {
    for bound in ["task", "pm"] {
        let f = ledger_fixture(stop_ack()).await.unwrap();
        let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
            .await
            .unwrap();
        let session = if bound == "task" {
            f.run.session_id
        } else {
            f.repo
                .create_session(chat(f.agent.id, "control-pm-source"), f.owner)
                .await
                .unwrap()
                .id
        };
        let task = Uuid::new_v4();
        db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO task_chat_bindings(session_id,tracker_instance_id,project_id,task_id,root_task_id,agent_id,owner_subject,idempotency_key)
             VALUES($1,'control-fixture',$2,$3,$3,$4,'fixture-owner','control-bound')",
            [session.into(), Uuid::new_v4().into(), task.into(), f.agent.id.into()])).await.unwrap();
        if bound == "pm" {
            db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "INSERT INTO pm_run_bindings(session_run_id,session_id,agent_id,reservation,dispatch_operation_key,runtime_session_id)
                 VALUES($1,$2,$3,'{}','control-pm-fixture',$4)",
                [f.run.id.into(), session.into(), f.agent.id.into(), "control-native-session".into()])).await.unwrap();
        }
        assert!(matches!(
            f.runtime.stop_run(&f.agent, &f.run, f.actor()).await,
            Err(shared::AppError::Conflict(_))
        ));
        assert!(matches!(
            f.runtime
                .steer_run(
                    &f.agent,
                    &f.run,
                    SteerSessionRunRequest {
                        input: "must not dispatch".into()
                    },
                    f.actor()
                )
                .await,
            Err(shared::AppError::Conflict(_))
        ));
        assert!(
            f.repo
                .list_runtime_controls(f.run.session_id, f.run.id)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(f.calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            f.repo.get_session_agent_run(f.run.id).await.unwrap().state,
            SessionRunState::Running
        );
    }
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn runtime_control_claim_rechecks_frozen_origin_after_reservation() {
    let f = ledger_fixture(stop_ack()).await.unwrap();
    let reservation = f
        .repo
        .reserve_runtime_control(
            &f.run,
            &f.actor(),
            domain::RuntimeControlOperation::Stop,
            None,
        )
        .await
        .unwrap();
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let old_port = f.agent.api_port.unwrap();
    let port = if old_port == 65535 { 65534 } else { 65535 };
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE agents SET api_port=$2 WHERE id=$1",
        [f.agent.id.into(), port.into()],
    ))
    .await
    .unwrap();
    assert!(matches!(
        f.repo.claim_runtime_control(reservation.receipt.id).await,
        Err(shared::AppError::Conflict(_))
    ));
    let receipt = f
        .repo
        .get_runtime_control(f.run.session_id, reservation.receipt.id)
        .await
        .unwrap();
    assert_eq!(receipt.state, domain::RuntimeControlState::Reserved);
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
}

struct ControlFixture {
    owner: Uuid,
    repo: Arc<PostgresFleetRepository>,
    runtime: infra::runtime::LocalRuntimeSupervisor,
    agent: Agent,
    run: SessionAgentRun,
    calls: Arc<AtomicUsize>,
    server: tokio::task::JoinHandle<()>,
}
impl Drop for ControlFixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}
impl ControlFixture {
    fn restarted(&self) -> infra::runtime::LocalRuntimeSupervisor {
        let config = AppConfig {
            fleet: shared::FleetConfig {
                runtime_token_secret: "owned-control-fixture".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        let (events, _) = tokio::sync::broadcast::channel(32);
        infra::runtime::LocalRuntimeSupervisor::new(Arc::new(config), self.repo.clone(), events)
    }

    fn actor(&self) -> domain::RuntimeControlActor {
        domain::RuntimeControlActor {
            user_id: self.owner,
            idempotency_key: Uuid::new_v4().to_string(),
        }
    }
}

async fn setup(
    body: Value,
    code: StatusCode,
    mime: &'static str,
    encoded: bool,
    delayed: bool,
    native_session: &'static str,
    drift_during_steer: bool,
) -> Option<ControlFixture> {
    let (repo, owner, _) = required_fixture().await?;
    let id = agent(&repo).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
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
    let session = repo
        .create_session(chat(id, "controls"), owner)
        .await
        .unwrap();
    let message = repo
        .create_session_message(session.id, prompt("control-prompt"), owner)
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE message_dispatch_outbox SET state='dispatching' WHERE message_id=$1",
        [message.id.into()],
    ))
    .await
    .unwrap();
    let config = AppConfig {
        fleet: shared::FleetConfig {
            runtime_token_secret: "owned-control-fixture".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let token = infra::agent_runtime_token(&config, id).unwrap();
    let fingerprint = format!(
        "{:x}",
        Sha256::digest(
            [
                b"fleet-hermes-default-profile-v1\0".as_slice(),
                token.as_bytes()
            ]
            .concat()
        )
    );
    let origin = format!("http://127.0.0.1:{port}");
    repo.prepare_hermes_dispatch(app::HermesDispatchDraft {
        message_id: message.id,
        session_id: session.id,
        agent_id: id,
        run_role: SessionRunRole::Primary,
        requested_session_id: format!("fleet:{}:{id}", session.id),
        input: message.body,
        origin: origin.clone(),
        credential_fingerprint: fingerprint.clone(),
        capabilities: hermes_protocol_fixture::capabilities(),
    })
    .await
    .unwrap();
    let intent = repo
        .claim_hermes_submission(message.id, origin, fingerprint)
        .await
        .unwrap()
        .unwrap();
    repo.accept_hermes_run(message.id, intent.run.id, "run_control".into())
        .await
        .unwrap();
    let (run, _) = repo
        .pin_hermes_run_session(
            intent.run.id,
            "run_control".into(),
            format!("fleet:{}:{id}", session.id),
            "control-native-session".into(),
        )
        .await
        .unwrap();
    let repo = Arc::new(repo);
    let calls = Arc::new(AtomicUsize::new(0));
    let bearer = format!("Bearer {token}");
    let count = calls.clone();
    let callback_repo = repo.clone();
    let run_id = run.id;
    let control = move |headers: HeaderMap| {
        assert_eq!(headers["authorization"], bearer);
        assert_eq!(headers["accept-encoding"], "identity");
        count.fetch_add(1, Ordering::SeqCst);
        let body = body.clone();
        let repo = callback_repo.clone();
        async move {
            let current = repo.get_session_agent_run(run_id).await.unwrap();
            let receipts = repo
                .list_runtime_controls(current.session_id, run_id)
                .await
                .unwrap();
            assert_eq!(
                receipts
                    .iter()
                    .filter(|r| r.state == domain::RuntimeControlState::Submitted)
                    .count(),
                1,
                "single-use permit must be committed before native HTTP"
            );
            if drift_during_steer {
                repo.update_session_agent_run_dispatch(
                    run_id,
                    Some("run_control".into()),
                    SessionRunState::Waiting,
                    None,
                )
                .await
                .unwrap();
            }
            if delayed {
                sleep(Duration::from_secs(12)).await;
            }
            let mut response = Response::builder()
                .status(code)
                .header("content-type", mime);
            if encoded {
                response = response.header("content-encoding", "gzip");
            }
            response.body(Body::from(body.to_string())).unwrap()
        }
    };
    let mut caps = hermes_protocol_fixture::capabilities();
    caps["features"]["run_steer"] = json!(true);
    caps["endpoints"]["run_steer"] = json!({"method":"POST","path":"/v1/runs/{run_id}/steer"});
    let router = Router::new()
        .route("/health", get(|| async { Json(json!({"status":"ok"})) }))
        .route(
            "/v1/capabilities",
            get(move || {
                let caps = caps.clone();
                async { Json(caps) }
            }),
        )
        .route(
            "/v1/runs/run_control",
            get(move || async move {
                Json(json!({"object":"hermes.run",
            "run_id":"run_control","session_id":native_session,"status":"running"}))
            }),
        )
        .route("/v1/runs/run_control/steer", post(control.clone()))
        .route("/v1/runs/run_control/stop", post(control));
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let agent = repo.get_agent(id).await.unwrap();
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime =
        infra::runtime::LocalRuntimeSupervisor::new(Arc::new(config), repo.clone(), events);
    Some(ControlFixture {
        owner,
        repo,
        runtime,
        agent,
        run,
        calls,
        server,
    })
}

fn steer_ack() -> Value {
    json!({"object":"hermes.run.steer","run_id":"run_control","accepted":true})
}
fn stop_ack() -> Value {
    json!({"run_id":"run_control","status":"stopping"})
}

fn control_http_context(f: &ControlFixture) -> Arc<app::AppContext> {
    let config = AppConfig {
        fleet: shared::FleetConfig {
            runtime_token_secret: "owned-control-fixture".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let config = Arc::new(config);
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime = Arc::new(infra::runtime::LocalRuntimeSupervisor::new(
        config.clone(),
        f.repo.clone(),
        events.clone(),
    ));
    let (restart_tx, _) = tokio::sync::mpsc::channel(1);
    Arc::new(app::AppContext::new(
        config,
        f.repo.clone(),
        Arc::new(infra::FilesystemProvisioner),
        runtime,
        events,
        restart_tx,
    ))
}

fn control_http_routes() -> Router<Arc<app::AppContext>> {
    Router::new()
        .route(
            "/api/v1/sessions/{session_id}/runs/{run_id}/steer",
            post(api::routes::sessions::steer_session_run),
        )
        .route(
            "/api/v1/sessions/{session_id}/runs/{run_id}/stop",
            post(api::routes::sessions::stop_session_run),
        )
        .route(
            "/api/v1/sessions/{session_id}/runs/{run_id}/controls",
            get(api::routes::sessions::list_controls),
        )
        .route(
            "/api/v1/sessions/{session_id}/runs/{run_id}/controls/{command_id}",
            get(api::routes::sessions::read_control),
        )
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn runtime_control_http_sessionless_principal_cannot_impersonate_human() {
    let Some(f) = ledger_fixture(steer_ack()).await else {
        return;
    };
    // This isolates the human proof boundary; it is not a central JWKS acceptance test.
    let router = control_http_routes()
        .layer(axum::Extension(api::middleware::CurrentUser {
            id: f.owner,
            role: domain::SystemRole::Admin,
            is_system_admin: true,
            central_write: None,
        }))
        .with_state(control_http_context(&f));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let client = reqwest::Client::new();
    for (session, run) in [
        (f.run.session_id, f.run.id),
        (Uuid::new_v4(), Uuid::new_v4()),
    ] {
        for operation in ["stop", "steer"] {
            let response = client
                .post(format!(
                    "{base}/api/v1/sessions/{session}/runs/{run}/{operation}"
                ))
                .header("Idempotency-Key", "machine-forged-human")
                .header("X-Verified-Human-Session", "true")
                .json(&json!({"input":"machine attempt"}))
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        }
    }
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
    assert!(
        f.repo
            .list_runtime_controls(f.run.session_id, f.run.id)
            .await
            .unwrap()
            .is_empty()
    );
    server.abort();
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn runtime_control_http_local_login_scopes_replay_and_revocation() {
    let Some(f) = ledger_fixture(json!({"object":"hermes.run.steer", "run_id":"run_control",
        "accepted":true, "status":"stopping"}))
    .await
    else {
        return;
    };
    let ctx = control_http_context(&f);
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let mut principals = Vec::new();
    for role in [
        domain::SystemRole::User,
        domain::SystemRole::Operator,
        domain::SystemRole::Admin,
    ] {
        let id = Uuid::new_v4();
        db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO users(id,email,username,display_name,password_hash,is_system_admin,system_role)
             VALUES($1,$2,$3,'HTTP control test','disabled',$4,$5)",
            [id.into(), format!("{id}@example.test").into(), id.to_string().into(),
             role.is_admin().into(), role.to_string().into()])).await.unwrap();
        let record = f.repo.find_user_by_id(id).await.unwrap().unwrap();
        principals.push((
            role,
            ctx.auth
                .issue_tokens(&record)
                .unwrap()
                .response
                .access_token,
        ));
    }
    let owner = ctx
        .auth
        .issue_tokens(&f.repo.find_user_by_id(f.owner).await.unwrap().unwrap())
        .unwrap()
        .response
        .access_token;
    let foreign = f
        .repo
        .create_session(chat(f.agent.id, "foreign-run-route"), f.owner)
        .await
        .unwrap();
    let router = control_http_routes()
        .route_layer(axum::middleware::from_fn_with_state(
            ctx.clone(),
            api::middleware::require_auth,
        ))
        .with_state(ctx);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let client = reqwest::Client::new();
    let run_url = format!(
        "{base}/api/v1/sessions/{}/runs/{}",
        f.run.session_id, f.run.id
    );
    let foreign_url = format!("{base}/api/v1/sessions/{}/runs/{}", foreign.id, f.run.id);
    for operation in ["steer", "stop"] {
        let url = format!("{run_url}/{operation}");
        for token in [None, Some("invalid-token"), Some(principals[0].1.as_str())] {
            let mut request = client
                .post(&url)
                .header("Idempotency-Key", "denied-command")
                .json(&json!({"input":"guidance"}));
            if let Some(token) = token {
                request = request.bearer_auth(token);
            }
            let expected = if token == Some(principals[0].1.as_str()) {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::UNAUTHORIZED
            };
            assert_eq!(request.send().await.unwrap().status(), expected);
        }
        let unkeyed = client
            .post(&url)
            .bearer_auth(&owner)
            .json(&json!({"input":"guidance"}))
            .send()
            .await
            .unwrap();
        assert!(unkeyed.status().is_client_error());
        for (first, second) in [
            ("same-key", "foreign-key"),
            ("foreign-key", "same-key"),
            ("same-key", "same-key"),
        ] {
            let response = client
                .post(&url)
                .bearer_auth(&owner)
                .header("Idempotency-Key", first)
                .header("Idempotency-Key", second)
                .json(&json!({"input":"guidance"}))
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        }
        assert_eq!(
            client
                .post(format!("{foreign_url}/{operation}"))
                .bearer_auth(&owner)
                .header("Idempotency-Key", "wrong-run")
                .json(&json!({"input":"guidance"}))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
    }
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
    assert!(
        f.repo
            .list_runtime_controls(f.run.session_id, f.run.id)
            .await
            .unwrap()
            .is_empty()
    );
    let mut receipts = Vec::new();
    for operation in ["steer", "stop"] {
        let url = format!("{run_url}/{operation}");
        let key = format!("owner-{operation}");
        for _ in 0..2 {
            let response = client
                .post(&url)
                .bearer_auth(&owner)
                .header("Idempotency-Key", &key)
                .json(&json!({"input":"guidance"}))
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let ack = response
                .json::<domain::RuntimeRunControlResponse>()
                .await
                .unwrap();
            assert!(ack.accepted);
            let receipt = ack.command.unwrap();
            assert_eq!(receipt.actor_user_id, f.owner);
            assert_eq!(receipt.state, domain::RuntimeControlState::Acknowledged);
            receipts.push(receipt);
        }
    }
    for pair in receipts.chunks_exact(2) {
        assert_eq!(pair[0].id, pair[1].id);
    }
    assert_eq!(f.calls.load(Ordering::SeqCst), 2);
    assert_eq!(
        client
            .post(format!("{run_url}/steer"))
            .bearer_auth(&owner)
            .header("Idempotency-Key", "owner-steer")
            .json(&json!({"input":"changed"}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    for (role, token) in &principals {
        let expected = if *role == domain::SystemRole::User {
            StatusCode::FORBIDDEN
        } else {
            StatusCode::OK
        };
        for suffix in [
            "/controls".to_string(),
            format!("/controls/{}", receipts[0].id),
        ] {
            assert_eq!(
                client
                    .get(format!("{run_url}{suffix}"))
                    .bearer_auth(token)
                    .send()
                    .await
                    .unwrap()
                    .status(),
                expected
            );
        }
    }
    assert_eq!(
        client
            .get(format!("{foreign_url}/controls/{}", receipts[0].id))
            .bearer_auth(&owner)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        client
            .get(format!("{run_url}/controls/{}", Uuid::new_v4()))
            .bearer_auth(&owner)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    let list = client
        .get(format!("{run_url}/controls"))
        .bearer_auth(&owner)
        .send()
        .await
        .unwrap()
        .json::<Vec<domain::RuntimeControlReceipt>>()
        .await
        .unwrap();
    assert_eq!(list.len(), 2);
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE users SET is_active=false WHERE id=$1",
        [f.owner.into()],
    ))
    .await
    .unwrap();
    for operation in ["steer", "stop"] {
        assert_eq!(
            client
                .post(format!("{run_url}/{operation}"))
                .bearer_auth(&owner)
                .header("Idempotency-Key", format!("revoked-{operation}"))
                .json(&json!({"input":"guidance"}))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        client
            .get(format!("{run_url}/controls"))
            .bearer_auth(&owner)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(f.calls.load(Ordering::SeqCst), 2);
    server.abort();
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn runtime_control_steer_ack_cannot_reset_concurrent_waiting_state() {
    let Some(f) = setup(
        steer_ack(),
        StatusCode::OK,
        "application/json",
        false,
        false,
        "control-native-session",
        true,
    )
    .await
    else {
        return;
    };
    let ack = f
        .runtime
        .steer_run(
            &f.agent,
            &f.run,
            SteerSessionRunRequest {
                input: "guidance".into(),
            },
            f.actor(),
        )
        .await
        .unwrap();
    assert!(ack.accepted);
    assert_eq!(ack.state, SessionRunState::Waiting);
    assert_eq!(f.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn runtime_control_stop_ack_is_only_stopping_and_keeps_capacity() {
    let Some(f) = setup(
        stop_ack(),
        StatusCode::OK,
        "application/json",
        false,
        false,
        "control-native-session",
        false,
    )
    .await
    else {
        return;
    };
    let ack = f
        .runtime
        .stop_run(&f.agent, &f.run, f.actor())
        .await
        .unwrap();
    assert!(ack.accepted);
    assert_eq!(ack.state, SessionRunState::Stopping);
    assert_eq!(f.calls.load(Ordering::SeqCst), 1);
    assert!(
        f.repo
            .list_session_messages(f.run.session_id)
            .await
            .unwrap()
            .iter()
            .all(|m| m.message_kind != MessageKind::Control)
    );
    assert!(
        f.repo
            .prepare_session_agent_run(
                f.run.session_id,
                f.agent.id,
                SessionRunRole::Primary,
                "new-run-forbidden".into()
            )
            .await
            .is_err()
    );
    assert!(
        f.repo
            .list_session_messages(f.run.session_id)
            .await
            .unwrap()
            .iter()
            .all(|m| m.message_kind != MessageKind::AssistantMessage)
    );
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn runtime_control_invalid_ack_transport_and_identity_do_not_mutate_or_retry() {
    let mut oversized = steer_ack();
    oversized["private"] = json!("synthetic".repeat(9000));
    let responses = [
        (
            json!({"accepted":true}),
            StatusCode::OK,
            "application/json",
            false,
        ),
        (
            json!({"object":"hermes.run.steer","run_id":"foreign","accepted":true}),
            StatusCode::OK,
            "application/json",
            false,
        ),
        (
            json!({"object":"hermes.run.steer","run_id":"run_control","accepted":false}),
            StatusCode::OK,
            "application/json",
            false,
        ),
        (steer_ack(), StatusCode::ACCEPTED, "application/json", false),
        (steer_ack(), StatusCode::OK, "text/plain", false),
        (steer_ack(), StatusCode::OK, "application/json", true),
        (oversized, StatusCode::OK, "application/json", false),
    ];
    for (body, status, mime, encoded) in responses {
        let Some(f) = setup(
            body,
            status,
            mime,
            encoded,
            false,
            "control-native-session",
            false,
        )
        .await
        else {
            return;
        };
        let before = f
            .repo
            .get_hermes_dispatch_intent_for_run(f.run.id)
            .await
            .unwrap()
            .unwrap();
        let unknown = f
            .runtime
            .steer_run(
                &f.agent,
                &f.run,
                SteerSessionRunRequest {
                    input: "guidance".into(),
                },
                f.actor(),
            )
            .await
            .unwrap();
        assert!(!unknown.accepted);
        assert_eq!(
            unknown.command.unwrap().state,
            domain::RuntimeControlState::Uncertain
        );
        assert_eq!(f.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            f.repo.get_session_agent_run(f.run.id).await.unwrap().state,
            SessionRunState::Running
        );
        let after = f
            .repo
            .get_hermes_dispatch_intent_for_run(f.run.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(after.request_body, before.request_body);
        assert_eq!(after.idempotency_key, before.idempotency_key);
    }
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn runtime_control_ack_deadline_does_not_retry_or_claim_success() {
    let Some(f) = setup(
        steer_ack(),
        StatusCode::OK,
        "application/json",
        false,
        true,
        "control-native-session",
        false,
    )
    .await
    else {
        return;
    };
    let unknown = f
        .runtime
        .steer_run(
            &f.agent,
            &f.run,
            SteerSessionRunRequest {
                input: "guidance".into(),
            },
            f.actor(),
        )
        .await
        .unwrap();
    assert!(!unknown.accepted);
    assert_eq!(
        unknown.command.unwrap().state,
        domain::RuntimeControlState::Uncertain
    );
    assert_eq!(f.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        f.repo.get_session_agent_run(f.run.id).await.unwrap().state,
        SessionRunState::Running
    );
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn runtime_control_foreign_native_session_and_stale_context_block_before_post() {
    let Some(f) = setup(
        steer_ack(),
        StatusCode::OK,
        "application/json",
        false,
        false,
        "foreign-native-session",
        false,
    )
    .await
    else {
        return;
    };
    assert!(
        f.runtime
            .stop_run(&f.agent, &f.run, f.actor())
            .await
            .is_err()
    );
    let mut stale = f.run.clone();
    stale.runtime_session_id = Some("other".into());
    assert!(
        f.runtime
            .stop_run(&f.agent, &stale, f.actor())
            .await
            .is_err()
    );
    let mut foreign_agent = f.agent.clone();
    foreign_agent.api_port = Some(1);
    assert!(
        f.runtime
            .stop_run(&foreign_agent, &f.run, f.actor())
            .await
            .is_err()
    );
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        f.repo.get_session_agent_run(f.run.id).await.unwrap().state,
        SessionRunState::Running
    );
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn runtime_control_terminal_stop_race_never_invents_stopping_or_completion() {
    let response = json!({"object":"hermes.run","run_id":"run_control","session_id":"control-native-session",
        "status":"completed","completed":true,"partial":false,"interrupted":false});
    let Some(f) = setup(
        response,
        StatusCode::OK,
        "application/json",
        false,
        false,
        "control-native-session",
        false,
    )
    .await
    else {
        return;
    };
    let ack = f
        .runtime
        .stop_run(&f.agent, &f.run, f.actor())
        .await
        .unwrap();
    assert!(ack.accepted);
    assert_eq!(ack.state, SessionRunState::Running);
    assert!(ack.message.contains("readback"));
    assert_eq!(f.calls.load(Ordering::SeqCst), 1);
    assert!(
        f.repo
            .list_session_messages(f.run.session_id)
            .await
            .unwrap()
            .iter()
            .all(|m| m.message_kind != MessageKind::AssistantMessage)
    );
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn runtime_control_legacy_run_wide_approval_cannot_reach_native_adapter() {
    let Some(f) = setup(
        steer_ack(),
        StatusCode::OK,
        "application/json",
        false,
        false,
        "control-native-session",
        false,
    )
    .await
    else {
        return;
    };
    assert!(matches!(
        f.runtime
            .resolve_approval(
                &f.agent,
                &f.run,
                domain::ResolveRuntimeApprovalRequest {
                    choice: "always".into(),
                    resolve_all: true,
                }
            )
            .await,
        Err(shared::AppError::Conflict(_))
    ));
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
}

async fn ledger_fixture(body: Value) -> Option<ControlFixture> {
    setup(
        body,
        StatusCode::OK,
        "application/json",
        false,
        false,
        "control-native-session",
        false,
    )
    .await
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn runtime_control_concurrent_identical_replay_posts_once_and_survives_restart() {
    let Some(f) = ledger_fixture(steer_ack()).await else {
        return;
    };
    let actor = f.actor();
    let command = SteerSessionRunRequest {
        input: "same guidance".into(),
    };
    let (a, b, c, d) = tokio::join!(
        f.runtime
            .steer_run(&f.agent, &f.run, command.clone(), actor.clone()),
        f.runtime
            .steer_run(&f.agent, &f.run, command.clone(), actor.clone()),
        f.runtime
            .steer_run(&f.agent, &f.run, command.clone(), actor.clone()),
        f.runtime
            .steer_run(&f.agent, &f.run, command.clone(), actor.clone())
    );
    let receipts = [a, b, c, d].map(|r| r.unwrap().command.unwrap());
    assert!(receipts.iter().all(|r| r.id == receipts[0].id));
    assert_eq!(f.calls.load(Ordering::SeqCst), 1);
    let replay = f
        .restarted()
        .steer_run(&f.agent, &f.run, command, actor.clone())
        .await
        .unwrap();
    assert!(replay.accepted);
    assert_eq!(replay.command.unwrap().id, receipts[0].id);
    assert!(matches!(
        f.runtime
            .steer_run(
                &f.agent,
                &f.run,
                SteerSessionRunRequest {
                    input: "changed guidance".into()
                },
                actor.clone()
            )
            .await,
        Err(shared::AppError::Conflict(_))
    ));
    assert!(matches!(
        f.runtime.stop_run(&f.agent, &f.run, actor).await,
        Err(shared::AppError::Conflict(_))
    ));
    assert_eq!(f.calls.load(Ordering::SeqCst), 1);
    let history = f
        .repo
        .list_runtime_controls(f.run.session_id, f.run.id)
        .await
        .unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].state, domain::RuntimeControlState::Acknowledged);
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn runtime_control_unknown_ack_holds_new_commands_and_never_reposts_after_restart() {
    let Some(f) = ledger_fixture(json!({"accepted":true})).await else {
        return;
    };
    let actor = f.actor();
    let command = SteerSessionRunRequest {
        input: "only once".into(),
    };
    let first = f
        .runtime
        .steer_run(&f.agent, &f.run, command.clone(), actor.clone())
        .await
        .unwrap();
    let first = first.command.unwrap();
    assert_eq!(first.state, domain::RuntimeControlState::Uncertain);
    let replay = f
        .restarted()
        .steer_run(&f.agent, &f.run, command, actor)
        .await
        .unwrap();
    assert!(!replay.accepted);
    assert_eq!(replay.command.unwrap().id, first.id);
    assert!(matches!(
        f.runtime.stop_run(&f.agent, &f.run, f.actor()).await,
        Err(shared::AppError::Conflict(_))
    ));
    assert_eq!(f.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        f.repo.get_session_agent_run(f.run.id).await.unwrap().state,
        SessionRunState::Running
    );
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn runtime_control_submitted_restart_is_read_only_and_claim_is_single_use() {
    let Some(f) = ledger_fixture(stop_ack()).await else {
        return;
    };
    let actor = f.actor();
    let reservation = f
        .repo
        .reserve_runtime_control(&f.run, &actor, domain::RuntimeControlOperation::Stop, None)
        .await
        .unwrap();
    let id = reservation.receipt.id;
    let (a, b) = tokio::join!(
        f.repo.claim_runtime_control(id),
        f.repo.claim_runtime_control(id)
    );
    assert_eq!(usize::from(a.unwrap()) + usize::from(b.unwrap()), 1);
    let replay = f
        .restarted()
        .stop_run(&f.agent, &f.run, actor)
        .await
        .unwrap();
    assert!(!replay.accepted);
    assert_eq!(
        replay.command.unwrap().state,
        domain::RuntimeControlState::Submitted
    );
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
    assert!(!f.repo.claim_runtime_control(id).await.unwrap());
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn runtime_control_claim_rechecks_actor_and_receipt_is_session_scoped() {
    let Some(f) = ledger_fixture(stop_ack()).await else {
        return;
    };
    let reservation = f
        .repo
        .reserve_runtime_control(
            &f.run,
            &f.actor(),
            domain::RuntimeControlOperation::Stop,
            None,
        )
        .await
        .unwrap();
    let id = reservation.receipt.id;
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE users SET is_active=false WHERE id=$1",
        [f.owner.into()],
    ))
    .await
    .unwrap();
    assert!(matches!(
        f.repo.claim_runtime_control(id).await,
        Err(shared::AppError::Forbidden)
    ));
    let retired = f.repo.retire_runtime_control(id, false).await.unwrap();
    assert_eq!(retired.state, domain::RuntimeControlState::Rejected);
    assert!(matches!(
        f.repo.get_runtime_control(Uuid::new_v4(), id).await,
        Err(shared::AppError::NotFound { .. })
    ));
    assert!(
        f.repo
            .list_runtime_controls(Uuid::new_v4(), f.run.id)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn runtime_control_ack_and_stopping_roll_back_together_on_audit_failure() {
    let Some(f) = ledger_fixture(stop_ack()).await else {
        return;
    };
    let reservation = f
        .repo
        .reserve_runtime_control(
            &f.run,
            &f.actor(),
            domain::RuntimeControlOperation::Stop,
            None,
        )
        .await
        .unwrap();
    let id = reservation.receipt.id;
    assert!(f.repo.claim_runtime_control(id).await.unwrap());
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let guard = format!("control_fault_{}", Uuid::new_v4().simple());
    db.execute_unprepared(&format!(
        "CREATE FUNCTION {guard}() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN IF NEW.entity_id='{id}' AND NEW.action='runtime_control.acknowledged' THEN
        RAISE EXCEPTION 'owned control audit failure'; END IF; RETURN NEW; END $$;
        CREATE TRIGGER {guard} BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION {guard}();"
    ))
    .await
    .unwrap();
    let result = f.repo.finish_runtime_control(id, "stopping", None).await;
    db.execute_unprepared(&format!(
        "DROP TRIGGER {guard} ON audit_log; DROP FUNCTION {guard}();"
    ))
    .await
    .unwrap();
    assert!(result.is_err());
    assert_eq!(
        f.repo
            .get_runtime_control(f.run.session_id, id)
            .await
            .unwrap()
            .state,
        domain::RuntimeControlState::Submitted
    );
    assert_eq!(
        f.repo.get_session_agent_run(f.run.id).await.unwrap().state,
        SessionRunState::Running
    );
    let committed = f
        .repo
        .finish_runtime_control(id, "stopping", None)
        .await
        .unwrap();
    assert_eq!(committed.state, domain::RuntimeControlState::Acknowledged);
    assert_eq!(
        f.repo.get_session_agent_run(f.run.id).await.unwrap().state,
        SessionRunState::Stopping
    );
    assert_eq!(
        serde_json::to_value(
            f.repo
                .finish_runtime_control(id, "stopping", None)
                .await
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(committed).unwrap()
    );
    let row = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT count(*) AS total FROM session_events WHERE session_id=$1 AND payload->>'command_id'=$2",
        [f.run.session_id.into(),id.to_string().into()])).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "total").unwrap(), 3);
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn runtime_control_terminal_reconciliation_requires_committed_mirror_not_run_flag() {
    let Some(f) = ledger_fixture(json!({"accepted":true})).await else {
        return;
    };
    let actor = f.actor();
    let first = f
        .runtime
        .stop_run(&f.agent, &f.run, actor.clone())
        .await
        .unwrap()
        .command
        .unwrap();
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE session_agent_runs SET state='completed',last_event_at=now() WHERE id=$1",
        [f.run.id.into()],
    ))
    .await
    .unwrap();
    f.repo.reconcile_runtime_controls().await.unwrap();
    assert_eq!(
        f.repo
            .get_runtime_control(f.run.session_id, first.id)
            .await
            .unwrap()
            .state,
        domain::RuntimeControlState::Uncertain
    );
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE session_agent_runs SET state='running' WHERE id=$1",
        [f.run.id.into()],
    ))
    .await
    .unwrap();
    let intent = f
        .repo
        .get_hermes_dispatch_intent_for_run(f.run.id)
        .await
        .unwrap()
        .unwrap();
    f.repo
        .commit_hermes_terminal(app::HermesTerminalCommit {
            message_id: intent.message_id,
            run_id: f.run.id,
            runtime_run_id: "run_control".into(),
            runtime_session_id: "control-native-session".into(),
            state: SessionRunState::Completed,
            body: Some("independent terminal mirror".into()),
            error: None,
        })
        .await
        .unwrap();
    f.repo.reconcile_runtime_controls().await.unwrap();
    let observed = f
        .repo
        .get_runtime_control(f.run.session_id, first.id)
        .await
        .unwrap();
    assert_eq!(
        observed.state,
        domain::RuntimeControlState::TerminalObserved
    );
    assert_eq!(
        observed.observed_run_state,
        Some(SessionRunState::Completed)
    );
    assert_eq!(observed.acknowledgement, None);
    f.repo.reconcile_runtime_controls().await.unwrap();
    assert_eq!(
        serde_json::to_value(
            f.repo
                .get_runtime_control(f.run.session_id, first.id)
                .await
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(observed).unwrap()
    );
    let replay = f
        .restarted()
        .stop_run(&f.agent, &f.run, actor)
        .await
        .unwrap();
    assert!(!replay.accepted);
    assert_eq!(f.calls.load(Ordering::SeqCst), 1);
}
