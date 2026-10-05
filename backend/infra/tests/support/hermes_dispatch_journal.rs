use super::*;
use app::{HermesDispatchDraft, HermesDispatchIntent};
use domain::MessageDeliveryState;
use migration::MigratorTrait;
use sea_orm::{DatabaseConnection, sea_query::Value as SqlValue};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use shared::AppError;
use tokio::time::timeout;

struct JournalFixture {
    repo: PostgresFleetRepository,
    db: DatabaseConnection,
    owner: Uuid,
    draft: HermesDispatchDraft,
}

fn capabilities() -> Value {
    hermes_protocol_fixture::capabilities()
}

async fn setup() -> Option<JournalFixture> {
    let (repo, owner, _) = fixture().await?;
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let agent_id = agent(&repo).await;
    let session = repo
        .create_session(chat(agent_id, "journal-session"), owner)
        .await
        .unwrap();
    let message = repo
        .create_session_message(session.id, prompt("journal-message"), owner)
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE message_dispatch_outbox SET state='dispatching' WHERE message_id=$1",
        [message.id.into()],
    ))
    .await
    .unwrap();
    let port = repo.get_agent(agent_id).await.unwrap().api_port.unwrap();
    Some(JournalFixture {
        repo,
        db,
        owner,
        draft: HermesDispatchDraft {
            message_id: message.id,
            session_id: session.id,
            agent_id,
            run_role: SessionRunRole::Primary,
            requested_session_id: format!("fleet:{}:{agent_id}", session.id),
            input: message.body,
            origin: format!("http://127.0.0.1:{port}"),
            credential_fingerprint: "a".repeat(64),
            capabilities: capabilities(),
        },
    })
}

async fn sql(p: &JournalFixture, sql: &str, values: Vec<SqlValue>) {
    p.db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        sql,
        values,
    ))
    .await
    .unwrap();
}

async fn count(p: &JournalFixture, sql: &str, id: Uuid) -> i64 {
    p.db.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        sql,
        [id.into()],
    ))
    .await
    .unwrap()
    .unwrap()
    .try_get("", "count")
    .unwrap()
}

async fn prepare(p: &JournalFixture) -> Result<HermesDispatchIntent, AppError> {
    p.repo.prepare_hermes_dispatch(p.draft.clone()).await
}

async fn claim(p: &JournalFixture) -> Result<Option<HermesDispatchIntent>, AppError> {
    p.repo
        .claim_hermes_submission(
            p.draft.message_id,
            p.draft.origin.clone(),
            p.draft.credential_fingerprint.clone(),
        )
        .await
}

#[tokio::test]
async fn journal_clock_regression_keeps_logical_progress_without_renewing_horizon() {
    let Some(p) = setup().await else { return };
    let name = format!("a_owned_clock_{}", Uuid::new_v4().simple());
    p.db.execute_unprepared(&format!("CREATE FUNCTION {name}() RETURNS trigger AS $$ BEGIN
        IF NEW.message_id='{id}'::uuid THEN
            NEW.created_at := clock_timestamp()+interval '30 seconds';
            NEW.recovery_deadline := NEW.created_at+interval '86340 seconds';
        END IF; RETURN NEW; END $$ LANGUAGE plpgsql;
        CREATE TRIGGER {name} BEFORE INSERT ON hermes_dispatch_journal FOR EACH ROW EXECUTE FUNCTION {name}()",id=p.draft.message_id)).await.unwrap();
    let prepared = prepare(&p).await.unwrap();
    p.db.execute_unprepared(&format!(
        "DROP TRIGGER {name} ON hermes_dispatch_journal; DROP FUNCTION {name}()"
    ))
    .await
    .unwrap();
    let claimed = claim(&p).await.unwrap().unwrap();
    assert_eq!(claimed.recovery_deadline, prepared.recovery_deadline);
    let accepted = p
        .repo
        .accept_hermes_run(p.draft.message_id, claimed.run.id, "run_clock_order".into())
        .await
        .unwrap();
    assert_eq!(accepted.runtime_run_id.as_deref(), Some("run_clock_order"));
    let facts = p.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT created_at>clock_timestamp() AS regressed,submitted_at>=created_at AS submission_order,
            accepted_at>=submitted_at AS acceptance_order,
            recovery_deadline=created_at+interval '86340 seconds' AS original_horizon,
            to_jsonb(j) AS journal FROM hermes_dispatch_journal j WHERE message_id=$1",
        [p.draft.message_id.into()])).await.unwrap().unwrap();
    for fact in [
        "regressed",
        "submission_order",
        "acceptance_order",
        "original_horizon",
    ] {
        assert!(facts.try_get::<bool>("", fact).unwrap(), "{fact}");
    }
    let before: Value = facts.try_get("", "journal").unwrap();
    p.repo
        .accept_hermes_run(p.draft.message_id, claimed.run.id, "run_clock_order".into())
        .await
        .unwrap();
    let after: Value =
        p.db.query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT to_jsonb(j) AS journal FROM hermes_dispatch_journal j WHERE message_id=$1",
            [p.draft.message_id.into()],
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "journal")
        .unwrap();
    assert_eq!(after, before);
    assert_eq!(
        p.repo
            .list_session_agent_runs(p.draft.session_id)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(claim(&p).await.unwrap().is_none());
    let downgrade = migration::Migrator::down(&p.db, Some(1)).await;
    assert!(downgrade.is_err(), "retained journal must block downgrade");
    let retained =
        p.db.query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT to_regprocedure('fleet_order_hermes_dispatch_time()') IS NOT NULL AS retained",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<bool>("", "retained")
        .unwrap();
    assert!(retained);
    assert_eq!(
        p.db.query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT to_jsonb(j) AS journal FROM hermes_dispatch_journal j WHERE message_id=$1",
            [p.draft.message_id.into()]
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<Value>("", "journal")
        .unwrap(),
        before
    );
}

#[tokio::test]
async fn journal_concurrent_prepare_and_claim_preserve_exact_seeded_bytes() {
    let Some(p) = setup().await else {
        return;
    };
    sql(&p,"UPDATE session_agent_runs SET model='pinned-model',provider='pinned-provider',model_options=$2 WHERE session_id=$1",
        vec![p.draft.session_id.into(),json!({"temperature":0.25,"max_tokens":1234}).into()]).await;
    let (a, b) = timeout(Duration::from_secs(10), async {
        tokio::join!(prepare(&p), prepare(&p))
    })
    .await
    .unwrap();
    let a = a.unwrap();
    let b = b.unwrap();
    assert_eq!(a.run.id, b.run.id);
    assert_eq!(a.request_body, b.request_body);
    assert_eq!(a.idempotency_key, p.draft.message_id.to_string());
    assert_eq!(
        a.request_hash,
        hex::encode(Sha256::digest(a.request_body.as_bytes()))
    );
    let body: Value = serde_json::from_str(&a.request_body).unwrap();
    assert_eq!(
        body,
        json!({"input":p.draft.input,"session_id":p.draft.requested_session_id,
        "model":"pinned-model","provider":"pinned-provider","model_options":{"temperature":0.25,"max_tokens":1234}})
    );
    assert_eq!(
        count(
            &p,
            "SELECT count(*) FROM session_agent_runs WHERE session_id=$1",
            p.draft.session_id
        )
        .await,
        1
    );
    let (a, b) = timeout(Duration::from_secs(10), async {
        tokio::join!(claim(&p), claim(&p))
    })
    .await
    .unwrap();
    let mut winners = vec![a.unwrap(), b.unwrap()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    assert_eq!(winners.len(), 1);
    let winner = winners.pop().unwrap();
    assert!(winner.submission_attempted);
    assert_eq!(winner.state, "submitted");
    assert!(winner.submitted_at.is_some());
    let replay = prepare(&p).await.unwrap();
    assert_eq!(replay.request_hash, winner.request_hash);
    assert_eq!(replay.request_body, winner.request_body);
    assert_eq!(replay.submitted_at, winner.submitted_at);
    assert!(claim(&p).await.unwrap().is_none());
    let read = p
        .repo
        .get_hermes_dispatch_intent(p.draft.message_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(read.request_body, winner.request_body);
    assert_eq!(read.run.id, winner.run.id);
    assert_eq!(read.submitted_at, winner.submitted_at);
    let row=p.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT recovery_deadline-created_at=interval '86340 seconds' AS valid FROM hermes_dispatch_journal WHERE message_id=$1",
        [p.draft.message_id.into()])).await.unwrap().unwrap();
    assert!(row.try_get::<bool>("", "valid").unwrap());
}

#[tokio::test]
async fn journal_late_prepared_failure_cannot_erase_submitted_or_accepted_dispatch() {
    let Some(p) = setup().await else {
        return;
    };
    let stale = prepare(&p).await.unwrap();
    assert!(!stale.submission_attempted);
    let submitted = claim(&p).await.unwrap().unwrap();
    assert!(submitted.submission_attempted);
    p.repo
        .update_session_message_delivery(
            p.draft.message_id,
            MessageDeliveryState::Failed,
            None,
            Some("late preflight error".into()),
        )
        .await
        .unwrap();
    let row = p.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT m.delivery_state,m.delivery_error,j.state AS journal_state,o.state AS outbox_state
         FROM session_messages m JOIN hermes_dispatch_journal j ON j.message_id=m.id
         JOIN message_dispatch_outbox o ON o.message_id=m.id WHERE m.id=$1",
        [p.draft.message_id.into()])).await.unwrap().unwrap();
    assert_eq!(
        row.try_get::<String>("", "delivery_state").unwrap(),
        "pending"
    );
    assert_eq!(
        row.try_get::<String>("", "delivery_error").unwrap(),
        "late preflight error"
    );
    assert_eq!(
        row.try_get::<String>("", "journal_state").unwrap(),
        "submitted"
    );
    assert_eq!(
        row.try_get::<String>("", "outbox_state").unwrap(),
        "dispatching"
    );
    let accepted = p
        .repo
        .accept_hermes_run(p.draft.message_id, submitted.run.id, "run_late_ack".into())
        .await
        .unwrap();
    assert_eq!(accepted.runtime_run_id.as_deref(), Some("run_late_ack"));
    p.repo
        .update_session_message_delivery(
            p.draft.message_id,
            MessageDeliveryState::Failed,
            None,
            Some("even later error".into()),
        )
        .await
        .unwrap();
    let row = p.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT m.delivery_state,m.delivery_error,m.runtime_message_id,j.state AS journal_state,o.state AS outbox_state
         FROM session_messages m JOIN hermes_dispatch_journal j ON j.message_id=m.id
         JOIN message_dispatch_outbox o ON o.message_id=m.id WHERE m.id=$1",
        [p.draft.message_id.into()])).await.unwrap().unwrap();
    assert_eq!(
        row.try_get::<String>("", "delivery_state").unwrap(),
        "dispatched"
    );
    assert_eq!(
        row.try_get::<Option<String>>("", "delivery_error").unwrap(),
        None
    );
    assert_eq!(
        row.try_get::<String>("", "runtime_message_id").unwrap(),
        "run_late_ack"
    );
    assert_eq!(
        row.try_get::<String>("", "journal_state").unwrap(),
        "accepted"
    );
    assert_eq!(
        row.try_get::<String>("", "outbox_state").unwrap(),
        "dispatched"
    );
}

#[tokio::test]
async fn journal_atomic_prepare_fault_rolls_back_run_reservation_and_events() {
    let Some(mut p) = setup().await else {
        return;
    };
    p.draft.input = "Private journal prompt must not be logged".into();
    sql(
        &p,
        "UPDATE session_messages SET body=$2 WHERE id=$1",
        vec![p.draft.message_id.into(), p.draft.input.clone().into()],
    )
    .await;
    // Exercise the concrete no-seed insertion branch as well as rollback of the reservation.
    sql(&p,"DELETE FROM session_agent_runs WHERE session_id=$1 AND runtime_session_id IS NULL AND runtime_run_id IS NULL",
        vec![p.draft.session_id.into()]).await;
    let before = p
        .repo
        .list_session_agent_runs(p.draft.session_id)
        .await
        .unwrap();
    let cursor = count(
        &p,
        "SELECT sequence AS count FROM session_event_cursors WHERE session_id=$1",
        p.draft.session_id,
    )
    .await;
    let name = format!("journal_fault_{}", Uuid::new_v4().simple());
    p.db.execute_unprepared(&format!(
        "CREATE FUNCTION {name}() RETURNS trigger AS $$ BEGIN IF NEW.message_id='{}'::uuid THEN
            RAISE EXCEPTION 'Private request=% fingerprint=%', NEW.request_body, NEW.credential_fingerprint;
            END IF; RETURN NEW; END $$ LANGUAGE plpgsql;
         CREATE TRIGGER {name} BEFORE INSERT ON hermes_dispatch_journal FOR EACH ROW EXECUTE FUNCTION {name}()",p.draft.message_id
    )).await.unwrap();
    let result = prepare(&p).await;
    p.db.execute_unprepared(&format!(
        "DROP TRIGGER {name} ON hermes_dispatch_journal; DROP FUNCTION {name}()"
    ))
    .await
    .unwrap();
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("private trigger fault must fail preparation"),
    };
    assert!(matches!(&error, AppError::Database(_)));
    assert_eq!(
        error.to_string(),
        "database error: Hermes dispatch journal database operation failed"
    );
    for rendered in [error.to_string(), format!("{error:?}")] {
        assert!(!rendered.contains("Private"));
        assert!(!rendered.contains(&p.draft.input));
        assert!(!rendered.contains(&p.draft.credential_fingerprint));
    }
    assert!(
        p.repo
            .get_hermes_dispatch_intent(p.draft.message_id)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        serde_json::to_value(
            p.repo
                .list_session_agent_runs(p.draft.session_id)
                .await
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(before).unwrap()
    );
    assert_eq!(
        count(
            &p,
            "SELECT sequence AS count FROM session_event_cursors WHERE session_id=$1",
            p.draft.session_id
        )
        .await,
        cursor
    );
    prepare(&p).await.unwrap();
}

#[tokio::test]
async fn journal_changed_payload_origin_credential_alias_role_capabilities_and_scope_conflict() {
    let Some(p) = setup().await else {
        return;
    };
    let original = prepare(&p).await.unwrap();
    let mut drafts = Vec::new();
    let mut d = p.draft.clone();
    d.input.push_str(" changed");
    drafts.push(d);
    let mut d = p.draft.clone();
    d.origin = "http://127.0.0.1:55555".into();
    drafts.push(d);
    let mut d = p.draft.clone();
    d.credential_fingerprint = "b".repeat(64);
    drafts.push(d);
    let mut d = p.draft.clone();
    d.run_role = SessionRunRole::Leader;
    drafts.push(d);
    let mut d = p.draft.clone();
    d.capabilities["extra_observed_field"] = json!(true);
    drafts.push(d);
    let other = p
        .repo
        .create_session(chat(p.draft.agent_id, "changed-journal-scope"), p.owner)
        .await
        .unwrap();
    let mut d = p.draft.clone();
    d.session_id = other.id;
    d.requested_session_id = format!("fleet:{}:{}", other.id, d.agent_id);
    drafts.push(d);
    for d in drafts {
        assert!(matches!(
            p.repo.prepare_hermes_dispatch(d).await,
            Err(AppError::Conflict(_))
        ));
    }
    let mut d = p.draft.clone();
    d.requested_session_id = "foreign-alias".into();
    assert!(p.repo.prepare_hermes_dispatch(d).await.is_err());
    assert!(matches!(
        p.repo
            .claim_hermes_submission(p.draft.message_id, p.draft.origin.clone(), "b".repeat(64))
            .await,
        Err(AppError::Conflict(_))
    ));
    let read = p
        .repo
        .get_hermes_dispatch_intent(p.draft.message_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(read.request_body, original.request_body);
    assert!(!read.submission_attempted);
}

#[tokio::test]
async fn journal_duplicate_category_is_generic_and_does_not_expose_private_trigger_diagnostics() {
    let Some(p) = setup().await else {
        return;
    };
    let name = format!("journal_private_duplicate_{}", Uuid::new_v4().simple());
    p.db.execute_unprepared(&format!(
        "CREATE FUNCTION {name}() RETURNS trigger AS $$ BEGIN IF NEW.message_id='{}'::uuid THEN
            RAISE EXCEPTION 'Private request=% fingerprint=%', NEW.request_body, NEW.credential_fingerprint
                USING ERRCODE='23505'; END IF; RETURN NEW; END $$ LANGUAGE plpgsql;
         CREATE TRIGGER {name} BEFORE INSERT ON hermes_dispatch_journal FOR EACH ROW EXECUTE FUNCTION {name}()",p.draft.message_id
    )).await.unwrap();
    let result = prepare(&p).await;
    p.db.execute_unprepared(&format!(
        "DROP TRIGGER {name} ON hermes_dispatch_journal; DROP FUNCTION {name}()"
    ))
    .await
    .unwrap();
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("duplicate-category trigger must fail preparation"),
    };
    assert!(matches!(&error, AppError::Conflict(_)));
    assert_eq!(
        error.to_string(),
        "conflict: Hermes dispatch journal identity or capacity conflict"
    );
    for rendered in [error.to_string(), format!("{error:?}")] {
        assert!(!rendered.contains("Private"));
        assert!(!rendered.contains(&p.draft.input));
        assert!(!rendered.contains(&p.draft.credential_fingerprint));
    }
    assert!(
        p.repo
            .get_hermes_dispatch_intent(p.draft.message_id)
            .await
            .unwrap()
            .is_none()
    );
    prepare(&p).await.unwrap();
}

#[tokio::test]
async fn journal_requires_exact_durable_capability_snapshot_for_free_chat() {
    let Some(p) = setup().await else {
        return;
    };
    for pointer in [
        "/object",
        "/platform",
        "/auth/type",
        "/auth/required",
        "/runtime/mode",
        "/runtime/tool_execution",
        "/runtime/split_runtime",
        "/features/run_submission",
        "/features/run_status",
        "/features/run_events_sse",
        "/features/run_stop",
        "/features/runs_idempotency/supported",
        "/features/runs_idempotency/durable",
        "/features/runs_idempotency/retention_seconds",
        "/endpoints/runs/method",
        "/endpoints/runs/path",
        "/endpoints/run_status/path",
        "/endpoints/run_events/path",
        "/endpoints/run_stop/path",
    ] {
        let mut d = p.draft.clone();
        *d.capabilities.pointer_mut(pointer).unwrap() = Value::Null;
        assert!(
            matches!(
                p.repo.prepare_hermes_dispatch(d).await,
                Err(AppError::Unavailable(_))
            ),
            "{pointer}"
        );
    }
    for retention in [json!(86400.0), json!("86400"), json!(86401), json!(0)] {
        let mut d = p.draft.clone();
        d.capabilities["features"]["runs_idempotency"]["retention_seconds"] = retention;
        assert!(p.repo.prepare_hermes_dispatch(d).await.is_err());
    }
    assert!(
        p.repo
            .get_hermes_dispatch_intent(p.draft.message_id)
            .await
            .unwrap()
            .is_none()
    );
    prepare(&p).await.unwrap();
}

#[tokio::test]
async fn journal_claim_rejects_failed_drain_port_change_and_changed_native_delivery() {
    let Some(p) = setup().await else {
        return;
    };
    let intent = prepare(&p).await.unwrap();
    sql(
        &p,
        "UPDATE message_dispatch_outbox SET state='failed' WHERE message_id=$1",
        vec![p.draft.message_id.into()],
    )
    .await;
    assert!(matches!(claim(&p).await, Err(AppError::Conflict(_))));
    sql(
        &p,
        "UPDATE message_dispatch_outbox SET state='dispatching' WHERE message_id=$1",
        vec![p.draft.message_id.into()],
    )
    .await;
    sql(&p,"INSERT INTO agent_config_revisions(agent_id,revision,state,snapshot,created_by_user_id) VALUES($1,1,'draft','{}',$2)",
        vec![p.draft.agent_id.into(),p.owner.into()]).await;
    sql(
        &p,
        "INSERT INTO agent_config_heads(agent_id,desired_revision,draining) VALUES($1,1,true)",
        vec![p.draft.agent_id.into()],
    )
    .await;
    assert!(matches!(claim(&p).await, Err(AppError::Conflict(_))));
    sql(
        &p,
        "UPDATE agent_config_heads SET draining=false WHERE agent_id=$1",
        vec![p.draft.agent_id.into()],
    )
    .await;
    let port = p
        .repo
        .get_agent(p.draft.agent_id)
        .await
        .unwrap()
        .api_port
        .unwrap();
    sql(
        &p,
        "UPDATE agents SET api_port=$2 WHERE id=$1",
        vec![p.draft.agent_id.into(), (port + 1).into()],
    )
    .await;
    assert!(matches!(claim(&p).await, Err(AppError::Conflict(_))));
    sql(
        &p,
        "UPDATE agents SET api_port=$2 WHERE id=$1",
        vec![p.draft.agent_id.into(), port.into()],
    )
    .await;
    sql(
        &p,
        "UPDATE session_messages SET runtime_message_id='untrusted-native' WHERE id=$1",
        vec![p.draft.message_id.into()],
    )
    .await;
    assert!(matches!(claim(&p).await, Err(AppError::Conflict(_))));
    sql(
        &p,
        "UPDATE session_messages SET runtime_message_id=NULL WHERE id=$1",
        vec![p.draft.message_id.into()],
    )
    .await;
    assert_eq!(claim(&p).await.unwrap().unwrap().run.id, intent.run.id);
}

#[tokio::test]
async fn journal_preparation_rejects_foreign_message_current_primary_and_executor_role() {
    let Some(p) = setup().await else {
        return;
    };
    let other = p
        .repo
        .create_session(chat(p.draft.agent_id, "foreign-message"), p.owner)
        .await
        .unwrap();
    let mut d = p.draft.clone();
    d.session_id = other.id;
    d.requested_session_id = format!("fleet:{}:{}", other.id, d.agent_id);
    assert!(matches!(
        p.repo.prepare_hermes_dispatch(d).await,
        Err(AppError::Conflict(_))
    ));
    let another = agent(&p.repo).await;
    let mut d = p.draft.clone();
    d.agent_id = another;
    d.requested_session_id = format!("fleet:{}:{another}", d.session_id);
    assert!(matches!(
        p.repo.prepare_hermes_dispatch(d).await,
        Err(AppError::Conflict(_))
    ));
    let mut d = p.draft.clone();
    d.run_role = SessionRunRole::Executor;
    assert!(matches!(
        p.repo.prepare_hermes_dispatch(d).await,
        Err(AppError::Conflict(_))
    ));
    assert!(
        p.repo
            .get_hermes_dispatch_intent(p.draft.message_id)
            .await
            .unwrap()
            .is_none()
    );
    prepare(&p).await.unwrap();
}

#[tokio::test]
async fn journal_free_legacy_primary_leader_is_supported_without_task_authority() {
    let Some(p) = setup().await else {
        return;
    };
    sql(
        &p,
        "UPDATE agents SET product_role='leader',sdlc_role='project_manager' WHERE id=$1",
        vec![p.draft.agent_id.into()],
    )
    .await;
    let mut d = p.draft.clone();
    d.run_role = SessionRunRole::Leader;
    let intent = p.repo.prepare_hermes_dispatch(d).await.unwrap();
    assert_eq!(intent.run.run_role, SessionRunRole::Leader);
    assert!(claim(&p).await.unwrap().is_some());
}

#[tokio::test]
async fn journal_new_input_must_match_original_bytes_or_exact_existing_leader_prefix() {
    let Some(p) = setup().await else {
        return;
    };
    let mut forged = p.draft.clone();
    forged.input = "Unrelated caller-authored replacement".into();
    assert!(matches!(
        p.repo.prepare_hermes_dispatch(forged).await,
        Err(AppError::Conflict(_))
    ));
    let leader = agent(&p.repo).await;
    sql(&p,"UPDATE session_messages SET author_type='agent',author_user_id=NULL,author_agent_id=$2 WHERE id=$1",
        vec![p.draft.message_id.into(),leader.into()]).await;
    assert!(matches!(prepare(&p).await, Err(AppError::Conflict(_))));
    let mut valid = p.draft.clone();
    valid.input = format!(
        "[Fleet Control]\nSession: Private task\nTask: not set\nMessage from leader agent: {leader}\n\n{}",
        p.draft.input
    );
    let intent = p.repo.prepare_hermes_dispatch(valid.clone()).await.unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&intent.request_body).unwrap()["input"],
        valid.input
    );
    assert!(claim(&p).await.unwrap().is_some());
}

#[tokio::test]
async fn journal_capacity_held_unknown_submission_does_not_allow_new_intent() {
    let Some(p) = setup().await else {
        return;
    };
    let first = prepare(&p).await.unwrap();
    claim(&p).await.unwrap().unwrap();
    let other = p
        .repo
        .create_session(chat(p.draft.agent_id, "held-journal-slot"), p.owner)
        .await
        .unwrap();
    let message = p
        .repo
        .create_session_message(other.id, prompt("held-next-message"), p.owner)
        .await
        .unwrap();
    let mut next = p.draft.clone();
    next.message_id = message.id;
    next.session_id = other.id;
    next.requested_session_id = format!("fleet:{}:{}", other.id, next.agent_id);
    next.input = message.body;
    assert!(matches!(
        p.repo.prepare_hermes_dispatch(next).await,
        Err(AppError::Conflict(_))
    ));
    let read = p
        .repo
        .get_hermes_dispatch_intent(p.draft.message_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(read.run.id, first.run.id);
    assert_eq!(read.run.state, SessionRunState::Pending);
    assert!(read.submission_attempted);
    assert!(claim(&p).await.unwrap().is_none());
    let states=p.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT m.delivery_state,m.runtime_message_id,o.state AS outbox_state FROM session_messages m
         JOIN message_dispatch_outbox o ON o.message_id=m.id WHERE m.id=$1",[p.draft.message_id.into()]))
        .await.unwrap().unwrap();
    assert_eq!(
        states.try_get::<String>("", "delivery_state").unwrap(),
        "pending"
    );
    assert_eq!(
        states
            .try_get::<Option<String>>("", "runtime_message_id")
            .unwrap(),
        None
    );
    assert_eq!(
        states.try_get::<String>("", "outbox_state").unwrap(),
        "dispatching"
    );
    assert!(
        p.repo
            .get_hermes_dispatch_intent(message.id)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn journal_task_binding_and_pm_binding_never_authorize_prepare_or_claim() {
    let Some(p) = setup().await else {
        return;
    };
    let intent = prepare(&p).await.unwrap();
    let project = Uuid::new_v4();
    let task = Uuid::new_v4();
    sql(&p,"INSERT INTO task_chat_bindings(session_id,tracker_instance_id,project_id,task_id,root_task_id,agent_id,owner_subject,idempotency_key)
        VALUES($1,'journal-fixture',$2,$3,$3,$4,'fixture-owner','journal-bound')",
        vec![p.draft.session_id.into(),project.into(),task.into(),p.draft.agent_id.into()]).await;
    assert!(matches!(prepare(&p).await, Err(AppError::Conflict(_))));
    assert!(matches!(claim(&p).await, Err(AppError::Conflict(_))));
    sql(&p,"INSERT INTO pm_run_bindings(session_run_id,session_id,agent_id,reservation,dispatch_operation_key,runtime_session_id)
        VALUES($1,$2,$3,'{}','journal-pm-fixture',$4)",vec![intent.run.id.into(),p.draft.session_id.into(),p.draft.agent_id.into(),p.draft.requested_session_id.clone().into()]).await;
    // Make the PM binding refer to a different task-bound chat, exercising PM rejection without this chat's task row.
    let other = p
        .repo
        .create_session(chat(p.draft.agent_id, "pm-source"), p.owner)
        .await
        .unwrap();
    sql(&p,"INSERT INTO task_chat_bindings(session_id,tracker_instance_id,project_id,task_id,root_task_id,agent_id,owner_subject,idempotency_key)
        VALUES($1,'journal-fixture',$2,$3,$3,$4,'fixture-owner','journal-other')",
        vec![other.id.into(),project.into(),Uuid::new_v4().into(),p.draft.agent_id.into()]).await;
    // Owned fixture SQL only: trigger protects updates, so replace the fixture binding before removing its task row.
    sql(
        &p,
        "DELETE FROM pm_run_bindings WHERE session_run_id=$1",
        vec![intent.run.id.into()],
    )
    .await;
    sql(&p,"INSERT INTO pm_run_bindings(session_run_id,session_id,agent_id,reservation,dispatch_operation_key,runtime_session_id)
        VALUES($1,$2,$3,'{}','journal-pm-fixture',$4)",vec![intent.run.id.into(),other.id.into(),p.draft.agent_id.into(),p.draft.requested_session_id.clone().into()]).await;
    sql(
        &p,
        "DELETE FROM task_chat_bindings WHERE session_id=$1",
        vec![p.draft.session_id.into()],
    )
    .await;
    assert!(matches!(prepare(&p).await, Err(AppError::Conflict(_))));
    assert!(matches!(claim(&p).await, Err(AppError::Conflict(_))));
}

#[tokio::test]
async fn journal_expired_horizon_readback_and_replay_never_renew_or_release_capacity() {
    let Some(p) = setup().await else {
        return;
    };
    let original = prepare(&p).await.unwrap();
    // Simulate elapsed DB time only on this disposable fixture row. Session-local bypass never disables shared triggers.
    let txn = p.db.begin().await.unwrap();
    txn.execute_unprepared("SET LOCAL session_replication_role=replica")
        .await
        .unwrap();
    txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE hermes_dispatch_journal SET created_at=created_at-interval '2 days',recovery_deadline=recovery_deadline-interval '2 days' WHERE message_id=$1",
        [p.draft.message_id.into()])).await.unwrap();
    txn.commit().await.unwrap();
    let before = p
        .repo
        .get_hermes_dispatch_intent(p.draft.message_id)
        .await
        .unwrap()
        .unwrap();
    assert!(claim(&p).await.unwrap().is_none());
    let replay = prepare(&p).await.unwrap();
    assert_eq!(replay.request_hash, original.request_hash);
    assert_eq!(replay.recovery_deadline, before.recovery_deadline);
    assert!(!replay.submission_attempted);
    assert_eq!(replay.run.state, SessionRunState::Pending);
    assert_eq!(
        replay.run.runtime_session_id,
        Some(p.draft.requested_session_id.clone())
    );
    assert!(
        p.repo
            .prepare_session_agent_run(
                p.draft.session_id,
                p.draft.agent_id,
                SessionRunRole::Primary,
                "replacement".into()
            )
            .await
            .is_err()
    );
}

#[tokio::test]
async fn journal_database_identity_delete_hash_and_state_tamper_fail_closed() {
    let Some(p) = setup().await else {
        return;
    };
    prepare(&p).await.unwrap();
    for statement in [
        "UPDATE hermes_dispatch_journal SET request_body='{}' WHERE message_id=$1",
        "UPDATE hermes_dispatch_journal SET request_hash=repeat('b',64) WHERE message_id=$1",
        "UPDATE hermes_dispatch_journal SET credential_fingerprint=repeat('b',64) WHERE message_id=$1",
        "UPDATE hermes_dispatch_journal SET recovery_deadline=recovery_deadline+interval '1 second' WHERE message_id=$1",
        "UPDATE hermes_dispatch_journal SET state='accepted' WHERE message_id=$1",
        "DELETE FROM hermes_dispatch_journal WHERE message_id=$1",
    ] {
        assert!(
            p.db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                statement,
                [p.draft.message_id.into()]
            ))
            .await
            .is_err()
        );
    }
    let first = claim(&p).await.unwrap().unwrap();
    assert!(p.db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE hermes_dispatch_journal SET state='prepared',submitted_at=NULL WHERE message_id=$1",[p.draft.message_id.into()])).await.is_err());
    assert!(
        p.db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE hermes_dispatch_journal SET state='accepted' WHERE message_id=$1",
            [p.draft.message_id.into()]
        ))
        .await
        .is_err()
    );
    assert_eq!(
        p.repo
            .get_hermes_dispatch_intent(p.draft.message_id)
            .await
            .unwrap()
            .unwrap()
            .submitted_at,
        first.submitted_at
    );
}

#[tokio::test]
async fn journal_accepted_ack_replay_is_terminal_and_never_reclaims_submission() {
    let Some(p) = setup().await else {
        return;
    };
    let prepared = prepare(&p).await.unwrap();
    let claimed = claim(&p).await.unwrap().unwrap();
    p.repo
        .accept_hermes_run(
            p.draft.message_id,
            prepared.run.id,
            "run_journal_ack".into(),
        )
        .await
        .unwrap();
    // ACK owner integration may already have transitioned; same-state replay is allowed unchanged.
    sql(
        &p,
        "UPDATE hermes_dispatch_journal SET state='accepted' WHERE message_id=$1",
        vec![p.draft.message_id.into()],
    )
    .await;
    let read = p
        .repo
        .get_hermes_dispatch_intent(p.draft.message_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(read.state, "accepted");
    assert_eq!(read.submitted_at, claimed.submitted_at);
    assert!(read.submission_attempted);
    assert!(claim(&p).await.unwrap().is_none());
    p.repo
        .pin_hermes_run_session(
            prepared.run.id,
            "run_journal_ack".into(),
            p.draft.requested_session_id.clone(),
            "native:journal-fixture".into(),
        )
        .await
        .unwrap();
    p.repo
        .update_session_agent_run_dispatch(
            prepared.run.id,
            Some("run_journal_ack".into()),
            SessionRunState::Completed,
            None,
        )
        .await
        .unwrap();
    p.repo
        .update_session_message_delivery(
            p.draft.message_id,
            domain::MessageDeliveryState::Completed,
            Some("run_journal_ack".into()),
            None,
        )
        .await
        .unwrap();
    p.repo
        .accept_hermes_run(
            p.draft.message_id,
            prepared.run.id,
            "run_journal_ack".into(),
        )
        .await
        .unwrap();
    let replay = prepare(&p).await.unwrap();
    assert_eq!(replay.request_body, prepared.request_body);
    assert_eq!(
        replay.run.runtime_run_id.as_deref(),
        Some("run_journal_ack")
    );
    assert_eq!(replay.run.state, SessionRunState::Completed);
    assert_eq!(
        replay.run.runtime_session_id.as_deref(),
        Some("native:journal-fixture")
    );
    assert_eq!(replay.submitted_at, claimed.submitted_at);
    assert!(claim(&p).await.unwrap().is_none());
}
