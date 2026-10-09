use super::*;
use app::RuntimeSupervisor;
use axum::{
    Json, Router,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use domain::MessageDeliveryState;
use serde_json::{Value, json};
use tokio::time::timeout;

struct Server(tokio::task::JoinHandle<()>);
impl Drop for Server {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn prepared_journal(
    repo: &PostgresFleetRepository,
    config: &AppConfig,
    agent: Uuid,
    session: &domain::AgentSession,
    message: &domain::SessionMessage,
    port: u16,
) -> app::HermesDispatchIntent {
    use sha2::{Digest, Sha256};
    let token = infra::agent_runtime_token(config, agent).unwrap();
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
        agent_id: agent,
        run_role: SessionRunRole::Primary,
        requested_session_id: format!("fleet:{}:{agent}", session.id),
        input: message.body.clone(),
        origin: origin.clone(),
        credential_fingerprint: fingerprint.clone(),
        capabilities: hermes_protocol_fixture::capabilities(),
    })
    .await
    .unwrap();
    repo.claim_hermes_submission(message.id, origin, fingerprint)
        .await
        .unwrap()
        .unwrap()
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn unknown_acceptance_has_a_durable_original_request_and_never_sends_a_second_post() {
    let Some((repo, owner, _)) = required_fixture().await else {
        return;
    };
    let repo = Arc::new(repo);
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let agent_id = agent(&repo).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE agents SET api_port=$2 WHERE id=$1",
        [agent_id.into(), i32::from(port).into()],
    ))
    .await
    .unwrap();
    let session = repo
        .create_session(chat(agent_id, "unknown-journal"), owner)
        .await
        .unwrap();
    let message = repo
        .create_session_message(session.id, prompt("unknown-journal"), owner)
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE message_dispatch_outbox SET state='dispatching' WHERE message_id=$1",
        [message.id.into()],
    ))
    .await
    .unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let persisted = repo.clone();
    let message_id = message.id;
    let router = hermes_protocol_fixture::preflight(Router::new().route(
        "/v1/runs",
        post(move |headers: HeaderMap, body: axum::body::Bytes| {
            let count = count.clone();
            let persisted = persisted.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                let intent = persisted
                    .get_hermes_dispatch_intent(message_id)
                    .await
                    .unwrap()
                    .unwrap();
                assert!(
                    intent.submission_attempted,
                    "permit must commit before HTTP"
                );
                assert_eq!(body.as_ref(), intent.request_body.as_bytes());
                assert_eq!(headers["idempotency-key"], intent.idempotency_key);
                assert!(intent.run.runtime_run_id.is_none());
                // Runtime accepted, but its response does not establish a verified run ID.
                (
                    StatusCode::ACCEPTED,
                    Json(json!({"status":"started","replayed":false})),
                )
            }
        }),
    ));
    let _server = Server(tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap()
    }));
    let config = AppConfig {
        fleet: shared::FleetConfig {
            runtime_token_secret: "journal-http-fixture-only".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime =
        infra::runtime::LocalRuntimeSupervisor::new(Arc::new(config), repo.clone(), events);
    let agent = repo.get_agent(agent_id).await.unwrap();
    let first = runtime
        .send_message(&agent, &session, &message)
        .await
        .unwrap();
    assert_eq!(first.status, AgentStatus::Failed);
    let original = repo
        .get_hermes_dispatch_intent(message.id)
        .await
        .unwrap()
        .unwrap();
    assert!(original.submission_attempted);
    assert_eq!(original.run.state, SessionRunState::Pending);
    assert!(original.run.runtime_run_id.is_none());
    let prompt = repo
        .list_session_messages(session.id)
        .await
        .unwrap()
        .into_iter()
        .find(|row| row.id == message.id)
        .unwrap();
    assert_eq!(prompt.delivery_state, MessageDeliveryState::Pending);
    assert!(prompt.delivery_error.is_some());
    let second = runtime
        .send_message(&agent, &session, &message)
        .await
        .unwrap();
    assert_eq!(second.status, AgentStatus::Failed);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let current = repo
        .get_hermes_dispatch_intent(message.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(current.run.id, original.run.id);
    assert_eq!(current.request_body, original.request_body);
    assert_eq!(current.request_hash, original.request_hash);
    assert_eq!(current.recovery_deadline, original.recovery_deadline);
    assert!(
        repo.prepare_session_agent_run(
            session.id,
            agent_id,
            SessionRunRole::Primary,
            format!("fleet:{}:{agent_id}", session.id)
        )
        .await
        .is_err(),
        "unknown acceptance must hold capacity"
    );
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn acceptance_readback_never_probes_legacy_or_changed_original_context() {
    let Some((repo, owner, _)) = required_fixture().await else {
        return;
    };
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let old_port = if port == 65535 { 65534 } else { 65535 };
    let config = AppConfig {
        fleet: shared::FleetConfig {
            runtime_token_secret: "original-context-fixture-only".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let mut rotated = config.clone();
    rotated.fleet.runtime_token_secret = "rotated-context-fixture-only".into();
    let mut held = Vec::new();
    let mut current = None;
    for mode in ["legacy", "rotated", "moved", "current"] {
        let agent_id = agent(&repo).await;
        let original_port = if mode == "moved" { old_port } else { port };
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE agents SET api_port=$2 WHERE id=$1",
            [agent_id.into(), i32::from(original_port).into()],
        ))
        .await
        .unwrap();
        let session = repo
            .create_session(chat(agent_id, &format!("scope-{mode}")), owner)
            .await
            .unwrap();
        let message = repo
            .create_session_message(session.id, prompt("scope-once"), owner)
            .await
            .unwrap();
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE message_dispatch_outbox SET state='dispatching' WHERE message_id=$1",
            [message.id.into()],
        ))
        .await
        .unwrap();
        let run = if mode == "legacy" {
            repo.prepare_session_agent_run(
                session.id,
                agent_id,
                SessionRunRole::Primary,
                format!("fleet:{}:{agent_id}", session.id),
            )
            .await
            .unwrap()
        } else {
            prepared_journal(
                &repo,
                if mode == "rotated" { &rotated } else { &config },
                agent_id,
                &session,
                &message,
                original_port,
            )
            .await
            .run
        };
        repo.accept_hermes_run(message.id, run.id, format!("run_context_{mode}"))
            .await
            .unwrap();
        if mode == "moved" {
            db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE agents SET api_port=$2 WHERE id=$1",
                [agent_id.into(), i32::from(port).into()],
            ))
            .await
            .unwrap();
        }
        if mode == "current" {
            current = Some(run.id);
        } else {
            held.push(run.id);
        }
    }
    let denied_reads = Arc::new(AtomicUsize::new(0));
    let count = denied_reads.clone();
    let current_reads = Arc::new(AtomicUsize::new(0));
    let allowed = current_reads.clone();
    let router = Router::new()
        .route("/v1/runs/{run_id}", get(move |axum::extract::Path(raw): axum::extract::Path<String>| {
            let valid = raw == "run_context_current";
            if valid { allowed.fetch_add(1, Ordering::SeqCst); } else { count.fetch_add(1, Ordering::SeqCst); }
            async move { Json(json!({"object":"hermes.run","run_id":raw,"session_id":"native-context-session","status":"running"})) }
        }))
        .route("/v1/runs/run_context_current/events", get(|| async {
            ([(axum::http::header::CONTENT_TYPE, "text/event-stream")],
                "event: run.completed\ndata: {\"run_id\":\"run_context_current\",\"completed\":true,\"partial\":false,\"interrupted\":false,\"output\":\"original context recovered\"}\n\n")
        }));
    let _server = Server(tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap()
    }));
    let repo = Arc::new(repo);
    let (events, _) = tokio::sync::broadcast::channel(32);
    let _runtime =
        infra::runtime::LocalRuntimeSupervisor::new(Arc::new(config), repo.clone(), events);
    timeout(Duration::from_secs(10), async {
        while repo
            .get_session_agent_run(current.unwrap())
            .await
            .unwrap()
            .state
            != SessionRunState::Completed
        {
            sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .unwrap();
    sleep(Duration::from_millis(350)).await;
    assert!(current_reads.load(Ordering::SeqCst) >= 1);
    assert_eq!(denied_reads.load(Ordering::SeqCst), 0);
    for run in held {
        assert_eq!(
            repo.get_session_agent_run(run).await.unwrap().state,
            SessionRunState::Pending
        );
    }
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn persisted_ack_recovers_after_restart_by_get_without_a_second_submission() {
    scenario(true).await;
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn production_dispatch_keeps_ack_and_capacity_until_effective_session_readback() {
    scenario(false).await;
}

async fn scenario(restarted: bool) {
    let Some((repo, owner, _)) = required_fixture().await else {
        return;
    };
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let agent_id = agent(&repo).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE agents SET api_port=$2 WHERE id=$1",
        [agent_id.into(), i32::from(port).into()],
    ))
    .await
    .unwrap();
    let session = repo
        .create_session(chat(agent_id, "readback-recovery"), owner)
        .await
        .unwrap();
    let message = repo
        .create_session_message(session.id, prompt("readback-recovery-once"), owner)
        .await
        .unwrap();
    let requested = format!("fleet:{}:{agent_id}", session.id);
    let effective = Uuid::new_v4().to_string();
    let phase = Arc::new(AtomicUsize::new(0));
    let posts = Arc::new(AtomicUsize::new(0));
    let reads = Arc::new(AtomicUsize::new(0));
    let controls = Arc::new(AtomicUsize::new(0));
    let stop_calls = controls.clone();
    let steer_calls = controls.clone();
    let config = AppConfig {
        fleet: shared::FleetConfig {
            runtime_token_secret: "acceptance-fixture-only".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let bearer = format!(
        "Bearer {}",
        infra::agent_runtime_token(&config, agent_id).unwrap()
    );
    let expected_bearer = bearer.clone();
    let count = posts.clone();
    let expected_requested = requested.clone();
    let expected_message = message.id;
    let get_bearer = bearer.clone();
    let current = phase.clone();
    let read_count = reads.clone();
    let native_session = effective.clone();
    let router = Router::new()
        .route("/v1/runs/run_readback_recovery/stop", post(move || {
            stop_calls.fetch_add(1, Ordering::SeqCst);
            async { Json(json!({"accepted":true})) }
        }))
        .route("/v1/runs/run_readback_recovery/steer", post(move || {
            steer_calls.fetch_add(1, Ordering::SeqCst);
            async { Json(json!({"accepted":true})) }
        }))
        .route("/v1/runs", post(move |headers: HeaderMap, Json(body): Json<Value>| {
            assert_eq!(headers["authorization"], expected_bearer);
            assert_eq!(headers["idempotency-key"], expected_message.to_string());
            assert_eq!(body["session_id"], expected_requested);
            count.fetch_add(1, Ordering::SeqCst);
            async { (StatusCode::ACCEPTED, Json(json!({"run_id":"run_readback_recovery","status":"started","replayed":false}))) }
        }))
        .route("/v1/runs/run_readback_recovery", get(move |headers: HeaderMap| {
            assert_eq!(headers["authorization"], get_bearer);
            assert_eq!(headers["accept-encoding"], "identity");
            read_count.fetch_add(1, Ordering::SeqCst);
            let phase = current.load(Ordering::SeqCst);
            let native = native_session.clone();
            async move { if phase == 0 {
                (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"private":"fixture-only-private-upstream"})))
            } else if phase == 1 {
                (StatusCode::OK, Json(json!({"object":"hermes.run","run_id":"foreign","session_id":native,"status":"running"})))
            } else {
                (StatusCode::OK, Json(json!({"object":"hermes.run","run_id":"run_readback_recovery","session_id":native,"status":"running"})))
            } }
        }))
        .route("/v1/runs/run_readback_recovery/events", get(move |headers: HeaderMap| {
            assert_eq!(headers["authorization"], bearer);
            async { ([(axum::http::header::CONTENT_TYPE, "text/event-stream")],
                "event: run.completed\ndata: {\"event\":\"run.completed\",\"run_id\":\"run_readback_recovery\",\"completed\":true,\"partial\":false,\"interrupted\":false,\"output\":\"recovered response\"}\n\n") }
        }));
    let router = hermes_protocol_fixture::preflight(router);
    let _server = Server(tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap()
    }));
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE message_dispatch_outbox SET state='dispatching' WHERE message_id=$1",
        [message.id.into()],
    ))
    .await
    .unwrap();
    if restarted {
        // Simulate a process ending after a real socket ACK committed, before status GET.
        let intent = prepared_journal(&repo, &config, agent_id, &session, &message, port).await;
        let response = reqwest::Client::new()
            .post(format!("http://127.0.0.1:{port}/v1/runs"))
            .bearer_auth(infra::agent_runtime_token(&config, agent_id).unwrap())
            .header("Idempotency-Key", message.id.to_string())
            .header("Content-Type", "application/json")
            .body(intent.request_body)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let payload: Value = response.json().await.unwrap();
        repo.accept_hermes_run(
            message.id,
            intent.run.id,
            payload["run_id"].as_str().unwrap().into(),
        )
        .await
        .unwrap();
    }
    let agent = repo.get_agent(agent_id).await.unwrap();
    let repo = Arc::new(repo);
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime =
        infra::runtime::LocalRuntimeSupervisor::new(Arc::new(config), repo.clone(), events);
    if !restarted {
        let result = runtime
            .send_message(&agent, &session, &message)
            .await
            .unwrap();
        assert_eq!(result.status, AgentStatus::Running);
        assert!(result.message.contains("readback is pending"));
    }
    timeout(Duration::from_secs(10), async {
        while reads.load(Ordering::SeqCst) == 0 {
            sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    let run = repo
        .list_session_agent_runs(session.id)
        .await
        .unwrap()
        .into_iter()
        .find(|run| run.runtime_run_id.as_deref() == Some("run_readback_recovery"))
        .unwrap();
    assert_eq!(run.state, SessionRunState::Pending);
    assert_eq!(run.runtime_session_id.as_deref(), Some(requested.as_str()));
    // A stale caller cannot bypass the fresh persisted unpinned state.
    let mut stale = run.clone();
    stale.state = SessionRunState::Running;
    assert!(matches!(
        runtime
            .stop_run(
                &agent,
                &stale,
                domain::RuntimeControlActor {
                    user_id: owner,
                    idempotency_key: "unreadback-stop".into(),
                }
            )
            .await,
        Err(shared::AppError::Conflict(_))
    ));
    assert!(matches!(
        runtime
            .steer_run(
                &agent,
                &stale,
                domain::SteerSessionRunRequest {
                    input: "must wait for pin".into(),
                },
                domain::RuntimeControlActor {
                    user_id: owner,
                    idempotency_key: "unreadback-steer".into(),
                }
            )
            .await,
        Err(shared::AppError::Conflict(_))
    ));
    assert_eq!(controls.load(Ordering::SeqCst), 0);
    assert_eq!(
        repo.get_session_agent_run(run.id).await.unwrap().state,
        SessionRunState::Pending
    );
    let persisted = repo
        .list_session_messages(session.id)
        .await
        .unwrap()
        .into_iter()
        .find(|item| item.id == message.id)
        .unwrap();
    assert_eq!(persisted.delivery_state, MessageDeliveryState::Dispatched);
    assert_eq!(
        persisted.runtime_message_id.as_deref(),
        Some("run_readback_recovery")
    );
    assert!(
        repo.prepare_session_agent_run(
            session.id,
            agent_id,
            SessionRunRole::Primary,
            requested.clone()
        )
        .await
        .is_err()
    );
    assert!(
        repo.list_pending_hermes_acceptances(None)
            .await
            .unwrap()
            .iter()
            .any(|(_, r)| r.id == run.id)
    );
    phase.store(1, Ordering::SeqCst);
    let read_mark = reads.load(Ordering::SeqCst);
    timeout(Duration::from_secs(12), async {
        while reads.load(Ordering::SeqCst) <= read_mark {
            sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        repo.get_session_agent_run(run.id).await.unwrap().state,
        SessionRunState::Pending
    );
    assert_eq!(posts.load(Ordering::SeqCst), 1);
    phase.store(2, Ordering::SeqCst);
    timeout(Duration::from_secs(15), async {
        loop {
            if repo.get_session_agent_run(run.id).await.unwrap().state == SessionRunState::Completed
            {
                break;
            }
            sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    let final_run = repo.get_session_agent_run(run.id).await.unwrap();
    assert_eq!(
        final_run.runtime_session_id.as_deref(),
        Some(effective.as_str())
    );
    assert_eq!(posts.load(Ordering::SeqCst), 1);
    let replies: Vec<_> = repo
        .list_session_messages(session.id)
        .await
        .unwrap()
        .into_iter()
        .filter(|m| m.message_kind == MessageKind::AssistantMessage)
        .collect();
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0].body, "recovered response");
    assert!(
        repo.list_pending_hermes_acceptances(None)
            .await
            .unwrap()
            .iter()
            .all(|(_, r)| r.id != run.id)
    );
    let replay = repo
        .create_session_message(session.id, prompt("readback-recovery-once"), owner)
        .await
        .unwrap();
    assert!(replay.replayed);
    sleep(Duration::from_millis(350)).await;
    assert_eq!(posts.load(Ordering::SeqCst), 1);
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn readback_keyset_reaches_later_ack_after_more_than_twenty_rejected_reads() {
    use std::collections::HashSet;
    let Some((repo, owner, _)) = required_fixture().await else {
        return;
    };
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let config = AppConfig {
        fleet: shared::FleetConfig {
            runtime_token_secret: "keyset-readback-fixture-only".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bearers = HashSet::new();
    let mut target = None;
    for index in 0..22 {
        let agent_id = agent(&repo).await;
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE agents SET api_port=$2 WHERE id=$1",
            [agent_id.into(), i32::from(port).into()],
        ))
        .await
        .unwrap();
        bearers.insert(format!(
            "Bearer {}",
            infra::agent_runtime_token(&config, agent_id).unwrap()
        ));
        let session = repo
            .create_session(chat(agent_id, &format!("keyset-readback-{index}")), owner)
            .await
            .unwrap();
        let message = repo
            .create_session_message(session.id, prompt("keyset-once"), owner)
            .await
            .unwrap();
        let run = repo
            .list_session_agent_runs(session.id)
            .await
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE message_dispatch_outbox SET state='dispatching' WHERE message_id=$1",
            [message.id.into()],
        ))
        .await
        .unwrap();
        // Fixture-only IDs establish a deterministic keyset ordering; no binding references them.
        let ordered = Uuid::from_u128(if index == 21 { u128::MAX } else { index + 1 });
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE session_agent_runs SET id=$2 WHERE id=$1",
            [run.id.into(), ordered.into()],
        ))
        .await
        .unwrap();
        let intent = prepared_journal(&repo, &config, agent_id, &session, &message, port).await;
        assert_eq!(intent.run.id, ordered);
        let raw = if index == 21 {
            "run_fair_target".to_string()
        } else {
            format!("run_rejected_{index}")
        };
        repo.accept_hermes_run(message.id, ordered, raw)
            .await
            .unwrap();
        if index == 21 {
            target = Some((session.id, ordered));
        }
    }
    let (session_id, run_id) = target.unwrap();
    let rejects = Arc::new(AtomicUsize::new(0));
    let count = rejects.clone();
    let posts = Arc::new(AtomicUsize::new(0));
    let post_count = posts.clone();
    let router = Router::new()
        .route("/v1/runs", post(move || { post_count.fetch_add(1, Ordering::SeqCst); async { StatusCode::INTERNAL_SERVER_ERROR } }))
        .route("/v1/runs/{run_id}", get(move |axum::extract::Path(raw): axum::extract::Path<String>, headers: HeaderMap| {
            assert!(bearers.contains(headers["authorization"].to_str().unwrap()));
            let target = raw == "run_fair_target";
            if !target { count.fetch_add(1, Ordering::SeqCst); }
            async move { if target {
                (StatusCode::OK, Json(json!({"object":"hermes.run","run_id":raw,"session_id":"native-fair-session","status":"running"})))
            } else { (StatusCode::NOT_FOUND, Json(json!({"error":"fixture run is unavailable"}))) } }
        }))
        .route("/v1/runs/run_fair_target/events", get(|| async {
            ([(axum::http::header::CONTENT_TYPE, "text/event-stream")],
                "event: run.completed\ndata: {\"run_id\":\"run_fair_target\",\"completed\":true,\"partial\":false,\"interrupted\":false,\"output\":\"later ACK recovered\"}\n\n")
        }));
    let _server = Server(tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap()
    }));
    let repo = Arc::new(repo);
    let (events, _) = tokio::sync::broadcast::channel(32);
    let _runtime =
        infra::runtime::LocalRuntimeSupervisor::new(Arc::new(config), repo.clone(), events);
    timeout(Duration::from_secs(25), async {
        loop {
            if repo.get_session_agent_run(run_id).await.unwrap().state == SessionRunState::Completed
            {
                break;
            }
            sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("an invalid first page must not starve later ACKs");
    assert!(rejects.load(Ordering::SeqCst) >= 21);
    assert_eq!(posts.load(Ordering::SeqCst), 0);
    let replies: Vec<_> = repo
        .list_session_messages(session_id)
        .await
        .unwrap()
        .into_iter()
        .filter(|m| m.message_kind == MessageKind::AssistantMessage)
        .collect();
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0].body, "later ACK recovered");
}

async fn required_fixture() -> Option<(PostgresFleetRepository, Uuid, Uuid)> {
    std::env::var("FLEET_TEST_DATABASE_URL")
        .expect("isolated PostgreSQL is required for Hermes journal tests");
    super::fixture().await
}
