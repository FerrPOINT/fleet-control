use super::*;

async fn database() -> sea_orm::DatabaseConnection {
    assert_eq!(
        std::env::var("FLEET_NATIVE_SUPERVISOR_TEST").as_deref(),
        Ok("1")
    );
    sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap()
}

pub(super) async fn hold_submission(session: Uuid) {
    database().await
        .execute_unprepared(&format!(
            "CREATE FUNCTION qa_hold_prepared_claim() RETURNS trigger AS $$ BEGIN
             IF NEW.session_id='{session}'::uuid AND OLD.state='prepared' AND NEW.state='submitted' THEN
                 RAISE EXCEPTION 'owned native QA pre-submission fault';
             END IF; RETURN NEW; END $$ LANGUAGE plpgsql;
             CREATE TRIGGER qa_hold_prepared_claim BEFORE UPDATE ON hermes_dispatch_journal
             FOR EACH ROW EXECUTE FUNCTION qa_hold_prepared_claim()"
        ))
        .await
        .unwrap();
}

pub(super) async fn release_submission(
    repo: Arc<PostgresFleetRepository>,
    config: Arc<AppConfig>,
    message: Uuid,
    model: Arc<Model>,
    prompt: &str,
) {
    let db = database().await;
    let original = timeout(Duration::from_secs(30), async {
        loop {
            if let Some(intent) = repo.get_hermes_dispatch_intent(message).await.unwrap() {
                let row = db
                    .query_one(Statement::from_sql_and_values(
                        DatabaseBackend::Postgres,
                        "SELECT state FROM message_dispatch_outbox WHERE message_id=$1",
                        [message.into()],
                    ))
                    .await
                    .unwrap()
                    .unwrap();
                if row.try_get::<String>("", "state").unwrap() == "uncertain" {
                    assert_eq!(intent.state, "prepared");
                    assert!(!intent.submission_attempted);
                    assert!(intent.submitted_at.is_none());
                    assert!(intent.run.runtime_run_id.is_none());
                    break intent;
                }
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("pre-submission fault did not retain the prepared native intent");
    let launch = repo
        .get_open_runtime_launch(original.run.agent_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(launch.state, "gateway_started");
    assert_eq!(original.capabilities["fleet_launch"]["version"], 1);
    assert_eq!(
        original.capabilities["fleet_launch"]["launch_id"],
        launch.binding.id.to_string()
    );
    let pending = repo
        .list_session_messages(original.run.session_id)
        .await
        .unwrap()
        .into_iter()
        .find(|item| item.id == message)
        .unwrap();
    assert_eq!(
        pending.delivery_state,
        domain::MessageDeliveryState::Pending
    );
    assert!(pending.delivery_error.is_some());
    assert!(!model.requests.lock().await.contains_key(prompt));

    let (events, _) = tokio::sync::broadcast::channel(32);
    let _foreign = LocalRuntimeSupervisor::new(config, repo.clone(), events);
    sleep(Duration::from_secs(6)).await;
    let held = repo
        .get_hermes_dispatch_intent(message)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(held.state, "prepared");
    assert!(!held.submission_attempted);
    assert_eq!(held.request_body, original.request_body);
    assert_eq!(held.capabilities, original.capabilities);
    assert_eq!(held.recovery_deadline, original.recovery_deadline);
    assert!(!model.requests.lock().await.contains_key(prompt));

    db.execute_unprepared(
        "DROP TRIGGER qa_hold_prepared_claim ON hermes_dispatch_journal;
             DROP FUNCTION qa_hold_prepared_claim()",
    )
    .await
    .unwrap();
    let accepted = timeout(Duration::from_secs(60), async {
        loop {
            let intent = repo
                .get_hermes_dispatch_intent(message)
                .await
                .unwrap()
                .unwrap();
            if intent.state == "accepted" {
                break intent;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("original controller did not resume its prepared native intent");
    assert_eq!(accepted.message_id, original.message_id);
    assert_eq!(accepted.run.id, original.run.id);
    assert_eq!(accepted.request_body, original.request_body);
    assert_eq!(accepted.request_hash, original.request_hash);
    assert_eq!(accepted.idempotency_key, original.idempotency_key);
    assert_eq!(accepted.origin, original.origin);
    assert_eq!(
        accepted.credential_fingerprint,
        original.credential_fingerprint
    );
    assert_eq!(accepted.capabilities, original.capabilities);
    assert_eq!(accepted.recovery_deadline, original.recovery_deadline);
    assert!(accepted.submission_attempted);
    assert!(accepted.submitted_at.is_some());
    let current = repo
        .get_open_runtime_launch(original.run.agent_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(current.binding.id, launch.binding.id);
    assert_eq!(current.pid, launch.pid);
}
