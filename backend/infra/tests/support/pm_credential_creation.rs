use super::*;
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use domain::{PmCredentialCommand, PmDraftOperation};
use infra::pm_credentials::PmCredentialCoordinator;
use migration::MigratorTrait;
use serde_json::{Value, json};
use tokio::sync::Mutex;

const PARENT: &str = "sdlc_pat_parent-test-secret-1234567890";
const CHILD: &str = "sdlc_pat_child-test-secret-1234567890";

struct Remote {
    repo: Arc<PostgresFleetRepository>,
    operation: PmDraftOperation,
    subject: String,
    scopes: Vec<String>,
    receipt: Mutex<Option<Value>>,
    posts: AtomicUsize,
    children: AtomicUsize,
    contexts: AtomicUsize,
    mode: AtomicUsize,
    lease_mode: AtomicUsize,
    leases: AtomicUsize,
    lease_posts: AtomicUsize,
    lease_queries: Mutex<Vec<Option<String>>>,
}

async fn introspection(
    State(remote): State<Arc<Remote>>,
    headers: HeaderMap,
) -> (StatusCode, Json<Value>) {
    let root = headers["authorization"] == format!("Bearer {PARENT}");
    if root && remote.mode.load(Ordering::SeqCst) == 8 {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error":"revoked parent detail"})),
        );
    }
    if !root {
        assert_eq!(headers["authorization"], format!("Bearer {CHILD}"));
        if remote.mode.load(Ordering::SeqCst) == 4 {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({"error":"revoked secret detail"})),
            );
        }
    }
    (
        StatusCode::OK,
        Json(json!({"sub":remote.subject,"email":"machine@example.test",
        "scopes":if root {vec!["task-tracker:read".to_string(),"task-tracker:write".to_string()]} else {remote.scopes.clone()}})),
    )
}

async fn delegate(
    State(remote): State<Arc<Remote>>,
    headers: HeaderMap,
    Json(command): Json<Value>,
) -> (StatusCode, HeaderMap, Json<Value>) {
    assert_eq!(headers["authorization"], format!("Bearer {PARENT}"));
    let saved = remote
        .repo
        .read_pm_draft_operation(remote.operation.id, remote.operation.owner_user_id)
        .await
        .unwrap();
    let journal = saved.credentials.unwrap();
    assert_eq!(journal.intent.command, command);
    assert_eq!(
        journal.intent.request_sha256,
        domain::pm_canonical_hash(&command)
    );
    assert_eq!(journal.intent.parent_fingerprint.len(), 64);
    assert_eq!(
        command["idempotency_key"],
        remote.operation.credential_key()
    );
    remote.posts.fetch_add(1, Ordering::SeqCst);
    let mut receipt = remote.receipt.lock().await;
    let replay = receipt.is_some();
    let value = receipt
        .get_or_insert_with(|| {
            remote.children.fetch_add(1, Ordering::SeqCst);
            json!({"secret":CHILD,"token_id":Uuid::new_v4(),"scopes":remote.scopes,
            "expires_at":chrono::Utc::now()+chrono::Duration::seconds(command["expires_in_seconds"].as_i64().unwrap())})
        })
        .clone();
    let mut headers = HeaderMap::new();
    headers.insert("cache-control", "no-store".parse().unwrap());
    if remote.mode.load(Ordering::SeqCst) == 1 {
        return (
            StatusCode::BAD_GATEWAY,
            headers,
            Json(json!({"error":"lost acknowledgement"})),
        );
    }
    (
        if replay {
            StatusCode::OK
        } else {
            StatusCode::CREATED
        },
        headers,
        Json(value),
    )
}

async fn context(
    State(remote): State<Arc<Remote>>,
    headers: HeaderMap,
) -> (StatusCode, Json<Value>) {
    assert_eq!(headers["authorization"], format!("Bearer {CHILD}"));
    remote.contexts.fetch_add(1, Ordering::SeqCst);
    let mode = remote.mode.load(Ordering::SeqCst);
    if mode == 2 {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error":"private Tracker detail"})),
        );
    }
    let op = &remote.operation;
    let identity = op.identity().unwrap();
    let mut value = json!({"contract_version":1,"tracker_instance_id":identity.tracker_instance_id,
        "project_id":identity.project_id,"task_id":identity.task_id,"root_task_id":identity.root_task_id,
        "owner_subject":identity.owner_subject,"stage":"Draft","requirement_revision":null,"waiting_reason":null,
        "permissions":{"can_answer":false,"can_confirm":false},"assignment":op.reservation.as_ref().unwrap().assignment});
    match mode {
        3 => value["assignment"]["assignment_id"] = json!(Uuid::new_v4()),
        5 => value["owner_subject"] = json!(remote.subject),
        6 => {
            value["assignment"]["agent_id"] = json!(op.request.agent_id.to_string().to_uppercase())
        }
        7 => value["permissions"]["can_confirm"] = json!(true),
        _ => (),
    }
    (StatusCode::OK, Json(value))
}

struct Fixture {
    remote: Arc<Remote>,
    config: AppConfig,
    server: tokio::task::JoinHandle<()>,
}

async fn lease_readback(
    State(remote): State<Arc<Remote>>,
    headers: HeaderMap,
    axum::extract::Query(query): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    assert_eq!(headers["authorization"], format!("Bearer {CHILD}"));
    assert_eq!(headers["cache-control"], "no-cache, no-store");
    assert_eq!(headers["accept-encoding"], "identity");
    remote.leases.fetch_add(1, Ordering::SeqCst);
    remote
        .lease_queries
        .lock()
        .await
        .push(query.get("idempotency_key").cloned());
    let mode = remote.lease_mode.load(Ordering::SeqCst);
    let reservation = remote.operation.reservation.as_ref().unwrap();
    let now = chrono::Utc::now();
    let nanos = |value: chrono::DateTime<chrono::Utc>| {
        value.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
    };
    let mut value = json!({"contract_version":1,"binding":reservation.binding,
        "owner_version":reservation.owner_cas.version,"fence":{
            "assignment_id":reservation.assignment.assignment_id,
            "execution_id":reservation.assignment.execution_id,
            "agent_id":reservation.assignment.agent_id,
            "assignment_version":reservation.assignment.version},
        "observed_at":nanos(now),"state":"unclaimed","current":null,"operation":null,
        "dispatch_allowed":false});
    match mode {
        1 => value["binding"]["owner_subject"] = json!(Uuid::new_v4()),
        2 => value["dispatch_allowed"] = json!(true),
        3 => {
            value.as_object_mut().unwrap().remove("current");
        }
        4 => value["fence"]["execution_id"] = json!(Uuid::new_v4()),
        5 | 6 => {
            let heartbeat = if mode == 5 {
                now
            } else {
                now - chrono::Duration::seconds(31)
            };
            value["state"] = json!(if mode == 5 { "active" } else { "expired" });
            value["current"] = json!({"lease_id":Uuid::new_v4(),"version":1,
                "holder_subject":remote.subject,"claimed_at":nanos(heartbeat),
                "heartbeat_at":nanos(heartbeat),
                "expires_at":nanos(heartbeat + chrono::Duration::seconds(30))});
        }
        7 => return (StatusCode::FORBIDDEN, Json(json!({"secret":CHILD}))).into_response(),
        8 => {
            return (
                StatusCode::TEMPORARY_REDIRECT,
                [("location", "/auth/tokens/delegate")],
            )
                .into_response();
        }
        9 => value["oversized"] = json!("x".repeat(16_385)),
        10 => return ([("content-encoding", "gzip")], Json(value)).into_response(),
        11 => tokio::time::sleep(Duration::from_secs(6)).await,
        12 => value["owner_version"] = json!(1.0),
        13 | 14 => {
            let claimed = now - chrono::Duration::seconds(20);
            let lease_id = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
            value["state"] = json!("active");
            value["current"] = json!({"lease_id":lease_id,"version":3,
                "holder_subject":remote.subject,"claimed_at":nanos(claimed),
                "heartbeat_at":nanos(now),"expires_at":nanos(now + chrono::Duration::seconds(30))});
            let command = lease_claim(&remote);
            let mut historical = value["current"].clone();
            historical["version"] = json!(1);
            historical["heartbeat_at"] = json!(nanos(claimed));
            historical["expires_at"] = json!(nanos(claimed + chrono::Duration::seconds(30)));
            value["operation"] = json!({"idempotency_key":command.idempotency_key(),
                "request_sha256":command.request_sha256(),"result":{
                    "contract_version":1,"binding":reservation.binding,
                    "owner_version":reservation.owner_cas.version,"fence":value["fence"],
                    "lease":historical,"ttl_seconds":30,"heartbeat_seconds":10,"dispatch_allowed":false}});
            if mode == 14 {
                value["operation"]["request_sha256"] = json!("a".repeat(64));
            }
        }
        _ => (),
    }
    Json(value).into_response()
}

fn lease_claim(remote: &Remote) -> domain::PmExecutionLeaseCommand {
    let reservation = remote.operation.reservation.as_ref().unwrap();
    domain::PmExecutionLeaseCommand::Claim(domain::PmExecutionLeaseClaim {
        expected_owner_version: reservation.owner_cas.version,
        fence: domain::PmExecutionLeaseFence {
            assignment_id: reservation.assignment.assignment_id,
            execution_id: reservation.assignment.execution_id,
            agent_id: reservation.assignment.agent_id,
            assignment_version: reservation.assignment.version,
        },
        idempotency_key: "original-lease-claim".into(),
    })
}

async fn lease_mutation(State(remote): State<Arc<Remote>>) -> StatusCode {
    remote.lease_posts.fetch_add(1, Ordering::SeqCst);
    StatusCode::FORBIDDEN
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}

async fn fixture() -> Option<Fixture> {
    let (repo, operation, subject) = super::pm_draft_creation::credential_fixture().await?;
    let scopes = PmCredentialCommand::tracker(
        &operation.execution_identity().unwrap(),
        operation.credential_key(),
        300,
    )
    .unwrap()
    .scopes()
    .to_vec();
    let remote = Arc::new(Remote {
        repo: Arc::new(repo),
        operation,
        subject: subject.clone(),
        scopes,
        receipt: Mutex::new(None),
        posts: AtomicUsize::new(0),
        children: AtomicUsize::new(0),
        contexts: AtomicUsize::new(0),
        mode: AtomicUsize::new(0),
        lease_mode: AtomicUsize::new(0),
        leases: AtomicUsize::new(0),
        lease_posts: AtomicUsize::new(0),
        lease_queries: Mutex::new(Vec::new()),
    });
    let router = Router::new()
        .route("/auth/tokens/introspect", get(introspection))
        .route("/auth/tokens/delegate", post(delegate))
        .route(
            &format!(
                "/api/v1/issues/{}/sdlc/context",
                remote.operation.identity().unwrap().task_id
            ),
            get(context),
        )
        .route(
            &format!(
                "/api/v1/issues/{}/sdlc/pm-draft-execution-lease",
                remote.operation.identity().unwrap().task_id
            ),
            get(lease_readback).post(lease_mutation),
        )
        .with_state(remote.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}/", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut config = AppConfig::default();
    config.tracker.url = origin.clone();
    config.pm.credentials = shared::PmCredentialsConfig {
        enabled: true,
        auth_url: origin,
        machine_subject: subject,
        parent_pat: PARENT.into(),
        ttl_seconds: 300,
    };
    Some(Fixture {
        remote,
        config,
        server,
    })
}

impl Fixture {
    fn coordinator(&self) -> PmCredentialCoordinator {
        PmCredentialCoordinator::configured(&self.config)
            .unwrap()
            .unwrap()
    }
    async fn operation(&self) -> PmDraftOperation {
        self.remote
            .repo
            .read_pm_draft_operation(
                self.remote.operation.id,
                self.remote.operation.owner_user_id,
            )
            .await
            .unwrap()
    }
    async fn prepare(&self) -> Result<(), shared::AppError> {
        self.coordinator()
            .prepare_credential(self.remote.repo.as_ref(), &self.operation().await)
            .await
            .map(|_| ())
    }
}

#[tokio::test]
async fn pm_credentials_pg_lease_readback_holds_foreign_malformed_claimed_and_expired_without_dispatch()
 {
    let Some(fixture) = fixture().await else {
        return;
    };
    fixture.prepare().await.unwrap();
    let saved = fixture.operation().await;
    assert_eq!(fixture.remote.leases.load(Ordering::SeqCst), 1);
    for mode in [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12] {
        fixture.remote.lease_mode.store(mode, Ordering::SeqCst);
        let error = fixture.prepare().await.unwrap_err();
        assert!(!error.to_string().contains(CHILD), "mode={mode}");
        assert_eq!(saved.credentials, fixture.operation().await.credentials);
        assert!(
            fixture
                .remote
                .repo
                .list_session_agent_runs(saved.session_id.unwrap())
                .await
                .unwrap()
                .is_empty()
        );
    }
    fixture.remote.lease_mode.store(0, Ordering::SeqCst);
    fixture.prepare().await.unwrap();
    assert_eq!(fixture.remote.children.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.remote.lease_posts.load(Ordering::SeqCst), 0);
    assert_eq!(saved.credentials, fixture.operation().await.credentials);
    assert!(!saved.response().dispatch_allowed);
}

#[tokio::test]
async fn pm_credentials_pg_lease_original_key_readback_preserves_history_without_claim_or_renewal()
{
    let Some(fixture) = fixture().await else {
        return;
    };
    let coordinator = PmCredentialCoordinator::configured(&fixture.config)
        .unwrap()
        .unwrap();
    let credential = coordinator
        .prepare_credential(fixture.remote.repo.as_ref(), &fixture.operation().await)
        .await
        .unwrap();
    let saved = fixture.operation().await;
    let command = lease_claim(&fixture.remote);
    fixture.remote.lease_mode.store(13, Ordering::SeqCst);
    let readback = coordinator
        .read_execution_lease(&saved, &credential, Some(&command))
        .await
        .unwrap();
    assert_eq!(readback.current.unwrap().version, 3);
    assert_eq!(readback.operation.unwrap().result.lease.version, 1);
    assert!(!readback.dispatch_allowed);
    fixture.remote.lease_mode.store(14, Ordering::SeqCst);
    assert!(matches!(
        coordinator
            .read_execution_lease(&saved, &credential, Some(&command))
            .await,
        Err(shared::AppError::Conflict(_))
    ));
    fixture.remote.lease_mode.store(13, Ordering::SeqCst);
    assert!(
        coordinator
            .read_execution_lease(&saved, &credential, None)
            .await
            .is_err()
    );
    assert_eq!(
        *fixture.remote.lease_queries.lock().await,
        vec![
            None,
            Some(command.idempotency_key().into()),
            Some(command.idempotency_key().into()),
            None
        ]
    );
    assert_eq!(fixture.remote.posts.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.remote.lease_posts.load(Ordering::SeqCst), 0);
    assert_eq!(saved.credentials, fixture.operation().await.credentials);
    assert!(
        fixture
            .remote
            .repo
            .list_session_agent_runs(saved.session_id.unwrap())
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn pm_credentials_pg_direct_lease_readback_rejects_changed_machine_before_http() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let coordinator = PmCredentialCoordinator::configured(&fixture.config)
        .unwrap()
        .unwrap();
    let credential = coordinator
        .prepare_credential(fixture.remote.repo.as_ref(), &fixture.operation().await)
        .await
        .unwrap();
    let mut changed = fixture.operation().await;
    changed
        .reservation
        .as_mut()
        .unwrap()
        .assignment
        .machine_subject = Uuid::new_v4().to_string();
    assert!(matches!(
        coordinator
            .read_execution_lease(&changed, &credential, None)
            .await,
        Err(shared::AppError::Conflict(_))
    ));
    assert_eq!(fixture.remote.leases.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.remote.contexts.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.remote.lease_posts.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn pm_credentials_pg_lease_readback_timeout_preserves_ack_and_does_not_claim_or_retry() {
    let Some(fixture) = fixture().await else {
        return;
    };
    fixture.remote.lease_mode.store(11, Ordering::SeqCst);
    let error = fixture.prepare().await.unwrap_err();
    assert!(matches!(error, shared::AppError::Unavailable(_)));
    let saved = fixture.operation().await;
    assert!(saved.credentials.as_ref().unwrap().receipt.is_some());
    assert_eq!(fixture.remote.leases.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.remote.lease_posts.load(Ordering::SeqCst), 0);
    assert!(
        fixture
            .remote
            .repo
            .list_session_agent_runs(saved.session_id.unwrap())
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn pm_credentials_pg_lost_ack_restart_replays_exact_child_without_run_or_secret() {
    let Some(fixture) = fixture().await else {
        return;
    };
    fixture.remote.mode.store(1, Ordering::SeqCst);
    assert!(matches!(
        super::pm_draft_creation::continue_with_credentials(
            fixture.remote.repo.as_ref(),
            fixture.operation().await,
            &fixture.coordinator(),
        )
        .await,
        Err(shared::AppError::Unavailable(_))
    ));
    let pending = fixture.operation().await;
    assert!(pending.credentials.as_ref().unwrap().receipt.is_none());
    assert_eq!(fixture.remote.children.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.remote.contexts.load(Ordering::SeqCst), 0);
    fixture.remote.mode.store(0, Ordering::SeqCst);
    let response = super::pm_draft_creation::continue_with_credentials(
        fixture.remote.repo.as_ref(),
        fixture.operation().await,
        &fixture.coordinator(),
    )
    .await
    .unwrap();
    assert_eq!(response.session_id, fixture.remote.operation.session_id);
    let saved = fixture.operation().await;
    fixture.prepare().await.unwrap();
    assert_eq!(saved.credentials, fixture.operation().await.credentials);
    let encoded = serde_json::to_string(&saved).unwrap();
    assert!(!encoded.contains(PARENT) && !encoded.contains(CHILD));
    assert!(
        !serde_json::to_string(&saved.response())
            .unwrap()
            .contains("credentials")
    );
    assert_eq!(fixture.remote.children.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.remote.posts.load(Ordering::SeqCst), 3);
    assert_eq!(fixture.remote.contexts.load(Ordering::SeqCst), 2);
    assert!(
        fixture
            .remote
            .repo
            .list_session_agent_runs(saved.session_id.unwrap())
            .await
            .unwrap()
            .is_empty()
    );
    assert!(!saved.response().dispatch_allowed);
}

#[tokio::test]
async fn pm_credentials_pg_partial_success_keeps_receipt_and_checks_current_context_on_replay() {
    let Some(fixture) = fixture().await else {
        return;
    };
    fixture.remote.mode.store(2, Ordering::SeqCst);
    let error = fixture.prepare().await.unwrap_err();
    assert!(!error.to_string().contains("private Tracker detail"));
    let saved = fixture.operation().await;
    assert!(saved.credentials.as_ref().unwrap().receipt.is_some());
    for mode in [3, 5, 6, 7] {
        fixture.remote.mode.store(mode, Ordering::SeqCst);
        assert!(fixture.prepare().await.is_err(), "mode={mode}");
        assert_eq!(saved.credentials, fixture.operation().await.credentials);
    }
    fixture.remote.mode.store(0, Ordering::SeqCst);
    fixture.prepare().await.unwrap();
    assert_eq!(fixture.remote.children.load(Ordering::SeqCst), 1);
    assert_eq!(saved.credentials, fixture.operation().await.credentials);
}

#[tokio::test]
async fn pm_credentials_pg_rotated_parent_origins_or_ttl_conflict_before_external_request() {
    let Some(fixture) = fixture().await else {
        return;
    };
    fixture.prepare().await.unwrap();
    let saved = fixture.operation().await;
    for kind in 0..4 {
        let mut config = fixture.config.clone();
        match kind {
            0 => {
                config.pm.credentials.parent_pat = "sdlc_pat_rotated-test-secret-1234567890".into()
            }
            1 => config.pm.credentials.auth_url = "http://127.0.0.1:1/".into(),
            2 => config.tracker.url = "http://127.0.0.1:1/".into(),
            _ => config.pm.credentials.ttl_seconds = 301,
        }
        let coordinator = PmCredentialCoordinator::configured(&config)
            .unwrap()
            .unwrap();
        assert!(matches!(
            coordinator
                .prepare_credential(fixture.remote.repo.as_ref(), &saved)
                .await,
            Err(shared::AppError::Conflict(_))
        ));
    }
    assert_eq!(fixture.remote.posts.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.remote.children.load(Ordering::SeqCst), 1);
    assert_eq!(saved.credentials, fixture.operation().await.credentials);
}

#[tokio::test]
async fn pm_credentials_pg_concurrent_preparation_freezes_one_receipt_and_rejects_payload_conflict()
{
    let Some(fixture) = fixture().await else {
        return;
    };
    let (first, second) = tokio::join!(fixture.prepare(), fixture.prepare());
    first.unwrap();
    second.unwrap();
    assert_eq!(fixture.remote.children.load(Ordering::SeqCst), 1);
    let saved = fixture.operation().await;
    let mut intent = saved.credentials.as_ref().unwrap().intent.clone();
    intent.command["scopes"] = json!(["task-tracker:read"]);
    intent.request_sha256 = domain::pm_canonical_hash(&intent.command);
    assert!(matches!(
        fixture
            .remote
            .repo
            .record_pm_draft_proof(
                saved.id,
                saved.owner_user_id,
                domain::PmDraftProof::CredentialIntent(intent)
            )
            .await,
        Err(shared::AppError::Conflict(_))
    ));
    assert_eq!(fixture.remote.posts.load(Ordering::SeqCst), 2);
    assert_eq!(saved.credentials, fixture.operation().await.credentials);
}

#[tokio::test]
async fn pm_credentials_pg_revoked_child_cannot_be_used() {
    let Some(fixture) = fixture().await else {
        return;
    };
    fixture.prepare().await.unwrap();
    fixture.remote.mode.store(4, Ordering::SeqCst);
    assert!(matches!(
        fixture.prepare().await,
        Err(shared::AppError::Forbidden)
    ));
    assert_eq!(fixture.remote.contexts.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.remote.posts.load(Ordering::SeqCst), 2);
    assert_eq!(fixture.remote.children.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn pm_credentials_pg_expired_receipt_stops_before_external_renewal() {
    let Some(mut fixture) = fixture().await else {
        return;
    };
    fixture.config.pm.credentials.ttl_seconds = 1;
    fixture.prepare().await.unwrap();
    tokio::time::sleep(Duration::from_millis(1100)).await;
    assert!(matches!(
        fixture.prepare().await,
        Err(shared::AppError::Unauthorized)
    ));
    assert_eq!(fixture.remote.posts.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.remote.children.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn pm_credentials_pg_database_guards_monotonic_journal_and_single_redacted_audit() {
    let Some(fixture) = fixture().await else {
        return;
    };
    fixture.prepare().await.unwrap();
    fixture.prepare().await.unwrap();
    let saved = fixture.operation().await;
    let original = serde_json::to_value(&saved).unwrap();
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let mut erase = original.clone();
    erase.as_object_mut().unwrap().remove("credentials");
    let mut pending = original.clone();
    pending["credentials"]["receipt"] = Value::Null;
    let mut parent = original.clone();
    parent["credentials"]["intent"]["parent_fingerprint"] = json!("a".repeat(64));
    let mut receipt = original.clone();
    receipt["credentials"]["receipt"]["token_id"] = json!(Uuid::new_v4());
    let mut secret = original.clone();
    secret["credentials"]["receipt"]["secret"] = json!(CHILD);
    let mut payload = original.clone();
    payload["request"]["title"] = json!("Different task");
    for value in [erase, pending, parent, receipt, secret, payload] {
        assert!(
            db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE pm_draft_creation_operations SET operation=$2 WHERE id=$1",
                [saved.id.into(), value.into()]
            ))
            .await
            .is_err()
        );
    }
    assert!(
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "DELETE FROM pm_draft_creation_operations WHERE id=$1",
            [saved.id.into()]
        ))
        .await
        .is_err()
    );
    assert_eq!(fixture.operation().await.credentials, saved.credentials);
    let rows = db.query_all(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT action,payload FROM audit_log WHERE entity_type='pm_draft_operation' AND entity_id=$1 ORDER BY action",
        [saved.id.to_string().into()])).await.unwrap();
    assert_eq!(rows.len(), 2);
    for row in rows {
        let action: String = row.try_get("", "action").unwrap();
        assert!(matches!(
            action.as_str(),
            "pm_credentials.intent" | "pm_credentials.acknowledged"
        ));
        let json: Value = row.try_get("", "payload").unwrap();
        let bytes = json.to_string();
        assert!(
            !bytes.contains(PARENT)
                && !bytes.contains(CHILD)
                && !bytes.contains("parent_fingerprint")
        );
    }
    let migrations = migration::Migrator::migrations();
    let credentials = migrations
        .iter()
        .find(|m| m.name() == "m20261004_000011_pm_credentials")
        .unwrap();
    assert!(
        credentials
            .down(&migration::SchemaManager::new(&db))
            .await
            .is_err()
    );
    assert_eq!(fixture.operation().await.credentials, saved.credentials);
}

#[tokio::test]
async fn pm_credentials_pg_first_intent_and_ack_require_readable_strict_shapes() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let op = fixture.operation().await;
    let command = serde_json::to_value(
        PmCredentialCommand::tracker(&op.execution_identity().unwrap(), op.credential_key(), 300)
            .unwrap(),
    )
    .unwrap();
    let intent = domain::PmCredentialIntent {
        request_sha256: domain::pm_canonical_hash(&command),
        command,
        parent_fingerprint: "a".repeat(64),
        base_origin: fixture.config.pm.credentials.auth_url.clone(),
        tracker_origin: fixture.config.tracker.url.clone(),
        machine_subject: fixture.remote.subject.clone(),
    };
    let mut valid = serde_json::to_value(&op).unwrap();
    valid["credentials"] = json!({"intent":intent,"receipt":null});
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    for field in [
        "base_origin",
        "tracker_origin",
        "machine_subject",
        "parent_fingerprint",
        "request_sha256",
    ] {
        let mut value = valid.clone();
        value["credentials"]["intent"][field] = json!({});
        assert!(
            db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE pm_draft_creation_operations SET operation=$2 WHERE id=$1",
                [op.id.into(), value.into()]
            ))
            .await
            .is_err()
        );
        assert!(fixture.operation().await.credentials.is_none());
    }
    for field in ["parent_fingerprint", "request_sha256"] {
        let expression = format!(
            "UPDATE pm_draft_creation_operations SET operation=jsonb_set($2,'{{credentials,intent,{field}}}',to_jsonb(1111111111111111111111111111111111111111111111111111111111111111::numeric)) WHERE id=$1"
        );
        assert!(
            db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                expression,
                [op.id.into(), valid.clone().into()]
            ))
            .await
            .is_err()
        );
        assert!(fixture.operation().await.credentials.is_none());
    }
    fixture
        .remote
        .repo
        .record_pm_draft_proof(
            op.id,
            op.owner_user_id,
            domain::PmDraftProof::CredentialIntent(intent),
        )
        .await
        .unwrap();
    let pending = fixture.operation().await;
    for expiry in [
        "not-a-date",
        "2026-02-30T12:00:00Z",
        "infinity",
        "2026-10-04T12:00:00.1234567890Z",
        "2026-10-04T24:00:00Z",
        "2026-10-04T12:00:00+24:00",
        "2026-10-04T12:00:60Z",
    ] {
        let mut value = serde_json::to_value(&pending).unwrap();
        value["credentials"]["receipt"] =
            json!({"token_id":Uuid::new_v4(),"expires_at":expiry,"scopes":fixture.remote.scopes});
        assert!(
            db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE pm_draft_creation_operations SET operation=$2 WHERE id=$1",
                [op.id.into(), value.into()]
            ))
            .await
            .is_err()
        );
        assert!(
            fixture
                .operation()
                .await
                .credentials
                .unwrap()
                .receipt
                .is_none()
        );
    }
    assert_eq!(fixture.remote.posts.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn pm_credentials_pg_revoked_parent_prevents_post_and_does_not_erase_ack() {
    let Some(fixture) = fixture().await else {
        return;
    };
    fixture.prepare().await.unwrap();
    let saved = fixture.operation().await;
    fixture.remote.mode.store(8, Ordering::SeqCst);
    assert!(matches!(
        fixture.prepare().await,
        Err(shared::AppError::Unauthorized)
    ));
    assert_eq!(fixture.remote.posts.load(Ordering::SeqCst), 1);
    assert_eq!(saved.credentials, fixture.operation().await.credentials);
}

#[tokio::test]
async fn pm_credentials_pg_audit_failure_rolls_back_journal_and_recovers_original_child() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let id = fixture.remote.operation.id;
    let guard = format!("fleet_test_credential_audit_{}", id.simple());
    for action in ["pm_credentials.intent", "pm_credentials.acknowledged"] {
        // Inject at the audit boundary after the journal UPDATE in the same transaction.
        db.execute_unprepared(&format!(
            "CREATE FUNCTION {guard}() RETURNS trigger AS $$ BEGIN
             IF NEW.entity_type='pm_draft_operation' AND NEW.entity_id=TG_ARGV[0]
                AND NEW.action=TG_ARGV[1] THEN
                RAISE EXCEPTION 'injected credential audit failure';
             END IF; RETURN NEW; END; $$ LANGUAGE plpgsql;
             CREATE TRIGGER {guard} BEFORE INSERT ON audit_log
                FOR EACH ROW EXECUTE FUNCTION {guard}('{id}','{action}');"
        ))
        .await
        .unwrap();
        assert!(fixture.prepare().await.is_err());
        db.execute_unprepared(&format!(
            "DROP TRIGGER {guard} ON audit_log; DROP FUNCTION {guard}();"
        ))
        .await
        .unwrap();
        let saved = fixture.operation().await;
        if action == "pm_credentials.intent" {
            assert!(saved.credentials.is_none());
            assert_eq!(fixture.remote.posts.load(Ordering::SeqCst), 0);
        } else {
            assert!(saved.credentials.unwrap().receipt.is_none());
            assert_eq!(fixture.remote.posts.load(Ordering::SeqCst), 1);
            assert_eq!(fixture.remote.children.load(Ordering::SeqCst), 1);
        }
    }
    let issued_id = fixture.remote.receipt.lock().await.as_ref().unwrap()["token_id"]
        .as_str()
        .unwrap()
        .to_string();
    fixture.prepare().await.unwrap();
    let receipt = fixture
        .operation()
        .await
        .credentials
        .unwrap()
        .receipt
        .unwrap();
    assert_eq!(receipt.token_id.to_string(), issued_id);
    assert_eq!(fixture.remote.posts.load(Ordering::SeqCst), 2);
    assert_eq!(fixture.remote.children.load(Ordering::SeqCst), 1);
    let rows = db
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT action FROM audit_log WHERE entity_type='pm_draft_operation'
             AND entity_id=$1 ORDER BY action",
            [id.to_string().into()],
        ))
        .await
        .unwrap();
    let actions: Vec<String> = rows
        .into_iter()
        .map(|row| row.try_get("", "action").unwrap())
        .collect();
    assert_eq!(
        actions,
        ["pm_credentials.acknowledged", "pm_credentials.intent"]
    );
}
