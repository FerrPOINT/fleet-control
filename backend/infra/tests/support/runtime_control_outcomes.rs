use super::*;
use domain::{RuntimeControlActor, RuntimeControlOperation, RuntimeControlState};
use sea_orm::DatabaseConnection;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

struct OutcomeFixture {
    repo: PostgresFleetRepository,
    db: DatabaseConnection,
    owner: Uuid,
    run: domain::SessionAgentRun,
    message: Uuid,
    native_run: String,
}

async fn setup() -> Option<OutcomeFixture> {
    let (repo, owner, _) = fixture().await?;
    let id = agent(&repo).await;
    let session = repo
        .create_session(chat(id, "control-outcome"), owner)
        .await
        .unwrap();
    let message = repo
        .create_session_message(session.id, prompt("outcome-prompt"), owner)
        .await
        .unwrap();
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
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
    repo.prepare_hermes_dispatch(app::HermesDispatchDraft {
        message_id: message.id,
        session_id: session.id,
        agent_id: id,
        run_role: SessionRunRole::Primary,
        requested_session_id: format!("fleet:{}:{id}", session.id),
        input: message.body,
        origin: "http://127.0.0.1:24003".into(),
        credential_fingerprint: "a".repeat(64),
        capabilities: hermes_protocol_fixture::capabilities(),
    })
    .await
    .unwrap();
    let intent = repo
        .claim_hermes_submission(message.id, "http://127.0.0.1:24003".into(), "a".repeat(64))
        .await
        .unwrap()
        .unwrap();
    let native_run = format!("run_{}", Uuid::new_v4().simple());
    repo.accept_hermes_run(message.id, intent.run.id, native_run.clone())
        .await
        .unwrap();
    let (run, _) = repo
        .pin_hermes_run_session(
            intent.run.id,
            native_run.clone(),
            format!("fleet:{}:{id}", session.id),
            "outcome-native-session".into(),
        )
        .await
        .unwrap();
    Some(OutcomeFixture {
        repo,
        db,
        owner,
        run,
        message: message.id,
        native_run,
    })
}

impl OutcomeFixture {
    async fn reserve(&self, input: Option<&str>) -> (Uuid, Value) {
        let operation = if input.is_some() {
            RuntimeControlOperation::Steer
        } else {
            RuntimeControlOperation::Stop
        };
        let reserved = self
            .repo
            .reserve_runtime_control(
                &self.run,
                &RuntimeControlActor {
                    user_id: self.owner,
                    idempotency_key: Uuid::new_v4().to_string(),
                },
                operation,
                input,
            )
            .await
            .unwrap();
        let body = input
            .map(|s| json!({"input":s}).to_string())
            .unwrap_or_default();
        let context = json!({
            "origin":"http://127.0.0.1:24003", "credential_fingerprint":"a".repeat(64),
            "command_id":reserved.receipt.id, "run_id":self.native_run, "operation":operation,
            "request_body":body, "request_sha256":format!("{:x}", Sha256::digest(body.as_bytes())),
            "capabilities":{
                "object":"fleet.hermes.controls.capabilities", "contract_version":1,
                "store_id":Uuid::new_v4(), "scope_fingerprint":"b".repeat(64), "profile":"default",
                "native_source_revision":"bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3",
                "single_send":true, "non_dispatch_lookup":true,
                "lookup":{"method":"GET","path":"/fleet/v1/controls/lookup"},
                "operations":["steer","stop","approval"]
            }
        });
        (reserved.receipt.id, context)
    }

    async fn terminal(&self) {
        self.repo
            .commit_hermes_terminal(app::HermesTerminalCommit {
                message_id: self.message,
                run_id: self.run.id,
                runtime_run_id: self.native_run.clone(),
                runtime_session_id: "outcome-native-session".into(),
                state: SessionRunState::Completed,
                body: Some("independent terminal mirror".into()),
                error: None,
            })
            .await
            .unwrap();
        self.repo.reconcile_runtime_controls().await.unwrap();
    }
}

#[tokio::test]
async fn original_control_outcome_claim_is_atomic_single_use_and_survives_new_repository() {
    let Some(f) = setup().await else { return };
    let (id, context) = f.reserve(Some("private guidance\nwith exact bytes")).await;
    let (a, b) = tokio::join!(
        f.repo.claim_runtime_control_outcome(id, context.clone()),
        f.repo.claim_runtime_control_outcome(id, context.clone())
    );
    assert_eq!(usize::from(a.unwrap()) + usize::from(b.unwrap()), 1);
    let reopened = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let restarted = PostgresFleetRepository::new(reopened);
    let recovered = restarted
        .get_runtime_control_outcome(id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(recovered.context, context);
    assert_eq!(recovered.receipt.state, RuntimeControlState::Submitted);
    let public = serde_json::to_string(&recovered.receipt).unwrap();
    assert!(!public.contains("private guidance"));
    assert!(!public.contains("credential_fingerprint"));
    let page = restarted
        .list_runtime_control_outcomes(Some(Uuid::nil()))
        .await
        .unwrap();
    assert!(page.len() <= 100);
    assert!(page.windows(2).all(|w| w[0].receipt.id < w[1].receipt.id));
    assert!(
        restarted
            .get_runtime_control_outcome(Uuid::new_v4())
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn original_control_outcome_requires_matching_payload_identity_and_closed_context() {
    let Some(f) = setup().await else { return };
    let (id, context) = f.reserve(Some("original")).await;
    for (field, value) in [
        ("command_id", json!(Uuid::new_v4())),
        ("run_id", json!(format!("run_{}", Uuid::new_v4().simple()))),
        ("origin", json!("http://127.0.0.1:24004")),
        ("credential_fingerprint", json!("c".repeat(64))),
        ("operation", json!("stop")),
        ("request_sha256", json!("d".repeat(64))),
        ("extra", json!(true)),
    ] {
        let mut bad = context.clone();
        bad[field] = value;
        assert!(
            f.repo.claim_runtime_control_outcome(id, bad).await.is_err(),
            "{field}"
        );
    }
    let mut altered = context.clone();
    altered["request_body"] = json!("{\"input\":\"different\"}");
    altered["request_sha256"] = json!(format!(
        "{:x}",
        Sha256::digest(altered["request_body"].as_str().unwrap().as_bytes())
    ));
    assert!(
        f.repo
            .claim_runtime_control_outcome(id, altered)
            .await
            .is_err()
    );
    assert_eq!(
        f.repo
            .get_runtime_control(f.run.session_id, id)
            .await
            .unwrap()
            .state,
        RuntimeControlState::Reserved
    );
    assert!(
        f.repo
            .get_runtime_control_outcome(id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        f.repo
            .claim_runtime_control_outcome(id, context)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn original_control_outcome_claim_rechecks_actor_and_never_backfills_legacy_submission() {
    let Some(f) = setup().await else { return };
    let (id, context) = f.reserve(None).await;
    f.db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE users SET is_active=false WHERE id=$1",
        [f.owner.into()],
    ))
    .await
    .unwrap();
    assert!(matches!(
        f.repo
            .claim_runtime_control_outcome(id, context.clone())
            .await,
        Err(shared::AppError::Forbidden)
    ));
    f.db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE users SET is_active=true WHERE id=$1",
        [f.owner.into()],
    ))
    .await
    .unwrap();
    assert!(f.repo.claim_runtime_control(id).await.unwrap());
    assert!(
        !f.repo
            .claim_runtime_control_outcome(id, context.clone())
            .await
            .unwrap()
    );
    assert!(
        f.repo
            .get_runtime_control_outcome(id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        f.repo
            .finish_runtime_control_outcome(id, context, "stopping")
            .await
            .is_err()
    );
    assert!(f.db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE runtime_control_commands SET state='uncertain',outcome_required=true WHERE id=$1",
        [id.into()])).await.is_err());
}

#[tokio::test]
async fn original_control_outcome_recovery_ack_is_exact_idempotent_and_preserves_active_capacity() {
    let Some(f) = setup().await else { return };
    let (id, context) = f.reserve(None).await;
    assert!(
        f.repo
            .claim_runtime_control_outcome(id, context.clone())
            .await
            .unwrap()
    );
    f.repo.retire_runtime_control(id, true).await.unwrap();
    assert!(f.repo.finish_runtime_control(id, "stopping").await.is_err());
    assert!(
        f.repo
            .finish_runtime_control_outcome(id, context.clone(), "already_terminal")
            .await
            .is_err()
    );
    let mut new_epoch = context.clone();
    new_epoch["capabilities"]["store_id"] = json!(Uuid::new_v4());
    assert!(
        f.repo
            .finish_runtime_control_outcome(id, new_epoch, "stopping")
            .await
            .is_err()
    );
    let receipt = f
        .repo
        .finish_runtime_control_outcome(id, context.clone(), "stopping")
        .await
        .unwrap();
    assert_eq!(receipt.state, RuntimeControlState::Acknowledged);
    assert_eq!(receipt.acknowledgement.as_deref(), Some("stopping"));
    assert_eq!(
        f.repo.get_session_agent_run(f.run.id).await.unwrap().state,
        SessionRunState::Stopping
    );
    assert_eq!(
        serde_json::to_value(
            f.repo
                .finish_runtime_control_outcome(id, context, "stopping")
                .await
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(&receipt).unwrap()
    );
    assert!(
        f.repo
            .prepare_session_agent_run(
                f.run.session_id,
                f.run.agent_id,
                SessionRunRole::Primary,
                "another-session".into()
            )
            .await
            .is_err()
    );
    let count = f.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT count(*) AS total FROM session_events WHERE session_id=$1 AND payload->>'command_id'=$2",
        [f.run.session_id.into(),id.to_string().into()])).await.unwrap().unwrap().try_get::<i64>("","total").unwrap();
    assert_eq!(count, 4);
}

#[tokio::test]
async fn original_control_outcome_late_ack_preserves_terminal_receipt_and_transcript() {
    let Some(f) = setup().await else { return };
    let (id, context) = f.reserve(None).await;
    assert!(
        f.repo
            .claim_runtime_control_outcome(id, context.clone())
            .await
            .unwrap()
    );
    f.repo.retire_runtime_control(id, true).await.unwrap();
    f.terminal().await;
    let before = f
        .repo
        .get_runtime_control(f.run.session_id, id)
        .await
        .unwrap();
    assert_eq!(before.state, RuntimeControlState::TerminalObserved);
    assert_eq!(before.acknowledgement, None);
    let run = serde_json::to_value(f.repo.get_session_agent_run(f.run.id).await.unwrap()).unwrap();
    let messages = serde_json::to_value(
        f.repo
            .list_session_messages(f.run.session_id)
            .await
            .unwrap(),
    )
    .unwrap();
    let after = f
        .repo
        .finish_runtime_control_outcome(id, context.clone(), "stopping")
        .await
        .unwrap();
    assert_eq!(after.state, RuntimeControlState::TerminalObserved);
    assert_eq!(after.observed_run_state, Some(SessionRunState::Completed));
    assert_eq!(after.acknowledgement.as_deref(), Some("stopping"));
    assert_eq!(after.updated_at, before.updated_at);
    assert_eq!(
        serde_json::to_value(f.repo.get_session_agent_run(f.run.id).await.unwrap()).unwrap(),
        run
    );
    assert_eq!(
        serde_json::to_value(
            f.repo
                .list_session_messages(f.run.session_id)
                .await
                .unwrap()
        )
        .unwrap(),
        messages
    );
    assert_eq!(
        serde_json::to_value(
            f.repo
                .finish_runtime_control_outcome(id, context, "stopping")
                .await
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(after).unwrap()
    );
}

#[tokio::test]
async fn original_control_outcome_audit_failure_rolls_back_witness_run_receipt_and_event() {
    let Some(f) = setup().await else { return };
    let (id, context) = f.reserve(None).await;
    assert!(
        f.repo
            .claim_runtime_control_outcome(id, context.clone())
            .await
            .unwrap()
    );
    let guard = format!("outcome_fault_{}", Uuid::new_v4().simple());
    f.db.execute_unprepared(&format!(
        "CREATE FUNCTION {guard}() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN IF NEW.entity_id='{id}' AND NEW.action='runtime_control.acknowledged' THEN
        RAISE EXCEPTION 'owned outcome audit failure'; END IF; RETURN NEW; END $$;
        CREATE TRIGGER {guard} BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION {guard}();"
    ))
    .await
    .unwrap();
    let result = f
        .repo
        .finish_runtime_control_outcome(id, context.clone(), "stopping")
        .await;
    f.db.execute_unprepared(&format!(
        "DROP TRIGGER {guard} ON audit_log; DROP FUNCTION {guard}();"
    ))
    .await
    .unwrap();
    assert!(result.is_err());
    let current = f
        .repo
        .get_runtime_control_outcome(id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(current.receipt.state, RuntimeControlState::Submitted);
    assert_eq!(current.receipt.acknowledgement, None);
    assert_eq!(
        f.repo.get_session_agent_run(f.run.id).await.unwrap().state,
        SessionRunState::Running
    );
    let value =
        f.db.query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT state FROM runtime_control_outcomes WHERE command_id=$1",
            [id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(value.try_get::<String>("", "state").unwrap(), "submitted");
    f.repo
        .finish_runtime_control_outcome(id, context, "stopping")
        .await
        .unwrap();
}

#[tokio::test]
async fn original_control_outcome_database_guards_missing_context_tamper_delete_and_split_ack() {
    let Some(f) = setup().await else { return };
    let (id, context) = f.reserve(None).await;
    let txn = f.db.begin().await.unwrap();
    txn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE runtime_control_commands SET state='submitted',outcome_required=true WHERE id=$1",
        [id.into()],
    ))
    .await
    .unwrap();
    assert!(txn.commit().await.is_err());
    assert!(
        f.repo
            .claim_runtime_control_outcome(id, context)
            .await
            .unwrap()
    );
    for sql in [
        "UPDATE runtime_control_outcomes SET context=context || '{\"origin\":\"http://foreign\"}'::jsonb WHERE command_id=$1",
        "DELETE FROM runtime_control_outcomes WHERE command_id=$1",
        "UPDATE runtime_control_commands SET state='uncertain',outcome_required=false WHERE id=$1",
        "UPDATE runtime_control_commands SET state='acknowledged',acknowledgement='stopping' WHERE id=$1",
    ] {
        assert!(
            f.db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                sql,
                [id.into()]
            ))
            .await
            .is_err()
        );
    }
    let txn = f.db.begin().await.unwrap();
    txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE runtime_control_outcomes SET state='acknowledged',acknowledgement='stopping',acknowledged_at=now()
            WHERE command_id=$1",[id.into()])).await.unwrap();
    assert!(txn.commit().await.is_err());
    assert_eq!(
        f.repo
            .get_runtime_control(f.run.session_id, id)
            .await
            .unwrap()
            .state,
        RuntimeControlState::Submitted
    );
}

#[tokio::test]
async fn run_progress_serializes_before_original_journal_session_lock() {
    let Some(f) = setup().await else { return };
    let holder = f.db.begin().await.unwrap();
    holder
        .execute_unprepared("SET LOCAL statement_timeout='5s'; SET LOCAL lock_timeout='5s'")
        .await
        .unwrap();
    let pid = holder
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT pg_backend_pid() AS pid",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i32>("", "pid")
        .unwrap();
    holder
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id FROM agent_sessions WHERE id=$1 FOR UPDATE",
            [f.run.session_id.into()],
        ))
        .await
        .unwrap();
    let progress = f.repo.update_session_agent_run_dispatch(
        f.run.id,
        Some(f.native_run.clone()),
        SessionRunState::Waiting,
        Some("invalid fixture SSE".into()),
    );
    let release = async {
        tokio::time::timeout(Duration::from_secs(10),async {
            loop {
                let blocked = f.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                    "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database()
                        AND pid<>pg_backend_pid() AND $1=ANY(pg_blocking_pids(pid))) AS blocked",
                    [pid.into()])).await?.unwrap().try_get::<bool>("","blocked")?;
                if blocked { break; }
                sleep(Duration::from_millis(10)).await;
            }
            Ok::<(),sea_orm::DbErr>(())
        }).await.map_err(|_| sea_orm::DbErr::Custom("run progress blocker was not observed".into()))??;
        // The journal's session holder must acquire the run before it releases the session.
        // A run-first progress writer instead waits for this session's event FK and deadlocks.
        holder
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT id FROM session_agent_runs WHERE id=$1 FOR UPDATE",
                [f.run.id.into()],
            ))
            .await?;
        holder.commit().await?;
        Ok::<(), sea_orm::DbErr>(())
    };
    let (progress, released) = tokio::time::timeout(Duration::from_secs(20), async {
        tokio::join!(progress, release)
    })
    .await
    .unwrap();
    released.unwrap();
    let progress = progress.unwrap();
    assert_eq!(progress.state, SessionRunState::Waiting);
    assert_eq!(
        progress.runtime_run_id.as_deref(),
        Some(f.native_run.as_str())
    );
    assert_eq!(
        progress.runtime_session_id.as_deref(),
        Some("outcome-native-session")
    );
    assert!(
        !f.repo
            .list_session_events(f.run.session_id, 0)
            .await
            .unwrap()
            .is_empty()
    );
}
