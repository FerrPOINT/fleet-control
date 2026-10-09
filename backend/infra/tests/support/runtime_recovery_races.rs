use super::*;
use sea_orm::{DatabaseConnection, DbErr};
use serde_json::{Value, json};
use shared::AppError;
use tokio::time::timeout;

const SOURCE: &str = "bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3";

struct RecoveryFixture {
    repo: PostgresFleetRepository,
    db: DatabaseConnection,
    agent_id: Uuid,
    session_id: Uuid,
    message_id: Uuid,
    run_id: Uuid,
    native_id: String,
    origin: String,
    facts: Value,
}

async fn setup() -> Option<RecoveryFixture> {
    let (repo, owner, _) = recovery_fixture().await?;
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let agent_id = agent(&repo).await;
    repo.update_agent_status(agent_id, AgentStatus::Ready)
        .await
        .unwrap();
    let key = Uuid::new_v4().to_string();
    let session = repo
        .create_session(chat(agent_id, &key), owner)
        .await
        .unwrap();
    let message = repo
        .create_session_message(session.id, prompt(&key), owner)
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE message_dispatch_outbox SET state='dispatching' WHERE message_id=$1",
        [message.id.into()],
    ))
    .await
    .unwrap();
    repo.update_agent_status(agent_id, AgentStatus::Running)
        .await
        .unwrap();
    let port = repo.get_agent(agent_id).await.unwrap().api_port.unwrap();
    let origin = format!("http://127.0.0.1:{port}");
    let mut facts = hermes_protocol_fixture::capabilities();
    facts["fleet_recovery"] = json!({
        "object":"fleet.hermes.recovery.capabilities", "contract_version":1,
        "store_id":Uuid::new_v4().to_string(), "profile":"default",
        "scope_fingerprint":"a".repeat(64), "native_source_revision":SOURCE,
        "lookup":{"method":"POST","path":"/fleet/v1/recovery/lookup"},
        "non_dispatch":true, "durable_witness":true,
    });
    let prepared = repo
        .prepare_hermes_dispatch(app::HermesDispatchDraft {
            message_id: message.id,
            session_id: session.id,
            agent_id,
            run_role: SessionRunRole::Primary,
            requested_session_id: format!("fleet:{}:{agent_id}", session.id),
            input: message.body,
            origin: origin.clone(),
            credential_fingerprint: "b".repeat(64),
            capabilities: facts.clone(),
        })
        .await
        .unwrap();
    let submitted = repo
        .claim_hermes_submission(message.id, origin.clone(), "b".repeat(64))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(prepared.run.id, submitted.run.id);
    assert_eq!(submitted.state, "submitted");
    assert!(submitted.recovery_allowed);
    assert_eq!(submitted.capabilities, facts);
    Some(RecoveryFixture {
        repo,
        db,
        agent_id,
        session_id: session.id,
        message_id: message.id,
        run_id: submitted.run.id,
        native_id: format!("run_{}", submitted.run.id.simple()),
        origin,
        facts,
    })
}

async fn snapshot(p: &RecoveryFixture) -> Value {
    p.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT jsonb_build_object('run',to_jsonb(r),'prompt',to_jsonb(m),
            'outbox',to_jsonb(o),'journal',to_jsonb(j),'session',to_jsonb(s),
            'transcript',(SELECT COALESCE(jsonb_agg(to_jsonb(t) ORDER BY t.append_sequence),'[]'::jsonb)
                FROM session_messages t WHERE t.session_id=s.id),
            'events',(SELECT COALESCE(jsonb_agg(to_jsonb(e) ORDER BY e.sequence),'[]'::jsonb)
                FROM session_events e WHERE e.session_id=s.id),
            'cursor',(SELECT sequence FROM session_event_cursors WHERE session_id=s.id),
            'run_count',(SELECT count(*) FROM session_agent_runs WHERE session_id=s.id),
            'journal_count',(SELECT count(*) FROM hermes_dispatch_journal WHERE session_id=s.id)) AS snapshot
         FROM session_agent_runs r JOIN agent_sessions s ON s.id=r.session_id
         JOIN session_messages m ON m.id=$2 JOIN message_dispatch_outbox o ON o.message_id=m.id
         JOIN hermes_dispatch_journal j ON j.message_id=m.id WHERE r.id=$1",
        [p.run_id.into(), p.message_id.into()])).await.unwrap().unwrap()
        .try_get("", "snapshot").unwrap()
}

async fn worker() -> (PostgresFleetRepository, i32) {
    let mut options =
        sea_orm::ConnectOptions::new(std::env::var("FLEET_TEST_DATABASE_URL").unwrap());
    options
        .max_connections(1)
        .min_connections(1)
        .connect_timeout(Duration::from_secs(10));
    let db = sea_orm::Database::connect(options).await.unwrap();
    let pid = db
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT pg_backend_pid() AS pid".to_string(),
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap();
    (PostgresFleetRepository::new(db), pid)
}

async fn wait_locked(
    db: &DatabaseConnection,
    worker_pid: i32,
    holder_pid: i32,
    marker: &str,
    queued_worker: Option<i32>,
) -> Result<(), DbErr> {
    timeout(Duration::from_secs(10), async {
        loop {
            let blocked: bool = db
                .query_one(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE pid=$1
                    AND datname=current_database() AND state='active'
                    AND wait_event_type='Lock'
                    AND (wait_event='transactionid' OR ($4 IS NOT NULL AND wait_event='tuple'))
                    AND ($2=ANY(pg_blocking_pids(pid)) OR $4=ANY(pg_blocking_pids(pid)))
                    AND position($3 in query)>0) AS blocked",
                    [
                        worker_pid.into(),
                        holder_pid.into(),
                        marker.into(),
                        queued_worker.into(),
                    ],
                ))
                .await?
                .unwrap()
                .try_get("", "blocked")?;
            if blocked {
                return Ok(());
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .map_err(|_| DbErr::Custom("owned recovery worker did not reach PG lock barrier".into()))?
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn recovery_race_ordinary_ack_and_original_key_acceptance_keep_one_mapping() {
    let Some(p) = setup().await else {
        return;
    };
    let before = snapshot(&p).await;
    let (ack_repo, ack_pid) = worker().await;
    let (recovery_repo, recovery_pid) = worker().await;
    let holder = p.db.begin().await.unwrap();
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
    holder
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id FROM agents WHERE id=$1 FOR NO KEY UPDATE",
            [p.agent_id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    let release = async {
        let result = async {
            // The second row-lock waiter can queue on the first waiter's tuple lock.
            wait_locked(&p.db, ack_pid, holder_pid, "agents", Some(recovery_pid)).await?;
            wait_locked(&p.db, recovery_pid, holder_pid, "agents", Some(ack_pid)).await
        }
        .await;
        if let Err(error) = result {
            holder.rollback().await?;
            return Err(error);
        }
        holder.commit().await
    };
    let (ack, recovered, released) = timeout(Duration::from_secs(25), async {
        tokio::join!(
            ack_repo.accept_hermes_run(p.message_id, p.run_id, p.native_id.clone()),
            recovery_repo.accept_recovered_hermes_run(
                p.message_id,
                p.run_id,
                p.native_id.clone(),
                p.facts.clone()
            ),
            release,
        )
    })
    .await
    .expect("ACK/recovery race exceeded its bounded barrier");
    released.unwrap();
    let ack = ack.unwrap();
    let recovered = recovered.unwrap();
    assert_eq!(
        serde_json::to_value(&ack).unwrap(),
        serde_json::to_value(&recovered).unwrap()
    );
    assert_eq!(ack.runtime_run_id.as_deref(), Some(p.native_id.as_str()));
    assert_eq!(ack.state, SessionRunState::Pending);
    let accepted = snapshot(&p).await;
    assert_eq!(accepted["run_count"], 1);
    assert_eq!(accepted["journal_count"], 1);
    assert_eq!(accepted["journal"]["state"], "accepted");
    assert!(accepted["journal"]["accepted_at"].is_string());
    assert_eq!(accepted["prompt"]["runtime_message_id"], p.native_id);
    assert_eq!(accepted["prompt"]["delivery_state"], "dispatched");
    assert_eq!(accepted["outbox"]["state"], "dispatched");
    assert_eq!(
        accepted["journal"]["request_body"],
        before["journal"]["request_body"]
    );
    assert_eq!(
        accepted["journal"]["request_hash"],
        before["journal"]["request_hash"]
    );
    assert_eq!(
        accepted["journal"]["idempotency_key"],
        before["journal"]["idempotency_key"]
    );
    assert_eq!(accepted["journal"]["capabilities"], p.facts);
    assert_eq!(
        accepted["journal"]["submitted_at"],
        before["journal"]["submitted_at"]
    );
    assert_eq!(
        accepted["journal"]["recovery_deadline"],
        before["journal"]["recovery_deadline"]
    );
    // Existing system messages are retained; ACK changes only this prompt, never adds an assistant.
    let mut expected_transcript = before["transcript"].clone();
    for message in expected_transcript.as_array_mut().unwrap() {
        if message["id"] == p.message_id.to_string() {
            *message = accepted["prompt"].clone();
        }
    }
    assert_eq!(accepted["transcript"], expected_transcript);
    for recovered in [false, true] {
        let replay = if recovered {
            p.repo
                .accept_recovered_hermes_run(
                    p.message_id,
                    p.run_id,
                    p.native_id.clone(),
                    p.facts.clone(),
                )
                .await
        } else {
            p.repo
                .accept_hermes_run(p.message_id, p.run_id, p.native_id.clone())
                .await
        }
        .unwrap();
        assert_eq!(replay.updated_at, ack.updated_at);
        let conflicting = format!("run_{}", Uuid::new_v4().simple());
        let rejected = if recovered {
            p.repo
                .accept_recovered_hermes_run(p.message_id, p.run_id, conflicting, p.facts.clone())
                .await
        } else {
            p.repo
                .accept_hermes_run(p.message_id, p.run_id, conflicting)
                .await
        };
        assert!(matches!(rejected, Err(AppError::Conflict(_))));
        assert_eq!(snapshot(&p).await, accepted);
    }
    // A second submission permit is withheld, even after acceptance; this test issues no HTTP POST.
    assert!(
        p.repo
            .claim_hermes_submission(p.message_id, p.origin.clone(), "b".repeat(64))
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(snapshot(&p).await, accepted);
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn recovery_race_deadline_expires_while_journal_locked_and_rolls_back_acceptance() {
    let Some(p) = setup().await else {
        return;
    };
    let (recovery_repo, recovery_pid) = worker().await;
    // Disposable PG fixture only: retain the duration CHECK and original submitted state.
    let arm = p.db.begin().await.unwrap();
    arm.execute_unprepared("SET LOCAL session_replication_role=replica")
        .await
        .unwrap();
    arm.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "WITH boundary AS MATERIALIZED (SELECT clock_timestamp()+interval '8 seconds' AS deadline)
         UPDATE hermes_dispatch_journal SET created_at=boundary.deadline-interval '86340 seconds',
            recovery_deadline=boundary.deadline FROM boundary WHERE message_id=$1",
        [p.message_id.into()],
    ))
    .await
    .unwrap();
    arm.commit().await.unwrap();
    let before = snapshot(&p).await;
    let observed = p
        .repo
        .get_hermes_dispatch_intent(p.message_id)
        .await
        .unwrap()
        .unwrap();
    assert!(
        observed.recovery_allowed,
        "pre-lookup deadline must still be live"
    );
    assert_eq!(observed.capabilities, p.facts);
    let holder = p.db.begin().await.unwrap();
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
    holder
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT message_id FROM hermes_dispatch_journal WHERE message_id=$1 FOR UPDATE",
            [p.message_id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    let release_after_expiry = async {
        let result = async {
            wait_locked(&p.db, recovery_pid, holder_pid, "hermes_dispatch_journal", None).await?;
            let live: bool = p.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "SELECT recovery_deadline>clock_timestamp() AS live FROM hermes_dispatch_journal WHERE message_id=$1",
                [p.message_id.into()])).await?.unwrap().try_get("", "live")?;
            if !live {
                return Err(DbErr::Custom("fixture deadline expired before actual journal wait".into()));
            }
            // DB clock, not a fixed sleep, determines expiry while the original SELECT is blocked.
            timeout(Duration::from_secs(15), async {
                loop {
                    let expired: bool = p.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                        "SELECT recovery_deadline<=clock_timestamp() AS expired FROM hermes_dispatch_journal WHERE message_id=$1",
                        [p.message_id.into()])).await?.unwrap().try_get("", "expired")?;
                    if expired {
                        return Ok::<(), DbErr>(());
                    }
                    sleep(Duration::from_millis(10)).await;
                }
            }).await.map_err(|_| DbErr::Custom("fixture DB deadline did not expire".into()))?
        }.await;
        if let Err(error) = result {
            holder.rollback().await?;
            return Err(error);
        }
        holder.commit().await
    };
    let (acceptance, released) = timeout(Duration::from_secs(30), async {
        tokio::join!(
            recovery_repo.accept_recovered_hermes_run(
                p.message_id,
                p.run_id,
                p.native_id.clone(),
                p.facts.clone()
            ),
            release_after_expiry,
        )
    })
    .await
    .expect("expiry/acceptance race exceeded its bounded barrier");
    released.unwrap();
    assert!(matches!(acceptance, Err(AppError::Conflict(_))));
    assert_eq!(
        snapshot(&p).await,
        before,
        "failed acceptance must roll back every write/event"
    );
    assert_eq!(before["run"]["state"], "pending");
    assert!(before["run"]["runtime_run_id"].is_null());
    assert_eq!(before["prompt"]["delivery_state"], "pending");
    assert!(before["prompt"]["runtime_message_id"].is_null());
    assert_eq!(before["outbox"]["state"], "dispatching");
    assert_eq!(before["journal"]["state"], "submitted");
    let held = p
        .repo
        .get_hermes_dispatch_intent(p.message_id)
        .await
        .unwrap()
        .unwrap();
    assert!(!held.recovery_allowed);
    assert!(held.run.runtime_run_id.is_none());
    assert_eq!(held.run.state, SessionRunState::Pending);
    assert_eq!(held.request_body, observed.request_body);
    assert_eq!(held.request_hash, observed.request_hash);
    assert_eq!(held.recovery_deadline, observed.recovery_deadline);
    assert_eq!(
        p.repo
            .list_session_messages(p.session_id)
            .await
            .unwrap()
            .len(),
        before["transcript"].as_array().unwrap().len()
    );
}
