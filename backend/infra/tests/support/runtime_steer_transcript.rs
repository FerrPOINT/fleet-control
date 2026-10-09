use super::*;

async fn db() -> sea_orm::DatabaseConnection {
    sea_orm::Database::connect(
        std::env::var("FLEET_TEST_DATABASE_URL").expect("isolated PostgreSQL is required"),
    )
    .await
    .unwrap()
}

async fn controls(f: &ControlFixture) -> Vec<domain::SessionMessage> {
    f.repo
        .list_session_messages(f.run.session_id)
        .await
        .unwrap()
        .into_iter()
        .filter(|m| m.message_kind == MessageKind::Control)
        .collect()
}

fn guidance(input: &str) -> SteerSessionRunRequest {
    SteerSessionRunRequest {
        input: input.into(),
    }
}

async fn submitted(f: &ControlFixture, actor: &domain::RuntimeControlActor, input: &str) -> Uuid {
    let reservation = f
        .repo
        .reserve_runtime_control(
            &f.run,
            actor,
            domain::RuntimeControlOperation::Steer,
            Some(input),
        )
        .await
        .unwrap();
    assert!(
        f.repo
            .claim_runtime_control(reservation.receipt.id)
            .await
            .unwrap()
    );
    reservation.receipt.id
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn ack_preserves_actor_redaction_scope_and_one_mirror_per_receipt() {
    let f = ledger_fixture(steer_ack()).await.unwrap();
    let db = db().await;
    let operator = Uuid::new_v4();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO users(id,email,username,display_name,password_hash,is_system_admin,system_role)
         VALUES($1,$2,$3,'Original operator','disabled',false,'operator')",
        [operator.into(),format!("{operator}@example.test").into(),operator.to_string().into()],
    )).await.unwrap();
    let actor = domain::RuntimeControlActor {
        user_id: operator,
        idempotency_key: Uuid::new_v4().to_string(),
    };
    let input = "guidance password=synthetic-private";
    let first = f
        .runtime
        .steer_run(&f.agent, &f.run, guidance(input), actor.clone())
        .await
        .unwrap();
    assert!(first.accepted);
    let id = first.command.unwrap().id;
    let left = f.restarted();
    let right = f.restarted();
    let (a, b) = tokio::join!(
        left.steer_run(&f.agent, &f.run, guidance(input), actor.clone()),
        right.steer_run(&f.agent, &f.run, guidance(input), actor.clone()),
    );
    for replay in [a.unwrap(), b.unwrap()] {
        assert!(replay.accepted);
        assert_eq!(replay.command.unwrap().id, id);
    }
    assert_eq!(f.calls.load(Ordering::SeqCst), 1);
    let messages = controls(&f).await;
    assert_eq!(messages.len(), 1);
    let message = &messages[0];
    assert_eq!(message.id, id);
    assert_eq!(message.session_id, f.run.session_id);
    assert_eq!(message.author_type, domain::MessageAuthorType::User);
    assert_eq!(message.author_user_id, Some(operator));
    assert_ne!(message.author_user_id, Some(f.owner));
    assert_eq!(message.author_agent_id, None);
    assert_eq!(message.author_display_name, "Original operator");
    assert_eq!(message.body, "guidance password=redacted");
    assert_eq!(
        message.delivery_state,
        domain::MessageDeliveryState::Mirrored
    );
    assert_eq!(message.delivery_error, None);
    assert_eq!(
        message.runtime_message_id,
        Some(format!("fleet-control:{}:{id}:steer", f.run.id))
    );
    let stored = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT body,created_by_user_id,(SELECT count(*) FROM message_dispatch_outbox WHERE message_id=$1) AS queued,
            (SELECT count(*) FROM session_events WHERE session_id=$2 AND event_type='session_message_changed'
                AND payload->>'entity_id'=$3) AS events
         FROM session_messages WHERE id=$1",
        [id.into(),f.run.session_id.into(),id.to_string().into()],
    )).await.unwrap().unwrap();
    assert_eq!(
        stored.try_get::<String>("", "body").unwrap(),
        "guidance password=redacted"
    );
    assert_eq!(
        stored.try_get::<Uuid>("", "created_by_user_id").unwrap(),
        operator
    );
    assert_eq!(stored.try_get::<i64>("", "queued").unwrap(), 0);
    assert_eq!(stored.try_get::<i64>("", "events").unwrap(), 1);
    let second = f
        .restarted()
        .steer_run(&f.agent, &f.run, guidance("second guidance"), f.actor())
        .await
        .unwrap();
    assert!(second.accepted);
    assert_ne!(second.command.unwrap().id, id);
    assert_eq!(controls(&f).await.len(), 2);
    assert_eq!(f.calls.load(Ordering::SeqCst), 2);
    assert_eq!(
        f.repo.get_session_agent_run(f.run.id).await.unwrap().state,
        SessionRunState::Running
    );
    assert!(
        f.repo
            .prepare_session_agent_run(
                f.run.session_id,
                f.agent.id,
                SessionRunRole::Primary,
                "no-capacity-release".into(),
            )
            .await
            .is_err()
    );
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn acknowledged_replay_repairs_legacy_mirror_without_native_post() {
    let f = ledger_fixture(steer_ack()).await.unwrap();
    let actor = f.actor();
    let input = "recover original guidance";
    let id = submitted(&f, &actor, input).await;
    let db = db().await;
    // Model an already committed pre-fix ACK. Its immutable hash is the only payload proof.
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE runtime_control_commands SET state='acknowledged',acknowledgement='steered' WHERE id=$1",
        [id.into()],
    )).await.unwrap();
    assert!(controls(&f).await.is_empty());
    assert!(
        f.restarted()
            .steer_run(
                &f.agent,
                &f.run,
                guidance("different guidance"),
                actor.clone(),
            )
            .await
            .is_err()
    );
    assert!(controls(&f).await.is_empty());
    let before = serde_json::to_value(
        f.repo
            .get_runtime_control(f.run.session_id, id)
            .await
            .unwrap(),
    )
    .unwrap();
    for _ in 0..2 {
        let replay = f
            .restarted()
            .steer_run(&f.agent, &f.run, guidance(input), actor.clone())
            .await
            .unwrap();
        assert!(replay.accepted);
        assert_eq!(
            serde_json::to_value(replay.command.unwrap()).unwrap(),
            before
        );
    }
    let messages = controls(&f).await;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].id, id);
    assert_eq!(messages[0].body, input);
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn uncertain_and_rejected_never_mirror_or_repost() {
    for rejected in [false, true] {
        let f = ledger_fixture(json!({"accepted":true})).await.unwrap();
        let actor = f.actor();
        let input = "not proven delivered";
        if rejected {
            let reservation = f
                .repo
                .reserve_runtime_control(
                    &f.run,
                    &actor,
                    domain::RuntimeControlOperation::Steer,
                    Some(input),
                )
                .await
                .unwrap();
            f.repo
                .retire_runtime_control(reservation.receipt.id, false)
                .await
                .unwrap();
        }
        for _ in 0..2 {
            let replay = f
                .restarted()
                .steer_run(&f.agent, &f.run, guidance(input), actor.clone())
                .await
                .unwrap();
            assert!(!replay.accepted);
            let command = replay.command.unwrap();
            assert_eq!(
                command.state,
                if rejected {
                    domain::RuntimeControlState::Rejected
                } else {
                    domain::RuntimeControlState::Uncertain
                }
            );
            assert!(
                f.repo
                    .finish_runtime_control(command.id, "steered", Some(input))
                    .await
                    .is_err()
            );
        }
        assert!(controls(&f).await.is_empty());
        assert_eq!(f.calls.load(Ordering::SeqCst), usize::from(!rejected));
        assert_eq!(
            f.repo.get_session_agent_run(f.run.id).await.unwrap().state,
            SessionRunState::Running
        );
        if !rejected {
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
                    runtime_run_id: "run_control".into(),
                    runtime_session_id: "control-native-session".into(),
                    state: SessionRunState::Completed,
                    body: Some("independent terminal answer".into()),
                    error: None,
                })
                .await
                .unwrap();
            f.repo.reconcile_runtime_controls().await.unwrap();
            let replay = f
                .restarted()
                .steer_run(&f.agent, &f.run, guidance(input), actor)
                .await
                .unwrap();
            assert!(!replay.accepted);
            assert_eq!(
                replay.command.unwrap().state,
                domain::RuntimeControlState::TerminalObserved
            );
            assert!(controls(&f).await.is_empty());
            assert_eq!(f.calls.load(Ordering::SeqCst), 1);
        }
    }
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn ack_payload_proof_denies_missing_or_changed_input() {
    let f = ledger_fixture(steer_ack()).await.unwrap();
    let input = "exact original";
    let id = submitted(&f, &f.actor(), input).await;
    for payload in [None, Some("different original")] {
        assert!(
            f.repo
                .finish_runtime_control(id, "steered", payload)
                .await
                .is_err()
        );
        assert!(controls(&f).await.is_empty());
        assert_eq!(
            f.repo
                .get_runtime_control(f.run.session_id, id)
                .await
                .unwrap()
                .state,
            domain::RuntimeControlState::Submitted
        );
    }
    f.repo
        .finish_runtime_control(id, "steered", Some(input))
        .await
        .unwrap();
    assert!(
        f.repo
            .finish_runtime_control(id, "steered", Some("changed after ACK"))
            .await
            .is_err()
    );
    assert_eq!(controls(&f).await[0].body, input);
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn audit_failure_rolls_back_ack_mirror_preview_and_events() {
    let f = ledger_fixture(steer_ack()).await.unwrap();
    let actor = f.actor();
    let input = "atomic guidance";
    let id = submitted(&f, &actor, input).await;
    let db = db().await;
    let before = f.repo.get_session(f.run.session_id).await.unwrap();
    let cursor = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT sequence FROM session_event_cursors WHERE session_id=$1",
            [f.run.session_id.into()],
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "sequence")
        .unwrap();
    let guard = format!("steer_audit_fault_{}", Uuid::new_v4().simple());
    db.execute_unprepared(&format!(
        "CREATE FUNCTION {guard}() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN IF NEW.entity_id='{id}' AND NEW.action='runtime_control.acknowledged' THEN
         RAISE EXCEPTION 'owned steer audit failure'; END IF; RETURN NEW; END $$;
         CREATE TRIGGER {guard} BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION {guard}();"
    ))
    .await
    .unwrap();
    let result = f
        .repo
        .finish_runtime_control(id, "steered", Some(input))
        .await;
    db.execute_unprepared(&format!(
        "DROP TRIGGER {guard} ON audit_log; DROP FUNCTION {guard}();"
    ))
    .await
    .unwrap();
    assert!(result.is_err());
    assert!(controls(&f).await.is_empty());
    assert_eq!(
        f.repo
            .get_runtime_control(f.run.session_id, id)
            .await
            .unwrap()
            .state,
        domain::RuntimeControlState::Submitted
    );
    let after = f.repo.get_session(f.run.session_id).await.unwrap();
    assert_eq!(after.last_message_preview, before.last_message_preview);
    assert_eq!(after.updated_at, before.updated_at);
    let after_cursor = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT sequence FROM session_event_cursors WHERE session_id=$1",
            [f.run.session_id.into()],
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "sequence")
        .unwrap();
    assert_eq!(after_cursor, cursor);
    let replay = f
        .restarted()
        .steer_run(&f.agent, &f.run, guidance(input), actor)
        .await
        .unwrap();
    assert!(!replay.accepted);
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
    f.repo
        .finish_runtime_control(id, "steered", Some(input))
        .await
        .unwrap();
    assert_eq!(controls(&f).await.len(), 1);
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn message_insert_failure_rolls_back_ack_and_does_not_retry_native() {
    let f = ledger_fixture(steer_ack()).await.unwrap();
    let actor = f.actor();
    let input = "failed transcript write";
    let db = db().await;
    let session = f.run.session_id;
    let guard = format!("steer_message_fault_{}", Uuid::new_v4().simple());
    db.execute_unprepared(&format!(
        "CREATE FUNCTION {guard}() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN IF NEW.session_id='{session}' AND NEW.message_kind='control' THEN
         RAISE EXCEPTION 'owned steer mirror failure';
         END IF; RETURN NEW; END $$;
         CREATE TRIGGER {guard} BEFORE INSERT ON session_messages FOR EACH ROW EXECUTE FUNCTION {guard}();"
    )).await.unwrap();
    let result = f
        .runtime
        .steer_run(&f.agent, &f.run, guidance(input), actor.clone())
        .await;
    db.execute_unprepared(&format!(
        "DROP TRIGGER {guard} ON session_messages; DROP FUNCTION {guard}();"
    ))
    .await
    .unwrap();
    assert!(result.is_err());
    assert_eq!(f.calls.load(Ordering::SeqCst), 1);
    let receipts = f
        .repo
        .list_runtime_controls(f.run.session_id, f.run.id)
        .await
        .unwrap();
    assert_eq!(receipts.len(), 1);
    let id = receipts[0].id;
    assert_eq!(
        f.repo
            .get_runtime_control(f.run.session_id, id)
            .await
            .unwrap()
            .state,
        domain::RuntimeControlState::Submitted
    );
    assert!(controls(&f).await.is_empty());
    let replay = f
        .restarted()
        .steer_run(&f.agent, &f.run, guidance(input), actor)
        .await
        .unwrap();
    assert!(!replay.accepted);
    assert_eq!(f.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn message_identity_collision_is_not_overwritten_or_acknowledged() {
    let f = ledger_fixture(steer_ack()).await.unwrap();
    let input = "original guidance";
    let id = submitted(&f, &f.actor(), input).await;
    let db = db().await;
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO session_messages(id,session_id,author_type,body,message_kind,delivery_state)
         VALUES($1,$2,'system','unrelated existing message','system_event','mirrored')",
        [id.into(), f.run.session_id.into()],
    ))
    .await
    .unwrap();
    assert!(
        f.repo
            .finish_runtime_control(id, "steered", Some(input))
            .await
            .is_err()
    );
    assert_eq!(
        f.repo
            .get_runtime_control(f.run.session_id, id)
            .await
            .unwrap()
            .state,
        domain::RuntimeControlState::Submitted
    );
    let messages = f
        .repo
        .list_session_messages(f.run.session_id)
        .await
        .unwrap();
    let message = messages.iter().find(|m| m.id == id).unwrap();
    assert_eq!(message.body, "unrelated existing message");
    assert_eq!(message.message_kind, MessageKind::SystemEvent);
    assert!(controls(&f).await.is_empty());
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
}
