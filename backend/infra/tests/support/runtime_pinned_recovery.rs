use super::*;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use domain::{MessageDeliveryState, SessionAgentRun};
use sea_orm::DatabaseConnection;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, sync::Mutex};
use tokio::time::timeout;

#[derive(Clone, Copy)]
enum Reply {
    Completed,
    Running,
    NotFound,
    ForeignRun,
    ForeignSession,
    Partial,
    InterruptedSuccess,
    MissingFlags,
    WrongObject,
    WaitingApproval,
    ForeignApproval,
    MissingApproval,
}

struct NativeRun {
    authorization: String,
    effective: String,
    reply: Mutex<Reply>,
    reads: AtomicUsize,
    posts: AtomicUsize,
    streams: AtomicUsize,
    bad_auth: AtomicUsize,
}

impl NativeRun {
    fn authorized(&self, headers: &HeaderMap) -> bool {
        let valid = headers
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            == Some(self.authorization.as_str());
        if !valid {
            self.bad_auth.fetch_add(1, Ordering::SeqCst);
        }
        valid
    }

    fn assert_get_only(&self) {
        assert_eq!(self.posts.load(Ordering::SeqCst), 0, "recovery resent POST");
        assert_eq!(
            self.streams.load(Ordering::SeqCst),
            0,
            "pinned recovery attached SSE"
        );
        assert_eq!(
            self.bad_auth.load(Ordering::SeqCst),
            0,
            "wrong bearer reached runtime"
        );
    }
}

#[derive(Default)]
struct HttpState {
    runs: Mutex<HashMap<String, Arc<NativeRun>>>,
    posts: AtomicUsize,
    approval_caps: std::sync::atomic::AtomicBool,
}

async fn status(
    State(http): State<Arc<HttpState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> (StatusCode, Json<Value>) {
    let native = http.runs.lock().unwrap().get(&id).cloned();
    let Some(native) = native else {
        return (StatusCode::NOT_FOUND, Json(json!({})));
    };
    native.reads.fetch_add(1, Ordering::SeqCst);
    if !native.authorized(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({})));
    }
    assert_eq!(headers.get("accept-encoding").unwrap(), "identity");
    let mut payload = json!({
        "object":"hermes.run", "run_id":id, "session_id":native.effective,
        "status":"completed", "completed":true, "partial":false, "interrupted":false,
        "output":"Recovered pinned reply"
    });
    let reply = *native.reply.lock().unwrap();
    match reply {
        Reply::Completed => {}
        Reply::Running => {
            payload["status"] = json!("running");
            payload["completed"] = json!(false);
            payload["partial"] = json!(true);
        }
        Reply::NotFound => return (StatusCode::NOT_FOUND, Json(json!({}))),
        Reply::ForeignRun => payload["run_id"] = json!("run_foreign_private"),
        Reply::ForeignSession => payload["session_id"] = json!("native:foreign-private-session"),
        Reply::Partial => payload["partial"] = json!(true),
        Reply::InterruptedSuccess => payload["interrupted"] = json!(true),
        Reply::MissingFlags => {
            payload.as_object_mut().unwrap().remove("completed");
        }
        Reply::WrongObject => payload["object"] = json!("foreign.run"),
        Reply::WaitingApproval | Reply::ForeignApproval | Reply::MissingApproval => {
            payload["status"] = json!("waiting_for_approval");
            payload["approval"] = json!({"event":"approval.request","run_id":id,
                "request_id":"request_recovered","description":"Review this exact action",
                "command":format!("echo fc_{}", "a".repeat(64)),"choices":["once","deny"]});
            match reply {
                Reply::ForeignApproval => payload["approval"]["run_id"] = json!("run_foreign"),
                Reply::MissingApproval => payload["approval"] = Value::Null,
                _ => {}
            }
        }
    }
    (StatusCode::OK, Json(payload))
}

async fn events(
    State(http): State<Arc<HttpState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> StatusCode {
    if let Some(native) = http.runs.lock().unwrap().get(&id).cloned() {
        native.streams.fetch_add(1, Ordering::SeqCst);
        native.authorized(&headers);
    }
    // Native status may survive restart while its in-memory event stream does not.
    StatusCode::NOT_FOUND
}

async fn submit(
    State(http): State<Arc<HttpState>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> StatusCode {
    // Count attempted submission before parsing, including malformed JSON or content type.
    http.posts.fetch_add(1, Ordering::SeqCst);
    let payload = serde_json::from_slice::<Value>(&body).unwrap_or(Value::Null);
    let requested = payload.get("session_id").and_then(Value::as_str);
    for native in http.runs.lock().unwrap().values() {
        if headers
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            == Some(native.authorization.as_str())
            || requested == Some(native.effective.as_str())
        {
            native.posts.fetch_add(1, Ordering::SeqCst);
        }
    }
    StatusCode::SERVICE_UNAVAILABLE
}

fn router(http: Arc<HttpState>) -> Router {
    Router::new()
        .route("/health", get(|| async { Json(json!({"status":"ok"})) }))
        .route(
            "/v1/capabilities",
            get(
                |State(http): State<Arc<HttpState>>, headers: HeaderMap| async move {
                    assert!(
                        headers["authorization"]
                            .to_str()
                            .unwrap()
                            .starts_with("Bearer fc_")
                    );
                    Json(if http.approval_caps.load(Ordering::SeqCst) {
                        runtime_targeted_approval::capabilities()
                    } else {
                        hermes_protocol_fixture::capabilities()
                    })
                },
            ),
        )
        .route("/v1/runs", post(submit))
        .route("/v1/runs/{run_id}", get(status))
        .route("/v1/runs/{run_id}/events", get(events))
        .with_state(http)
}

struct Server(tokio::task::JoinHandle<()>);

impl Drop for Server {
    fn drop(&mut self) {
        self.0.abort();
    }
}

struct Fixture {
    repo: Arc<PostgresFleetRepository>,
    db: DatabaseConnection,
    owner: Uuid,
    config: AppConfig,
    port: u16,
    http: Arc<HttpState>,
    _server: Server,
}

struct Pinned {
    agent_id: Uuid,
    session_id: Uuid,
    message_id: Uuid,
    run: SessionAgentRun,
    native_id: String,
    native: Arc<NativeRun>,
    journal: Value,
}

impl Fixture {
    async fn new() -> Option<Self> {
        let (repo, owner, _) = fixture().await?;
        let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
            .await
            .unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let http = Arc::new(HttpState::default());
        let router = router(http.clone());
        let server = Server(tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        }));
        let mut config = AppConfig::default();
        config.fleet.runtime_token_secret = format!("pinned-recovery-fixture-{}", Uuid::new_v4());
        Some(Self {
            repo: Arc::new(repo),
            db,
            owner,
            config,
            port,
            http,
            _server: server,
        })
    }

    async fn sql(&self, sql: &str, values: Vec<sea_orm::Value>) {
        self.db
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                sql,
                values,
            ))
            .await
            .unwrap();
    }

    async fn seed(
        &self,
        reply: Reply,
        state: SessionRunState,
        legacy: bool,
        rotated: bool,
    ) -> Pinned {
        let agent_id = agent(&self.repo).await;
        // Do not expose a queued prompt to another fixture's global dispatcher.
        self.repo
            .update_agent_status(agent_id, AgentStatus::Ready)
            .await
            .unwrap();
        self.sql(
            "UPDATE agents SET api_port=$2 WHERE id=$1",
            vec![agent_id.into(), i32::from(self.port).into()],
        )
        .await;
        let key = Uuid::new_v4().to_string();
        let session = self
            .repo
            .create_session(chat(agent_id, &key), self.owner)
            .await
            .unwrap();
        let message = self
            .repo
            .create_session_message(session.id, prompt(&key), self.owner)
            .await
            .unwrap();
        self.sql(
            "UPDATE message_dispatch_outbox SET state='dispatching' WHERE message_id=$1",
            vec![message.id.into()],
        )
        .await;
        self.repo
            .update_agent_status(agent_id, AgentStatus::Running)
            .await
            .unwrap();
        let mut original = self.config.clone();
        if rotated {
            original.fleet.runtime_token_secret = "original-pinned-recovery-fixture-only".into();
        }
        let token = infra::agent_runtime_token(&original, agent_id).unwrap();
        let requested = format!("fleet:{}:{agent_id}", session.id);
        let run = if legacy {
            self.repo
                .prepare_session_agent_run(
                    session.id,
                    agent_id,
                    SessionRunRole::Primary,
                    requested.clone(),
                )
                .await
                .unwrap()
        } else {
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
            let origin = format!("http://127.0.0.1:{}", self.port);
            self.repo
                .prepare_hermes_dispatch(app::HermesDispatchDraft {
                    message_id: message.id,
                    session_id: session.id,
                    agent_id,
                    run_role: SessionRunRole::Primary,
                    requested_session_id: requested.clone(),
                    input: message.body.clone(),
                    origin: origin.clone(),
                    credential_fingerprint: fingerprint.clone(),
                    capabilities: hermes_protocol_fixture::capabilities(),
                })
                .await
                .unwrap();
            self.repo
                .claim_hermes_submission(message.id, origin, fingerprint)
                .await
                .unwrap()
                .unwrap()
                .run
        };
        let native_id = format!("run_{}", Uuid::new_v4().simple());
        let effective = format!("native:{}", Uuid::new_v4());
        self.repo
            .accept_hermes_run(message.id, run.id, native_id.clone())
            .await
            .unwrap();
        let (run, first) = self
            .repo
            .pin_hermes_run_session(run.id, native_id.clone(), requested, effective.clone())
            .await
            .unwrap();
        assert!(first);
        assert_eq!(run.state, SessionRunState::Running);
        let run = if state != SessionRunState::Running {
            self.repo
                .update_session_agent_run_dispatch(run.id, Some(native_id.clone()), state, None)
                .await
                .unwrap()
        } else {
            run
        };
        assert_eq!(run.state, state);
        let native = Arc::new(NativeRun {
            authorization: format!("Bearer {token}"),
            effective,
            reply: Mutex::new(reply),
            reads: AtomicUsize::new(0),
            posts: AtomicUsize::new(0),
            streams: AtomicUsize::new(0),
            bad_auth: AtomicUsize::new(0),
        });
        self.http
            .runs
            .lock()
            .unwrap()
            .insert(native_id.clone(), native.clone());
        let journal = self.db.query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT COALESCE((SELECT to_jsonb(j) FROM hermes_dispatch_journal j WHERE message_id=$1),'null'::jsonb) AS value",
            [message.id.into()],
        )).await.unwrap().unwrap().try_get("", "value").unwrap();
        Pinned {
            agent_id,
            session_id: session.id,
            message_id: message.id,
            run,
            native_id,
            native,
            journal,
        }
    }

    fn restart(&self) -> infra::runtime::LocalRuntimeSupervisor {
        let (events, _) = tokio::sync::broadcast::channel(32);
        // The pin was committed before any supervisor existed; no first-winner worker survives.
        infra::runtime::LocalRuntimeSupervisor::new(
            Arc::new(self.config.clone()),
            self.repo.clone(),
            events,
        )
    }

    async fn wait_reads(&self, pinned: &[Pinned]) {
        timeout(Duration::from_secs(45), async {
            while pinned
                .iter()
                .any(|p| p.native.reads.load(Ordering::SeqCst) == 0)
            {
                sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .expect("background recovery did not read all pinned runs");
        sleep(Duration::from_millis(100)).await;
    }

    async fn wait_completed(&self, p: &Pinned) {
        timeout(Duration::from_secs(45), async {
            while self
                .repo
                .get_session_agent_run(p.run.id)
                .await
                .unwrap()
                .state
                != SessionRunState::Completed
            {
                sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .expect("known-ID GET did not complete the pinned run");
    }

    async fn terminal_snapshot(&self, p: &Pinned) -> Value {
        self.db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT jsonb_build_object(
                'run',(SELECT to_jsonb(r) FROM session_agent_runs r WHERE r.id=$1),
                'prompt',(SELECT to_jsonb(m) FROM session_messages m WHERE m.id=$2),
                'assistant',(SELECT jsonb_agg(to_jsonb(a) ORDER BY a.id) FROM session_messages a
                    WHERE a.session_id=$3 AND a.message_kind='assistant_message'),
                'cursor',(SELECT sequence FROM session_event_cursors WHERE session_id=$3),
                'events',(SELECT count(*) FROM session_events WHERE session_id=$3)) AS value",
                [p.run.id.into(), p.message_id.into(), p.session_id.into()],
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "value")
            .unwrap()
    }

    async fn assert_persistence(&self, p: &Pinned, completed: bool) {
        let row = self.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT m.delivery_state,o.state AS outbox_state,
                COALESCE((SELECT to_jsonb(j) FROM hermes_dispatch_journal j WHERE j.message_id=m.id),'null'::jsonb) AS journal,
                (SELECT count(*) FROM session_agent_runs r WHERE r.agent_id=$2
                    AND r.state IN ('pending','running','waiting','stopping') AND r.runtime_session_id IS NOT NULL) AS capacity,
                (SELECT count(*) FROM session_messages a WHERE a.session_id=m.session_id AND a.message_kind='assistant_message') AS replies
             FROM session_messages m JOIN message_dispatch_outbox o ON o.message_id=m.id WHERE m.id=$1",
            [p.message_id.into(), p.agent_id.into()])).await.unwrap().unwrap();
        assert_eq!(
            row.try_get::<String>("", "delivery_state").unwrap(),
            if completed { "completed" } else { "dispatched" }
        );
        assert_eq!(
            row.try_get::<String>("", "outbox_state").unwrap(),
            "dispatched"
        );
        assert_eq!(row.try_get::<Value>("", "journal").unwrap(), p.journal);
        assert_eq!(
            row.try_get::<i64>("", "capacity").unwrap(),
            if completed { 0 } else { 1 }
        );
        assert_eq!(
            row.try_get::<i64>("", "replies").unwrap(),
            if completed { 1 } else { 0 }
        );
        let run = self.repo.get_session_agent_run(p.run.id).await.unwrap();
        assert_eq!(run.runtime_run_id.as_deref(), Some(p.native_id.as_str()));
        assert_eq!(
            run.runtime_session_id.as_deref(),
            Some(p.native.effective.as_str())
        );
        if completed {
            assert_eq!(run.state, SessionRunState::Completed);
            let assistant = self.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "SELECT author_type,author_agent_id,author_user_id,delivery_state,delivery_error FROM session_messages
                 WHERE session_id=$1 AND message_kind='assistant_message'",
                [p.session_id.into()])).await.unwrap().unwrap();
            assert_eq!(
                assistant.try_get::<String>("", "author_type").unwrap(),
                "agent"
            );
            assert_eq!(
                assistant
                    .try_get::<Option<Uuid>>("", "author_agent_id")
                    .unwrap(),
                Some(p.agent_id)
            );
            assert_eq!(
                assistant
                    .try_get::<Option<Uuid>>("", "author_user_id")
                    .unwrap(),
                None
            );
            assert_eq!(
                assistant.try_get::<String>("", "delivery_state").unwrap(),
                "mirrored"
            );
            assert_eq!(
                assistant
                    .try_get::<Option<String>>("", "delivery_error")
                    .unwrap(),
                None
            );
            let messages = self.repo.list_session_messages(p.session_id).await.unwrap();
            let assistant = messages
                .iter()
                .find(|m| m.message_kind == MessageKind::AssistantMessage)
                .unwrap();
            assert_eq!(
                assistant.runtime_message_id.as_deref(),
                Some(p.native_id.as_str())
            );
            assert_eq!(assistant.body, "Recovered pinned reply");
            assert_eq!(
                messages
                    .iter()
                    .find(|m| m.id == p.message_id)
                    .unwrap()
                    .delivery_state,
                MessageDeliveryState::Completed
            );
        } else {
            assert!(matches!(
                run.state,
                SessionRunState::Running | SessionRunState::Waiting | SessionRunState::Stopping
            ));
            assert!(
                self.repo
                    .prepare_session_agent_run(
                        p.session_id,
                        p.agent_id,
                        SessionRunRole::Primary,
                        format!("fleet:{}:{}", p.session_id, p.agent_id)
                    )
                    .await
                    .is_err()
            );
        }
        p.native.assert_get_only();
        assert_eq!(self.http.posts.load(Ordering::SeqCst), 0);
    }

    async fn bind_task(&self, p: &Pinned) {
        self.sql(
            "INSERT INTO task_chat_bindings(session_id,tracker_instance_id,project_id,task_id,root_task_id,agent_id,owner_subject,idempotency_key)
             VALUES($1,'pinned-recovery-fixture',$2,$3,$3,$4,$5,$6)",
            vec![p.session_id.into(), Uuid::new_v4().into(), Uuid::new_v4().into(), p.agent_id.into(), self.owner.to_string().into(), Uuid::new_v4().to_string().into()],
        ).await;
    }
}

#[tokio::test]
async fn pinned_recovery_restart_after_pin_completes_by_authenticated_get_without_sse_or_post() {
    let Some(f) = Fixture::new().await else {
        return;
    };
    let p = f
        .seed(Reply::Completed, SessionRunState::Running, false, false)
        .await;
    let _runtime = f.restart();
    f.wait_completed(&p).await;
    assert!(p.native.reads.load(Ordering::SeqCst) > 0);
    f.assert_persistence(&p, true).await;
    // A second fresh supervisor cannot reopen an already committed terminal result.
    let reads = p.native.reads.load(Ordering::SeqCst);
    let snapshot = f.terminal_snapshot(&p).await;
    let _second = f.restart();
    sleep(Duration::from_millis(300)).await;
    f.assert_persistence(&p, true).await;
    assert_eq!(p.native.reads.load(Ordering::SeqCst), reads);
    assert_eq!(f.terminal_snapshot(&p).await, snapshot);
}

#[tokio::test]
async fn pinned_recovery_waiting_stopping_and_draining_still_complete_by_get() {
    let Some(f) = Fixture::new().await else {
        return;
    };
    let mut pinned = Vec::new();
    for state in [
        SessionRunState::Running,
        SessionRunState::Waiting,
        SessionRunState::Stopping,
    ] {
        let p = f.seed(Reply::Completed, state, false, false).await;
        let revision = f
            .repo
            .create_config_revision(
                p.agent_id,
                UpdateAgentConfigRequest {
                    config_json: json!({}),
                    soul_md: "# Pinned recovery drain fixture".into(),
                    env_json: json!({}),
                },
                f.owner,
            )
            .await
            .unwrap();
        assert_eq!(revision.state, "draft");
        f.sql(
            "UPDATE agent_config_heads SET draining=true WHERE agent_id=$1",
            vec![p.agent_id.into()],
        )
        .await;
        assert!(f.repo.agent_is_draining(p.agent_id).await.unwrap());
        pinned.push(p);
    }
    let _runtime = f.restart();
    for p in &pinned {
        f.wait_completed(p).await;
        assert!(p.native.reads.load(Ordering::SeqCst) > 0);
        f.assert_persistence(p, true).await;
        assert!(f.repo.agent_is_draining(p.agent_id).await.unwrap());
    }
}

#[tokio::test]
async fn pinned_recovery_bad_or_nonterminal_readbacks_hold_capacity_until_exact_terminal() {
    let Some(f) = Fixture::new().await else {
        return;
    };
    let mut pinned = Vec::new();
    for reply in [
        Reply::Running,
        Reply::NotFound,
        Reply::ForeignRun,
        Reply::ForeignSession,
        Reply::Partial,
        Reply::InterruptedSuccess,
        Reply::MissingFlags,
        Reply::WrongObject,
    ] {
        pinned.push(f.seed(reply, SessionRunState::Running, false, false).await);
    }
    let _runtime = f.restart();
    f.wait_reads(&pinned).await;
    for p in &pinned {
        f.assert_persistence(p, false).await;
    }
    for p in &pinned {
        *p.native.reply.lock().unwrap() = Reply::Completed;
    }
    for p in &pinned {
        f.wait_completed(p).await;
        f.assert_persistence(p, true).await;
    }
}

#[tokio::test]
async fn pinned_recovery_original_context_legacy_private_other_and_task_pm_fail_before_http() {
    let Some(f) = Fixture::new().await else {
        return;
    };
    let mut held = Vec::new();
    for mode in ["rotated", "moved", "legacy", "private-other", "task", "pm"] {
        let p = f
            .seed(
                Reply::Completed,
                SessionRunState::Running,
                mode == "legacy",
                mode == "rotated",
            )
            .await;
        match mode {
            "moved" => {
                // Change the persisted origin while keeping a real listener at that new origin.
                // Preparing at an old port would otherwise prove only a connection failure.
                let other_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                let other_port = other_listener.local_addr().unwrap().port();
                let router = router(f.http.clone());
                let server = Server(tokio::spawn(async move {
                    axum::serve(other_listener, router).await.unwrap();
                }));
                f.sql(
                    "UPDATE agents SET api_port=$2 WHERE id=$1",
                    vec![p.agent_id.into(), i32::from(other_port).into()],
                )
                .await;
                held.push((p, Some(server)));
                continue;
            }
            "private-other" => {
                let other_agent = agent(&f.repo).await;
                f.sql(
                    "UPDATE agent_sessions SET agent_id=$2 WHERE id=$1",
                    vec![p.session_id.into(), other_agent.into()],
                )
                .await;
            }
            "task" | "pm" => {
                f.bind_task(&p).await;
                if mode == "pm" {
                    f.sql(
                        "INSERT INTO pm_run_bindings(session_run_id,session_id,agent_id,reservation,dispatch_operation_key,runtime_session_id)
                         VALUES($1,$2,$3,'{}'::jsonb,$4,$5)",
                        vec![p.run.id.into(), p.session_id.into(), p.agent_id.into(), Uuid::new_v4().to_string().into(), p.native.effective.clone().into()],
                    ).await;
                }
            }
            _ => {}
        }
        held.push((p, None));
    }
    // A positive control proves the background readback loop actually executed.
    let valid = f
        .seed(Reply::Completed, SessionRunState::Running, false, false)
        .await;
    let _runtime = f.restart();
    f.wait_completed(&valid).await;
    f.assert_persistence(&valid, true).await;
    sleep(Duration::from_secs(6)).await;
    for (p, _) in &held {
        assert_eq!(
            p.native.reads.load(Ordering::SeqCst),
            0,
            "denied context reached status HTTP"
        );
        f.assert_persistence(p, false).await;
    }
}

#[tokio::test]
async fn pinned_recovery_keyset_reaches_valid_terminal_after_two_hundred_invalid_pinned_runs() {
    let Some(f) = Fixture::new().await else {
        return;
    };
    let mut pinned = Vec::new();
    for _ in 0..202 {
        pinned.push(
            f.seed(Reply::Partial, SessionRunState::Running, false, false)
                .await,
        );
    }
    pinned.sort_by_key(|p| p.run.id);
    let valid = pinned.pop().unwrap();
    *valid.native.reply.lock().unwrap() = Reply::Completed;
    let _runtime = f.restart();
    f.wait_completed(&valid).await;
    f.assert_persistence(&valid, true).await;
    assert!(
        pinned
            .iter()
            .all(|p| p.native.reads.load(Ordering::SeqCst) > 0),
        "later page skipped earlier invalid pinned runs"
    );
    for p in &pinned {
        f.assert_persistence(p, false).await;
    }
}

#[tokio::test]
async fn pinned_approval_recovery_concurrent_supervisors_restore_once_by_get() {
    let Some(f) = Fixture::new().await else {
        return;
    };
    f.http.approval_caps.store(true, Ordering::SeqCst);
    let p = f
        .seed(
            Reply::WaitingApproval,
            SessionRunState::Running,
            false,
            false,
        )
        .await;
    let transcript_before =
        serde_json::to_value(f.repo.list_session_messages(p.session_id).await.unwrap()).unwrap();
    let _one = f.restart();
    let _two = f.restart();
    timeout(Duration::from_secs(45), async {
        loop {
            let rows = f.repo.list_session_approvals(p.session_id).await.unwrap();
            let run = f.repo.get_session_agent_run(p.run.id).await.unwrap();
            if rows.len() == 1 && run.state == SessionRunState::Waiting {
                break;
            }
            sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("pending snapshot was not recovered");
    let before = f
        .repo
        .list_session_approvals(p.session_id)
        .await
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(
        before.runtime_approval_id.as_deref(),
        Some("request_recovered")
    );
    assert!(!before.detail.to_string().contains(&"a".repeat(64)));
    let cursor = f.repo.session_event_cursor(p.session_id).await.unwrap();
    let reads = p.native.reads.load(Ordering::SeqCst);
    // Recovery scans retained invalid pins in bounded 20-row keyset pages.
    timeout(Duration::from_secs(45), async {
        while p.native.reads.load(Ordering::SeqCst) < reads + 2 {
            sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .unwrap();
    sleep(Duration::from_millis(100)).await;
    let after = f.repo.list_session_approvals(p.session_id).await.unwrap();
    assert_eq!(serde_json::to_value(&after).unwrap(), json!([before]));
    assert_eq!(
        f.repo.session_event_cursor(p.session_id).await.unwrap(),
        cursor
    );
    assert_eq!(
        serde_json::to_value(f.repo.list_session_messages(p.session_id).await.unwrap()).unwrap(),
        transcript_before
    );
    f.assert_persistence(&p, false).await;
}

fn recovery_request(p: &Pinned) -> app::RuntimeApprovalCreate {
    app::RuntimeApprovalCreate {
        session_id: p.session_id,
        session_run_id: p.run.id,
        agent_id: p.agent_id,
        runtime_run_id: p.native_id.clone(),
        runtime_approval_id: Some("request_recovered".into()),
        prompt: "Review this exact action".into(),
        detail: json!({"event":"approval.request",
            "run_id":p.native_id,"request_id":"request_recovered","description":"Review this exact action",
            "choices":["once","deny"]}),
    }
}

async fn record(
    f: &Fixture,
    p: &Pinned,
    req: app::RuntimeApprovalCreate,
) -> Result<(domain::RuntimeApprovalRequest, bool), shared::AppError> {
    let intent = f
        .repo
        .get_hermes_dispatch_intent(p.message_id)
        .await
        .unwrap()
        .unwrap();
    f.repo
        .recover_hermes_approval(
            req,
            p.native.effective.clone(),
            intent.origin,
            intent.credential_fingerprint,
        )
        .await
}

#[tokio::test]
async fn pinned_approval_recovery_rolls_back_request_and_waiting_when_event_commit_fails() {
    let Some(f) = Fixture::new().await else {
        return;
    };
    let p = f
        .seed(Reply::Running, SessionRunState::Running, false, false)
        .await;
    let name = format!("approval_recovery_fail_{}", p.run.id.simple());
    f.sql(&format!("CREATE FUNCTION {name}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
        IF NEW.id='{id}'::uuid AND NEW.state='waiting' THEN RAISE EXCEPTION 'owned approval recovery failure';
        END IF; RETURN NEW; END $$",id=p.run.id),vec![]).await;
    f.sql(&format!("CREATE TRIGGER {name} BEFORE UPDATE ON session_agent_runs FOR EACH ROW EXECUTE FUNCTION {name}()"),vec![]).await;
    let cursor = f.repo.session_event_cursor(p.session_id).await.unwrap();
    assert!(record(&f, &p, recovery_request(&p)).await.is_err());
    assert!(
        f.repo
            .list_session_approvals(p.session_id)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        f.repo.session_event_cursor(p.session_id).await.unwrap(),
        cursor
    );
    assert_eq!(
        f.repo.get_session_agent_run(p.run.id).await.unwrap().state,
        SessionRunState::Running
    );
    f.sql(
        &format!("DROP TRIGGER {name} ON session_agent_runs"),
        vec![],
    )
    .await;
    f.sql(&format!("DROP FUNCTION {name}()"), vec![]).await;
    assert!(record(&f, &p, recovery_request(&p)).await.unwrap().1);
    assert!(!record(&f, &p, recovery_request(&p)).await.unwrap().1);
    f.assert_persistence(&p, false).await;
}

#[tokio::test]
async fn pinned_approval_recovery_preserves_stop_and_rejects_changed_snapshot_or_context() {
    let Some(f) = Fixture::new().await else {
        return;
    };
    let p = f
        .seed(Reply::Running, SessionRunState::Stopping, false, false)
        .await;
    assert!(record(&f, &p, recovery_request(&p)).await.unwrap().1);
    let cursor = f.repo.session_event_cursor(p.session_id).await.unwrap();
    assert_eq!(
        f.repo.get_session_agent_run(p.run.id).await.unwrap().state,
        SessionRunState::Stopping
    );
    let mut changed = recovery_request(&p);
    changed.prompt = "Another action".into();
    assert!(record(&f, &p, changed).await.is_err());
    let intent = f
        .repo
        .get_hermes_dispatch_intent(p.message_id)
        .await
        .unwrap()
        .unwrap();
    for (session, origin, fp) in [
        (
            "foreign".into(),
            intent.origin.clone(),
            intent.credential_fingerprint.clone(),
        ),
        (
            p.native.effective.clone(),
            "http://127.0.0.1:1".into(),
            intent.credential_fingerprint.clone(),
        ),
        (
            p.native.effective.clone(),
            intent.origin.clone(),
            "b".repeat(64),
        ),
    ] {
        assert!(
            f.repo
                .recover_hermes_approval(recovery_request(&p), session, origin, fp)
                .await
                .is_err()
        );
    }
    assert_eq!(
        f.repo.session_event_cursor(p.session_id).await.unwrap(),
        cursor
    );
    assert_eq!(
        f.repo
            .list_session_approvals(p.session_id)
            .await
            .unwrap()
            .len(),
        1
    );
    f.assert_persistence(&p, false).await;
}

#[tokio::test]
async fn pinned_approval_recovery_missing_capability_or_foreign_snapshot_retains_capacity() {
    let Some(f) = Fixture::new().await else {
        return;
    };
    let pending = f
        .seed(
            Reply::WaitingApproval,
            SessionRunState::Running,
            false,
            false,
        )
        .await;
    let _runtime = f.restart();
    f.wait_reads(std::slice::from_ref(&pending)).await;
    assert!(
        f.repo
            .list_session_approvals(pending.session_id)
            .await
            .unwrap()
            .is_empty()
    );
    f.http.approval_caps.store(true, Ordering::SeqCst);
    let foreign = f
        .seed(
            Reply::ForeignApproval,
            SessionRunState::Running,
            false,
            false,
        )
        .await;
    let missing = f
        .seed(
            Reply::MissingApproval,
            SessionRunState::Running,
            false,
            false,
        )
        .await;
    let invalid = [foreign, missing];
    f.wait_reads(&invalid).await;
    for p in &invalid {
        assert!(
            f.repo
                .list_session_approvals(p.session_id)
                .await
                .unwrap()
                .is_empty()
        );
        f.assert_persistence(p, false).await;
    }
    // Pending now has a valid capability; it may recover, but malformed records may not.
    for session in f
        .repo
        .list_session_agent_runs(pending.session_id)
        .await
        .unwrap()
    {
        assert_eq!(session.id, pending.run.id);
    }
    assert_eq!(f.http.posts.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn pinned_approval_recovery_never_reopens_a_resolved_request() {
    let Some(f) = Fixture::new().await else {
        return;
    };
    for state in ["approved", "denied", "cancelled"] {
        let p = f
            .seed(Reply::Running, SessionRunState::Running, false, false)
            .await;
        let (approval, _) = record(&f, &p, recovery_request(&p)).await.unwrap();
        f.sql("UPDATE runtime_approval_requests SET state=$2,resolved_at=clock_timestamp(),resolved_by_user_id=$3 WHERE id=$1",
            vec![approval.id.into(), state.into(), f.owner.into()]).await;
        f.sql(
            "UPDATE session_agent_runs SET state='running' WHERE id=$1",
            vec![p.run.id.into()],
        )
        .await;
        let before = f.repo.list_session_approvals(p.session_id).await.unwrap();
        let cursor = f.repo.session_event_cursor(p.session_id).await.unwrap();
        let (replayed, created) = record(&f, &p, recovery_request(&p)).await.unwrap();
        assert!(!created);
        assert_eq!(
            serde_json::to_value(&replayed).unwrap(),
            serde_json::to_value(&before[0]).unwrap()
        );
        assert_eq!(
            f.repo.get_session_agent_run(p.run.id).await.unwrap().state,
            SessionRunState::Running
        );
        assert_eq!(
            f.repo.session_event_cursor(p.session_id).await.unwrap(),
            cursor
        );
        f.assert_persistence(&p, false).await;
    }
}

#[tokio::test]
async fn pinned_approval_recovery_denies_terminal_legacy_task_pm_and_archived_contexts() {
    let Some(f) = Fixture::new().await else {
        return;
    };
    for mode in [
        "terminal",
        "legacy",
        "task",
        "pm",
        "archived",
        "other-primary",
    ] {
        let p = f
            .seed(
                Reply::Running,
                SessionRunState::Running,
                mode == "legacy",
                false,
            )
            .await;
        match mode {
            "terminal" => {
                f.sql(
                    "UPDATE session_agent_runs SET state='completed' WHERE id=$1",
                    vec![p.run.id.into()],
                )
                .await
            }
            "task" => f.bind_task(&p).await,
            "pm" => {
                f.bind_task(&p).await;
                f.sql("INSERT INTO pm_run_bindings(session_run_id,session_id,agent_id,reservation,dispatch_operation_key,runtime_session_id)
                 VALUES($1,$2,$3,'{}'::jsonb,$4,$5)",
                vec![p.run.id.into(),p.session_id.into(),p.agent_id.into(),Uuid::new_v4().to_string().into(),p.native.effective.clone().into()]).await;
            }
            "archived" => {
                f.sql(
                    "UPDATE agents SET archived_at=clock_timestamp() WHERE id=$1",
                    vec![p.agent_id.into()],
                )
                .await
            }
            "other-primary" => {
                let other = agent(&f.repo).await;
                f.sql(
                    "UPDATE agent_sessions SET agent_id=$2 WHERE id=$1",
                    vec![p.session_id.into(), other.into()],
                )
                .await;
            }
            _ => {}
        }
        let cursor = f.repo.session_event_cursor(p.session_id).await.unwrap();
        let before = f.repo.get_session_agent_run(p.run.id).await.unwrap();
        let result = if mode == "legacy" {
            f.repo
                .recover_hermes_approval(
                    recovery_request(&p),
                    p.native.effective.clone(),
                    format!("http://127.0.0.1:{}", f.port),
                    "a".repeat(64),
                )
                .await
        } else {
            record(&f, &p, recovery_request(&p)).await
        };
        assert!(result.is_err(), "{mode}");
        assert!(
            f.repo
                .list_session_approvals(p.session_id)
                .await
                .unwrap()
                .is_empty(),
            "{mode}"
        );
        assert_eq!(
            f.repo.session_event_cursor(p.session_id).await.unwrap(),
            cursor,
            "{mode}"
        );
        assert_eq!(
            serde_json::to_value(f.repo.get_session_agent_run(p.run.id).await.unwrap()).unwrap(),
            serde_json::to_value(before).unwrap(),
            "{mode}"
        );
        p.native.assert_get_only();
    }
}
