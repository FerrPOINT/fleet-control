use super::*;
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use domain::{PmCredentialCommand, PmDraftOperation};
use infra::pm_credentials::PmCredentialCoordinator;
use migration::MigratorTrait;
use serde_json::{Value, json};
use std::sync::atomic::AtomicBool;
use tokio::sync::Mutex;

const PARENT: &str = "sdlc_pat_parent-test-secret-1234567890";
const CHILD: &str = "sdlc_pat_child-test-secret-1234567890";
const WORKFLOW: &str = "workflow-assignment-test-secret-1234567890";
const ROTATED_WORKFLOW: &str = "rotated-workflow-assignment-test-secret";

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
    claimed_lease: Mutex<Option<Value>>,
    heartbeat: Mutex<Option<Value>>,
    heartbeat_posts: AtomicUsize,
    heartbeat_lost: AtomicBool,
    lease_age: AtomicUsize,
    workflow_receipt: Mutex<Option<Value>>,
    workflow_posts: AtomicUsize,
    workflow_created: AtomicUsize,
    workflow_lost: AtomicBool,
    workflow_corrupt: AtomicBool,
    workflow_unready: AtomicBool,
    workflow_rotated_authority: AtomicBool,
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
        Json(
            json!({"sub":remote.subject,"email":"machine@example.test","display_name":"PM service",
        "scopes":if root {vec!["task-tracker:read".to_string(),"task-tracker:write".to_string()]} else {remote.scopes.clone()}}),
        ),
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
    let now = chrono::Utc::now() + chrono::Duration::seconds(if mode == 18 { 31 } else { 0 });
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
    if matches!(mode, 15..=18) {
        if let Some(original) = remote.claimed_lease.lock().await.as_ref() {
            value["state"] = json!("active");
            value["current"] = original["result"]["lease"].clone();
            if let Some(heartbeat) = remote.heartbeat.lock().await.as_ref() {
                value["current"] = heartbeat["result"]["lease"].clone();
                if query
                    .get("idempotency_key")
                    .is_some_and(|key| heartbeat["idempotency_key"] == *key)
                {
                    value["operation"] = heartbeat.clone();
                }
            }
            if chrono::DateTime::parse_from_rfc3339(
                value["current"]["expires_at"].as_str().unwrap(),
            )
            .unwrap()
                <= now
            {
                value["state"] = json!("expired");
            }
            if query
                .get("idempotency_key")
                .is_some_and(|key| original["idempotency_key"] == *key)
            {
                value["operation"] = original.clone();
            }
        }
        return Json(value).into_response();
    }
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

async fn lease_mutation(
    State(remote): State<Arc<Remote>>,
    headers: HeaderMap,
    Json(command): Json<domain::PmExecutionLeaseClaim>,
) -> axum::response::Response {
    remote.lease_posts.fetch_add(1, Ordering::SeqCst);
    if !matches!(remote.lease_mode.load(Ordering::SeqCst), 15..=17) {
        return StatusCode::FORBIDDEN.into_response();
    }
    assert_eq!(headers["authorization"], format!("Bearer {CHILD}"));
    let saved = remote
        .repo
        .read_pm_draft_operation(remote.operation.id, remote.operation.owner_user_id)
        .await
        .unwrap();
    let journal = saved.execution_lease.unwrap();
    assert_eq!(journal.claim, command);
    let original = domain::PmExecutionLeaseCommand::Claim(command.clone());
    assert_eq!(journal.request_sha256, original.request_sha256());
    let now = chrono::Utc::now()
        - chrono::Duration::seconds(remote.lease_age.load(Ordering::SeqCst) as i64);
    let stamp =
        |x: chrono::DateTime<chrono::Utc>| x.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
    let reservation = remote.operation.reservation.as_ref().unwrap();
    let receipt = json!({"contract_version":1,"binding":reservation.binding,"owner_version":command.expected_owner_version,
        "fence":command.fence,"lease":{"lease_id":Uuid::new_v4(),"version":1,"holder_subject":remote.subject,
        "claimed_at":stamp(now),"heartbeat_at":stamp(now),"expires_at":stamp(now+chrono::Duration::seconds(30))},
        "ttl_seconds":30,"heartbeat_seconds":10,"dispatch_allowed":false});
    *remote.claimed_lease.lock().await = Some(
        json!({"idempotency_key":command.idempotency_key,"request_sha256":original.request_sha256(),"result":receipt}),
    );
    if remote.lease_mode.load(Ordering::SeqCst) == 16 {
        return StatusCode::BAD_GATEWAY.into_response();
    }
    if remote.lease_mode.load(Ordering::SeqCst) == 17 {
        let mut conflicting = receipt;
        conflicting["lease"]["lease_id"] = json!(Uuid::new_v4());
        return (StatusCode::CREATED, Json(conflicting)).into_response();
    }
    (StatusCode::CREATED, Json(receipt)).into_response()
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}

async fn heartbeat_mutation(
    State(remote): State<Arc<Remote>>,
    headers: HeaderMap,
    Json(command): Json<domain::PmExecutionLeaseHeartbeat>,
) -> axum::response::Response {
    assert_eq!(headers["authorization"], format!("Bearer {CHILD}"));
    remote.heartbeat_posts.fetch_add(1, Ordering::SeqCst);
    let claim = remote.claimed_lease.lock().await;
    let mut stored = remote.heartbeat.lock().await;
    let wire = domain::PmExecutionLeaseCommand::Heartbeat(command.clone());
    let result = if let Some(previous) = stored.as_ref() {
        if previous["idempotency_key"] != command.idempotency_key
            || previous["request_sha256"] != wire.request_sha256()
        {
            return StatusCode::CONFLICT.into_response();
        }
        previous["result"].clone()
    } else {
        let mut receipt = claim.as_ref().unwrap()["result"].clone();
        assert_eq!(receipt["lease"]["lease_id"], json!(command.lease_id));
        assert_eq!(
            receipt["lease"]["version"],
            json!(command.expected_lease_version)
        );
        let now = chrono::Utc::now();
        let stamp = |x: chrono::DateTime<chrono::Utc>| {
            x.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
        };
        receipt["lease"]["version"] = json!(command.expected_lease_version + 1);
        receipt["lease"]["heartbeat_at"] = json!(stamp(now));
        receipt["lease"]["expires_at"] = json!(stamp(now + chrono::Duration::seconds(30)));
        *stored = Some(json!({"idempotency_key":command.idempotency_key,
            "request_sha256":wire.request_sha256(),"result":receipt}));
        receipt
    };
    if remote.heartbeat_lost.load(Ordering::SeqCst) {
        return StatusCode::BAD_GATEWAY.into_response();
    }
    Json(result).into_response()
}

#[tokio::test]
async fn pm_credentials_pg_heartbeat_lost_ack_concurrent_cas_and_expiry() {
    let Some(fixture) = fixture().await else {
        return;
    };
    fixture.remote.lease_mode.store(15, Ordering::SeqCst);
    fixture.remote.lease_age.store(11, Ordering::SeqCst);
    let coordinator = fixture.coordinator();
    let credential = coordinator
        .prepare_credential(fixture.remote.repo.as_ref(), &fixture.operation().await)
        .await
        .unwrap();
    coordinator
        .claim_execution_lease(
            fixture.remote.repo.as_ref(),
            &fixture.operation().await,
            &credential,
        )
        .await
        .unwrap();
    fixture.remote.heartbeat_lost.store(true, Ordering::SeqCst);
    let op = fixture.operation().await;
    let (a, b) = tokio::join!(
        coordinator.renew_execution_lease(&op, &credential),
        coordinator.renew_execution_lease(&op, &credential)
    );
    for result in [a, b] {
        assert_eq!(result.unwrap().current.unwrap().version, 2);
    }
    let posts = fixture.remote.heartbeat_posts.load(Ordering::SeqCst);
    assert!((1..=2).contains(&posts));
    let replay = coordinator
        .renew_execution_lease(&op, &credential)
        .await
        .unwrap();
    assert_eq!(replay.current.unwrap().version, 2);
    assert_eq!(fixture.remote.heartbeat_posts.load(Ordering::SeqCst), posts);
    assert_eq!(fixture.remote.lease_posts.load(Ordering::SeqCst), 1);
    assert_eq!(
        fixture.operation().await.execution_lease,
        op.execution_lease
    );
    // The historical ACK cannot authorize renewal after current expiry.
    fixture.remote.lease_mode.store(18, Ordering::SeqCst);
    assert!(
        coordinator
            .renew_execution_lease(&op, &credential)
            .await
            .is_err()
    );
    assert_eq!(fixture.remote.heartbeat_posts.load(Ordering::SeqCst), posts);
}

fn workflow_compatibility() -> Value {
    json!({"catalogVersion":2,"catalogRevision":"a".repeat(40),"catalogSha256":"b".repeat(64),
        "skillsRevision":"c".repeat(40),"skillsManifestSha256":"d".repeat(64),
        "capabilityRevision":"hermes-sdlc-runtime/v2","capabilitySha256":"e".repeat(64)})
}

async fn workflow_capabilities(
    State(remote): State<Arc<Remote>>,
    headers: HeaderMap,
) -> axum::response::Response {
    let valid = headers["authorization"] == format!("Bearer {WORKFLOW}")
        || (remote.workflow_rotated_authority.load(Ordering::SeqCst)
            && headers["authorization"] == format!("Bearer {ROTATED_WORKFLOW}"));
    if !valid {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    if remote.workflow_unready.load(Ordering::SeqCst) {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    Json(
        json!({"ok":true,"role_key":"project_manager","credential_kind":"assignment",
        "readiness":{"service":"ready","schema":"ready","catalog":"ready"},
        "runtimeCompatibility":workflow_compatibility(),"pm_continuation":{
            "contract_version":1,"base_path":"/internal/runtime/v1/pm",
            "commands":["assign","bind","resume","rebind","readback"],
            "terminal_proof":"configured-runtime-readback","dispatch_owner":"fleet"}}),
    )
    .into_response()
}

async fn workflow_assign(
    State(remote): State<Arc<Remote>>,
    headers: HeaderMap,
    Json(command): Json<Value>,
) -> axum::response::Response {
    assert_eq!(headers["authorization"], format!("Bearer {WORKFLOW}"));
    let saved = remote
        .repo
        .read_pm_draft_operation(remote.operation.id, remote.operation.owner_user_id)
        .await
        .unwrap();
    let journal = saved.workflow_assignment.unwrap();
    assert_eq!(command, journal.intent.command);
    assert_eq!(
        journal.intent.request_sha256,
        domain::pm_canonical_hash(&command)
    );
    remote.workflow_posts.fetch_add(1, Ordering::SeqCst);
    let mut original = remote.workflow_receipt.lock().await;
    let value = original.get_or_insert_with(|| {
        remote.workflow_created.fetch_add(1, Ordering::SeqCst);
        json!({"ok":true,"exit_code":0,"result":{
            "task_key":command["task"],"workflow_id":7,"workflow_key":"hermes-sdlc:project_manager",
            "mode_id":8,"mode_key":"draft","cycle_number":0,"attempt_number":1,
            "assignment_operation_key":command["assignment_operation_key"],"assignment_revision":1,
            "role_key":"project_manager","execution_scope":"business","stage_key":"draft",
            "business_task_ref":command["task_ref"],"root_task_ref":command["root_ref"],
            "work_item_ref":null,"work_item_revision":null,"queue_item_ref":null,"task_workspace_ref":null,
            "workspace_revision":null,"tech_execution_workspace_ref":null,"tech_execution_attempt_ref":null,
            "decomposition_revision_ref":null,"stage_revision":"1","assignment_ref":command["assignment_ref"],
            "binding_ref":null,"hermes_run_ref":null,"bind_operation_key":null,"concrete_agent_ref":null,
            "workspace_generation":null,"lease_generation":1,"exact_input_refs":[{"kind":"pm_draft_input",
                "ref":command["input_snapshot_ref"],"hash":command["input_sha256"]}],"binding_state":"unbound",
            "status":"active","current_phase_id":9,"current_phase_code":"PM-DRAFT-01","current_phase_name":"Draft intake"}})
    }).clone();
    if remote.workflow_lost.load(Ordering::SeqCst) {
        return StatusCode::BAD_GATEWAY.into_response();
    }
    let mut value = value;
    if remote.workflow_corrupt.load(Ordering::SeqCst) {
        value["result"]["mode_key"] = json!("analysis");
    }
    Json(value).into_response()
}

#[tokio::test]
async fn pm_credentials_pg_workflow_original_command_lost_ack_and_database_guards() {
    let Some(mut fixture) = fixture().await else {
        return;
    };
    fixture.config.pm.workflow.enabled = true;
    let coordinator = fixture.coordinator();
    let credential = coordinator
        .prepare_credential(fixture.remote.repo.as_ref(), &fixture.operation().await)
        .await
        .unwrap();
    fixture.remote.lease_mode.store(15, Ordering::SeqCst);
    coordinator
        .claim_execution_lease(
            fixture.remote.repo.as_ref(),
            &fixture.operation().await,
            &credential,
        )
        .await
        .unwrap();
    let workflow = infra::pm_workflow::PmWorkflowClient::configured(&fixture.config)
        .unwrap()
        .unwrap();
    fixture
        .remote
        .workflow_unready
        .store(true, Ordering::SeqCst);
    assert!(
        workflow
            .prepare_assignment(
                fixture.remote.repo.as_ref(),
                &fixture.operation().await,
                &coordinator,
                &credential
            )
            .await
            .is_err()
    );
    assert!(fixture.operation().await.workflow_assignment.is_none());
    fixture
        .remote
        .workflow_unready
        .store(false, Ordering::SeqCst);
    fixture.remote.workflow_lost.store(true, Ordering::SeqCst);
    assert!(
        workflow
            .prepare_assignment(
                fixture.remote.repo.as_ref(),
                &fixture.operation().await,
                &coordinator,
                &credential
            )
            .await
            .is_err()
    );
    let pending = fixture.operation().await;
    assert!(
        pending
            .workflow_assignment
            .as_ref()
            .unwrap()
            .receipt
            .is_none()
    );
    assert_eq!(fixture.remote.workflow_posts.load(Ordering::SeqCst), 1);
    fixture.remote.workflow_lost.store(false, Ordering::SeqCst);
    workflow
        .prepare_assignment(
            fixture.remote.repo.as_ref(),
            &pending,
            &coordinator,
            &credential,
        )
        .await
        .unwrap();
    let saved = fixture.operation().await;
    workflow
        .prepare_assignment(
            fixture.remote.repo.as_ref(),
            &saved,
            &coordinator,
            &credential,
        )
        .await
        .unwrap();
    assert_eq!(fixture.remote.workflow_created.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.remote.workflow_posts.load(Ordering::SeqCst), 2);
    assert_eq!(fixture.operation().await, saved);
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let original = serde_json::to_value(&saved).unwrap();
    let mut erased = original.clone();
    erased
        .as_object_mut()
        .unwrap()
        .remove("workflow_assignment");
    let mut changed = original.clone();
    changed["workflow_assignment"]["intent"]["command"]["agent_ref"] = json!(Uuid::new_v4());
    let mut receipt = original.clone();
    receipt["workflow_assignment"]["receipt"]["mode_id"] = json!(88);
    let mut secret = original;
    secret["workflow_assignment"]["intent"]["secret"] = json!(WORKFLOW);
    for value in [erased, changed, receipt, secret] {
        assert!(
            db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE pm_draft_creation_operations SET operation=$2 WHERE id=$1",
                [saved.id.into(), value.into()]
            ))
            .await
            .is_err()
        );
        assert_eq!(fixture.operation().await, saved);
    }
    let rows = db.query_all(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT action,payload FROM audit_log WHERE entity_id=$1 AND action LIKE 'pm_workflow.%' ORDER BY action",
        [saved.id.to_string().into()])).await.unwrap();
    assert_eq!(rows.len(), 2);
    for row in rows {
        let payload: Value = row.try_get("", "payload").unwrap();
        assert!(!payload.to_string().contains(WORKFLOW));
    }
    assert!(!saved.response().dispatch_allowed);
    assert!(
        fixture
            .remote
            .repo
            .list_session_agent_runs(saved.session_id.unwrap())
            .await
            .unwrap()
            .is_empty()
    );
    let migrations = migration::Migrator::migrations();
    let migration = migrations
        .iter()
        .find(|v| v.name() == "m20261009_000024_pm_workflow_assignment")
        .unwrap();
    assert!(
        migration
            .down(&migration::SchemaManager::new(&db))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn pm_credentials_pg_workflow_rejects_foreign_response_and_rotated_adapter() {
    let Some(mut fixture) = fixture().await else {
        return;
    };
    fixture.config.pm.workflow.enabled = true;
    let coordinator = fixture.coordinator();
    let credential = coordinator
        .prepare_credential(fixture.remote.repo.as_ref(), &fixture.operation().await)
        .await
        .unwrap();
    fixture.remote.lease_mode.store(15, Ordering::SeqCst);
    coordinator
        .claim_execution_lease(
            fixture.remote.repo.as_ref(),
            &fixture.operation().await,
            &credential,
        )
        .await
        .unwrap();
    fixture
        .remote
        .workflow_corrupt
        .store(true, Ordering::SeqCst);
    let workflow = infra::pm_workflow::PmWorkflowClient::configured(&fixture.config)
        .unwrap()
        .unwrap();
    assert!(
        workflow
            .prepare_assignment(
                fixture.remote.repo.as_ref(),
                &fixture.operation().await,
                &coordinator,
                &credential
            )
            .await
            .is_err()
    );
    let saved = fixture.operation().await;
    assert!(
        saved
            .workflow_assignment
            .as_ref()
            .unwrap()
            .receipt
            .is_none()
    );
    fixture.config.pm.workflow.assignment_token = ROTATED_WORKFLOW.into();
    fixture
        .remote
        .workflow_rotated_authority
        .store(true, Ordering::SeqCst);
    let rotated = infra::pm_workflow::PmWorkflowClient::configured(&fixture.config)
        .unwrap()
        .unwrap();
    // The new token is valid at Workflow, but cannot take over the original Fleet journal.
    assert!(matches!(
        rotated
            .prepare_assignment(
                fixture.remote.repo.as_ref(),
                &saved,
                &coordinator,
                &credential
            )
            .await,
        Err(shared::AppError::Conflict(_))
    ));
    assert_eq!(fixture.remote.workflow_posts.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.operation().await, saved);
}

#[tokio::test]
async fn pm_credentials_pg_workflow_creation_prepares_owners_without_native_dispatch() {
    let Some(mut fixture) = fixture().await else {
        return;
    };
    fixture.config.pm.workflow.enabled = true;
    fixture.remote.lease_mode.store(15, Ordering::SeqCst);
    let coordinator = fixture.coordinator();
    let response = super::pm_draft_creation::continue_with_credentials(
        fixture.remote.repo.as_ref(),
        fixture.operation().await,
        &coordinator,
    )
    .await
    .unwrap();
    assert!(!response.dispatch_allowed);
    let saved = fixture.operation().await;
    assert!(
        saved
            .workflow_assignment
            .as_ref()
            .unwrap()
            .receipt
            .is_some()
    );
    assert_eq!(fixture.remote.workflow_created.load(Ordering::SeqCst), 1);
    super::pm_draft_creation::continue_with_credentials(
        fixture.remote.repo.as_ref(),
        saved.clone(),
        &coordinator,
    )
    .await
    .unwrap();
    assert_eq!(fixture.remote.lease_posts.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.remote.workflow_posts.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.operation().await, saved);
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
        claimed_lease: Mutex::new(None),
        heartbeat: Mutex::new(None),
        heartbeat_posts: AtomicUsize::new(0),
        heartbeat_lost: AtomicBool::new(false),
        lease_age: AtomicUsize::new(0),
        workflow_receipt: Mutex::new(None),
        workflow_posts: AtomicUsize::new(0),
        workflow_created: AtomicUsize::new(0),
        workflow_lost: AtomicBool::new(false),
        workflow_corrupt: AtomicBool::new(false),
        workflow_unready: AtomicBool::new(false),
        workflow_rotated_authority: AtomicBool::new(false),
    });
    let router = Router::new()
        .route("/auth/tokens/introspect", get(introspection))
        .route("/auth/tokens/delegate", post(delegate))
        .route("/internal/runtime/capabilities", get(workflow_capabilities))
        .route("/internal/runtime/v1/pm/assign", post(workflow_assign))
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
        .route(
            &format!(
                "/api/v1/issues/{}/sdlc/pm-draft-execution-lease/heartbeat",
                remote.operation.identity().unwrap().task_id
            ),
            post(heartbeat_mutation),
        )
        .with_state(remote.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}/", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut config = AppConfig::default();
    config.tracker.url = origin.clone();
    config.pm.credentials = shared::PmCredentialsConfig {
        enabled: true,
        auth_url: origin.clone(),
        machine_subject: subject,
        parent_pat: PARENT.into(),
        ttl_seconds: 300,
    };
    config.fleet.project_workflow_url = Some(origin);
    config.pm.workflow = shared::PmWorkflowConfig {
        enabled: false,
        native_fleet_origin: String::new(),
        assignment_token: WORKFLOW.into(),
        runtime_token: "workflow-runtime-test-secret-1234567890".into(),
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
async fn pm_credentials_pg_lease_claim_records_intent_and_recovers_original_lost_ack() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let coordinator = fixture.coordinator();
    let credential = coordinator
        .prepare_credential(fixture.remote.repo.as_ref(), &fixture.operation().await)
        .await
        .unwrap();
    let prepared = fixture.operation().await;
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let command = domain::PmExecutionLeaseCommand::Claim(prepared.lease_claim().unwrap());
    let mut pending = serde_json::to_value(&prepared).unwrap();
    pending["execution_lease"] =
        json!({"claim":command.payload(),"request_sha256":command.request_sha256(),"receipt":null});
    let mut foreign = pending.clone();
    foreign["execution_lease"]["claim"]["fence"]["agent_id"] = json!(Uuid::new_v4());
    let mut extra = pending.clone();
    extra["execution_lease"]["secret"] = json!(CHILD);
    let mut hash = pending.clone();
    hash["execution_lease"]["request_sha256"] = json!(123);
    let mut unacknowledged = pending.clone();
    unacknowledged["credentials"]["receipt"] = Value::Null;
    for altered in [foreign, extra, hash, unacknowledged] {
        assert!(
            db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE pm_draft_creation_operations SET operation=$2 WHERE id=$1",
                [prepared.id.into(), altered.into()],
            ))
            .await
            .is_err()
        );
        assert!(fixture.operation().await.execution_lease.is_none());
    }
    fixture.remote.lease_mode.store(16, Ordering::SeqCst);
    assert!(
        coordinator
            .claim_execution_lease(
                fixture.remote.repo.as_ref(),
                &fixture.operation().await,
                &credential
            )
            .await
            .is_err()
    );
    let unknown = fixture.operation().await;
    assert!(unknown.execution_lease.as_ref().unwrap().receipt.is_none());
    assert_eq!(fixture.remote.lease_posts.load(Ordering::SeqCst), 1);
    fixture.remote.lease_mode.store(15, Ordering::SeqCst);
    let acknowledged = coordinator
        .claim_execution_lease(fixture.remote.repo.as_ref(), &unknown, &credential)
        .await
        .unwrap();
    assert_eq!(acknowledged.state, domain::PmExecutionLeaseState::Active);
    let saved = fixture.operation().await;
    assert!(saved.execution_lease.as_ref().unwrap().receipt.is_some());
    coordinator
        .claim_execution_lease(fixture.remote.repo.as_ref(), &saved, &credential)
        .await
        .unwrap();
    assert_eq!(fixture.remote.lease_posts.load(Ordering::SeqCst), 1);
    assert_eq!(saved, fixture.operation().await);
    assert!(!saved.response().dispatch_allowed);

    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let original = serde_json::to_value(&saved).unwrap();
    let mut erase = original.clone();
    erase.as_object_mut().unwrap().remove("execution_lease");
    let mut pending = original.clone();
    pending["execution_lease"]["receipt"] = Value::Null;
    let mut replacement = original.clone();
    replacement["execution_lease"]["receipt"]["lease"]["lease_id"] = json!(Uuid::new_v4());
    let mut key = original.clone();
    key["execution_lease"]["claim"]["idempotency_key"] = json!("another-claim");
    let mut secret = original.clone();
    secret["execution_lease"]["receipt"]["secret"] = json!(CHILD);
    for altered in [erase, pending, replacement, key, secret] {
        assert!(
            db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE pm_draft_creation_operations SET operation=$2 WHERE id=$1",
                [saved.id.into(), altered.into()],
            ))
            .await
            .is_err()
        );
        assert_eq!(fixture.operation().await, saved);
    }
    let events = db.query_all(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT action,payload FROM audit_log WHERE entity_id=$1 AND action LIKE 'pm_lease.%' ORDER BY action",
        [saved.id.to_string().into()])).await.unwrap();
    assert_eq!(events.len(), 2);
    for row in events {
        let value: Value = row.try_get("", "payload").unwrap();
        assert!(!value.to_string().contains(CHILD));
    }
    let migrations = migration::Migrator::migrations();
    let lease = migrations
        .iter()
        .find(|m| m.name() == "m20261009_000023_pm_execution_lease")
        .unwrap();
    assert!(
        lease
            .down(&migration::SchemaManager::new(&db))
            .await
            .is_err()
    );
    assert_eq!(fixture.operation().await, saved);
}

#[tokio::test]
async fn pm_credentials_pg_lease_success_reuses_credential_and_conflicting_ack_stays_pending() {
    for mode in [15, 17] {
        let Some(fixture) = fixture().await else {
            return;
        };
        let coordinator = fixture.coordinator();
        let credential = coordinator
            .prepare_credential(fixture.remote.repo.as_ref(), &fixture.operation().await)
            .await
            .unwrap();
        fixture.remote.lease_mode.store(mode, Ordering::SeqCst);
        let result = coordinator
            .claim_execution_lease(
                fixture.remote.repo.as_ref(),
                &fixture.operation().await,
                &credential,
            )
            .await;
        let saved = fixture.operation().await;
        if mode == 15 {
            assert!(result.is_ok());
            assert!(saved.execution_lease.as_ref().unwrap().receipt.is_some());
            let replay = coordinator
                .prepare_credential(fixture.remote.repo.as_ref(), &saved)
                .await
                .unwrap();
            assert_eq!(replay.token_id(), credential.token_id());
            assert_eq!(saved, fixture.operation().await);
        } else {
            assert!(matches!(result, Err(shared::AppError::Conflict(_))));
            assert!(saved.execution_lease.as_ref().unwrap().receipt.is_none());
        }
        assert_eq!(fixture.remote.lease_posts.load(Ordering::SeqCst), 1);
        assert!(!saved.response().dispatch_allowed);
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
