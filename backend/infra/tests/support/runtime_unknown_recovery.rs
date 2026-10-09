use super::*;
use app::RuntimeSupervisor;
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::Mutex;
use tokio::time::timeout;

const SOURCE: &str = "bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3";

struct Native {
    repo: Arc<PostgresFleetRepository>,
    message_id: Uuid,
    token: String,
    facts: Value,
    current: Mutex<Value>,
    lookup_enabled: std::sync::atomic::AtomicBool,
    posts: AtomicUsize,
    lookups: AtomicUsize,
    reads: AtomicUsize,
    streams: AtomicUsize,
}

impl Native {
    fn authorize(&self, headers: &HeaderMap) {
        assert_eq!(headers["authorization"], format!("Bearer {}", self.token));
        assert_eq!(headers["accept-encoding"], "identity");
    }
}

async fn capabilities(State(native): State<Arc<Native>>, headers: HeaderMap) -> Json<Value> {
    native.authorize(&headers);
    Json(native.current.lock().unwrap().clone())
}

async fn submit(
    State(native): State<Arc<Native>>,
    headers: HeaderMap,
    bytes: axum::body::Bytes,
) -> (StatusCode, Json<Value>) {
    native.authorize(&headers);
    native.posts.fetch_add(1, Ordering::SeqCst);
    let intent = native
        .repo
        .get_hermes_dispatch_intent(native.message_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(intent.state, "submitted", "permit must precede HTTP");
    assert_eq!(
        headers["x-fleet-recovery-store-id"],
        native.facts["store_id"].as_str().unwrap()
    );
    assert_eq!(headers["idempotency-key"], intent.idempotency_key);
    assert_eq!(bytes.as_ref(), intent.request_body.as_bytes());
    // Native accepted, but the acknowledgement cannot establish its ID in Fleet.
    (
        StatusCode::ACCEPTED,
        Json(json!({"status":"started","replayed":false})),
    )
}

async fn lookup(
    State(native): State<Arc<Native>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> (StatusCode, Json<Value>) {
    native.authorize(&headers);
    native.lookups.fetch_add(1, Ordering::SeqCst);
    if !native.lookup_enabled.load(Ordering::SeqCst) {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(json!({})));
    }
    let intent = native
        .repo
        .get_hermes_dispatch_intent(native.message_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        body,
        json!({"contract_version":1, "store_id":native.facts["store_id"],
        "idempotency_key":intent.idempotency_key, "request_json":intent.request_body, "request_sha256":intent.request_hash})
    );
    (
        StatusCode::OK,
        Json(
            json!({"object":"fleet.hermes.recovery.lookup", "contract_version":1,
        "store_id":native.facts["store_id"], "idempotency_key":intent.idempotency_key,
        "request_sha256":intent.request_hash, "scope_fingerprint":native.facts["scope_fingerprint"],
        "profile":"default", "found":true,"run_id":"run_original_recovery"}),
        ),
    )
}

async fn status(State(native): State<Arc<Native>>, headers: HeaderMap) -> Json<Value> {
    native.authorize(&headers);
    native.reads.fetch_add(1, Ordering::SeqCst);
    Json(
        json!({"object":"hermes.run", "run_id":"run_original_recovery", "session_id":"native:recovered",
        "status":"completed","completed":true,"partial":false,"interrupted":false,"output":"Original recovered response"}),
    )
}

async fn events(State(native): State<Arc<Native>>) -> StatusCode {
    native.streams.fetch_add(1, Ordering::SeqCst);
    StatusCode::NOT_FOUND
}

struct Server(tokio::task::JoinHandle<()>);
impl Drop for Server {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn wait_completed(repo: &PostgresFleetRepository, run: Uuid) {
    let mut after = None;
    let mut pages = 0;
    loop {
        let page = repo
            .list_recoverable_hermes_acceptances(after)
            .await
            .unwrap();
        let Some((_, last)) = page.last() else {
            break;
        };
        after = Some(last.id);
        pages += 1;
    }
    timeout(Duration::from_secs(15 + (pages + 1) * 5), async {
        loop {
            if repo.get_session_agent_run(run).await.unwrap().state == SessionRunState::Completed {
                break;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("unknown acceptance did not recover");
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn runtime_unknown_recovery_restores_original_id_without_second_submission() {
    let Some((repo, owner, _)) = recovery_fixture().await else {
        return;
    };
    let repo = Arc::new(repo);
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let agent_id = agent(&repo).await;
    repo.update_agent_status(agent_id, AgentStatus::Ready)
        .await
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE agents SET api_port=$2 WHERE id=$1",
        [agent_id.into(), i32::from(port).into()],
    ))
    .await
    .unwrap();
    let key = Uuid::new_v4().to_string();
    let session = repo
        .create_session(chat(agent_id, &key), owner)
        .await
        .unwrap();
    let message = repo
        .create_session_message(session.id, prompt(&key), owner)
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE message_dispatch_outbox SET state='dispatching' WHERE message_id=$1",
        [message.id.into()],
    ))
    .await
    .unwrap();
    repo.update_agent_status(agent_id, AgentStatus::Running)
        .await
        .unwrap();
    let config = AppConfig {
        fleet: shared::FleetConfig {
            runtime_token_secret: format!("recovery-fixture-{}", Uuid::new_v4()),
            hermes_recovery_extension_enabled: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let token = infra::agent_runtime_token(&config, agent_id).unwrap();
    let scope = format!(
        "{:x}",
        Sha256::digest([b"default\0".as_slice(), token.as_bytes()].concat())
    );
    let facts = json!({"object":"fleet.hermes.recovery.capabilities","contract_version":1,
        "store_id":Uuid::new_v4().to_string(),"profile":"default","scope_fingerprint":scope,
        "native_source_revision":SOURCE,"lookup":{"method":"POST","path":"/fleet/v1/recovery/lookup"},
        "non_dispatch":true,"durable_witness":true});
    let native = Arc::new(Native {
        repo: repo.clone(),
        message_id: message.id,
        token,
        facts: facts.clone(),
        current: Mutex::new(facts),
        lookup_enabled: std::sync::atomic::AtomicBool::new(false),
        posts: AtomicUsize::new(0),
        lookups: AtomicUsize::new(0),
        reads: AtomicUsize::new(0),
        streams: AtomicUsize::new(0),
    });
    let router = hermes_protocol_fixture::preflight(
        Router::new()
            .route("/fleet/v1/recovery/capabilities", get(capabilities))
            .route("/fleet/v1/recovery/lookup", post(lookup))
            .route("/v1/runs", post(submit))
            .route("/v1/runs/run_original_recovery", get(status))
            .route("/v1/runs/run_original_recovery/events", get(events))
            .with_state(native.clone()),
    );
    let _server = Server(tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    }));
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime =
        infra::runtime::LocalRuntimeSupervisor::new(Arc::new(config), repo.clone(), events);
    let saved = runtime
        .send_message(&repo.get_agent(agent_id).await.unwrap(), &session, &message)
        .await
        .unwrap();
    assert_eq!(saved.status, AgentStatus::Failed);
    let original = repo
        .get_hermes_dispatch_intent(message.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(original.state, "submitted");
    assert!(original.run.runtime_run_id.is_none());
    assert!(original.recovery_allowed);
    assert_eq!(original.capabilities["fleet_recovery"], native.facts);
    native.lookup_enabled.store(true, Ordering::SeqCst);
    wait_completed(&repo, original.run.id).await;
    let final_intent = repo
        .get_hermes_dispatch_intent(message.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(final_intent.state, "accepted");
    assert_eq!(
        final_intent.run.runtime_run_id.as_deref(),
        Some("run_original_recovery")
    );
    assert_eq!(
        final_intent.run.runtime_session_id.as_deref(),
        Some("native:recovered")
    );
    assert_eq!(final_intent.request_body, original.request_body);
    assert_eq!(final_intent.recovery_deadline, original.recovery_deadline);
    let messages = repo.list_session_messages(session.id).await.unwrap();
    assert_eq!(
        messages
            .iter()
            .filter(|m| m.message_kind == MessageKind::AssistantMessage)
            .count(),
        1
    );
    assert_eq!(native.posts.load(Ordering::SeqCst), 1);
    assert!(native.lookups.load(Ordering::SeqCst) >= 1);
    assert!(native.reads.load(Ordering::SeqCst) >= 1);
    assert_eq!(native.streams.load(Ordering::SeqCst), 0);
    let replay = repo
        .accept_recovered_hermes_run(
            message.id,
            original.run.id,
            "run_original_recovery".into(),
            original.capabilities.clone(),
        )
        .await
        .unwrap();
    assert_eq!(replay.updated_at, final_intent.run.updated_at);
    assert!(
        repo.accept_recovered_hermes_run(
            message.id,
            original.run.id,
            "run_replacement".into(),
            original.capabilities
        )
        .await
        .is_err()
    );
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn runtime_unknown_recovery_commit_rechecks_original_facts_and_database_deadline() {
    let Some((repo, owner, _)) = recovery_fixture().await else {
        return;
    };
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let agent_id = agent(&repo).await;
    repo.update_agent_status(agent_id, AgentStatus::Ready)
        .await
        .unwrap();
    let key = Uuid::new_v4().to_string();
    let session = repo
        .create_session(chat(agent_id, &key), owner)
        .await
        .unwrap();
    let message = repo
        .create_session_message(session.id, prompt(&key), owner)
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE message_dispatch_outbox SET state='dispatching' WHERE message_id=$1",
        [message.id.into()],
    ))
    .await
    .unwrap();
    repo.update_agent_status(agent_id, AgentStatus::Running)
        .await
        .unwrap();
    let native = repo.get_agent(agent_id).await.unwrap();
    let origin = format!("http://127.0.0.1:{}", native.api_port.unwrap());
    let mut facts = hermes_protocol_fixture::capabilities();
    facts["fleet_recovery"] = json!({"object":"fleet.hermes.recovery.capabilities","contract_version":1,
        "store_id":Uuid::new_v4().to_string(),"profile":"default","scope_fingerprint":"a".repeat(64),
        "native_source_revision":SOURCE,"lookup":{"method":"POST","path":"/fleet/v1/recovery/lookup"},
        "non_dispatch":true,"durable_witness":true});
    let draft = app::HermesDispatchDraft {
        message_id: message.id,
        session_id: session.id,
        agent_id,
        run_role: SessionRunRole::Primary,
        requested_session_id: format!("fleet:{}:{agent_id}", session.id),
        input: message.body,
        origin: origin.clone(),
        credential_fingerprint: "b".repeat(64),
        capabilities: facts.clone(),
    };
    repo.prepare_hermes_dispatch(draft).await.unwrap();
    let submitted = repo
        .claim_hermes_submission(message.id, origin, "b".repeat(64))
        .await
        .unwrap()
        .unwrap();
    let transcript_before =
        serde_json::to_value(repo.list_session_messages(session.id).await.unwrap()).unwrap();
    let mut foreign = facts.clone();
    foreign["fleet_recovery"]["store_id"] = json!(Uuid::new_v4().to_string());
    assert!(
        repo.accept_recovered_hermes_run(
            message.id,
            submitted.run.id,
            "run_original_deadline".into(),
            foreign
        )
        .await
        .is_err()
    );
    // Controlled expiry, preserving the original duration constraint. Not a production mutation.
    let txn = db.begin().await.unwrap();
    txn.execute_unprepared("SET LOCAL session_replication_role=replica")
        .await
        .unwrap();
    txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE hermes_dispatch_journal SET created_at=created_at-interval '2 days',recovery_deadline=recovery_deadline-interval '2 days' WHERE message_id=$1",
        [message.id.into()])).await.unwrap();
    txn.commit().await.unwrap();
    let expired = repo
        .get_hermes_dispatch_intent(message.id)
        .await
        .unwrap()
        .unwrap();
    assert!(!expired.recovery_allowed);
    assert!(
        submitted.recovery_allowed,
        "original pre-lookup observation was still valid"
    );
    assert!(
        repo.accept_recovered_hermes_run(
            message.id,
            submitted.run.id,
            "run_original_deadline".into(),
            facts
        )
        .await
        .is_err()
    );
    let held = repo
        .get_hermes_dispatch_intent(message.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(held.state, "submitted");
    assert!(held.run.runtime_run_id.is_none());
    assert_eq!(held.run.state, SessionRunState::Pending);
    let messages = repo.list_session_messages(session.id).await.unwrap();
    assert_eq!(serde_json::to_value(&messages).unwrap(), transcript_before);
    assert!(
        messages
            .iter()
            .find(|saved| saved.id == message.id)
            .unwrap()
            .runtime_message_id
            .is_none()
    );
}
