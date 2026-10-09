use super::*;
use app::{HermesDispatchDraft, HermesTerminalCommit};
use domain::{MessageDeliveryState, SessionAgentRun, SessionMessage};
use sea_orm::{DatabaseConnection, sea_query::Value as SqlValue};
use serde_json::{Value, json};
use shared::AppError;
use tokio::time::timeout;

async fn required_fixture() -> Option<(PostgresFleetRepository, Uuid, Uuid)> {
    std::env::var("FLEET_TEST_DATABASE_URL")
        .expect("isolated PostgreSQL is required for atomic terminal tests");
    super::fixture().await
}

async fn blocked_pid(
    monitor: &DatabaseConnection,
    blocker: i32,
    query_marker: &str,
) -> Result<i32, sea_orm::DbErr> {
    timeout(Duration::from_secs(10), async {
        loop {
            let row = monitor
                .query_one(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    "SELECT pid FROM pg_stat_activity
                 WHERE datname=current_database() AND pid<>pg_backend_pid()
                   AND $1=ANY(pg_blocking_pids(pid)) AND position($2 in query)>0
                 ORDER BY pid LIMIT 1",
                    [blocker.into(), query_marker.into()],
                ))
                .await?;
            if let Some(row) = row {
                return row.try_get("", "pid");
            }
            // Poll actual PG blocking state; elapsed time alone never opens the barrier.
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .map_err(|_| sea_orm::DbErr::Custom("owned terminal blocker was not observed".into()))?
}

async fn progress_lock_regression(prompt: bool) {
    let Some(p) = setup(true, true, SessionRunRole::Primary).await else {
        return;
    };
    let before = snapshot(&p).await;
    let holder = p.db.begin().await.unwrap();
    holder
        .execute_unprepared("SET LOCAL statement_timeout='5s'; SET LOCAL lock_timeout='5s'")
        .await
        .unwrap();
    let holder_pid: i32 = holder
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT pg_backend_pid() AS pid".to_string(),
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap();
    let (table, id, lock, update) = if prompt {
        (
            "session_messages",
            p.message_id,
            "SELECT id FROM session_messages WHERE id=$1 FOR UPDATE",
            "UPDATE session_messages SET delivery_error='fixture progress' WHERE id=$1",
        )
    } else {
        ("session_agent_runs", p.run_id,
            "SELECT id FROM session_agent_runs WHERE id=$1 FOR UPDATE",
            "UPDATE session_agent_runs SET last_error='fixture progress',last_event_at=clock_timestamp(),
                updated_at=clock_timestamp() WHERE id=$1")
    };
    holder
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            lock,
            [id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    let monitor = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let progress = async move {
        blocked_pid(&monitor, holder_pid, table).await?;
        // Terminal now holds the session and is blocked on our run/prompt. This real UPDATE
        // must be able to append its event (session FK key-share) before releasing the row.
        holder
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                update,
                [id.into()],
            ))
            .await?;
        holder.commit().await?;
        Ok::<(), sea_orm::DbErr>(())
    };
    let (terminal, progress) = timeout(Duration::from_secs(25), async {
        tokio::join!(
            commit(
                &p,
                SessionRunState::Completed,
                Some("answer after progress"),
                None
            ),
            progress
        )
    })
    .await
    .unwrap();
    progress.unwrap();
    let (run, assistant, first) = terminal.unwrap();
    assert!(first);
    assert_eq!(run.state, SessionRunState::Completed);
    assert_eq!(assistant.unwrap().body, "answer after progress");
    let after = snapshot(&p).await;
    assert_eq!(after["prompt"]["delivery_state"], "completed");
    assert_eq!(after["prompt"]["delivery_error"], Value::Null);
    assert_eq!(after["run"]["last_error"], Value::Null);
    assert_eq!(after["assistants"].as_array().unwrap().len(), 1);
    assert_eq!(after["journal"], before["journal"]);
    assert_eq!(after["outbox"], before["outbox"]);
    assert_eq!(
        after["cursor"].as_i64().unwrap(),
        before["cursor"].as_i64().unwrap() + 5
    );
    assert_eq!(
        after["events"].as_array().unwrap().len(),
        before["events"].as_array().unwrap().len() + 5
    );
    assert!(
        !commit(
            &p,
            SessionRunState::Completed,
            Some("answer after progress"),
            None
        )
        .await
        .unwrap()
        .2
    );
    assert_eq!(snapshot(&p).await, after);
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn terminal_session_serialization_allows_run_progress_event_fk_before_commit() {
    progress_lock_regression(false).await;
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn terminal_session_serialization_allows_prompt_progress_event_fk_before_commit() {
    progress_lock_regression(true).await;
}

struct TerminalFixture {
    repo: PostgresFleetRepository,
    db: DatabaseConnection,
    owner: Uuid,
    agent_id: Uuid,
    session_id: Uuid,
    message_id: Uuid,
    run_id: Uuid,
    native_run: String,
    effective_session: String,
}

async fn setup(journal: bool, pin: bool, role: SessionRunRole) -> Option<TerminalFixture> {
    let (repo, owner, _) = required_fixture().await?;
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let agent_id = agent(&repo).await;
    if role == SessionRunRole::Leader {
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE agents SET product_role='leader',sdlc_role='project_manager' WHERE id=$1",
            [agent_id.into()],
        ))
        .await
        .unwrap();
    }
    let session = repo
        .create_session(chat(agent_id, "terminal-session"), owner)
        .await
        .unwrap();
    let message = repo
        .create_session_message(session.id, prompt("terminal-prompt"), owner)
        .await
        .unwrap();
    // Claim only this fixture's outbox, never a different test's pending dispatch.
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE message_dispatch_outbox SET state='dispatching' WHERE message_id=$1",
        [message.id.into()],
    ))
    .await
    .unwrap();
    let requested = format!("fleet:{}:{agent_id}", session.id);
    let run = if journal {
        let port = repo.get_agent(agent_id).await.unwrap().api_port.unwrap();
        let origin = format!("http://127.0.0.1:{port}");
        let intent = repo
            .prepare_hermes_dispatch(HermesDispatchDraft {
                message_id: message.id,
                session_id: session.id,
                agent_id,
                run_role: role,
                requested_session_id: requested.clone(),
                input: message.body,
                origin: origin.clone(),
                credential_fingerprint: "a".repeat(64),
                capabilities: hermes_protocol_fixture::capabilities(),
            })
            .await
            .unwrap();
        let claimed = repo
            .claim_hermes_submission(message.id, origin, "a".repeat(64))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(claimed.run.id, intent.run.id);
        assert!(claimed.submission_attempted);
        intent.run
    } else {
        repo.prepare_session_agent_run(session.id, agent_id, role, requested.clone())
            .await
            .unwrap()
    };
    let native_run = format!("run_{}", run.id.simple());
    repo.accept_hermes_run(message.id, run.id, native_run.clone())
        .await
        .unwrap();
    let effective_session = format!("fleet:effective:{}", session.id);
    if pin {
        let (pinned, first) = repo
            .pin_hermes_run_session(
                run.id,
                native_run.clone(),
                requested,
                effective_session.clone(),
            )
            .await
            .unwrap();
        assert!(first);
        assert_eq!(pinned.state, SessionRunState::Running);
    }
    Some(TerminalFixture {
        repo,
        db,
        owner,
        agent_id,
        session_id: session.id,
        message_id: message.id,
        run_id: run.id,
        native_run,
        effective_session,
    })
}

fn command(
    p: &TerminalFixture,
    state: SessionRunState,
    body: Option<&str>,
    error: Option<&str>,
) -> HermesTerminalCommit {
    HermesTerminalCommit {
        message_id: p.message_id,
        run_id: p.run_id,
        runtime_run_id: p.native_run.clone(),
        runtime_session_id: p.effective_session.clone(),
        state,
        body: body.map(str::to_string),
        error: error.map(str::to_string),
    }
}

async fn commit(
    p: &TerminalFixture,
    state: SessionRunState,
    body: Option<&str>,
    error: Option<&str>,
) -> Result<(SessionAgentRun, Option<SessionMessage>, bool), AppError> {
    p.repo
        .commit_hermes_terminal(command(p, state, body, error))
        .await
}

async fn sql(p: &TerminalFixture, query: &str, values: Vec<SqlValue>) {
    p.db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        query,
        values,
    ))
    .await
    .unwrap();
}

async fn snapshot(p: &TerminalFixture) -> Value {
    p.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT jsonb_build_object('run',to_jsonb(r),'prompt',to_jsonb(m),'outbox',to_jsonb(o),
            'session',to_jsonb(s),
            'journal',(SELECT to_jsonb(j) FROM hermes_dispatch_journal j WHERE j.message_id=m.id),
            'transcript',(SELECT COALESCE(jsonb_agg(to_jsonb(t) ORDER BY t.append_sequence),'[]'::jsonb)
                FROM session_messages t WHERE t.session_id=r.session_id),
            'assistants',(SELECT COALESCE(jsonb_agg(to_jsonb(t) ORDER BY t.append_sequence),'[]'::jsonb)
                FROM session_messages t WHERE t.session_id=r.session_id AND t.message_kind='assistant_message'),
            'cursor',(SELECT sequence FROM session_event_cursors WHERE session_id=r.session_id),
            'approvals',(SELECT COALESCE(jsonb_agg(to_jsonb(a) ORDER BY a.id),'[]'::jsonb)
                FROM runtime_approval_requests a WHERE a.session_id=r.session_id),
            'decisions',(SELECT COALESCE(jsonb_agg(to_jsonb(d) ORDER BY d.id),'[]'::jsonb)
                FROM runtime_approval_decisions d WHERE d.session_id=r.session_id),
            'events',(SELECT COALESCE(jsonb_agg(to_jsonb(e) ORDER BY e.sequence),'[]'::jsonb)
                FROM session_events e WHERE e.session_id=r.session_id)) AS snapshot
         FROM session_agent_runs r JOIN agent_sessions s ON s.id=r.session_id
         JOIN session_messages m ON m.id=$2 JOIN message_dispatch_outbox o ON o.message_id=m.id
         WHERE r.id=$1", [p.run_id.into(),p.message_id.into()]))
        .await.unwrap().unwrap().try_get("", "snapshot").unwrap()
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn terminal_concurrent_commit_once_and_exact_replay_preserve_transcript_and_timestamps() {
    let Some(p) = setup(true, true, SessionRunRole::Primary).await else {
        return;
    };
    let before = snapshot(&p).await;
    let (a, b) = timeout(Duration::from_secs(10), async {
        tokio::join!(
            commit(
                &p,
                SessionRunState::Completed,
                Some("  answer token=fixture-private  "),
                None
            ),
            commit(
                &p,
                SessionRunState::Completed,
                Some("  answer token=fixture-private  "),
                None
            )
        )
    })
    .await
    .unwrap();
    let (a, b) = (a.unwrap(), b.unwrap());
    assert_ne!(a.2, b.2);
    assert_eq!(a.0.state, SessionRunState::Completed);
    assert_eq!(b.0.state, SessionRunState::Completed);
    assert_eq!(a.0.updated_at, b.0.updated_at);
    assert_eq!(a.0.last_event_at, b.0.last_event_at);
    let (a_message, b_message) = (a.1.unwrap(), b.1.unwrap());
    assert_eq!(a_message.id, b_message.id);
    assert_eq!(a_message.created_at, b_message.created_at);
    assert_eq!(a_message.body, "answer token=redacted");
    assert_eq!(a_message.delivery_state, MessageDeliveryState::Mirrored);
    let after = snapshot(&p).await;
    assert_eq!(after["assistants"].as_array().unwrap().len(), 1);
    assert_eq!(after["prompt"]["delivery_state"], "completed");
    assert_eq!(after["run"]["state"], "completed");
    assert_eq!(
        after["session"]["last_message_preview"],
        "answer token=redacted"
    );
    assert_eq!(after["journal"], before["journal"]);
    assert_eq!(after["outbox"], before["outbox"]);
    assert_eq!(
        after["events"].as_array().unwrap().len(),
        before["events"].as_array().unwrap().len() + 4
    );
    let replay = commit(
        &p,
        SessionRunState::Completed,
        Some("answer token=fixture-private"),
        None,
    )
    .await
    .unwrap();
    assert!(!replay.2);
    assert!(replay.1.unwrap().replayed);
    assert_eq!(snapshot(&p).await, after);
    // An actual terminal commit, unlike a lease expiry, releases the active-run guard.
    let other = p
        .repo
        .create_session(chat(p.agent_id, "terminal-next"), p.owner)
        .await
        .unwrap();
    p.repo
        .prepare_session_agent_run(
            other.id,
            p.agent_id,
            SessionRunRole::Primary,
            format!("fleet:{}:{}", other.id, p.agent_id),
        )
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn terminal_replay_rejects_changed_native_pin_state_body_and_error_without_updates() {
    let Some(p) = setup(true, true, SessionRunRole::Primary).await else {
        return;
    };
    commit(&p, SessionRunState::Completed, Some("answer"), None)
        .await
        .unwrap();
    let before = snapshot(&p).await;
    let mut wrong_run = command(&p, SessionRunState::Completed, Some("answer"), None);
    wrong_run.runtime_run_id = "run_different".into();
    let mut wrong_pin = command(&p, SessionRunState::Completed, Some("answer"), None);
    wrong_pin.runtime_session_id = "fleet:different".into();
    for request in [
        wrong_run,
        wrong_pin,
        command(&p, SessionRunState::Failed, None, None),
        command(
            &p,
            SessionRunState::Completed,
            Some("different answer"),
            None,
        ),
        command(&p, SessionRunState::Completed, None, None),
        command(
            &p,
            SessionRunState::Completed,
            Some("answer"),
            Some("different error"),
        ),
    ] {
        assert!(matches!(
            p.repo.commit_hermes_terminal(request).await,
            Err(AppError::Conflict(_))
        ));
        assert_eq!(snapshot(&p).await, before);
    }
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn terminal_late_assistant_prompt_and_run_faults_roll_back_entire_packet_and_event_cursor() {
    for point in ["assistant", "prompt", "run"] {
        let Some(p) = setup(true, true, SessionRunRole::Primary).await else {
            return;
        };
        let before = snapshot(&p).await;
        let name = format!("terminal_fault_{}", Uuid::new_v4().simple());
        let (table, action, condition) = match point {
            "assistant" => (
                "session_messages",
                "INSERT",
                format!(
                    "NEW.session_id='{}'::uuid AND NEW.message_kind='assistant_message'",
                    p.session_id
                ),
            ),
            "prompt" => (
                "session_messages",
                "UPDATE",
                format!(
                    "NEW.id='{}'::uuid AND NEW.delivery_state='completed'",
                    p.message_id
                ),
            ),
            _ => (
                "session_agent_runs",
                "UPDATE",
                format!("NEW.id='{}'::uuid AND NEW.state='completed'", p.run_id),
            ),
        };
        p.db.execute_unprepared(&format!(
            "CREATE FUNCTION {name}() RETURNS trigger AS $$ BEGIN IF {condition} THEN
             RAISE EXCEPTION 'fixture-private-terminal-body'; END IF; RETURN NEW; END; $$ LANGUAGE plpgsql;
             CREATE TRIGGER {name} AFTER {action} ON {table} FOR EACH ROW EXECUTE FUNCTION {name}()"
        )).await.unwrap();
        let result = timeout(
            Duration::from_secs(10),
            commit(
                &p,
                SessionRunState::Completed,
                Some("fixture-private-terminal-body"),
                None,
            ),
        )
        .await;
        // Drop only this fixture's fault before assertions, including a timed-out call.
        p.db.execute_unprepared(&format!(
            "DROP TRIGGER {name} ON {table}; DROP FUNCTION {name}()"
        ))
        .await
        .unwrap();
        let error = result.unwrap().unwrap_err();
        assert!(matches!(error, AppError::Database(_)));
        assert!(!error.to_string().contains("fixture-private-terminal-body"));
        assert!(!format!("{error:?}").contains("fixture-private-terminal-body"));
        assert_eq!(snapshot(&p).await, before, "fault at {point}");
        assert!(
            commit(
                &p,
                SessionRunState::Completed,
                Some("answer after rollback"),
                None
            )
            .await
            .unwrap()
            .2
        );
    }
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn terminal_failed_and_cancelled_are_atomic_without_assistant_and_replay_unchanged() {
    for (state, delivery) in [
        (SessionRunState::Failed, "failed"),
        (SessionRunState::Cancelled, "completed"),
    ] {
        let Some(p) = setup(true, true, SessionRunRole::Primary).await else {
            return;
        };
        let before = snapshot(&p).await;
        let result = commit(
            &p,
            state,
            None,
            Some("native failed api_key=fixture-private"),
        )
        .await
        .unwrap();
        assert!(result.2);
        assert!(result.1.is_none());
        assert_eq!(result.0.state, state);
        assert_eq!(
            result.0.last_error.as_deref(),
            Some("native failed api_key=redacted")
        );
        let after = snapshot(&p).await;
        assert_eq!(after["assistants"], json!([]));
        assert_eq!(after["prompt"]["delivery_state"], delivery);
        assert_eq!(after["run"]["state"], state.as_str());
        assert_eq!(after["session"], before["session"]);
        assert_eq!(after["journal"], before["journal"]);
        assert_eq!(after["outbox"], before["outbox"]);
        assert_eq!(
            after["prompt"]["delivery_error"],
            if state == SessionRunState::Failed {
                json!("native failed api_key=redacted")
            } else {
                Value::Null
            }
        );
        assert_eq!(
            after["events"].as_array().unwrap().len(),
            before["events"].as_array().unwrap().len() + 2
        );
        assert!(
            !commit(
                &p,
                state,
                None,
                Some("native failed api_key=fixture-private")
            )
            .await
            .unwrap()
            .2
        );
        assert!(matches!(
            commit(&p, state, Some("forbidden assistant"), None).await,
            Err(AppError::Validation(_))
        ));
        assert_eq!(snapshot(&p).await, after);
    }
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn terminal_empty_completed_body_never_fabricates_assistant_or_changes_preview() {
    for body in [None, Some(""), Some(" \n\t ")] {
        let Some(p) = setup(true, true, SessionRunRole::Primary).await else {
            return;
        };
        let before = snapshot(&p).await;
        let result = commit(&p, SessionRunState::Completed, body, None)
            .await
            .unwrap();
        assert!(result.2);
        assert!(result.1.is_none());
        let after = snapshot(&p).await;
        assert_eq!(after["assistants"], json!([]));
        assert_eq!(
            after["transcript"].as_array().unwrap().len(),
            before["transcript"].as_array().unwrap().len()
        );
        assert_eq!(after["session"], before["session"]);
        assert_eq!(after["prompt"]["delivery_state"], "completed");
        assert_eq!(after["run"]["state"], "completed");
        assert!(
            !commit(&p, SessionRunState::Completed, Some("  "), None)
                .await
                .unwrap()
                .2
        );
        assert_eq!(snapshot(&p).await, after);
        assert!(matches!(
            commit(&p, SessionRunState::Completed, Some("late answer"), None).await,
            Err(AppError::Conflict(_))
        ));
        assert_eq!(snapshot(&p).await, after);
    }
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn terminal_requires_exact_message_native_pin_current_primary_and_accepted_journal() {
    let Some(p) = setup(true, true, SessionRunRole::Primary).await else {
        return;
    };
    let foreign = p
        .repo
        .create_session(chat(p.agent_id, "terminal-foreign"), p.owner)
        .await
        .unwrap();
    let foreign_message = p
        .repo
        .create_session_message(foreign.id, prompt("foreign-prompt"), p.owner)
        .await
        .unwrap();
    let before = snapshot(&p).await;
    let mut wrong_message = command(&p, SessionRunState::Completed, Some("answer"), None);
    wrong_message.message_id = foreign_message.id;
    let mut wrong_run = command(&p, SessionRunState::Completed, Some("answer"), None);
    wrong_run.runtime_run_id = "run_other".into();
    let mut wrong_pin = command(&p, SessionRunState::Completed, Some("answer"), None);
    wrong_pin.runtime_session_id = "fleet:other".into();
    for request in [wrong_message, wrong_run, wrong_pin] {
        assert!(matches!(
            p.repo.commit_hermes_terminal(request).await,
            Err(AppError::Conflict(_))
        ));
        assert_eq!(snapshot(&p).await, before);
    }
    assert!(matches!(
        commit(&p, SessionRunState::Running, None, None).await,
        Err(AppError::Validation(_))
    ));
    assert_eq!(snapshot(&p).await, before);
    sql(
        &p,
        "UPDATE session_messages SET runtime_message_id='run_inconsistent' WHERE id=$1",
        vec![p.message_id.into()],
    )
    .await;
    let inconsistent = snapshot(&p).await;
    assert!(matches!(
        commit(&p, SessionRunState::Completed, Some("answer"), None).await,
        Err(AppError::Conflict(_))
    ));
    assert_eq!(snapshot(&p).await, inconsistent);
    sql(
        &p,
        "UPDATE session_messages SET runtime_message_id=$2 WHERE id=$1",
        vec![p.message_id.into(), p.native_run.clone().into()],
    )
    .await;
    sql(
        &p,
        "UPDATE message_dispatch_outbox SET state='uncertain' WHERE message_id=$1",
        vec![p.message_id.into()],
    )
    .await;
    let inconsistent_outbox = snapshot(&p).await;
    assert!(matches!(
        commit(&p, SessionRunState::Completed, Some("answer"), None).await,
        Err(AppError::Conflict(_))
    ));
    assert_eq!(snapshot(&p).await, inconsistent_outbox);
    sql(
        &p,
        "UPDATE message_dispatch_outbox SET state='dispatched' WHERE message_id=$1",
        vec![p.message_id.into()],
    )
    .await;
    let other_agent = agent(&p.repo).await;
    sql(
        &p,
        "UPDATE agent_sessions SET agent_id=$2 WHERE id=$1",
        vec![p.session_id.into(), other_agent.into()],
    )
    .await;
    let changed_primary = snapshot(&p).await;
    assert!(matches!(
        commit(&p, SessionRunState::Completed, Some("answer"), None).await,
        Err(AppError::Conflict(_))
    ));
    assert_eq!(snapshot(&p).await, changed_primary);
    for (journal, pin) in [(false, false), (true, false)] {
        let p = setup(journal, pin, SessionRunRole::Primary).await.unwrap();
        let before = snapshot(&p).await;
        let mut request = command(&p, SessionRunState::Completed, Some("answer"), None);
        if !pin {
            request.runtime_session_id = p
                .repo
                .get_session_agent_run(p.run_id)
                .await
                .unwrap()
                .runtime_session_id
                .unwrap();
        }
        assert!(matches!(
            p.repo.commit_hermes_terminal(request).await,
            Err(AppError::Conflict(_))
        ));
        assert_eq!(snapshot(&p).await, before);
    }
}

async fn bind_task(p: &TerminalFixture, session: Uuid) {
    let task = Uuid::new_v4();
    sql(p, "INSERT INTO task_chat_bindings(session_id,tracker_instance_id,project_id,task_id,root_task_id,agent_id,owner_subject,idempotency_key)
        VALUES($1,'terminal-fixture',$2,$3,$3,$4,'fixture-owner','terminal-bound')",
        vec![session.into(),Uuid::new_v4().into(),task.into(),p.agent_id.into()]).await;
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn terminal_task_and_pm_bindings_never_authorize_freechat_packet() {
    for bound in ["task", "pm"] {
        let Some(p) = setup(true, true, SessionRunRole::Primary).await else {
            return;
        };
        if bound == "task" {
            bind_task(&p, p.session_id).await;
        } else {
            // The PM row references a separate task chat, so denial exercises the run binding alone.
            let source = p
                .repo
                .create_session(chat(p.agent_id, "terminal-pm-source"), p.owner)
                .await
                .unwrap();
            bind_task(&p, source.id).await;
            sql(&p, "INSERT INTO pm_run_bindings(session_run_id,session_id,agent_id,reservation,dispatch_operation_key,runtime_session_id)
                VALUES($1,$2,$3,'{}','terminal-pm-fixture',$4)",
                vec![p.run_id.into(),source.id.into(),p.agent_id.into(),p.effective_session.clone().into()]).await;
        }
        let before = snapshot(&p).await;
        assert!(matches!(
            commit(&p, SessionRunState::Completed, Some("answer"), None).await,
            Err(AppError::Conflict(_))
        ));
        assert_eq!(snapshot(&p).await, before);
    }
}
