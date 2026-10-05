use super::*;
use app::RuntimeSupervisor;
use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use domain::{
    RuntimeControlActor, RuntimeControlOperation, RuntimeControlState, SteerSessionRunRequest,
};
use sea_orm::DatabaseConnection;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{Mutex, atomic::AtomicBool},
};
use tokio::sync::Notify;

// This is an HTTP/PG consumer fixture, not native Hermes or an OS restart claim.
struct Producer {
    repo: Arc<PostgresFleetRepository>,
    token: String,
    native: String,
    epoch: String,
    bad_caps: AtomicBool,
    posts: AtomicUsize,
    reads: AtomicUsize,
    caps_reads: AtomicUsize,
    mode: AtomicUsize, // 0 normal, 1 lost ACK, 2 unknown effect, 3 held normal ACK
    records: Mutex<HashMap<Uuid, Value>>,
    fault: Mutex<Option<(StatusCode, Value)>>,
    status: Mutex<Value>,
    started: Notify,
    release: Notify,
}

impl Producer {
    fn authenticated(&self, headers: &HeaderMap) {
        assert_eq!(headers["authorization"], format!("Bearer {}", self.token));
        assert_eq!(headers["accept-encoding"], "identity");
    }
    fn caps(&self) -> Value {
        json!({"object":"fleet.hermes.controls.capabilities","contract_version":1,
            "store_id":self.epoch,"scope_fingerprint":format!("{:x}",Sha256::digest(format!("default\0{}",self.token).as_bytes())),
            "profile":"default","native_source_revision":"bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3",
            "single_send":true,"non_dispatch_lookup":true,"operations":["steer","stop","approval"],
            "lookup":{"method":"GET","path":"/fleet/v1/controls/lookup"}})
    }
}

async fn control_post(
    State(p): State<Arc<Producer>>,
    Path((native, op)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    p.authenticated(&headers);
    let id = Uuid::parse_str(headers["idempotency-key"].to_str().unwrap()).unwrap();
    assert_eq!(headers["x-fleet-control-store-id"], p.epoch);
    assert_eq!(native, p.native);
    let context = if op == "approval" {
        let saved = p.repo.get_approval_outcome(id).await.unwrap().unwrap();
        assert_eq!(
            saved.decision.state,
            domain::ApprovalDecisionState::Uncertain
        );
        saved.context
    } else {
        let saved = p
            .repo
            .get_runtime_control_outcome(id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(saved.receipt.state, RuntimeControlState::Submitted);
        saved.context
    };
    assert_eq!(
        context["request_body"].as_str().unwrap().as_bytes(),
        body.as_ref()
    );
    assert_eq!(context["operation"], op);
    assert_eq!(context["capabilities"]["store_id"], p.epoch);
    p.posts.fetch_add(1, Ordering::SeqCst);
    let ack = match op.as_str() {
        "steer" => json!({"object":"hermes.run.steer","run_id":native,"accepted":true}),
        "stop" => {
            assert!(body.is_empty());
            json!({"run_id":native,"status":"stopping"})
        }
        "approval" => {
            let body: Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(body["request_id"], "request_one");
            assert_eq!(body["resolve_all"], false);
            assert!(matches!(body["choice"].as_str(), Some("once" | "deny")));
            assert_eq!(body.as_object().unwrap().len(), 3);
            json!({"object":"hermes.run.approval_response","run_id":native,
                "request_id":body["request_id"],"choice":body["choice"],"resolved":1})
        }
        _ => panic!("unexpected operation"),
    };
    let mode = p.mode.load(Ordering::SeqCst);
    let witness = json!({"object":"fleet.hermes.controls.lookup","contract_version":1,
        "store_id":p.epoch,"scope_fingerprint":context["capabilities"]["scope_fingerprint"],"profile":"default",
        "command_id":id,"run_id":native,"operation":op,"request_sha256":context["request_sha256"],
        "state":if mode==2 {"uncertain"} else {"acknowledged"},"ack":if mode==2 {Value::Null} else {ack.clone()}});
    assert!(
        p.records.lock().unwrap().insert(id, witness).is_none(),
        "consumer sent a second POST"
    );
    p.started.notify_one();
    if mode == 3 {
        p.release.notified().await;
    }
    if mode == 1 || mode == 2 {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error":"unknown_ack"})),
        )
            .into_response();
    }
    Json(ack).into_response()
}

async fn control_lookup(
    State(p): State<Arc<Producer>>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    p.authenticated(&headers);
    assert!(body.is_empty());
    assert_eq!(query.len(), 5);
    assert_eq!(query["store_id"], p.epoch);
    assert_eq!(query["run_id"], p.native);
    p.reads.fetch_add(1, Ordering::SeqCst);
    if let Some((status, value)) = p.fault.lock().unwrap().clone() {
        return (status, Json(value)).into_response();
    }
    let id = Uuid::parse_str(&query["command_id"]).unwrap();
    let Some(record) = p.records.lock().unwrap().get(&id).cloned() else {
        return (StatusCode::NOT_FOUND, Json(json!({"error":"missing"}))).into_response();
    };
    assert_eq!(query["request_sha256"], record["request_sha256"]);
    assert_eq!(query["operation"], record["operation"]);
    Json(record).into_response()
}

struct HttpFixture {
    repo: Arc<PostgresFleetRepository>,
    db: DatabaseConnection,
    owner: Uuid,
    run: domain::SessionAgentRun,
    agent: domain::Agent,
    config: AppConfig,
    producer: Arc<Producer>,
    server: tokio::task::JoinHandle<()>,
}
impl Drop for HttpFixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}
impl HttpFixture {
    async fn approval(&self) -> domain::RuntimeApprovalRequest {
        let approval = self
            .repo
            .upsert_runtime_approval_request(app::RuntimeApprovalCreate {
                session_id: self.run.session_id,
                session_run_id: self.run.id,
                agent_id: self.agent.id,
                runtime_run_id: self.producer.native.clone(),
                runtime_approval_id: Some("request_one".into()),
                prompt: "Exact original approval".into(),
                detail: json!({}),
            })
            .await
            .unwrap();
        let mut status = self.producer.status.lock().unwrap();
        status["status"] = json!("waiting_for_approval");
        status["approval"] = json!({"event":"approval.request","run_id":self.producer.native,
            "request_id":"request_one"});
        approval
    }
    async fn reserve_approval(
        &self,
        approval: &domain::RuntimeApprovalRequest,
        choice: domain::ApprovalChoice,
    ) -> domain::ApprovalDecision {
        self.repo
            .reserve_original_approval_decision(
                self.run.session_id,
                approval.id,
                self.owner,
                domain::ApprovalDecisionRequest {
                    choice,
                    idempotency_key: Uuid::new_v4().to_string(),
                },
            )
            .await
            .unwrap()
            .decision
    }
    async fn delivered(&self, approval: Uuid) -> domain::ApprovalDecision {
        tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                let decision = self
                    .repo
                    .approval_decision(self.run.session_id, approval)
                    .await
                    .unwrap();
                if decision.state == domain::ApprovalDecisionState::Delivered {
                    return decision;
                }
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap()
    }
    fn runtime(&self) -> infra::runtime::LocalRuntimeSupervisor {
        let (events, _) = tokio::sync::broadcast::channel(32);
        infra::runtime::LocalRuntimeSupervisor::new(
            Arc::new(self.config.clone()),
            self.repo.clone(),
            events,
        )
    }
    fn actor(&self) -> RuntimeControlActor {
        RuntimeControlActor {
            user_id: self.owner,
            idempotency_key: Uuid::new_v4().to_string(),
        }
    }
    async fn receipt(&self, id: Uuid) -> domain::RuntimeControlReceipt {
        self.repo
            .get_runtime_control(self.run.session_id, id)
            .await
            .unwrap()
    }
    async fn acknowledged(&self, id: Uuid) {
        tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                if self.receipt(id).await.acknowledgement.is_some() {
                    break;
                }
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
    }
    async fn seeded_context(&self) -> Uuid {
        let reserved = self
            .repo
            .reserve_runtime_control(
                &self.run,
                &self.actor(),
                RuntimeControlOperation::Stop,
                None,
            )
            .await
            .unwrap();
        let origin = format!("http://127.0.0.1:{}", self.agent.api_port.unwrap());
        let context = infra::runtime::control_outcome_wire::prepare(
            &reqwest::Client::new(),
            &origin,
            &self.producer.token,
            reserved.receipt.id,
            &self.producer.native,
            infra::runtime::control_outcome_wire::Request::Stop,
        )
        .await
        .unwrap();
        assert!(
            self.repo
                .claim_runtime_control_outcome(
                    reserved.receipt.id,
                    serde_json::to_value(context).unwrap()
                )
                .await
                .unwrap()
        );
        reserved.receipt.id
    }
}

#[tokio::test]
async fn approval_outcome_http_single_post_concurrent_once_deny_and_no_replay_effect() {
    for choice in [domain::ApprovalChoice::Once, domain::ApprovalChoice::Deny] {
        let Some(f) = setup(0).await else {
            return;
        };
        let approval = f.approval().await;
        let decision = f.reserve_approval(&approval, choice).await;
        let runtime = f.runtime();
        let (a, b) = tokio::join!(
            runtime.resolve_original_approval(&f.agent, &f.run, &approval, &decision),
            runtime.resolve_original_approval(&f.agent, &f.run, &approval, &decision)
        );
        a.unwrap();
        b.unwrap();
        let delivered = f.delivered(approval.id).await;
        assert_eq!(delivered.choice, choice);
        assert_eq!(f.producer.posts.load(Ordering::SeqCst), 1);
        let before = f.producer.caps_reads.load(Ordering::SeqCst);
        let replay = runtime
            .resolve_original_approval(&f.agent, &f.run, &approval, &decision)
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(replay).unwrap(),
            serde_json::to_value(delivered).unwrap()
        );
        assert_eq!(f.producer.caps_reads.load(Ordering::SeqCst), before);
        assert_eq!(f.producer.posts.load(Ordering::SeqCst), 1);
        let count=f.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT count(*) AS count FROM audit_log WHERE entity_id=$1 AND action='approval.decision_delivered'",
            [decision.id.to_string().into()])).await.unwrap().unwrap().try_get::<i64>("","count").unwrap();
        assert_eq!(count, 1);
    }
}

#[tokio::test]
async fn approval_outcome_http_lost_ack_get_recovery_with_new_repository_and_supervisor() {
    let Some(f) = setup(1).await else {
        return;
    };
    let approval = f.approval().await;
    let decision = f
        .reserve_approval(&approval, domain::ApprovalChoice::Once)
        .await;
    *f.producer.fault.lock().unwrap() = Some((StatusCode::NOT_FOUND, json!({"error":"held"})));
    assert!(
        f.runtime()
            .resolve_original_approval(&f.agent, &f.run, &approval, &decision)
            .await
            .is_err()
    );
    let saved = f
        .repo
        .get_approval_outcome(decision.id)
        .await
        .unwrap()
        .unwrap()
        .context;
    let repo = Arc::new(PostgresFleetRepository::new(
        sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
            .await
            .unwrap(),
    ));
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime =
        infra::runtime::LocalRuntimeSupervisor::new(Arc::new(f.config.clone()), repo, events);
    *f.producer.fault.lock().unwrap() = None;
    f.delivered(approval.id).await;
    assert_eq!(
        runtime
            .resolve_original_approval(&f.agent, &f.run, &approval, &decision)
            .await
            .unwrap()
            .state,
        domain::ApprovalDecisionState::Delivered
    );
    assert_eq!(
        f.repo
            .get_approval_outcome(decision.id)
            .await
            .unwrap()
            .unwrap()
            .context,
        saved
    );
    assert_eq!(f.producer.posts.load(Ordering::SeqCst), 1);
    assert_eq!(f.producer.caps_reads.load(Ordering::SeqCst), 1);
    assert!(f.producer.reads.load(Ordering::SeqCst) > 0);
}

#[tokio::test]
async fn approval_outcome_http_bad_capability_pending_request_and_legacy_cannot_send() {
    for change in ["capability", "request", "session", "legacy"] {
        let Some(f) = setup(0).await else {
            return;
        };
        let approval = f.approval().await;
        let req = domain::ApprovalDecisionRequest {
            choice: domain::ApprovalChoice::Once,
            idempotency_key: Uuid::new_v4().to_string(),
        };
        let decision = if change == "legacy" {
            f.repo
                .reserve_approval_decision(f.run.session_id, approval.id, f.owner, req)
                .await
                .unwrap()
                .decision
        } else {
            f.repo
                .reserve_original_approval_decision(f.run.session_id, approval.id, f.owner, req)
                .await
                .unwrap()
                .decision
        };
        match change {
            "capability" => f.producer.bad_caps.store(true, Ordering::SeqCst),
            "request" => {
                f.producer.status.lock().unwrap()["approval"]["request_id"] = json!("other")
            }
            "session" => f.producer.status.lock().unwrap()["session_id"] = json!("foreign"),
            _ => (),
        }
        assert!(
            f.runtime()
                .resolve_original_approval(&f.agent, &f.run, &approval, &decision)
                .await
                .is_err(),
            "{change}"
        );
        assert_eq!(f.producer.posts.load(Ordering::SeqCst), 0, "{change}");
        assert!(
            f.repo
                .get_approval_outcome(decision.id)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            f.repo
                .approval_decision(f.run.session_id, approval.id)
                .await
                .unwrap()
                .state,
            domain::ApprovalDecisionState::Uncertain
        );
    }
}

#[tokio::test]
async fn approval_outcome_http_unknown_witness_cannot_complete_or_release_claim() {
    let Some(f) = setup(2).await else {
        return;
    };
    let approval = f.approval().await;
    let decision = f
        .reserve_approval(&approval, domain::ApprovalChoice::Once)
        .await;
    let runtime = f.runtime();
    assert!(
        runtime
            .resolve_original_approval(&f.agent, &f.run, &approval, &decision)
            .await
            .is_err()
    );
    tokio::time::timeout(Duration::from_secs(15), async {
        while f.producer.reads.load(Ordering::SeqCst) == 0 {
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        runtime
            .resolve_original_approval(&f.agent, &f.run, &approval, &decision)
            .await
            .unwrap()
            .state,
        domain::ApprovalDecisionState::Uncertain
    );
    assert!(
        f.repo
            .fail_undispatched_approval_decision(decision.id)
            .await
            .is_err()
    );
    assert_eq!(f.producer.posts.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn approval_outcome_http_foreign_epoch_cannot_be_adopted_and_recovery_is_get_only() {
    let Some(f) = setup(1).await else {
        return;
    };
    let approval = f.approval().await;
    let decision = f
        .reserve_approval(&approval, domain::ApprovalChoice::Deny)
        .await;
    *f.producer.fault.lock().unwrap() = Some((StatusCode::NOT_FOUND, json!({"error":"held"})));
    let runtime = f.runtime();
    assert!(
        runtime
            .resolve_original_approval(&f.agent, &f.run, &approval, &decision)
            .await
            .is_err()
    );
    let saved = f
        .repo
        .get_approval_outcome(decision.id)
        .await
        .unwrap()
        .unwrap()
        .context;
    let mut foreign = f.producer.records.lock().unwrap()[&decision.id].clone();
    foreign["store_id"] = json!(Uuid::new_v4());
    *f.producer.fault.lock().unwrap() = Some((StatusCode::OK, foreign));
    let reads = f.producer.reads.load(Ordering::SeqCst);
    tokio::time::timeout(Duration::from_secs(15), async {
        while f.producer.reads.load(Ordering::SeqCst) == reads {
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        f.repo
            .approval_decision(f.run.session_id, approval.id)
            .await
            .unwrap()
            .state,
        domain::ApprovalDecisionState::Uncertain
    );
    *f.producer.fault.lock().unwrap() = None;
    f.delivered(approval.id).await;
    assert_eq!(
        f.repo
            .get_approval_outcome(decision.id)
            .await
            .unwrap()
            .unwrap()
            .context,
        saved
    );
    assert_eq!(f.producer.posts.load(Ordering::SeqCst), 1);
    assert_eq!(f.producer.caps_reads.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn approval_outcome_http_db_ack_failure_keeps_hold_then_recovers_without_post() {
    let Some(f) = setup(0).await else {
        return;
    };
    let approval = f.approval().await;
    let decision = f
        .reserve_approval(&approval, domain::ApprovalChoice::Once)
        .await;
    f.db.execute_unprepared("CREATE FUNCTION fail_approval_http_ack() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
        IF NEW.action='approval.decision_delivered' THEN RAISE EXCEPTION 'owned approval ACK fault';END IF;RETURN NEW;END $$;
        CREATE TRIGGER fail_approval_http_ack BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION fail_approval_http_ack();").await.unwrap();
    let runtime = f.runtime();
    assert!(
        runtime
            .resolve_original_approval(&f.agent, &f.run, &approval, &decision)
            .await
            .is_err()
    );
    assert_eq!(
        f.repo
            .approval_decision(f.run.session_id, approval.id)
            .await
            .unwrap()
            .state,
        domain::ApprovalDecisionState::Uncertain
    );
    f.db.execute_unprepared(
        "DROP TRIGGER fail_approval_http_ack ON audit_log;DROP FUNCTION fail_approval_http_ack();",
    )
    .await
    .unwrap();
    f.delivered(approval.id).await;
    assert_eq!(f.producer.posts.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn approval_outcome_http_late_ack_after_cancel_and_revocation_preserves_terminal_history() {
    let Some(f) = setup(1).await else {
        return;
    };
    let approval = f.approval().await;
    let decision = f
        .reserve_approval(&approval, domain::ApprovalChoice::Once)
        .await;
    *f.producer.fault.lock().unwrap() = Some((StatusCode::NOT_FOUND, json!({"error":"held"})));
    let runtime = f.runtime();
    assert!(
        runtime
            .resolve_original_approval(&f.agent, &f.run, &approval, &decision)
            .await
            .is_err()
    );
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
            runtime_run_id: f.producer.native.clone(),
            runtime_session_id: "control-outcome-http-session".into(),
            state: SessionRunState::Completed,
            body: None,
            error: None,
        })
        .await
        .unwrap();
    f.repo
        .resolve_runtime_approval_requests_for_run(
            f.run.id,
            domain::ResolveRuntimeApprovalRequest {
                choice: "cancelled".into(),
                resolve_all: false,
            },
            f.owner,
        )
        .await
        .unwrap();
    let before =
        serde_json::to_value(f.repo.get_session_agent_run(f.run.id).await.unwrap()).unwrap();
    f.db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE users SET is_active=false WHERE id=$1",
        [f.owner.into()],
    ))
    .await
    .unwrap();
    *f.producer.fault.lock().unwrap() = None;
    f.delivered(approval.id).await;
    assert_eq!(
        serde_json::to_value(f.repo.get_session_agent_run(f.run.id).await.unwrap()).unwrap(),
        before
    );
    assert_eq!(
        f.repo
            .list_session_approvals(f.run.session_id)
            .await
            .unwrap()[0]
            .state,
        domain::RuntimeApprovalState::Cancelled
    );
    assert_eq!(f.producer.posts.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn approval_outcome_http_disabled_or_rotated_context_never_probes_or_backfills() {
    for rotated in [false, true] {
        let Some(mut f) = setup(1).await else {
            return;
        };
        let approval = f.approval().await;
        let decision = f
            .reserve_approval(&approval, domain::ApprovalChoice::Once)
            .await;
        // Send without starting background workers, then restart with changed deployment facts.
        let mut config = f.config.clone();
        config.fleet.hermes_control_outcome_enabled = false;
        let (events, _) = tokio::sync::broadcast::channel(32);
        let disabled =
            infra::runtime::LocalRuntimeSupervisor::new(Arc::new(config), f.repo.clone(), events);
        assert!(
            disabled
                .resolve_original_approval(&f.agent, &f.run, &approval, &decision)
                .await
                .is_err()
        );
        assert_eq!(f.producer.caps_reads.load(Ordering::SeqCst), 0);
        let origin = format!("http://127.0.0.1:{}", f.agent.api_port.unwrap());
        let context = infra::runtime::control_outcome_wire::prepare(
            &reqwest::Client::new(),
            &origin,
            &f.producer.token,
            decision.id,
            &f.producer.native,
            infra::runtime::control_outcome_wire::Request::Approval {
                request_id: "request_one",
                choice: decision.choice,
            },
        )
        .await
        .unwrap();
        f.repo
            .claim_approval_outcome(decision.id, serde_json::to_value(context).unwrap())
            .await
            .unwrap();
        if rotated {
            f.config.fleet.runtime_token_secret = "rotated-approval-outcome-fixture".into();
        } else {
            f.config.fleet.hermes_control_outcome_enabled = false;
        }
        let _runtime = f.runtime();
        sleep(Duration::from_secs(6)).await;
        assert_eq!(f.producer.reads.load(Ordering::SeqCst), 0);
        assert_eq!(f.producer.posts.load(Ordering::SeqCst), 0);
        assert_eq!(f.producer.caps_reads.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn approval_outcome_http_normal_ack_races_original_lookup_with_one_atomic_delivery() {
    let Some(f) = setup(3).await else {
        return;
    };
    let approval = f.approval().await;
    let decision = f
        .reserve_approval(&approval, domain::ApprovalChoice::Once)
        .await;
    let runtime = f.runtime();
    let send = runtime.resolve_original_approval(&f.agent, &f.run, &approval, &decision);
    let release = async {
        f.producer.started.notified().await;
        let saved = f
            .repo
            .get_approval_outcome(decision.id)
            .await
            .unwrap()
            .unwrap();
        let context = serde_json::from_value(saved.context.clone()).unwrap();
        let origin = format!("http://127.0.0.1:{}", f.agent.api_port.unwrap());
        assert_eq!(
            infra::runtime::control_outcome_wire::lookup(
                &reqwest::Client::new(),
                &context,
                &origin,
                &f.producer.token
            )
            .await
            .unwrap(),
            infra::runtime::control_outcome_wire::Outcome::Acknowledged(
                infra::runtime::control_outcome_wire::Acknowledgement::ApprovalResolved
            )
        );
        f.producer.release.notify_one();
        f.repo
            .finish_approval_outcome(decision.id, saved.context)
            .await
            .unwrap();
    };
    let (result, _) = tokio::time::timeout(Duration::from_secs(25), async {
        tokio::join!(send, release)
    })
    .await
    .unwrap();
    assert_eq!(
        result.unwrap().state,
        domain::ApprovalDecisionState::Delivered
    );
    assert_eq!(f.producer.posts.load(Ordering::SeqCst), 1);
    let count=f.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT count(*) AS count FROM audit_log WHERE entity_id=$1 AND action='approval.decision_delivered'",
        [decision.id.to_string().into()])).await.unwrap().unwrap().try_get::<i64>("","count").unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn approval_outcome_http_api_reserves_original_mode_denies_foreign_and_replay_never_sends() {
    for bad_caps in [false, true] {
        let Some(f) = setup(0).await else {
            return;
        };
        let approval = f.approval().await;
        f.producer.bad_caps.store(bad_caps, Ordering::SeqCst);
        let (events, _) = tokio::sync::broadcast::channel(32);
        let (restart_tx, _) = tokio::sync::mpsc::channel(1);
        let ctx = Arc::new(app::AppContext::new(
            Arc::new(f.config.clone()),
            f.repo.clone(),
            Arc::new(infra::FilesystemProvisioner),
            Arc::new(f.runtime()),
            events,
            restart_tx,
        ));
        let token = ctx
            .auth
            .issue_tokens(&f.repo.find_user_by_id(f.owner).await.unwrap().unwrap())
            .unwrap()
            .response
            .access_token;
        let stranger=f.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT id FROM users WHERE id<>$1 AND is_active AND system_role='user' ORDER BY id LIMIT 1",
            [f.owner.into()])).await.unwrap().unwrap().try_get::<Uuid>("","id").unwrap();
        let stranger_token = ctx
            .auth
            .issue_tokens(&f.repo.find_user_by_id(stranger).await.unwrap().unwrap())
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
            f.run.session_id,
            approval.id
        );
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let body = json!({"choice":"once","idempotency_key":Uuid::new_v4().to_string()});
        assert_eq!(
            client
                .post(&url)
                .bearer_auth(&stranger_token)
                .json(&body)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
        assert!(
            f.repo
                .approval_decision(f.run.session_id, approval.id)
                .await
                .is_err()
        );
        let response = client
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let payload = response.json::<Value>().await.unwrap();
        for private in [
            "context",
            "outcome_required",
            "submission_claimed",
            "request_body",
            "credential_fingerprint",
        ] {
            assert!(payload.get(private).is_none());
        }
        let decision: domain::ApprovalDecision = serde_json::from_value(payload).unwrap();
        assert_eq!(
            decision.state,
            if bad_caps {
                domain::ApprovalDecisionState::Uncertain
            } else {
                domain::ApprovalDecisionState::Delivered
            }
        );
        let original =
            f.db.query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT outcome_required FROM runtime_approval_decisions WHERE id=$1",
                [decision.id.into()],
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get::<bool>("", "outcome_required")
            .unwrap();
        assert!(original);
        f.producer.bad_caps.store(false, Ordering::SeqCst);
        let calls = f.producer.caps_reads.load(Ordering::SeqCst);
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
        assert_eq!(
            serde_json::to_value(replay).unwrap(),
            serde_json::to_value(&decision).unwrap()
        );
        assert_eq!(f.producer.caps_reads.load(Ordering::SeqCst), calls);
        assert_eq!(
            f.producer.posts.load(Ordering::SeqCst),
            usize::from(!bad_caps)
        );
        let conflict = json!({"choice":"deny","idempotency_key":body["idempotency_key"]});
        assert_eq!(
            client
                .post(&url)
                .bearer_auth(&token)
                .json(&conflict)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            client
                .get(&url)
                .bearer_auth(&stranger_token)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
        server.abort();
    }
}

async fn setup(mode: usize) -> Option<HttpFixture> {
    setup_at(mode, None).await
}

async fn setup_at(mode: usize, url: Option<&str>) -> Option<HttpFixture> {
    let url = url
        .map(str::to_owned)
        .or_else(|| std::env::var("FLEET_TEST_DATABASE_URL").ok())?;
    let (repo, owner, _) = fixture_at(url.clone()).await;
    let id = agent(&repo).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let db = sea_orm::Database::connect(url).await.unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE agents SET api_port=$2 WHERE id=$1",
        [id.into(), i32::from(port).into()],
    ))
    .await
    .unwrap();
    let session = repo
        .create_session(chat(id, "control-outcome-http"), owner)
        .await
        .unwrap();
    let message = repo
        .create_session_message(session.id, prompt("control-http-prompt"), owner)
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE message_dispatch_outbox SET state='dispatching' WHERE message_id=$1",
        [message.id.into()],
    ))
    .await
    .unwrap();
    let mut config = AppConfig::default();
    config.fleet.runtime_token_secret = "owned-control-outcome-http-fixture".into();
    config.fleet.hermes_control_outcome_enabled = true;
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
    let native = format!("run_{}", Uuid::new_v4().simple());
    repo.accept_hermes_run(message.id, intent.run.id, native.clone())
        .await
        .unwrap();
    let (run, _) = repo
        .pin_hermes_run_session(
            intent.run.id,
            native.clone(),
            format!("fleet:{}:{id}", session.id),
            "control-outcome-http-session".into(),
        )
        .await
        .unwrap();
    let repo = Arc::new(repo);
    let producer = Arc::new(Producer {
        repo: repo.clone(),
        token,
        native: native.clone(),
        epoch: Uuid::new_v4().to_string(),
        bad_caps: AtomicBool::new(false),
        posts: AtomicUsize::new(0),
        reads: AtomicUsize::new(0),
        caps_reads: AtomicUsize::new(0),
        mode: AtomicUsize::new(mode),
        records: Mutex::new(HashMap::new()),
        fault: Mutex::new(None),
        status: Mutex::new(json!({"object":"hermes.run","run_id":native,
            "session_id":"control-outcome-http-session","status":"running"})),
        started: Notify::new(),
        release: Notify::new(),
    });
    let mut caps = hermes_protocol_fixture::capabilities();
    caps["features"]["run_steer"] = json!(true);
    caps["endpoints"]["run_steer"] = json!({"method":"POST","path":"/v1/runs/{run_id}/steer"});
    caps["features"]["run_approval_response"] = json!(true);
    caps["features"]["approval_events"] = json!(true);
    caps["endpoints"]["run_approval"] =
        json!({"method":"POST","path":"/v1/runs/{run_id}/approval"});
    let router =
        Router::new()
            .route("/health", get(|| async { Json(json!({"status":"ok"})) }))
            .route(
                "/v1/capabilities",
                get(move || {
                    let caps = caps.clone();
                    async { Json(caps) }
                }),
            )
            .route(
                "/v1/runs/{run_id}",
                get(
                    |State(p): State<Arc<Producer>>,
                     Path(run): Path<String>,
                     headers: HeaderMap| async move {
                        p.authenticated(&headers);
                        assert_eq!(run, p.native);
                        Json(p.status.lock().unwrap().clone())
                    },
                ),
            )
            .route(
                "/fleet/v1/controls/capabilities",
                get(
                    |State(p): State<Arc<Producer>>, headers: HeaderMap| async move {
                        p.authenticated(&headers);
                        p.caps_reads.fetch_add(1, Ordering::SeqCst);
                        let mut caps = p.caps();
                        if p.bad_caps.load(Ordering::SeqCst) {
                            caps["single_send"] = json!(false);
                        }
                        Json(caps)
                    },
                ),
            )
            .route("/fleet/v1/controls/lookup", get(control_lookup))
            .route("/v1/runs/{run_id}/{operation}", post(control_post))
            .with_state(producer.clone());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let agent = repo.get_agent(id).await.unwrap();
    Some(HttpFixture {
        repo,
        db,
        owner,
        run,
        agent,
        config,
        producer,
        server,
    })
}

#[tokio::test]
async fn control_outcome_http_single_post_persists_context_before_effect_and_replay() {
    for steer in [false, true] {
        let Some(f) = setup(0).await else {
            return;
        };
        let runtime = f.runtime();
        let actor = f.actor();
        let result = if steer {
            runtime
                .steer_run(
                    &f.agent,
                    &f.run,
                    SteerSessionRunRequest {
                        input: "precise \"guidance\"\nnext".into(),
                    },
                    actor.clone(),
                )
                .await
                .unwrap()
        } else {
            runtime
                .stop_run(&f.agent, &f.run, actor.clone())
                .await
                .unwrap()
        };
        assert!(result.accepted);
        let receipt = result.command.unwrap();
        assert_eq!(
            receipt.acknowledgement.as_deref(),
            Some(if steer { "steered" } else { "stopping" })
        );
        let replay = if steer {
            runtime
                .steer_run(
                    &f.agent,
                    &f.run,
                    SteerSessionRunRequest {
                        input: "precise \"guidance\"\nnext".into(),
                    },
                    actor,
                )
                .await
                .unwrap()
        } else {
            runtime.stop_run(&f.agent, &f.run, actor).await.unwrap()
        };
        assert_eq!(
            serde_json::to_value(replay.command.unwrap()).unwrap(),
            serde_json::to_value(receipt).unwrap()
        );
        assert_eq!(f.producer.posts.load(Ordering::SeqCst), 1);
        assert_eq!(f.producer.caps_reads.load(Ordering::SeqCst), 1);
        assert!(matches!(
            f.repo.get_session_agent_run(f.run.id).await.unwrap().state,
            SessionRunState::Running | SessionRunState::Stopping
        ));
    }
}

#[tokio::test]
async fn control_outcome_http_lost_ack_recovers_by_original_get_across_fresh_supervisors() {
    let Some(f) = setup(1).await else {
        return;
    };
    *f.producer.fault.lock().unwrap() = Some((StatusCode::NOT_FOUND, json!({"error":"held"})));
    let actor = f.actor();
    let result = f
        .runtime()
        .stop_run(&f.agent, &f.run, actor.clone())
        .await
        .unwrap();
    assert!(!result.accepted);
    let id = result.command.unwrap().id;
    assert_eq!(f.receipt(id).await.state, RuntimeControlState::Uncertain);
    let reopened = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime = infra::runtime::LocalRuntimeSupervisor::new(
        Arc::new(f.config.clone()),
        Arc::new(PostgresFleetRepository::new(reopened)),
        events,
    );
    *f.producer.fault.lock().unwrap() = None;
    f.acknowledged(id).await;
    assert!(
        runtime
            .stop_run(&f.agent, &f.run, actor)
            .await
            .unwrap()
            .accepted
    );
    assert_eq!(f.producer.posts.load(Ordering::SeqCst), 1);
    assert_eq!(f.producer.caps_reads.load(Ordering::SeqCst), 1);
    assert!(f.producer.reads.load(Ordering::SeqCst) > 0);
    assert_eq!(
        f.repo.get_session_agent_run(f.run.id).await.unwrap().state,
        SessionRunState::Stopping
    );
}

#[tokio::test]
async fn control_outcome_http_normal_ack_and_readback_race_commit_once() {
    let Some(f) = setup(3).await else {
        return;
    };
    let runtime = f.runtime();
    let send = runtime.stop_run(&f.agent, &f.run, f.actor());
    let release = async {
        f.producer.started.notified().await;
        let id = *f.producer.records.lock().unwrap().keys().next().unwrap();
        let saved = f
            .repo
            .get_runtime_control_outcome(id)
            .await
            .unwrap()
            .unwrap();
        let context = serde_json::from_value(saved.context.clone()).unwrap();
        let origin = format!("http://127.0.0.1:{}", f.agent.api_port.unwrap());
        let witness = infra::runtime::control_outcome_wire::lookup(
            &reqwest::Client::new(),
            &context,
            &origin,
            &f.producer.token,
        )
        .await
        .unwrap();
        assert!(matches!(
            witness,
            infra::runtime::control_outcome_wire::Outcome::Acknowledged(
                infra::runtime::control_outcome_wire::Acknowledgement::Stopping
            )
        ));
        // Race keyed GET/DB settlement against the normal response, not queue traversal time.
        f.producer.release.notify_one();
        f.repo
            .finish_runtime_control_outcome(id, saved.context, "stopping")
            .await
            .unwrap();
    };
    let (result, _) = tokio::time::timeout(Duration::from_secs(25), async {
        tokio::join!(send, release)
    })
    .await
    .unwrap();
    assert!(result.unwrap().accepted);
    assert_eq!(f.producer.posts.load(Ordering::SeqCst), 1);
    let count=f.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT count(*) AS count FROM audit_log WHERE action='runtime_control.acknowledged' AND payload->>'run_id'=$1",
        [f.run.id.to_string().into()])).await.unwrap().unwrap().try_get::<i64>("","count").unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn control_outcome_http_bad_capability_denies_before_claim_and_never_falls_back() {
    let Some(f) = setup(0).await else {
        return;
    };
    f.producer.bad_caps.store(true, Ordering::SeqCst);
    assert!(
        f.runtime()
            .stop_run(&f.agent, &f.run, f.actor())
            .await
            .is_err()
    );
    assert_eq!(f.producer.posts.load(Ordering::SeqCst), 0);
    let commands = f
        .repo
        .list_runtime_controls(f.run.session_id, f.run.id)
        .await
        .unwrap();
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].state, RuntimeControlState::Rejected);
    assert!(
        f.repo
            .get_runtime_control_outcome(commands[0].id)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn control_outcome_http_unknown_lookup_retains_claim_and_never_reposts() {
    let Some(f) = setup(2).await else {
        return;
    };
    let runtime = f.runtime();
    let actor = f.actor();
    let response = runtime
        .stop_run(&f.agent, &f.run, actor.clone())
        .await
        .unwrap();
    let id = response.command.unwrap().id;
    tokio::time::timeout(Duration::from_secs(15), async {
        while f.producer.reads.load(Ordering::SeqCst) == 0 {
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(f.receipt(id).await.state, RuntimeControlState::Uncertain);
    assert!(
        !runtime
            .stop_run(&f.agent, &f.run, actor)
            .await
            .unwrap()
            .accepted
    );
    assert_eq!(f.producer.posts.load(Ordering::SeqCst), 1);
    assert!(runtime.stop_run(&f.agent, &f.run, f.actor()).await.is_err());
    assert_eq!(
        f.repo.get_session_agent_run(f.run.id).await.unwrap().state,
        SessionRunState::Running
    );
}

#[tokio::test]
async fn control_outcome_http_db_ack_failure_recovers_after_fault_without_second_post() {
    let Some(f) = setup(0).await else {
        return;
    };
    f.db.execute_unprepared("CREATE FUNCTION fail_control_http_ack() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
        IF NEW.action='runtime_control.acknowledged' THEN RAISE EXCEPTION 'owned control outcome fault'; END IF;RETURN NEW;END $$;
        CREATE TRIGGER fail_control_http_ack BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION fail_control_http_ack();").await.unwrap();
    let result = f
        .runtime()
        .stop_run(&f.agent, &f.run, f.actor())
        .await
        .unwrap();
    let id = result.command.unwrap().id;
    assert!(!result.accepted);
    assert_eq!(f.receipt(id).await.state, RuntimeControlState::Uncertain);
    assert_eq!(
        f.repo.get_session_agent_run(f.run.id).await.unwrap().state,
        SessionRunState::Running
    );
    f.db.execute_unprepared(
        "DROP TRIGGER fail_control_http_ack ON audit_log;DROP FUNCTION fail_control_http_ack();",
    )
    .await
    .unwrap();
    f.acknowledged(id).await;
    assert_eq!(f.producer.posts.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn control_outcome_http_rotated_credentials_or_disabled_worker_do_not_probe_or_backfill() {
    for rotated in [false, true] {
        let Some(mut f) = setup(0).await else {
            return;
        };
        let id = f.seeded_context().await;
        if rotated {
            f.config.fleet.runtime_token_secret = "rotated-control-fixture".into();
        } else {
            f.config.fleet.hermes_control_outcome_enabled = false;
        }
        let _runtime = f.runtime();
        sleep(Duration::from_secs(6)).await;
        assert_eq!(f.producer.reads.load(Ordering::SeqCst), 0);
        assert_eq!(f.producer.posts.load(Ordering::SeqCst), 0);
        assert_eq!(f.receipt(id).await.state, RuntimeControlState::Submitted);
        assert_eq!(f.producer.caps_reads.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn control_outcome_http_late_ack_preserves_terminal_history_after_actor_revocation() {
    let Some(f) = setup(1).await else {
        return;
    };
    *f.producer.fault.lock().unwrap() = Some((StatusCode::NOT_FOUND, json!({"error":"held"})));
    let actor = f.actor();
    let response = f
        .runtime()
        .stop_run(&f.agent, &f.run, actor.clone())
        .await
        .unwrap();
    let id = response.command.unwrap().id;
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
            runtime_run_id: f.producer.native.clone(),
            runtime_session_id: "control-outcome-http-session".into(),
            state: SessionRunState::Completed,
            body: Some("independent terminal fixture".into()),
            error: None,
        })
        .await
        .unwrap();
    f.repo.reconcile_runtime_controls().await.unwrap();
    let before = f.receipt(id).await;
    assert_eq!(before.state, RuntimeControlState::TerminalObserved);
    let run_before =
        serde_json::to_value(f.repo.get_session_agent_run(f.run.id).await.unwrap()).unwrap();
    f.db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE users SET is_active=false WHERE id=$1",
        [f.owner.into()],
    ))
    .await
    .unwrap();
    *f.producer.fault.lock().unwrap() = None;
    f.acknowledged(id).await;
    let after = f.receipt(id).await;
    assert_eq!(after.state, before.state);
    assert_eq!(after.observed_run_state, before.observed_run_state);
    assert_eq!(after.updated_at, before.updated_at);
    assert_eq!(after.acknowledgement.as_deref(), Some("stopping"));
    assert_eq!(
        serde_json::to_value(f.repo.get_session_agent_run(f.run.id).await.unwrap()).unwrap(),
        run_before
    );
    assert!(f.runtime().stop_run(&f.agent, &f.run, actor).await.is_err());
    assert_eq!(f.producer.posts.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn control_outcome_http_foreign_epoch_never_adopts_a_new_witness() {
    let Some(f) = setup(1).await else {
        return;
    };
    *f.producer.fault.lock().unwrap() = Some((StatusCode::NOT_FOUND, json!({"error":"held"})));
    let result = f
        .runtime()
        .stop_run(&f.agent, &f.run, f.actor())
        .await
        .unwrap();
    let id = result.command.unwrap().id;
    let saved = f
        .repo
        .get_runtime_control_outcome(id)
        .await
        .unwrap()
        .unwrap()
        .context;
    let mut foreign = f.producer.records.lock().unwrap()[&id].clone();
    foreign["store_id"] = json!(Uuid::new_v4());
    *f.producer.fault.lock().unwrap() = Some((StatusCode::OK, foreign));
    let reads = f.producer.reads.load(Ordering::SeqCst);
    tokio::time::timeout(Duration::from_secs(15), async {
        while f.producer.reads.load(Ordering::SeqCst) == reads {
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(f.receipt(id).await.state, RuntimeControlState::Uncertain);
    assert_eq!(
        f.repo
            .get_runtime_control_outcome(id)
            .await
            .unwrap()
            .unwrap()
            .context,
        saved
    );
    *f.producer.fault.lock().unwrap() = None;
    f.acknowledged(id).await;
    assert_eq!(f.producer.posts.load(Ordering::SeqCst), 1);
    assert_eq!(f.producer.caps_reads.load(Ordering::SeqCst), 1);
}

#[tokio::test]
#[ignore = "requires its own disposable database for one hundred retained invalid contexts"]
async fn control_outcome_http_keyset_reaches_valid_witness_after_one_hundred_invalid_contexts() {
    let url = std::env::var("FLEET_CONTROL_OUTCOME_KEYSET_TEST_DATABASE_URL")
        .expect("owned keyset database is required");
    let db = sea_orm::Database::connect(&url).await.unwrap();
    let empty=db.query_one(Statement::from_string(DatabaseBackend::Postgres,
        "SELECT NOT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_type='BASE TABLE') AS empty"))
        .await.unwrap().unwrap().try_get::<bool>("","empty").unwrap();
    assert!(empty, "keyset requires its own empty disposable database");
    let mut candidates = Vec::new();
    let mut highest: Option<(Uuid, HttpFixture)> = None;
    for _ in 0..101 {
        let Some(f) = setup_at(0, Some(&url)).await else {
            return;
        };
        let reserved = f
            .repo
            .reserve_runtime_control(&f.run, &f.actor(), RuntimeControlOperation::Stop, None)
            .await
            .unwrap();
        let origin = format!("http://127.0.0.1:{}", f.agent.api_port.unwrap());
        let context = infra::runtime::control_outcome_wire::prepare(
            &reqwest::Client::new(),
            &origin,
            &f.producer.token,
            reserved.receipt.id,
            &f.producer.native,
            infra::runtime::control_outcome_wire::Request::Stop,
        )
        .await
        .unwrap();
        let id = reserved.receipt.id;
        candidates.push((id, serde_json::to_value(context).unwrap()));
        if highest.as_ref().is_none_or(|(previous, _)| id > *previous) {
            highest = Some((id, f));
        }
    }
    let (id, f) = highest.unwrap();
    for (candidate, mut context) in candidates {
        // Structurally valid old metadata is not bearer authority. It must fail before HTTP.
        if candidate != id {
            context["capabilities"]["scope_fingerprint"] = json!("0".repeat(64));
        }
        assert!(
            f.repo
                .claim_runtime_control_outcome(candidate, context.clone())
                .await
                .unwrap()
        );
        if candidate == id {
            f.producer.records.lock().unwrap().insert(id,json!({"object":"fleet.hermes.controls.lookup","contract_version":1,
                "store_id":f.producer.epoch,"scope_fingerprint":context["capabilities"]["scope_fingerprint"],"profile":"default",
                "command_id":id,"run_id":f.producer.native,"operation":"stop","request_sha256":context["request_sha256"],
                "state":"acknowledged","ack":{"run_id":f.producer.native,"status":"stopping"}}));
        }
    }
    let first = f.repo.list_runtime_control_outcomes(None).await.unwrap();
    assert_eq!(first.len(), 100);
    assert!(first.iter().all(|row| row.receipt.id != id));
    let _runtime = f.runtime();
    f.acknowledged(id).await;
    assert_eq!(f.producer.posts.load(Ordering::SeqCst), 0);
    assert_eq!(f.producer.caps_reads.load(Ordering::SeqCst), 1);
    assert!(f.producer.reads.load(Ordering::SeqCst) > 0);
}

#[tokio::test]
#[ignore = "requires its own disposable approval keyset database"]
async fn approval_outcome_http_keyset_reaches_original_witness_after_one_hundred_invalid_contexts()
{
    let url = std::env::var("FLEET_APPROVAL_OUTCOME_KEYSET_TEST_DATABASE_URL")
        .expect("owned approval keyset database is required");
    let db = sea_orm::Database::connect(&url).await.unwrap();
    let empty=db.query_one(Statement::from_string(DatabaseBackend::Postgres,
        "SELECT NOT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_type='BASE TABLE') AS empty"))
        .await.unwrap().unwrap().try_get::<bool>("","empty").unwrap();
    assert!(
        empty,
        "approval keyset requires its own empty disposable database"
    );
    let mut candidates = Vec::new();
    let mut highest: Option<(Uuid, domain::RuntimeApprovalRequest, HttpFixture)> = None;
    for _ in 0..101 {
        let f = setup_at(0, Some(&url)).await.unwrap();
        let approval = f.approval().await;
        let decision = f
            .reserve_approval(&approval, domain::ApprovalChoice::Once)
            .await;
        let origin = format!("http://127.0.0.1:{}", f.agent.api_port.unwrap());
        let context = infra::runtime::control_outcome_wire::prepare(
            &reqwest::Client::new(),
            &origin,
            &f.producer.token,
            decision.id,
            &f.producer.native,
            infra::runtime::control_outcome_wire::Request::Approval {
                request_id: "request_one",
                choice: decision.choice,
            },
        )
        .await
        .unwrap();
        candidates.push((decision.id, serde_json::to_value(context).unwrap()));
        if highest
            .as_ref()
            .is_none_or(|(previous, _, _)| decision.id > *previous)
        {
            highest = Some((decision.id, approval, f));
        }
    }
    let (id, approval, f) = highest.unwrap();
    for (candidate, mut context) in candidates {
        if candidate != id {
            context["capabilities"]["scope_fingerprint"] = json!("0".repeat(64));
        }
        assert!(
            f.repo
                .claim_approval_outcome(candidate, context.clone())
                .await
                .unwrap()
        );
        if candidate == id {
            f.producer.records.lock().unwrap().insert(id,json!({"object":"fleet.hermes.controls.lookup","contract_version":1,
                "store_id":f.producer.epoch,"scope_fingerprint":context["capabilities"]["scope_fingerprint"],"profile":"default",
                "command_id":id,"run_id":f.producer.native,"operation":"approval","request_sha256":context["request_sha256"],
                "state":"acknowledged","ack":{"object":"hermes.run.approval_response","run_id":f.producer.native,
                    "request_id":"request_one","choice":"once","resolved":1}}));
        }
    }
    let first = f.repo.list_approval_outcomes(None).await.unwrap();
    assert_eq!(first.len(), 100);
    assert!(first.iter().all(|row| row.decision.id != id));
    let _runtime = f.runtime();
    f.delivered(approval.id).await;
    assert_eq!(f.producer.posts.load(Ordering::SeqCst), 0);
    assert_eq!(f.producer.caps_reads.load(Ordering::SeqCst), 1);
    assert!(f.producer.reads.load(Ordering::SeqCst) > 0);
}
