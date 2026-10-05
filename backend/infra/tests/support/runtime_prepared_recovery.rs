use super::*;
use app::{HermesDispatchDraft, HermesDispatchIntent};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use domain::{AgentSession, SessionMessage};
use sea_orm::DatabaseConnection;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::{net::TcpListener, time::timeout};

struct Prepared {
    repo: Arc<PostgresFleetRepository>,
    db: DatabaseConnection,
    owner: Uuid,
    config: AppConfig,
    session: AgentSession,
    message: SessionMessage,
    intent: HermesDispatchIntent,
    listener: Option<TcpListener>,
}

async fn setup() -> Option<Prepared> {
    let (repo, owner, _) = fixture().await?;
    let repo = Arc::new(repo);
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let id = agent(&repo).await;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = i32::from(listener.local_addr().unwrap().port());
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE agents SET api_port=$2 WHERE id=$1",
        [id.into(), port.into()],
    ))
    .await
    .unwrap();
    let key = Uuid::new_v4().to_string();
    let session = repo.create_session(chat(id, &key), owner).await.unwrap();
    let message = repo
        .create_session_message(
            session.id,
            CreateSessionMessageRequest {
                body: "Original prepared prompt\nwith \"quoted\" bytes and backslash \\".into(),
                author_agent_id: None,
                message_kind: Some(MessageKind::UserPrompt),
                runtime_message_id: None,
                idempotency_key: Some(key),
            },
            owner,
        )
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE message_dispatch_outbox SET state='dispatching' WHERE message_id=$1",
        [message.id.into()],
    ))
    .await
    .unwrap();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE session_agent_runs SET model='prepared-model',provider='prepared-provider',model_options=$2 WHERE session_id=$1",
        [session.id.into(),json!({"temperature":0.25,"max_tokens":128}).into()])).await.unwrap();
    repo.update_agent_status(id, AgentStatus::Running)
        .await
        .unwrap();
    let mut config = AppConfig::default();
    config.fleet.runtime_token_secret = format!("prepared-fixture-{}", Uuid::new_v4());
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
    let intent = repo
        .prepare_hermes_dispatch(HermesDispatchDraft {
            message_id: message.id,
            session_id: session.id,
            agent_id: id,
            run_role: SessionRunRole::Primary,
            requested_session_id: format!("fleet:{}:{id}", session.id),
            input: message.body.clone(),
            origin: format!("http://127.0.0.1:{port}"),
            credential_fingerprint: fingerprint,
            capabilities: hermes_protocol_fixture::capabilities(),
        })
        .await
        .unwrap();
    Some(Prepared {
        repo,
        db,
        owner,
        config,
        session,
        message,
        intent,
        listener: Some(listener),
    })
}

async fn queued(p: &Prepared) -> bool {
    let mut after = None;
    for _ in 0..100 {
        let rows = p.repo.list_prepared_hermes_dispatches(after).await.unwrap();
        if rows
            .iter()
            .any(|(_, intent)| intent.message_id == p.message.id)
        {
            return true;
        }
        let Some((_, last)) = rows.last() else {
            return false;
        };
        after = Some(last.run.id);
    }
    panic!("prepared queue did not terminate within bounded fixture pages");
}

struct Native {
    repo: Arc<PostgresFleetRepository>,
    message_id: Uuid,
    token: String,
    original_body: String,
    malformed_ack: bool,
    invalid_capabilities: bool,
    posts: AtomicUsize,
    probes: AtomicUsize,
    streams: AtomicUsize,
}

impl Native {
    fn auth(&self, headers: &HeaderMap) -> Result<(), (StatusCode, Json<Value>)> {
        // A reused loopback port may receive a stale fixture's read; reject like native HTTP.
        if headers
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            != Some(format!("Bearer {}", self.token).as_str())
        {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error":"unauthorized"})),
            ));
        }
        assert_eq!(headers["accept-encoding"], "identity");
        Ok(())
    }
}

async fn capabilities(
    State(native): State<Arc<Native>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    native.auth(&headers)?;
    native.probes.fetch_add(1, Ordering::SeqCst);
    let mut caps = hermes_protocol_fixture::capabilities();
    if native.invalid_capabilities {
        caps["features"]["runs_idempotency"]["durable"] = json!(false);
    }
    Ok(Json(caps))
}

async fn submit(
    State(native): State<Arc<Native>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    native.auth(&headers)?;
    native.posts.fetch_add(1, Ordering::SeqCst);
    let journal = native
        .repo
        .get_hermes_dispatch_intent(native.message_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(journal.state, "submitted", "permit must commit before HTTP");
    assert!(journal.submission_attempted);
    assert_eq!(headers["idempotency-key"], native.message_id.to_string());
    assert_eq!(body.as_ref(), native.original_body.as_bytes());
    assert!(headers.get("x-fleet-recovery-store-id").is_none());
    Ok((
        StatusCode::ACCEPTED,
        Json(if native.malformed_ack {
            json!({"status":"started","replayed":false})
        } else {
            json!({"run_id":"run_prepared_original","status":"started","replayed":false})
        }),
    ))
}

async fn status(
    State(native): State<Arc<Native>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    native.auth(&headers)?;
    Ok(Json(
        json!({"object":"hermes.run","run_id":"run_prepared_original","session_id":"native:prepared",
        "status":"completed","completed":true,"partial":false,"interrupted":false,"output":"Prepared original answer"}),
    ))
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

fn server(
    p: &mut Prepared,
    malformed_ack: bool,
    invalid_capabilities: bool,
) -> (Arc<Native>, Server) {
    let native = Arc::new(Native {
        repo: p.repo.clone(),
        message_id: p.message.id,
        token: infra::agent_runtime_token(&p.config, p.intent.run.agent_id).unwrap(),
        original_body: p.intent.request_body.clone(),
        malformed_ack,
        invalid_capabilities,
        posts: AtomicUsize::new(0),
        probes: AtomicUsize::new(0),
        streams: AtomicUsize::new(0),
    });
    let router = Router::new()
        .route("/health", get(|| async { Json(json!({"status":"ok"})) }))
        .route("/v1/capabilities", get(capabilities))
        .route("/v1/runs", post(submit))
        .route("/v1/runs/run_prepared_original", get(status))
        .route("/v1/runs/run_prepared_original/events", get(events))
        .with_state(native.clone());
    let listener = p.listener.take().unwrap();
    (
        native,
        Server(tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        })),
    )
}

fn supervisor(p: &Prepared, config: AppConfig) -> infra::runtime::LocalRuntimeSupervisor {
    let (events, _) = tokio::sync::broadcast::channel(32);
    infra::runtime::LocalRuntimeSupervisor::new(Arc::new(config), p.repo.clone(), events)
}

#[tokio::test]
async fn prepared_native_fixture_rejects_foreign_token_before_effect_or_probe() {
    let Some(mut p) = setup().await else {
        return;
    };
    let (native, _server) = server(&mut p, false, false);
    let client = reqwest::Client::new();
    for request in [
        client.get(format!("{}/v1/capabilities", p.intent.origin)),
        client
            .post(format!("{}/v1/runs", p.intent.origin))
            .body("foreign prompt"),
        client.get(format!("{}/v1/runs/run_prepared_original", p.intent.origin)),
    ] {
        let response = request
            .bearer_auth("foreign-fixture-token")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
        assert_eq!(
            response.json::<Value>().await.unwrap(),
            json!({"error":"unauthorized"})
        );
    }
    assert_eq!(native.probes.load(Ordering::SeqCst), 0);
    assert_eq!(native.posts.load(Ordering::SeqCst), 0);
    assert!(
        !p.repo
            .get_hermes_dispatch_intent(p.message.id)
            .await
            .unwrap()
            .unwrap()
            .submission_attempted
    );
    p.repo
        .update_agent_status(p.intent.run.agent_id, AgentStatus::Ready)
        .await
        .unwrap();
}

async fn deadline(p: &Prepared) -> Duration {
    let mut after = None;
    let mut pages = 0;
    loop {
        let rows = p.repo.list_prepared_hermes_dispatches(after).await.unwrap();
        let Some((_, last)) = rows.last() else {
            break;
        };
        after = Some(last.run.id);
        pages += 1;
    }
    Duration::from_secs(15 + (pages + 1) * 5)
}

#[tokio::test]
async fn prepared_recovery_fresh_supervisors_send_one_original_request_and_terminal_mirror() {
    let Some(mut p) = setup().await else {
        return;
    };
    assert!(queued(&p).await);
    let body: Value = serde_json::from_str(&p.intent.request_body).unwrap();
    assert_eq!(body["model"], "prepared-model");
    assert_eq!(body["model_options"]["temperature"], 0.25);
    let wait = deadline(&p).await;
    let (native, _server) = server(&mut p, false, false);
    let _one = supervisor(&p, p.config.clone());
    let _two = supervisor(&p, p.config.clone());
    timeout(wait, async {
        loop {
            if p.repo
                .get_session_agent_run(p.intent.run.id)
                .await
                .unwrap()
                .state
                == SessionRunState::Completed
            {
                break;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("prepared intent did not submit after a fresh supervisor");
    let current = p
        .repo
        .get_hermes_dispatch_intent(p.message.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(current.run.id, p.intent.run.id);
    assert_eq!(current.request_body, p.intent.request_body);
    assert_eq!(current.request_hash, p.intent.request_hash);
    assert_eq!(current.recovery_deadline, p.intent.recovery_deadline);
    assert_eq!(current.idempotency_key, p.intent.idempotency_key);
    assert_eq!(
        current.run.runtime_run_id.as_deref(),
        Some("run_prepared_original")
    );
    assert_eq!(
        current.run.runtime_session_id.as_deref(),
        Some("native:prepared")
    );
    assert!(!queued(&p).await);
    let messages = p.repo.list_session_messages(p.session.id).await.unwrap();
    assert_eq!(
        messages
            .iter()
            .filter(|m| m.message_kind == MessageKind::AssistantMessage)
            .count(),
        1
    );
    assert_eq!(native.posts.load(Ordering::SeqCst), 1);
    assert_eq!(native.streams.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn prepared_recovery_uncertain_outbox_claim_has_one_atomic_winner() {
    let Some(p) = setup().await else {
        return;
    };
    p.db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE message_dispatch_outbox SET state='uncertain' WHERE message_id=$1",
        [p.message.id.into()],
    ))
    .await
    .unwrap();
    assert!(queued(&p).await);
    let claim = || {
        p.repo.claim_hermes_submission(
            p.message.id,
            p.intent.origin.clone(),
            p.intent.credential_fingerprint.clone(),
        )
    };
    let (a, b) = timeout(Duration::from_secs(10), async {
        tokio::join!(claim(), claim())
    })
    .await
    .unwrap();
    assert_eq!(
        [a.unwrap(), b.unwrap()]
            .iter()
            .filter(|i| i.is_some())
            .count(),
        1
    );
    let row=p.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT o.state AS outbox_state,j.state AS journal_state FROM message_dispatch_outbox o JOIN hermes_dispatch_journal j ON j.message_id=o.message_id WHERE o.message_id=$1",
        [p.message.id.into()])).await.unwrap().unwrap();
    assert_eq!(
        row.try_get::<String>("", "outbox_state").unwrap(),
        "dispatching"
    );
    assert_eq!(
        row.try_get::<String>("", "journal_state").unwrap(),
        "submitted"
    );
    assert!(!queued(&p).await);
    assert!(claim().await.unwrap().is_none());
}

#[tokio::test]
async fn prepared_recovery_unknown_ack_never_repeats_original_post() {
    let Some(mut p) = setup().await else {
        return;
    };
    let wait = deadline(&p).await;
    let (native, _server) = server(&mut p, true, false);
    let _one = supervisor(&p, p.config.clone());
    timeout(wait, async {
        loop {
            let row =
                p.db.query_one(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    "SELECT state FROM message_dispatch_outbox WHERE message_id=$1",
                    [p.message.id.into()],
                ))
                .await
                .unwrap()
                .unwrap();
            if row.try_get::<String>("", "state").unwrap() == "uncertain" {
                break;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("prepared submission did not persist uncertain acceptance");
    let _two = supervisor(&p, p.config.clone());
    sleep(Duration::from_secs(6)).await;
    let current = p
        .repo
        .get_hermes_dispatch_intent(p.message.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(current.state, "submitted");
    assert_eq!(current.run.state, SessionRunState::Pending);
    assert!(current.run.runtime_run_id.is_none());
    assert!(!queued(&p).await);
    assert_eq!(native.posts.load(Ordering::SeqCst), 1);
    assert!(
        p.repo
            .has_pending_session_dispatch(p.session.id)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn prepared_recovery_stale_credentials_or_capabilities_never_consume_permit() {
    for rotate in [true, false] {
        let Some(mut p) = setup().await else {
            return;
        };
        let wait = deadline(&p).await;
        let (native, _server) = server(&mut p, false, !rotate);
        let mut config = p.config.clone();
        if rotate {
            config.fleet.runtime_token_secret = "rotated-prepared-fixture-only".into();
        }
        let _runtime = supervisor(&p, config);
        if rotate {
            sleep(Duration::from_secs(1)).await;
        } else {
            timeout(wait, async {
                while native.probes.load(Ordering::SeqCst) == 0 {
                    sleep(Duration::from_millis(50)).await;
                }
            })
            .await
            .unwrap();
        }
        let current = p
            .repo
            .get_hermes_dispatch_intent(p.message.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(current.state, "prepared");
        assert!(!current.submission_attempted);
        assert_eq!(current.recovery_deadline, p.intent.recovery_deadline);
        assert_eq!(native.posts.load(Ordering::SeqCst), 0);
        p.repo
            .update_agent_status(p.intent.run.agent_id, AgentStatus::Ready)
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn prepared_recovery_queue_excludes_drain_task_failed_expired_and_submitted_records() {
    for blocker in ["drain", "task", "failed", "expired", "submitted"] {
        let Some(p) = setup().await else {
            return;
        };
        assert!(queued(&p).await);
        match blocker {
            "drain" => {
                p.db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                    "INSERT INTO agent_config_revisions(agent_id,revision,state,snapshot,created_by_user_id) VALUES($1,1,'draft','{}',$2)",
                    [p.intent.run.agent_id.into(),p.owner.into()])).await.unwrap();
                p.db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                    "INSERT INTO agent_config_heads(agent_id,desired_revision,draining) VALUES($1,1,true)",
                    [p.intent.run.agent_id.into()])).await.unwrap();
            }
            "task" => {
                p.db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                    "INSERT INTO task_chat_bindings(session_id,tracker_instance_id,project_id,task_id,root_task_id,agent_id,owner_subject,idempotency_key)
                     VALUES($1,'prepared-test',$2,$3,$3,$4,'prepared-owner','prepared-binding')",
                    [p.session.id.into(),Uuid::new_v4().into(),Uuid::new_v4().into(),p.intent.run.agent_id.into()])).await.unwrap();
            }
            "failed" => {
                p.db.execute(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    "UPDATE session_messages SET delivery_state='failed' WHERE id=$1",
                    [p.message.id.into()],
                ))
                .await
                .unwrap();
            }
            "expired" => {
                let txn = p.db.begin().await.unwrap();
                txn.execute_unprepared("SET LOCAL session_replication_role=replica")
                    .await
                    .unwrap();
                txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                    "WITH ts AS (SELECT clock_timestamp()-interval '2 days' AS value)
                     UPDATE hermes_dispatch_journal SET created_at=ts.value,recovery_deadline=ts.value+interval '86340 seconds' FROM ts WHERE message_id=$1",
                    [p.message.id.into()])).await.unwrap();
                txn.commit().await.unwrap();
            }
            "submitted" => {
                p.repo
                    .claim_hermes_submission(
                        p.message.id,
                        p.intent.origin.clone(),
                        p.intent.credential_fingerprint.clone(),
                    )
                    .await
                    .unwrap()
                    .unwrap();
            }
            _ => unreachable!(),
        }
        assert!(!queued(&p).await, "{blocker}");
        if matches!(blocker, "drain" | "task" | "failed" | "expired") {
            let result = p
                .repo
                .claim_hermes_submission(
                    p.message.id,
                    p.intent.origin.clone(),
                    p.intent.credential_fingerprint.clone(),
                )
                .await;
            assert!(result.is_err() || result.unwrap().is_none(), "{blocker}");
        }
        assert!(
            p.repo
                .get_hermes_dispatch_intent(p.message.id)
                .await
                .unwrap()
                .unwrap()
                .run
                .runtime_run_id
                .is_none()
        );
    }
}
