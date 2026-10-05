use super::*;
use app::RuntimeSupervisor;
use axum::{
    Json, Router,
    body::Body,
    http::{HeaderMap, Response, StatusCode},
    routing::{get, post},
};
use domain::{AgentSession, ApprovalChoice, RuntimeApprovalRequest, SessionAgentRun};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::Mutex;

pub(super) fn capabilities() -> Value {
    let mut value = hermes_protocol_fixture::capabilities();
    value["features"]["run_approval_response"] = json!(true);
    value["features"]["approval_events"] = json!(true);
    value["endpoints"]["run_approval"] =
        json!({"method":"POST","path":"/v1/runs/{run_id}/approval"});
    value
}

pub(super) fn pending_status(request_id: &str) -> Value {
    json!({"object":"hermes.run","run_id":"run_approval_test","session_id":"approval-native-session",
        "status":"waiting_for_approval","approval":{"event":"approval.request",
            "run_id":"run_approval_test","request_id":request_id}})
}

pub(super) async fn accepted_approval(
    repo: &PostgresFleetRepository,
    owner: Uuid,
    key: &str,
    port: u16,
    config: &AppConfig,
) -> (AgentSession, SessionAgentRun, RuntimeApprovalRequest) {
    let id = agent(repo).await;
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
    let session = repo.create_session(chat(id, key), owner).await.unwrap();
    let message = repo
        .create_session_message(session.id, prompt(key), owner)
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE message_dispatch_outbox SET state='dispatching' WHERE message_id=$1",
        [message.id.into()],
    ))
    .await
    .unwrap();
    let token = infra::agent_runtime_token(config, id).unwrap();
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
    repo.accept_hermes_run(message.id, intent.run.id, "run_approval_test".into())
        .await
        .unwrap();
    repo.pin_hermes_run_session(
        intent.run.id,
        "run_approval_test".into(),
        format!("fleet:{}:{id}", session.id),
        "approval-native-session".into(),
    )
    .await
    .unwrap();
    let run = repo
        .update_session_agent_run_dispatch(
            intent.run.id,
            Some("run_approval_test".into()),
            SessionRunState::Waiting,
            None,
        )
        .await
        .unwrap();
    let approval = repo
        .upsert_runtime_approval_request(app::RuntimeApprovalCreate {
            session_id: session.id,
            session_run_id: run.id,
            agent_id: id,
            runtime_run_id: "run_approval_test".into(),
            runtime_approval_id: Some("request_one".into()),
            prompt: "Exact owned approval".into(),
            detail: json!({}),
        })
        .await
        .unwrap();
    (session, run, approval)
}

struct ApprovalFixture {
    owner: Uuid,
    repo: Arc<PostgresFleetRepository>,
    config: Arc<AppConfig>,
    runtime: infra::runtime::LocalRuntimeSupervisor,
    session: AgentSession,
    run: SessionAgentRun,
    approval: RuntimeApprovalRequest,
    status: Arc<Mutex<Value>>,
    caps: Arc<Mutex<Value>>,
    calls: Arc<AtomicUsize>,
    server: tokio::task::JoinHandle<()>,
}
impl Drop for ApprovalFixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}

async fn setup(
    code: StatusCode,
    mime: &'static str,
    encoded: bool,
    large: bool,
) -> Option<ApprovalFixture> {
    let (repo, owner, _) = fixture().await?;
    let mut config = AppConfig::default();
    config.fleet.runtime_token_secret = "owned-targeted-approval-fixture".into();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let (session, run, approval) =
        accepted_approval(&repo, owner, "targeted-approval", port, &config).await;
    let bearer = format!(
        "Bearer {}",
        infra::agent_runtime_token(&config, run.agent_id).unwrap()
    );
    let calls = Arc::new(AtomicUsize::new(0));
    let status = Arc::new(Mutex::new(pending_status("request_one")));
    let caps = Arc::new(Mutex::new(capabilities()));
    let expected = bearer.clone();
    let cap = caps.clone();
    let run_status = status.clone();
    let count = calls.clone();
    let router = Router::new()
        .route("/health", get(|| async { Json(json!({"status":"ok"})) }))
        .route(
            "/v1/capabilities",
            get(move |headers: HeaderMap| {
                assert_eq!(headers["authorization"], expected);
                assert_eq!(headers["accept-encoding"], "identity");
                let cap = cap.lock().unwrap().clone();
                async move { Json(cap) }
            }),
        )
        .route(
            "/v1/runs/run_approval_test",
            get(move |headers: HeaderMap| {
                assert_eq!(headers["authorization"], bearer);
                let status = run_status.lock().unwrap().clone();
                async move { Json(status) }
            }),
        )
        .route(
            "/v1/runs/run_approval_test/approval",
            post(move |headers: HeaderMap, Json(body): Json<Value>| {
                assert_eq!(headers["accept-encoding"], "identity");
                assert_eq!(
                    body,
                    json!({"choice":"once","request_id":"request_one","resolve_all":false})
                );
                count.fetch_add(1, Ordering::SeqCst);
                async move {
                    let mut response = Response::builder()
                        .status(code)
                        .header("content-type", mime);
                    if encoded {
                        response = response.header("content-encoding", "gzip");
                    }
                    let body = if large {
                        "x".repeat(65_537)
                    } else {
                        json!({"object":"hermes.run.approval_response","run_id":"run_approval_test",
                        "request_id":"request_one","choice":"once","resolved":1})
                        .to_string()
                    };
                    response.body(Body::from(body)).unwrap()
                }
            }),
        );
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let repo = Arc::new(repo);
    let config = Arc::new(config);
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime = infra::runtime::LocalRuntimeSupervisor::new(config.clone(), repo.clone(), events);
    Some(ApprovalFixture {
        owner,
        repo,
        config,
        runtime,
        session,
        run,
        approval,
        status,
        caps,
        calls,
        server,
    })
}

#[tokio::test]
async fn approval_requires_fresh_exact_native_waiting_identity_and_capability() {
    for (field, value) in [
        ("status", json!("running")),
        ("session_id", json!("foreign")),
        ("run_id", json!("foreign")),
        (
            "approval",
            json!({"event":"approval.request","run_id":"run_approval_test","request_id":"foreign"}),
        ),
    ] {
        let Some(f) = setup(StatusCode::OK, "application/json", false, false).await else {
            return;
        };
        f.status.lock().unwrap()[field] = value;
        let agent = f.repo.get_agent(f.run.agent_id).await.unwrap();
        assert!(
            f.runtime
                .resolve_targeted_approval(&agent, &f.run, &f.approval, ApprovalChoice::Once)
                .await
                .is_err()
        );
        assert_eq!(f.calls.load(Ordering::SeqCst), 0);
    }
    for (pointer, value) in [
        ("/features/run_approval_response", json!(false)),
        ("/features/approval_events", json!(false)),
        ("/endpoints/run_approval/method", json!("GET")),
        ("/endpoints/run_approval/path", json!("/approval")),
    ] {
        let Some(f) = setup(StatusCode::OK, "application/json", false, false).await else {
            return;
        };
        *f.caps.lock().unwrap().pointer_mut(pointer).unwrap() = value;
        let agent = f.repo.get_agent(f.run.agent_id).await.unwrap();
        assert!(
            f.runtime
                .resolve_targeted_approval(&agent, &f.run, &f.approval, ApprovalChoice::Once)
                .await
                .is_err()
        );
        assert_eq!(f.calls.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn approval_rejects_changed_origin_credentials_and_legacy_run_without_post() {
    for change in [
        "origin",
        "credential",
        "legacy",
        "stale_native_session",
        "terminal",
    ] {
        let Some(f) = setup(StatusCode::OK, "application/json", false, false).await else {
            return;
        };
        let mut run = f.run.clone();
        let mut approval = f.approval.clone();
        let mut config = (*f.config).clone();
        let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
            .await
            .unwrap();
        match change {
            "origin" => {
                db.execute(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    "UPDATE agents SET api_port=api_port+1 WHERE id=$1",
                    [run.agent_id.into()],
                ))
                .await
                .unwrap();
            }
            "credential" => {
                config.fleet.runtime_token_secret = "rotated-owned-secret".into();
            }
            "legacy" => {
                let (_, legacy, request) =
                    approval_fixture(&f.repo, f.owner, "legacy-unproven").await;
                run = legacy;
                approval = request;
            }
            "stale_native_session" => {
                run.runtime_session_id = Some("foreign".into());
            }
            "terminal" => {
                let intent = f
                    .repo
                    .get_hermes_dispatch_intent_for_run(run.id)
                    .await
                    .unwrap()
                    .unwrap();
                f.repo
                    .commit_hermes_terminal(app::HermesTerminalCommit {
                        message_id: intent.message_id,
                        run_id: run.id,
                        runtime_run_id: run.runtime_run_id.clone().unwrap(),
                        runtime_session_id: run.runtime_session_id.clone().unwrap(),
                        state: SessionRunState::Completed,
                        body: None,
                        error: None,
                    })
                    .await
                    .unwrap();
            }
            _ => unreachable!(),
        }
        let (events, _) = tokio::sync::broadcast::channel(32);
        let runtime =
            infra::runtime::LocalRuntimeSupervisor::new(Arc::new(config), f.repo.clone(), events);
        let agent = f.repo.get_agent(run.agent_id).await.unwrap();
        assert!(
            runtime
                .resolve_targeted_approval(&agent, &run, &approval, ApprovalChoice::Once)
                .await
                .is_err(),
            "{change}"
        );
        assert_eq!(f.calls.load(Ordering::SeqCst), 0, "{change}");
    }
}

#[tokio::test]
async fn approval_accepts_only_exact_http_200_json_unencoded_bounded_ack() {
    for (code, mime, encoded, large, valid) in [
        (
            StatusCode::OK,
            "application/json; charset=utf-8",
            false,
            false,
            true,
        ),
        (
            StatusCode::ACCEPTED,
            "application/json",
            false,
            false,
            false,
        ),
        (
            StatusCode::NO_CONTENT,
            "application/json",
            false,
            false,
            false,
        ),
        (StatusCode::OK, "text/plain", false, false, false),
        (StatusCode::OK, "application/json", true, false, false),
        (StatusCode::OK, "application/json", false, true, false),
    ] {
        let Some(f) = setup(code, mime, encoded, large).await else {
            return;
        };
        let agent = f.repo.get_agent(f.run.agent_id).await.unwrap();
        assert_eq!(
            f.runtime
                .resolve_targeted_approval(&agent, &f.run, &f.approval, ApprovalChoice::Once)
                .await
                .is_ok(),
            valid
        );
        assert_eq!(f.calls.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn approval_http_preflight_hold_cannot_be_replayed_as_a_new_native_effect() {
    let Some(f) = setup(StatusCode::OK, "application/json", false, false).await else {
        return;
    };
    f.caps.lock().unwrap()["features"]["approval_events"] = json!(false);
    let (events, _) = tokio::sync::broadcast::channel(32);
    let (restart_tx, _) = tokio::sync::mpsc::channel(1);
    let ctx = Arc::new(app::AppContext::new(
        f.config.clone(),
        f.repo.clone(),
        Arc::new(infra::FilesystemProvisioner),
        Arc::new(f.runtime.clone()),
        events,
        restart_tx,
    ));
    let token = ctx
        .auth
        .issue_tokens(&f.repo.find_user_by_id(f.owner).await.unwrap().unwrap())
        .unwrap()
        .response
        .access_token;
    let router = Router::new()
        .route(
            "/api/v1/sessions/{session_id}/approvals/{approval_id}/decision",
            get(api::routes::approvals::read).post(api::routes::approvals::decide),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            ctx.clone(),
            api::middleware::require_auth,
        ))
        .with_state(ctx);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!(
        "http://{}/api/v1/sessions/{}/approvals/{}/decision",
        listener.local_addr().unwrap(),
        f.session.id,
        f.approval.id
    );
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let body = domain::ApprovalDecisionRequest {
        choice: ApprovalChoice::Once,
        idempotency_key: Uuid::new_v4().to_string(),
    };
    let first = client
        .post(&url)
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    let first = first.json::<domain::ApprovalDecision>().await.unwrap();
    assert_eq!(first.state, domain::ApprovalDecisionState::Uncertain);
    assert_eq!(first.actor_user_id, f.owner);
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
    *f.caps.lock().unwrap() = capabilities();
    for _ in 0..2 {
        let replay = client
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .unwrap()
            .json::<domain::ApprovalDecision>()
            .await
            .unwrap();
        assert_eq!(replay.id, first.id);
        assert_eq!(replay.state, domain::ApprovalDecisionState::Uncertain);
    }
    assert_eq!(
        client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .json::<domain::ApprovalDecision>()
            .await
            .unwrap()
            .id,
        first.id
    );
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        f.repo.list_session_approvals(f.session.id).await.unwrap()[0].state,
        domain::RuntimeApprovalState::Pending
    );
    server.abort();
}
