use super::*;
use domain::{MessageDeliveryState, SessionAgentRun};
use sea_orm::DatabaseConnection;
use serde_json::Value;
use shared::AppError;
use tokio::time::timeout;

struct Prepared {
    repo: PostgresFleetRepository,
    db: DatabaseConnection,
    owner: Uuid,
    agent_id: Uuid,
    session_id: Uuid,
    message_id: Uuid,
    run: SessionAgentRun,
    requested: String,
}

async fn database() -> DatabaseConnection {
    sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap()
}

async fn prepared() -> Option<Prepared> {
    let (repo, owner, _) = fixture().await?;
    let db = database().await;
    let agent_id = agent(&repo).await;
    let session = repo
        .create_session(chat(agent_id, "acceptance"), owner)
        .await
        .unwrap();
    let message = repo
        .create_session_message(session.id, prompt("accept-once"), owner)
        .await
        .unwrap();
    let requested = format!("fleet:{}:{agent_id}", session.id);
    let run = repo
        .prepare_session_agent_run(
            session.id,
            agent_id,
            SessionRunRole::Primary,
            requested.clone(),
        )
        .await
        .unwrap();
    // Fixture-only claim of this message; never consume another test's queued dispatch.
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE message_dispatch_outbox SET state='dispatching' WHERE message_id=$1",
        [message.id.into()],
    ))
    .await
    .unwrap();
    Some(Prepared {
        repo,
        db,
        owner,
        agent_id,
        session_id: session.id,
        message_id: message.id,
        run,
        requested,
    })
}

async fn snapshot(p: &Prepared) -> Value {
    p.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT jsonb_build_object('run',to_jsonb(r),'message',to_jsonb(m),'outbox',to_jsonb(o),
            'journal',(SELECT to_jsonb(j) FROM hermes_dispatch_journal j WHERE j.run_id=r.id),
            'cursor',(SELECT sequence FROM session_event_cursors WHERE session_id=r.session_id),
            'events',(SELECT count(*) FROM session_events WHERE session_id=r.session_id)) AS snapshot
         FROM session_agent_runs r,session_messages m,message_dispatch_outbox o
         WHERE r.id=$1 AND m.id=$2 AND o.message_id=m.id", [p.run.id.into(), p.message_id.into()]))
        .await.unwrap().unwrap().try_get("", "snapshot").unwrap()
}

async fn accept(p: &Prepared, native: &str) -> Result<SessionAgentRun, AppError> {
    p.repo
        .accept_hermes_run(p.message_id, p.run.id, native.into())
        .await
}

async fn pin(
    p: &Prepared,
    native: &str,
    effective: &str,
) -> Result<(SessionAgentRun, bool), AppError> {
    p.repo
        .pin_hermes_run_session(
            p.run.id,
            native.into(),
            p.requested.clone(),
            effective.into(),
        )
        .await
}

#[tokio::test]
async fn acceptance_late_outbox_failure_rolls_back_run_message_and_event_cursor() {
    let Some(p) = prepared().await else {
        return;
    };
    let before = snapshot(&p).await;
    let name = format!("acceptance_fault_{}", Uuid::new_v4().simple());
    p.db.execute_unprepared(&format!(
        "CREATE FUNCTION {name}() RETURNS trigger AS $$ BEGIN
         IF NEW.message_id='{}'::uuid AND NEW.state='dispatched' THEN
           RAISE EXCEPTION 'injected acceptance rollback'; END IF; RETURN NEW; END; $$ LANGUAGE plpgsql;
         CREATE TRIGGER {name} BEFORE UPDATE ON message_dispatch_outbox FOR EACH ROW EXECUTE FUNCTION {name}()", p.message_id
    )).await.unwrap();
    let result = accept(&p, "run_atomic").await;
    p.db.execute_unprepared(&format!(
        "DROP TRIGGER {name} ON message_dispatch_outbox; DROP FUNCTION {name}()"
    ))
    .await
    .unwrap();
    assert!(result.is_err());
    assert_eq!(snapshot(&p).await, before);
    let run = accept(&p, "run_atomic").await.unwrap();
    assert_eq!(run.state, SessionRunState::Pending);
    let after = snapshot(&p).await;
    assert_eq!(after["run"]["runtime_run_id"], "run_atomic");
    assert_eq!(after["message"]["runtime_message_id"], "run_atomic");
    assert_eq!(after["message"]["delivery_state"], "dispatched");
    assert_eq!(after["outbox"]["state"], "dispatched");
}

#[tokio::test]
async fn acceptance_concurrent_replay_is_atomic_immutable_and_holds_capacity_before_pin() {
    let Some(p) = prepared().await else {
        return;
    };
    let (a, b) = timeout(Duration::from_secs(10), async {
        tokio::join!(accept(&p, "run_concurrent"), accept(&p, "run_concurrent"))
    })
    .await
    .unwrap();
    for result in [a, b] {
        let run = result.unwrap();
        assert_eq!(run.state, SessionRunState::Pending);
        assert_eq!(run.runtime_run_id.as_deref(), Some("run_concurrent"));
        assert_eq!(
            run.runtime_session_id.as_deref(),
            Some(p.requested.as_str())
        );
    }
    let before = snapshot(&p).await;
    accept(&p, "run_concurrent").await.unwrap();
    assert!(matches!(
        accept(&p, "run_changed").await,
        Err(AppError::Conflict(_))
    ));
    assert_eq!(snapshot(&p).await, before);
    let other = p
        .repo
        .create_session(chat(p.agent_id, "capacity-second"), p.owner)
        .await
        .unwrap();
    assert!(matches!(
        p.repo
            .prepare_session_agent_run(
                other.id,
                p.agent_id,
                SessionRunRole::Primary,
                format!("fleet:{}:{}", other.id, p.agent_id)
            )
            .await,
        Err(AppError::Conflict(_))
    ));
}

#[tokio::test]
async fn pin_concurrent_winner_once_and_late_ack_replays_preserve_terminal_delivery_and_timestamps()
{
    let Some(p) = prepared().await else {
        return;
    };
    assert!(matches!(
        pin(&p, "run_pin", "effective").await,
        Err(AppError::Conflict(_))
    ));
    accept(&p, "run_pin").await.unwrap();
    assert!(matches!(
        p.repo
            .pin_hermes_run_session(
                p.run.id,
                "run_pin".into(),
                "wrong_alias".into(),
                "effective".into()
            )
            .await,
        Err(AppError::Conflict(_))
    ));
    assert!(matches!(
        pin(&p, "different_run", "effective").await,
        Err(AppError::Conflict(_))
    ));
    // Hermes may retain the requested fleet: alias as the actual effective session.
    let (a, b) = timeout(Duration::from_secs(10), async {
        tokio::join!(
            pin(&p, "run_pin", &p.requested),
            pin(&p, "run_pin", &p.requested)
        )
    })
    .await
    .unwrap();
    let (a, b) = (a.unwrap(), b.unwrap());
    assert_ne!(a.1, b.1);
    assert_eq!(a.0.state, SessionRunState::Running);
    assert_eq!(b.0.state, SessionRunState::Running);
    let before = snapshot(&p).await;
    assert!(!pin(&p, "run_pin", &p.requested).await.unwrap().1);
    assert!(matches!(
        pin(&p, "run_pin", "foreign_effective").await,
        Err(AppError::Conflict(_))
    ));
    assert_eq!(snapshot(&p).await, before);
    for (state, delivery) in [
        (SessionRunState::Completed, MessageDeliveryState::Completed),
        (SessionRunState::Failed, MessageDeliveryState::Failed),
        (SessionRunState::Cancelled, MessageDeliveryState::Failed),
    ] {
        let p = prepared().await.unwrap();
        accept(&p, "run_pin").await.unwrap();
        pin(&p, "run_pin", &p.requested).await.unwrap();
        p.repo
            .update_session_agent_run_dispatch(
                p.run.id,
                Some("run_pin".into()),
                state,
                Some("terminal test".into()),
            )
            .await
            .unwrap();
        p.repo
            .update_session_message_delivery(
                p.message_id,
                delivery,
                Some("run_pin".into()),
                Some("terminal test".into()),
            )
            .await
            .unwrap();
        let before = snapshot(&p).await;
        assert_eq!(accept(&p, "run_pin").await.unwrap().state, state);
        let (run, winner) = pin(&p, "run_pin", &p.requested).await.unwrap();
        assert_eq!(run.state, state);
        assert!(!winner);
        assert!(matches!(
            pin(&p, "run_pin", "changed_effective").await,
            Err(AppError::Conflict(_))
        ));
        assert!(matches!(
            accept(&p, "changed_run").await,
            Err(AppError::Conflict(_))
        ));
        assert_eq!(snapshot(&p).await, before);
    }
}

#[tokio::test]
async fn pin_changes_alias_once_and_rejects_inconsistent_dispatch_proof() {
    let Some(p) = prepared().await else {
        return;
    };
    accept(&p, "run_actual").await.unwrap();
    let before = snapshot(&p).await;
    p.db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE message_dispatch_outbox SET state='failed' WHERE message_id=$1",
        [p.message_id.into()],
    ))
    .await
    .unwrap();
    assert!(matches!(
        pin(&p, "run_actual", "session_actual").await,
        Err(AppError::Conflict(_))
    ));
    assert_eq!(snapshot(&p).await["run"], before["run"]);
    p.db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE message_dispatch_outbox SET state='dispatched' WHERE message_id=$1",
        [p.message_id.into()],
    ))
    .await
    .unwrap();
    let (run, winner) = pin(&p, "run_actual", "session_actual").await.unwrap();
    assert!(winner);
    assert_eq!(run.runtime_session_id.as_deref(), Some("session_actual"));
    assert!(!pin(&p, "run_actual", "session_actual").await.unwrap().1);
    assert!(matches!(
        pin(&p, "run_actual", "session_changed").await,
        Err(AppError::Conflict(_))
    ));
}

#[tokio::test]
async fn acceptance_rejects_wrong_message_session_agent_primary_kind_and_unclaimed_outbox() {
    let Some(p) = prepared().await else {
        return;
    };
    let other_agent = agent(&p.repo).await;
    let other_session = p
        .repo
        .create_session(chat(other_agent, "other-acceptance"), p.owner)
        .await
        .unwrap();
    let other_message = p
        .repo
        .create_session_message(other_session.id, prompt("other-message"), p.owner)
        .await
        .unwrap();
    let before = snapshot(&p).await;
    assert!(matches!(
        p.repo
            .accept_hermes_run(other_message.id, p.run.id, "run_wrong".into())
            .await,
        Err(AppError::Conflict(_))
    ));
    assert_eq!(snapshot(&p).await, before);
    for (sql, restore) in [
        (
            "UPDATE message_dispatch_outbox SET agent_id=$2 WHERE message_id=$1",
            "UPDATE message_dispatch_outbox SET agent_id=$2 WHERE message_id=$1",
        ),
        (
            "UPDATE agent_sessions SET agent_id=$2 WHERE id=$1",
            "UPDATE agent_sessions SET agent_id=$2 WHERE id=$1",
        ),
    ] {
        let id = if sql.contains("agent_sessions") {
            p.session_id
        } else {
            p.message_id
        };
        p.db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            sql,
            [id.into(), other_agent.into()],
        ))
        .await
        .unwrap();
        assert!(matches!(
            accept(&p, "run_wrong").await,
            Err(AppError::Conflict(_))
        ));
        p.db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            restore,
            [id.into(), p.agent_id.into()],
        ))
        .await
        .unwrap();
    }
    p.db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE session_messages SET message_kind='assistant_message' WHERE id=$1",
        [p.message_id.into()],
    ))
    .await
    .unwrap();
    assert!(matches!(
        accept(&p, "run_wrong").await,
        Err(AppError::Conflict(_))
    ));
    p.db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE session_messages SET message_kind='user_prompt' WHERE id=$1",
        [p.message_id.into()],
    ))
    .await
    .unwrap();
    p.db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE message_dispatch_outbox SET state='pending' WHERE message_id=$1",
        [p.message_id.into()],
    ))
    .await
    .unwrap();
    assert!(matches!(
        accept(&p, "run_wrong").await,
        Err(AppError::Conflict(_))
    ));
    for invalid in ["../run", "run:unsafe", "", "run with space"] {
        assert!(accept(&p, invalid).await.is_err());
    }
    assert!(
        p.repo
            .accept_hermes_run(Uuid::nil(), p.run.id, "run_wrong".into())
            .await
            .is_err()
    );
    assert!(
        p.repo
            .accept_hermes_run(p.message_id, Uuid::nil(), "run_wrong".into())
            .await
            .is_err()
    );
    assert_eq!(
        p.repo
            .get_session_agent_run(p.run.id)
            .await
            .unwrap()
            .runtime_run_id,
        None
    );
}

#[tokio::test]
async fn acceptance_control_agent_author_and_unbound_pm_leader_are_not_assignment_scopes() {
    let Some(p) = prepared().await else {
        return;
    };
    p.db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE agents SET sdlc_role='project_manager' WHERE id=$1",
        [p.agent_id.into()],
    ))
    .await
    .unwrap();
    p.db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE session_agent_runs SET run_role='leader' WHERE id=$1",
        [p.run.id.into()],
    ))
    .await
    .unwrap();
    p.db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE session_messages SET message_kind='control',author_type='agent',author_user_id=NULL,author_agent_id=$2 WHERE id=$1",
        [p.message_id.into(), p.agent_id.into()])).await.unwrap();
    let run = accept(&p, "run_free_control").await.unwrap();
    assert_eq!(run.state, SessionRunState::Pending);
    assert_eq!(run.run_role, SessionRunRole::Leader);
    assert!(
        pin(&p, "run_free_control", "session_free_control")
            .await
            .unwrap()
            .1
    );
}

#[tokio::test]
async fn acceptance_runtime_id_cannot_be_reused_by_another_run_or_message_on_same_agent() {
    let Some(p) = prepared().await else {
        return;
    };
    accept(&p, "run_unique").await.unwrap();
    pin(&p, "run_unique", "session_unique").await.unwrap();
    p.repo
        .update_session_agent_run_dispatch(
            p.run.id,
            Some("run_unique".into()),
            SessionRunState::Completed,
            None,
        )
        .await
        .unwrap();
    let next = p
        .repo
        .create_session(chat(p.agent_id, "next-runtime"), p.owner)
        .await
        .unwrap();
    let message = p
        .repo
        .create_session_message(next.id, prompt("next-runtime-message"), p.owner)
        .await
        .unwrap();
    let run = p
        .repo
        .prepare_session_agent_run(
            next.id,
            p.agent_id,
            SessionRunRole::Primary,
            format!("fleet:{}:{}", next.id, p.agent_id),
        )
        .await
        .unwrap();
    p.db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE message_dispatch_outbox SET state='dispatching' WHERE message_id=$1",
        [message.id.into()],
    ))
    .await
    .unwrap();
    assert!(matches!(
        p.repo
            .accept_hermes_run(message.id, run.id, "run_unique".into())
            .await,
        Err(AppError::Conflict(_))
    ));
    assert_eq!(
        p.repo
            .get_session_agent_run(run.id)
            .await
            .unwrap()
            .runtime_run_id,
        None
    );
    p.repo
        .accept_hermes_run(message.id, run.id, "run_next".into())
        .await
        .unwrap();
    assert!(matches!(
        p.repo
            .accept_hermes_run(p.message_id, run.id, "run_next".into())
            .await,
        Err(AppError::Conflict(_))
    ));
}

#[tokio::test]
async fn acceptance_and_pin_cannot_bypass_task_chat_or_pm_run_binding() {
    let Some((repo, request)) = pm_fixture().await else {
        return;
    };
    let db = database().await;
    repo.reserve_pm_run(request.clone()).await.unwrap();
    let message = repo
        .insert_session_message_mirror(
            request.session_id,
            Some(request.identity.agent_ref.parse().unwrap()),
            "bound fixture".into(),
            MessageKind::UserPrompt,
            None,
        )
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO message_dispatch_outbox(message_id,agent_id,state) VALUES($1,$2,'dispatching')",
        [message.id.into(), request.identity.agent_ref.parse::<Uuid>().unwrap().into()])).await.unwrap();
    let before =
        serde_json::to_value(repo.get_pm_run(request.session_run_id).await.unwrap()).unwrap();
    assert!(matches!(
        repo.accept_hermes_run(message.id, request.session_run_id, "run_bound".into())
            .await,
        Err(AppError::Conflict(_))
    ));
    assert!(matches!(
        repo.pin_hermes_run_session(
            request.session_run_id,
            "run_bound".into(),
            request.runtime_session_id(),
            "actual_bound".into()
        )
        .await,
        Err(AppError::Conflict(_))
    ));
    assert_eq!(
        serde_json::to_value(repo.get_pm_run(request.session_run_id).await.unwrap()).unwrap(),
        before
    );
    repo.accept_pm_run(
        request.session_run_id,
        "run_bound".into(),
        request.runtime_session_id(),
    )
    .await
    .unwrap();
    // Keep the task binding after removing the PM row to exercise the task-only boundary.
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "DELETE FROM pm_run_bindings WHERE session_run_id=$1",
        [request.session_run_id.into()],
    ))
    .await
    .unwrap();
    assert!(matches!(
        repo.accept_hermes_run(message.id, request.session_run_id, "run_bound".into())
            .await,
        Err(AppError::Conflict(_))
    ));
    assert!(matches!(
        repo.pin_hermes_run_session(
            request.session_run_id,
            "run_bound".into(),
            request.runtime_session_id(),
            "actual_bound".into()
        )
        .await,
        Err(AppError::Conflict(_))
    ));
}

#[tokio::test]
async fn pin_failure_rolls_back_alias_and_state_then_retry_has_exactly_one_winner() {
    let Some(p) = prepared().await else {
        return;
    };
    accept(&p, "run_pin_rollback").await.unwrap();
    let before = snapshot(&p).await;
    let name = format!("pin_fault_{}", Uuid::new_v4().simple());
    p.db.execute_unprepared(&format!(
        "CREATE FUNCTION {name}() RETURNS trigger AS $$ BEGIN
         IF NEW.id='{}'::uuid AND NEW.state='running' THEN
           RAISE EXCEPTION 'injected pin rollback'; END IF; RETURN NEW; END; $$ LANGUAGE plpgsql;
         CREATE TRIGGER {name} AFTER UPDATE ON session_agent_runs FOR EACH ROW EXECUTE FUNCTION {name}()", p.run.id
    )).await.unwrap();
    let result = pin(&p, "run_pin_rollback", "actual_pin_rollback").await;
    p.db.execute_unprepared(&format!(
        "DROP TRIGGER {name} ON session_agent_runs; DROP FUNCTION {name}()"
    ))
    .await
    .unwrap();
    assert!(result.is_err());
    assert_eq!(snapshot(&p).await, before);
    assert!(
        pin(&p, "run_pin_rollback", "actual_pin_rollback")
            .await
            .unwrap()
            .1
    );
    assert!(
        !pin(&p, "run_pin_rollback", "actual_pin_rollback")
            .await
            .unwrap()
            .1
    );
}

#[tokio::test]
async fn acceptance_postcommit_agent_read_failure_keeps_ack_recoverable_without_resubmission() {
    use futures_util::FutureExt;
    use std::panic::AssertUnwindSafe;

    let Some(p) = prepared().await else {
        return;
    };
    // This fault now includes the production request journal, not a legacy unattested ACK.
    p.db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE session_agent_runs SET runtime_session_id=NULL WHERE id=$1",
        [p.run.id.into()],
    ))
    .await
    .unwrap();
    let agent = p.repo.get_agent(p.agent_id).await.unwrap();
    let message = p
        .repo
        .list_session_messages(p.session_id)
        .await
        .unwrap()
        .into_iter()
        .find(|message| message.id == p.message_id)
        .unwrap();
    let origin = format!("http://127.0.0.1:{}", agent.api_port.unwrap());
    p.repo
        .prepare_hermes_dispatch(app::HermesDispatchDraft {
            message_id: p.message_id,
            session_id: p.session_id,
            agent_id: p.agent_id,
            run_role: SessionRunRole::Primary,
            requested_session_id: p.requested.clone(),
            input: message.body,
            origin: origin.clone(),
            credential_fingerprint: "a".repeat(64),
            capabilities: hermes_protocol_fixture::capabilities(),
        })
        .await
        .unwrap();
    p.repo
        .claim_hermes_submission(p.message_id, origin, "a".repeat(64))
        .await
        .unwrap()
        .unwrap();
    let role = format!("acceptance_read_fault_{}", Uuid::new_v4().simple());
    p.db.execute_unprepared(&format!("CREATE ROLE {role} NOLOGIN NOINHERIT"))
        .await
        .unwrap();
    let mut limited_db = None;
    // Catch assertion failures so the unique disposable role is cleaned up as well.
    let outcome = AssertUnwindSafe(async {
        p.db.execute_unprepared(&format!(
            "GRANT USAGE ON SCHEMA public TO {role};
             GRANT SELECT,INSERT,UPDATE,DELETE ON agent_sessions,session_agent_runs,session_messages,
                message_dispatch_outbox,hermes_dispatch_journal,task_chat_bindings,pm_run_bindings,
                session_event_cursors,session_events TO {role};
             GRANT SELECT(id,kind),UPDATE(kind) ON agents TO {role}"
        )).await.unwrap();
        let mut url = reqwest::Url::parse(&std::env::var("FLEET_TEST_DATABASE_URL").unwrap()).unwrap();
        let pairs: Vec<(String, String)> = url.query_pairs().map(|(key, value)| (key.into_owned(), value.into_owned())).collect();
        let mut options = String::new();
        url.set_query(None);
        for (key, value) in pairs {
            if key == "options" {
                options.push_str(&value);
                options.push(' ');
            } else {
                url.query_pairs_mut().append_pair(&key, &value);
            }
        }
        options.push_str(&format!("-c role={role}"));
        // Startup options apply the restricted role on every physical connection/reconnect.
        url.query_pairs_mut().append_pair("options", &options);
        let mut config = sea_orm::ConnectOptions::new(url.to_string());
        config.max_connections(1).min_connections(1).connect_timeout(Duration::from_secs(10));
        let db = sea_orm::Database::connect(config).await.unwrap();
        limited_db = Some(DatabaseConnection::from(db.get_postgres_connection_pool().clone()));
        let privileges = db.query_one(Statement::from_string(DatabaseBackend::Postgres,
            "SELECT current_user AS role,has_column_privilege(current_user,'agents','kind','SELECT') AS narrow,
                has_table_privilege(current_user,'agents','SELECT') AS full".to_string())).await.unwrap().unwrap();
        assert_eq!(privileges.try_get::<String>("", "role").unwrap(), role);
        assert!(privileges.try_get::<bool>("", "narrow").unwrap());
        assert!(!privileges.try_get::<bool>("", "full").unwrap());
        let limited_repo = PostgresFleetRepository::new(db);
        let result = limited_repo.accept_hermes_run(p.message_id, p.run.id, "run_postcommit_fault".into()).await;
        assert!(matches!(result, Err(AppError::Database(_))));
        // Real post-commit failure: native mapping, delivery, outbox and journal all survived.
        let durable = snapshot(&p).await;
        assert_eq!(durable["run"]["state"], "pending");
        assert_eq!(durable["run"]["runtime_run_id"], "run_postcommit_fault");
        assert_eq!(durable["message"]["runtime_message_id"], "run_postcommit_fault");
        assert_eq!(durable["message"]["delivery_state"], "dispatched");
        assert_eq!(durable["outbox"]["state"], "dispatched");
        assert_eq!(durable["journal"]["state"], "accepted");
        assert!(durable["journal"]["submitted_at"].is_string());
        assert!(durable["journal"]["accepted_at"].is_string());
        // Reproduce send_message's late error path using the same restricted connection.
        limited_repo.update_session_message_delivery(p.message_id, MessageDeliveryState::Failed,
            None, Some("post-commit result read failed".into())).await.unwrap();
        assert_eq!(snapshot(&p).await, durable);
        let queued = p.repo.list_pending_hermes_acceptances(None).await.unwrap();
        assert!(queued.iter().any(|(message, run)| message.id == p.message_id && run.id == p.run.id
            && run.runtime_run_id.as_deref() == Some("run_postcommit_fault")));
        assert!(p.repo.prepare_session_agent_run(p.session_id, p.agent_id, SessionRunRole::Primary,
            p.requested.clone()).await.is_err());
        // Recovery uses only the existing native ID, never prepares/submits another run.
        let (pinned, first) = pin(&p, "run_postcommit_fault", "session_postcommit_fault").await.unwrap();
        assert!(first);
        assert_eq!(pinned.state, SessionRunState::Running);
        assert!(!pin(&p, "run_postcommit_fault", "session_postcommit_fault").await.unwrap().1);
        let runs = p.repo.list_session_agent_runs(p.session_id).await.unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].id, p.run.id);
        assert!(p.repo.list_pending_hermes_acceptances(None).await.unwrap().iter()
            .all(|(_, run)| run.id != p.run.id));
    }).catch_unwind().await;
    let mut reset = Ok(());
    let mut closed = Ok(());
    if let Some(db) = limited_db {
        reset = db.execute_unprepared("RESET ROLE").await.map(|_| ());
        closed = db.close().await;
    }
    let owned =
        p.db.execute_unprepared(&format!("DROP OWNED BY {role}"))
            .await;
    let dropped = p.db.execute_unprepared(&format!("DROP ROLE {role}")).await;
    reset.unwrap();
    closed.unwrap();
    owned.unwrap();
    dropped.unwrap();
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn delivery_known_native_identity_rejects_conflicts_and_preserves_terminal_replays() {
    for terminal in [
        MessageDeliveryState::Completed,
        MessageDeliveryState::Failed,
    ] {
        let Some(p) = prepared().await else {
            return;
        };
        accept(&p, "run_delivery_cas").await.unwrap();
        let before = snapshot(&p).await;
        p.repo
            .update_session_message_delivery(
                p.message_id,
                MessageDeliveryState::Failed,
                None,
                Some("unknown late submit error".into()),
            )
            .await
            .unwrap();
        assert!(matches!(
            p.repo
                .update_session_message_delivery(
                    p.message_id,
                    MessageDeliveryState::Dispatched,
                    Some("foreign_native_id".into()),
                    None
                )
                .await,
            Err(AppError::Conflict(_))
        ));
        assert!(matches!(
            p.repo
                .update_session_message_delivery(
                    p.message_id,
                    MessageDeliveryState::Pending,
                    Some("run_delivery_cas".into()),
                    None
                )
                .await,
            Err(AppError::Conflict(_))
        ));
        assert_eq!(snapshot(&p).await, before);
        let (finished, late_error) = tokio::join!(
            p.repo.update_session_message_delivery(
                p.message_id,
                terminal,
                Some("run_delivery_cas".into()),
                None
            ),
            p.repo.update_session_message_delivery(
                p.message_id,
                MessageDeliveryState::Failed,
                None,
                Some("concurrent late error".into())
            )
        );
        finished.unwrap();
        late_error.unwrap();
        let stable = snapshot(&p).await;
        assert_eq!(stable["message"]["delivery_state"], terminal.as_str());
        for state in [
            MessageDeliveryState::Pending,
            MessageDeliveryState::Dispatched,
            MessageDeliveryState::Completed,
            MessageDeliveryState::Failed,
            MessageDeliveryState::Mirrored,
        ] {
            for native in [None, Some("run_delivery_cas".into())] {
                p.repo
                    .update_session_message_delivery(
                        p.message_id,
                        state,
                        native,
                        Some("late replay must not change proof".into()),
                    )
                    .await
                    .unwrap();
            }
        }
        assert!(matches!(
            p.repo
                .update_session_message_delivery(
                    p.message_id,
                    terminal,
                    Some("foreign_native_id".into()),
                    None
                )
                .await,
            Err(AppError::Conflict(_))
        ));
        assert_eq!(snapshot(&p).await, stable);
    }
}
