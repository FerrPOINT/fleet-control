use super::*;
use domain::{
    ApprovalChoice, ApprovalDecisionRequest, ApprovalDecisionState, RuntimeApprovalRequest,
};
use sea_orm::DatabaseConnection;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

struct Fixture {
    repo: PostgresFleetRepository,
    db: DatabaseConnection,
    owner: Uuid,
    run: domain::SessionAgentRun,
    approval: RuntimeApprovalRequest,
    fingerprint: String,
}

async fn setup() -> Option<Fixture> {
    let (repo, owner, _) = fixture().await?;
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let id = agent(&repo).await;
    let session = repo
        .create_session(chat(id, "approval-outcome"), owner)
        .await
        .unwrap();
    let message = repo
        .create_session_message(session.id, prompt("approval-outcome"), owner)
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE agents SET api_port=24003 WHERE id=$1",
        [id.into()],
    ))
    .await
    .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE message_dispatch_outbox SET state='dispatching' WHERE message_id=$1",
        [message.id.into()],
    ))
    .await
    .unwrap();
    let fingerprint = "a".repeat(64);
    repo.prepare_hermes_dispatch(app::HermesDispatchDraft {
        message_id: message.id,
        session_id: session.id,
        agent_id: id,
        run_role: SessionRunRole::Primary,
        requested_session_id: format!("fleet:{}:{id}", session.id),
        input: message.body,
        origin: "http://127.0.0.1:24003".into(),
        credential_fingerprint: fingerprint.clone(),
        capabilities: hermes_protocol_fixture::capabilities(),
    })
    .await
    .unwrap();
    let intent = repo
        .claim_hermes_submission(
            message.id,
            "http://127.0.0.1:24003".into(),
            fingerprint.clone(),
        )
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
            "approval-native-session".into(),
        )
        .await
        .unwrap();
    let approval = repo
        .upsert_runtime_approval_request(app::RuntimeApprovalCreate {
            session_id: session.id,
            session_run_id: run.id,
            agent_id: id,
            runtime_run_id: native,
            runtime_approval_id: Some("request_one".into()),
            prompt: "Exact owned approval".into(),
            detail: json!({}),
        })
        .await
        .unwrap();
    Some(Fixture {
        repo,
        db,
        owner,
        run,
        approval,
        fingerprint,
    })
}

impl Fixture {
    async fn reserve(&self, choice: ApprovalChoice, original: bool) -> (Uuid, Value) {
        let command = ApprovalDecisionRequest {
            choice,
            idempotency_key: Uuid::new_v4().to_string(),
        };
        let reserved = if original {
            self.repo
                .reserve_original_approval_decision(
                    self.run.session_id,
                    self.approval.id,
                    self.owner,
                    command,
                )
                .await
        } else {
            self.repo
                .reserve_approval_decision(
                    self.run.session_id,
                    self.approval.id,
                    self.owner,
                    command,
                )
                .await
        }
        .unwrap();
        assert!(reserved.dispatch);
        let body = json!({"request_id":self.approval.runtime_approval_id,
            "choice":choice,"resolve_all":false})
        .to_string();
        let context = json!({
            "origin":"http://127.0.0.1:24003", "credential_fingerprint":self.fingerprint,
            "command_id":reserved.decision.id, "run_id":self.approval.runtime_run_id,
            "operation":"approval", "request_body":body,
            "request_sha256":format!("{:x}",Sha256::digest(body.as_bytes())),
            "capabilities":{
                "object":"fleet.hermes.controls.capabilities", "contract_version":1,
                "store_id":Uuid::new_v4(), "scope_fingerprint":"b".repeat(64), "profile":"default",
                "native_source_revision":"bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3",
                "single_send":true, "non_dispatch_lookup":true,
                "lookup":{"method":"GET","path":"/fleet/v1/controls/lookup"},
                "operations":["steer","stop","approval"]
            }
        });
        (reserved.decision.id, context)
    }

    async fn audits(&self, id: Uuid, action: &str) -> i64 {
        self.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT count(*) AS count FROM audit_log WHERE entity_type='approval_decision' AND entity_id=$1 AND action=$2",
            [id.to_string().into(),action.into()])).await.unwrap().unwrap().try_get("", "count").unwrap()
    }

    async fn decision(&self) -> domain::ApprovalDecision {
        self.repo
            .approval_decision(self.run.session_id, self.approval.id)
            .await
            .unwrap()
    }

    async fn terminal(&self) {
        let message_id: Uuid = self
            .db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT message_id FROM hermes_dispatch_journal WHERE run_id=$1",
                [self.run.id.into()],
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "message_id")
            .unwrap();
        self.repo
            .commit_hermes_terminal(app::HermesTerminalCommit {
                message_id,
                run_id: self.run.id,
                runtime_run_id: self.approval.runtime_run_id.clone(),
                runtime_session_id: "approval-native-session".into(),
                state: SessionRunState::Completed,
                body: Some("independent terminal".into()),
                error: None,
            })
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn approval_outcome_claim_is_atomic_single_use_and_restart_readable() {
    let Some(f) = setup().await else { return };
    let (id, context) = f.reserve(ApprovalChoice::Once, true).await;
    let (a, b) = tokio::join!(
        f.repo.claim_approval_outcome(id, context.clone()),
        f.repo.claim_approval_outcome(id, context.clone())
    );
    assert_eq!(usize::from(a.unwrap()) + usize::from(b.unwrap()), 1);
    let restarted = PostgresFleetRepository::new(
        sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
            .await
            .unwrap(),
    );
    let intent = restarted.get_approval_outcome(id).await.unwrap().unwrap();
    assert_eq!(intent.context, context);
    assert_eq!(intent.decision.state, ApprovalDecisionState::Uncertain);
    let public = serde_json::to_string(&intent.decision).unwrap();
    assert!(!public.contains("credential_fingerprint"));
    assert!(!public.contains("request_body"));
    assert!(!public.contains("capabilities"));
    assert_eq!(f.audits(id, "approval.decision_submitted").await, 1);
    let page = restarted
        .list_approval_outcomes(Some(Uuid::nil()))
        .await
        .unwrap();
    assert!(page.len() <= 100);
    assert!(page.windows(2).all(|w| w[0].decision.id < w[1].decision.id));
    assert!(
        restarted
            .list_approval_outcomes(Some(id))
            .await
            .unwrap()
            .iter()
            .all(|i| i.decision.id > id)
    );
    assert!(
        restarted
            .get_approval_outcome(Uuid::new_v4())
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn approval_outcome_context_rejects_changed_scope_action_and_legacy_backfill() {
    let Some(f) = setup().await else { return };
    let (id, context) = f.reserve(ApprovalChoice::Deny, true).await;
    for (field, value) in [
        ("command_id", json!(Uuid::new_v4())),
        ("run_id", json!("run_wrong")),
        ("origin", json!("http://127.0.0.1:24004")),
        ("operation", json!("steer")),
        ("credential_fingerprint", json!("c".repeat(64))),
        ("extra", json!(true)),
    ] {
        let mut bad = context.clone();
        bad[field] = value;
        assert!(
            f.repo.claim_approval_outcome(id, bad).await.is_err(),
            "{field}"
        );
    }
    for body in [
        json!({"request_id":"request_wrong","choice":"deny","resolve_all":false}),
        json!({"request_id":"request_one","choice":"once","resolve_all":false}),
        json!({"request_id":"request_one","choice":"deny","resolve_all":true}),
        json!({"request_id":"request_one","choice":"always","resolve_all":false}),
    ] {
        let body = body.to_string();
        let mut bad = context.clone();
        bad["request_body"] = json!(body);
        bad["request_sha256"] = json!(format!("{:x}", Sha256::digest(body.as_bytes())));
        assert!(f.repo.claim_approval_outcome(id, bad).await.is_err());
    }
    assert!(f.repo.get_approval_outcome(id).await.unwrap().is_none());
    assert!(
        f.repo
            .claim_approval_outcome(id, context.clone())
            .await
            .unwrap()
    );
    let mut changed = context;
    changed["capabilities"]["store_id"] = json!(Uuid::new_v4());
    assert!(f.repo.claim_approval_outcome(id, changed).await.is_err());
    let legacy = setup().await.unwrap();
    let (legacy_id, legacy_context) = legacy.reserve(ApprovalChoice::Once, false).await;
    assert!(
        legacy
            .repo
            .claim_approval_outcome(legacy_id, legacy_context)
            .await
            .is_err()
    );
    assert!(
        legacy
            .repo
            .get_approval_outcome(legacy_id)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn approval_outcome_claim_rechecks_actor_terminal_and_primary_agent() {
    let Some(f) = setup().await else { return };
    let (id, context) = f.reserve(ApprovalChoice::Once, true).await;
    f.db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE users SET is_active=false WHERE id=$1",
        [f.owner.into()],
    ))
    .await
    .unwrap();
    assert!(matches!(
        f.repo.claim_approval_outcome(id, context.clone()).await,
        Err(shared::AppError::Forbidden)
    ));
    f.db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE users SET is_active=true WHERE id=$1",
        [f.owner.into()],
    ))
    .await
    .unwrap();
    let other = agent(&f.repo).await;
    f.db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE agent_sessions SET agent_id=$2 WHERE id=$1",
        [f.run.session_id.into(), other.into()],
    ))
    .await
    .unwrap();
    assert!(
        f.repo
            .claim_approval_outcome(id, context.clone())
            .await
            .is_err()
    );
    f.db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE agent_sessions SET agent_id=$2 WHERE id=$1",
        [f.run.session_id.into(), f.run.agent_id.into()],
    ))
    .await
    .unwrap();
    f.terminal().await;
    assert!(f.repo.claim_approval_outcome(id, context).await.is_err());
    assert!(f.repo.get_approval_outcome(id).await.unwrap().is_none());
}

#[tokio::test]
async fn approval_outcome_database_rejects_split_claim_ack_and_history_changes() {
    let Some(f) = setup().await else { return };
    let (id, context) = f.reserve(ApprovalChoice::Once, true).await;
    for sql in [
        "UPDATE runtime_approval_decisions SET submission_claimed=true WHERE id=$1",
        "UPDATE runtime_approval_decisions SET state='delivered',delivered_at=now() WHERE id=$1",
        "UPDATE runtime_approval_decisions SET outcome_required=false WHERE id=$1",
        "DELETE FROM runtime_approval_decisions WHERE id=$1",
    ] {
        assert!(
            f.db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                sql,
                [id.into()]
            ))
            .await
            .is_err(),
            "{sql}"
        );
    }
    assert!(
        f.repo
            .claim_approval_outcome(id, context.clone())
            .await
            .unwrap()
    );
    for sql in [
        "UPDATE runtime_approval_outcomes SET state='acknowledged',acknowledged_at=now() WHERE decision_id=$1",
        "UPDATE runtime_approval_outcomes SET context=context || '{\"extra\":true}'::jsonb WHERE decision_id=$1",
        "DELETE FROM runtime_approval_outcomes WHERE decision_id=$1",
        "UPDATE runtime_approval_decisions SET submission_claimed=false WHERE id=$1",
        "UPDATE runtime_approval_decisions SET state='failed' WHERE id=$1",
    ] {
        assert!(
            f.db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                sql,
                [id.into()]
            ))
            .await
            .is_err(),
            "{sql}"
        );
    }
    assert!(
        f.repo
            .fail_undispatched_approval_decision(id)
            .await
            .is_err()
    );
    assert!(f.repo.deliver_approval_decision(id).await.is_err());
    assert_eq!(f.decision().await.state, ApprovalDecisionState::Uncertain);
    assert_eq!(
        f.repo
            .list_session_approvals(f.run.session_id)
            .await
            .unwrap()[0]
            .state,
        domain::RuntimeApprovalState::Pending
    );
    assert_eq!(f.audits(id, "approval.decision_delivered").await, 0);
    f.repo.finish_approval_outcome(id, context).await.unwrap();
}

#[tokio::test]
async fn approval_outcome_finish_once_or_deny_is_concurrent_idempotent_and_audited() {
    let Some(first) = setup().await else { return };
    for (f, choice, state) in [
        (
            first,
            ApprovalChoice::Once,
            domain::RuntimeApprovalState::Approved,
        ),
        (
            setup().await.unwrap(),
            ApprovalChoice::Deny,
            domain::RuntimeApprovalState::Denied,
        ),
    ] {
        let (id, context) = f.reserve(choice, true).await;
        f.repo
            .claim_approval_outcome(id, context.clone())
            .await
            .unwrap();
        let mut bad = context.clone();
        bad["command_id"] = json!(Uuid::new_v4());
        assert!(f.repo.finish_approval_outcome(id, bad).await.is_err());
        let (a, b) = tokio::join!(
            f.repo.finish_approval_outcome(id, context.clone()),
            f.repo.finish_approval_outcome(id, context.clone())
        );
        assert_eq!(a.unwrap().state, ApprovalDecisionState::Delivered);
        assert_eq!(b.unwrap().state, ApprovalDecisionState::Delivered);
        assert_eq!(
            f.repo
                .list_session_approvals(f.run.session_id)
                .await
                .unwrap()[0]
                .state,
            state
        );
        assert_eq!(f.audits(id, "approval.decision_delivered").await, 1);
        assert!(
            f.repo
                .list_approval_outcomes(None)
                .await
                .unwrap()
                .iter()
                .all(|i| i.decision.id != id)
        );
        assert!(!f.repo.claim_approval_outcome(id, context).await.unwrap());
    }
}

#[tokio::test]
async fn approval_outcome_late_ack_preserves_terminal_and_cancelled_history_after_revocation() {
    let Some(f) = setup().await else { return };
    let (id, context) = f.reserve(ApprovalChoice::Once, true).await;
    f.repo
        .claim_approval_outcome(id, context.clone())
        .await
        .unwrap();
    f.terminal().await;
    assert_eq!(
        f.repo
            .resolve_runtime_approval_requests_for_run(
                f.run.id,
                domain::ResolveRuntimeApprovalRequest {
                    choice: "cancelled".into(),
                    resolve_all: false,
                },
                f.owner
            )
            .await
            .unwrap(),
        1
    );
    f.db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE users SET is_active=false WHERE id=$1",
        [f.owner.into()],
    ))
    .await
    .unwrap();
    let snapshot = || {
        Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT jsonb_build_object('run',(SELECT to_jsonb(r) FROM session_agent_runs r WHERE id=$1),
            'approval',(SELECT to_jsonb(a) FROM runtime_approval_requests a WHERE id=$2)) AS value",
        [f.run.id.into(),f.approval.id.into()])
    };
    let before: Value =
        f.db.query_one(snapshot())
            .await
            .unwrap()
            .unwrap()
            .try_get("", "value")
            .unwrap();
    assert_eq!(before["approval"]["state"], "cancelled");
    assert!(
        f.repo
            .list_approval_outcomes(None)
            .await
            .unwrap()
            .iter()
            .any(|i| i.decision.id == id)
    );
    f.repo.finish_approval_outcome(id, context).await.unwrap();
    let after: Value =
        f.db.query_one(snapshot())
            .await
            .unwrap()
            .unwrap()
            .try_get("", "value")
            .unwrap();
    assert_eq!(before, after);
    assert_eq!(f.audits(id, "approval.decision_delivered").await, 1);
}

#[tokio::test]
async fn approval_outcome_audit_failure_rolls_back_receipt_and_keeps_recovery_queue() {
    let Some(f) = setup().await else { return };
    let (id, context) = f.reserve(ApprovalChoice::Once, true).await;
    f.repo
        .claim_approval_outcome(id, context.clone())
        .await
        .unwrap();
    let name = format!("approval_audit_fault_{}", Uuid::new_v4().simple());
    f.db.execute_unprepared(&format!(
        "CREATE FUNCTION {name}() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN IF NEW.entity_id='{id}' AND NEW.action='approval.decision_delivered' THEN
            RAISE EXCEPTION 'injected approval audit fault'; END IF; RETURN NEW; END $$;
        CREATE TRIGGER {name} BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION {name}();"
    ))
    .await
    .unwrap();
    let result = f.repo.finish_approval_outcome(id, context.clone()).await;
    f.db.execute_unprepared(&format!(
        "DROP TRIGGER {name} ON audit_log; DROP FUNCTION {name}();"
    ))
    .await
    .unwrap();
    assert!(result.is_err());
    assert_eq!(f.decision().await.state, ApprovalDecisionState::Uncertain);
    assert_eq!(
        f.repo
            .list_session_approvals(f.run.session_id)
            .await
            .unwrap()[0]
            .state,
        domain::RuntimeApprovalState::Pending
    );
    assert_eq!(f.audits(id, "approval.decision_delivered").await, 0);
    assert!(
        f.repo
            .list_approval_outcomes(None)
            .await
            .unwrap()
            .iter()
            .any(|i| i.decision.id == id)
    );
    f.repo.finish_approval_outcome(id, context).await.unwrap();
    assert_eq!(f.audits(id, "approval.decision_delivered").await, 1);
}
