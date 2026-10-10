use super::*;
use domain::{
    PmDispatchIntent, PmHumanControlScope, RuntimeControlActor, RuntimeControlOperation,
    RuntimeControlState,
};
use serde_json::{Value, json};

async fn accepted() -> (
    PostgresFleetRepository,
    domain::SessionAgentRun,
    PmHumanControlScope,
) {
    std::env::var("FLEET_TEST_DATABASE_URL")
        .expect("isolated PM human controls PostgreSQL is required");
    let (repo, reservation) = pm_fixture().await.expect("configured PostgreSQL fixture");
    repo.reserve_pm_run(reservation.clone()).await.unwrap();
    repo.prepare_pm_dispatch(PmDispatchIntent {
        session_run_id: reservation.session_run_id,
        origin: "http://127.0.0.1:23810".into(),
        credential_fingerprint: "a".repeat(64),
        request_body:
            json!({"input":"Original PM task","session_id":reservation.runtime_session_id()})
                .to_string(),
        workflow_assignment: json!({"operation_key":reservation.identity.assignment_operation_key}),
        workflow_origin: "http://workflow.test".into(),
        workflow_credential_fingerprint: "b".repeat(64),
        runtime_context: json!({}),
        submitted: false,
        hermes_run_ref: None,
    })
    .await
    .unwrap();
    assert!(
        repo.claim_pm_submission(reservation.session_run_id)
            .await
            .unwrap()
    );
    repo.record_pm_submission(reservation.session_run_id, "run_human".into())
        .await
        .unwrap();
    let record = repo
        .accept_pm_run(
            reservation.session_run_id,
            "run_human".into(),
            "effective-human-session".into(),
        )
        .await
        .unwrap();
    repo.observe_pm_run(reservation.session_run_id, domain::PmRuntimeStatus::Running)
        .await
        .unwrap();
    assert!(matches!(
        repo.claim_pm_guidance(
            reservation.session_run_id,
            json!({"input":"Verified initial Workflow guidance"}).to_string()
        )
        .await
        .unwrap(),
        domain::PmGuidancePermit::Claimed
    ));
    repo.finish_pm_guidance(reservation.session_run_id)
        .await
        .unwrap();
    let session = repo.get_session(reservation.session_id).await.unwrap();
    let binding = repo
        .get_task_chat_binding(session.id)
        .await
        .unwrap()
        .unwrap();
    let scope = PmHumanControlScope {
        record,
        intent: repo
            .get_pm_dispatch(reservation.session_run_id)
            .await
            .unwrap()
            .unwrap(),
        owner_subject: binding.owner_subject,
        owner_user_id: session.user_id,
    };
    let run = repo
        .get_session_agent_run(reservation.session_run_id)
        .await
        .unwrap();
    (repo, run, scope)
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn pm_human_receipt_claim_is_once_and_unknown_never_becomes_a_new_command() {
    let (repo, run, scope) = accepted().await;
    let actor = RuntimeControlActor {
        user_id: scope.owner_user_id,
        idempotency_key: Uuid::new_v4().to_string(),
    };
    // The legacy admission path remains closed even for an accepted PM run.
    assert!(
        repo.reserve_runtime_control(
            &run,
            &actor,
            RuntimeControlOperation::Steer,
            Some("Original guidance")
        )
        .await
        .is_err()
    );
    let receipt = repo
        .reserve_pm_runtime_control(
            &run,
            &actor,
            RuntimeControlOperation::Steer,
            Some("Original guidance"),
            &scope,
        )
        .await
        .unwrap();
    assert!(receipt.dispatch);
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let saved = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT runtime_session_id FROM runtime_control_commands WHERE id=$1",
            [receipt.receipt.id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        saved.try_get::<String>("", "runtime_session_id").unwrap(),
        "effective-human-session"
    );
    assert_ne!(
        run.runtime_session_id.as_deref(),
        Some("effective-human-session")
    );
    let replay = repo
        .reserve_pm_runtime_control(
            &run,
            &actor,
            RuntimeControlOperation::Steer,
            Some("Original guidance"),
            &scope,
        )
        .await
        .unwrap();
    assert_eq!(replay.receipt.id, receipt.receipt.id);
    assert!(
        repo.reserve_pm_runtime_control(
            &run,
            &actor,
            RuntimeControlOperation::Steer,
            Some("Changed guidance"),
            &scope
        )
        .await
        .is_err()
    );
    let (a, b) = tokio::join!(
        repo.claim_pm_runtime_control(receipt.receipt.id, &scope),
        repo.claim_pm_runtime_control(receipt.receipt.id, &scope)
    );
    assert_ne!(a.unwrap(), b.unwrap());
    assert_eq!(
        repo.retire_runtime_control(receipt.receipt.id, true)
            .await
            .unwrap()
            .state,
        RuntimeControlState::Uncertain
    );
    let replay = repo
        .reserve_pm_runtime_control(
            &run,
            &actor,
            RuntimeControlOperation::Steer,
            Some("Original guidance"),
            &scope,
        )
        .await
        .unwrap();
    assert!(!replay.dispatch);
    assert_eq!(replay.receipt.id, receipt.receipt.id);
    assert!(
        !repo
            .claim_pm_runtime_control(receipt.receipt.id, &scope)
            .await
            .unwrap()
    );
    let successor = RuntimeControlActor {
        idempotency_key: Uuid::new_v4().to_string(),
        ..actor
    };
    assert!(
        repo.reserve_pm_runtime_control(
            &run,
            &successor,
            RuntimeControlOperation::Steer,
            Some("Original guidance"),
            &scope
        )
        .await
        .is_err()
    );
    assert_eq!(
        repo.list_runtime_controls(run.session_id, run.id)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_pm_stop_claim_held(&repo, run.id).await;
    repo.reconcile_runtime_controls().await.unwrap();
    assert_eq!(
        repo.get_runtime_control(run.session_id, receipt.receipt.id)
            .await
            .unwrap()
            .state,
        RuntimeControlState::Uncertain
    );
    repo.observe_pm_run(run.id, domain::PmRuntimeStatus::Completed)
        .await
        .unwrap();
    repo.reconcile_runtime_controls().await.unwrap();
    assert_eq!(
        repo.get_runtime_control(run.session_id, receipt.receipt.id)
            .await
            .unwrap()
            .state,
        RuntimeControlState::Uncertain
    );
    // A native status read alone cannot release custody before the atomic terminal mirror.
    repo.commit_pm_terminal(
        app::HermesTerminalCommit {
            message_id: run.id,
            run_id: run.id,
            runtime_run_id: "run_human".into(),
            runtime_session_id: "effective-human-session".into(),
            state: SessionRunState::Completed,
            body: Some("Verified PM reply".into()),
            error: None,
        },
        domain::PmRuntimeStatus::Completed,
    )
    .await
    .unwrap();
    repo.reconcile_runtime_controls().await.unwrap();
    let observed = repo
        .get_runtime_control(run.session_id, receipt.receipt.id)
        .await
        .unwrap();
    assert_eq!(observed.state, RuntimeControlState::TerminalObserved);
    assert_eq!(
        observed.observed_run_state,
        Some(SessionRunState::Completed)
    );
    assert!(observed.acknowledgement.is_none());
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn pm_human_custody_drift_or_revoked_owner_cannot_claim_and_stop_ack_is_not_terminal() {
    let (repo, run, scope) = accepted().await;
    let actor = RuntimeControlActor {
        user_id: scope.owner_user_id,
        idempotency_key: Uuid::new_v4().to_string(),
    };
    let other_subject = Uuid::new_v4().to_string();
    let other = repo
        .find_or_create_central_user(
            &other_subject,
            &format!("{other_subject}@example.test"),
            "Nonowner operator",
        )
        .await
        .unwrap();
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE users SET system_role='admin' WHERE id=$1",
        [other.id.into()],
    ))
    .await
    .unwrap();
    let mut foreign = actor.clone();
    foreign.user_id = other.id;
    assert!(
        repo.reserve_pm_runtime_control(
            &run,
            &foreign,
            RuntimeControlOperation::Stop,
            None,
            &scope
        )
        .await
        .is_err()
    );
    for change in [
        "fence",
        "native",
        "body",
        "subject",
        "assignment",
        "project",
        "origin",
    ] {
        let mut drift = scope.clone();
        match change {
            "fence" => drift.record.reservation.fence += 1,
            "native" => drift.record.hermes_run_ref = Some("run_other".into()),
            "body" => drift.intent.request_body = json!({"input":"Changed"}).to_string(),
            "assignment" => drift.record.reservation.identity.assignment_revision += 1,
            "project" => {
                drift.record.reservation.identity.tracker_project_ref = Uuid::new_v4().to_string()
            }
            "origin" => drift.intent.origin = "http://127.0.0.1:23811".into(),
            _ => drift.owner_subject = "other-owner".into(),
        }
        assert!(
            repo.reserve_pm_runtime_control(
                &run,
                &actor,
                RuntimeControlOperation::Stop,
                None,
                &drift
            )
            .await
            .is_err()
        );
    }
    assert!(
        repo.list_runtime_controls(run.session_id, run.id)
            .await
            .unwrap()
            .is_empty()
    );
    let command = repo
        .reserve_pm_runtime_control(&run, &actor, RuntimeControlOperation::Stop, None, &scope)
        .await
        .unwrap();
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE users SET is_active=false WHERE id=$1",
        [actor.user_id.into()],
    ))
    .await
    .unwrap();
    assert!(
        repo.claim_pm_runtime_control(command.receipt.id, &scope)
            .await
            .is_err()
    );
    assert_eq!(
        repo.get_runtime_control(run.session_id, command.receipt.id)
            .await
            .unwrap()
            .state,
        RuntimeControlState::Reserved
    );
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE users SET is_active=true WHERE id=$1",
        [actor.user_id.into()],
    ))
    .await
    .unwrap();
    assert!(
        repo.claim_pm_runtime_control(command.receipt.id, &scope)
            .await
            .unwrap()
    );
    let ack = repo
        .finish_runtime_control(command.receipt.id, "stopping", None)
        .await
        .unwrap();
    assert_eq!(ack.state, RuntimeControlState::Acknowledged);
    assert!(ack.observed_run_state.is_none());
    assert_eq!(
        repo.get_session_agent_run(run.id).await.unwrap().state,
        SessionRunState::Stopping
    );
    assert!(
        repo.get_pm_run(run.id)
            .await
            .unwrap()
            .terminal_status
            .is_none()
    );
    assert!(
        !repo
            .reserve_pm_runtime_control(&run, &actor, RuntimeControlOperation::Stop, None, &scope)
            .await
            .unwrap()
            .dispatch
    );
    assert_human_controls_downgrade_refused().await;
    assert_pm_stop_claim_held(&repo, run.id).await;
    assert_eq!(
        serde_json::to_value(
            repo.get_runtime_control(run.session_id, ack.id)
                .await
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(ack).unwrap()
    );
}

// Real API handlers and PostgreSQL custody, with controlled Tracker HTTP only.
// Missing runtime configuration deliberately holds continuation; no live acceptance claim.
#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn http_delivered_answer_pending_resume_survives_reload_revoke_and_exact_id_recovery() {
    use axum::{
        Extension, Json, Router,
        http::StatusCode,
        routing::{get, post},
    };
    use domain::{
        ClarificationAnswerCommand, ClarificationAnswerRequest, ClarificationCommandActor,
        ClarificationContinuationState as C, ClarificationDeliveryState as D,
    };
    use std::sync::atomic::AtomicBool;
    std::env::var("FLEET_TEST_DATABASE_URL").expect("isolated PostgreSQL is required");
    let (repo, op, _) = super::pm_draft_creation::credential_fixture()
        .await
        .expect("configured PostgreSQL fixture");
    let binding = op.identity().unwrap();
    let actor = ClarificationCommandActor {
        session_id: op.session_id.unwrap(),
        user_id: op.owner_user_id,
        subject: op.owner_subject.clone(),
        binding: binding.clone(),
    };
    let original = ClarificationAnswerRequest {
        expected_question_version: 1,
        requirement_revision: 2,
        selected_option_ids: vec![],
        text: Some("Original saved answer".into()),
        comment: None,
        idempotency_key: Uuid::new_v4().to_string(),
    };
    let question = Uuid::new_v4();
    let answer = domain::TrackerAnswer {
        id: Uuid::new_v4(),
        question_id: question,
        question_version: 1,
        requirement_revision: 2,
        selected_option_ids: vec![],
        text: original.text.clone(),
        comment: None,
        author_subject: actor.subject.clone(),
        created_at: chrono::Utc::now(),
    };
    let allowed = Arc::new(AtomicBool::new(true));
    let posts = Arc::new(AtomicUsize::new(0));
    let assignment = op.reservation.as_ref().unwrap().assignment.clone();
    let tracker = Router::new().route("/api/v1/issues/{task}/sdlc/context", get({
        let allowed = allowed.clone(); let binding = binding.clone();
        move || { let allowed=allowed.clone(); let binding=binding.clone(); let assignment=assignment.clone(); async move {
            if !allowed.load(Ordering::SeqCst) { return (StatusCode::FORBIDDEN, Json(json!({"error":{"code":"forbidden"}}))); }
            (StatusCode::OK, Json(json!({"contract_version":1,"tracker_instance_id":binding.tracker_instance_id,
                "project_id":binding.project_id,"task_id":binding.task_id,"root_task_id":binding.root_task_id,
                "owner_subject":binding.owner_subject,"stage":"Clarification","requirement_revision":2,"waiting_reason":"waiting_for_owner_answers",
                "permissions":{"can_answer":true,"can_confirm":false},"assignment":assignment})))
        }}
    })).route("/api/v1/issues/{task}/sdlc/clarifications/{question}/answers", post({
        let posts=posts.clone(); let expected=serde_json::to_value(&original).unwrap();
        move |Json(body): Json<Value>| { let posts=posts.clone(); let expected=expected.clone(); let answer=answer.clone(); async move {
            assert_eq!(body, expected); posts.fetch_add(1,Ordering::SeqCst);
            Json(serde_json::to_value(answer).unwrap())
        }}
    }));
    let (tracker_url, _tracker) = serve(tracker).await;
    let mut config = AppConfig::default();
    config.tracker.url = tracker_url;
    config.tracker.instance_id = binding.tracker_instance_id.clone();
    config.pm.dispatch.enabled = true;
    let config = Arc::new(config);
    let repo = Arc::new(repo);
    let (events, _) = tokio::sync::broadcast::channel(32);
    let (restart, _) = tokio::sync::mpsc::channel(1);
    let runtime = Arc::new(infra::runtime::LocalRuntimeSupervisor::new(
        config.clone(),
        repo.clone(),
        events.clone(),
    ));
    let ctx = Arc::new(app::AppContext::new(
        config,
        repo.clone(),
        Arc::new(infra::FilesystemProvisioner),
        runtime,
        events,
        restart,
    ));
    let user = api::middleware::CurrentUser {
        id: actor.user_id,
        role: domain::SystemRole::User,
        is_system_admin: false,
        central_write: Some(true),
    };
    let routes = Router::new()
        .route(
            "/api/v1/sessions/{session_id}/chat-controls",
            get(api::routes::task_chats::controls),
        )
        .route(
            "/api/v1/sessions/{session_id}/runs/{run_id}/steer",
            post(api::routes::sessions::steer_session_run),
        )
        .route(
            "/api/v1/sessions/{session_id}/runs/{run_id}/stop",
            post(api::routes::sessions::stop_session_run),
        )
        .route(
            "/api/v1/sessions/{session_id}/clarifications/{question_id}/answer-commands",
            post(api::routes::clarification_commands::store),
        )
        .route(
            "/api/v1/sessions/{session_id}/clarification-answer-commands",
            get(api::routes::clarification_commands::pending),
        )
        .route(
            "/api/v1/sessions/{session_id}/clarification-answer-commands/{command_id}/delivery",
            post(api::routes::clarification_commands::delivery),
        )
        .layer(Extension(user))
        .layer(Extension(api::middleware::VerifiedHumanSession))
        .layer(Extension(api::middleware::VerifiedCentralSubject(
            actor.subject.clone(),
        )))
        .with_state(ctx);
    let (url, _fleet) = serve(routes).await;
    let client = reqwest::Client::new();
    let base = format!("{url}/api/v1/sessions/{}", actor.session_id);
    let idle: domain::ChatControls = client
        .get(format!("{base}/chat-controls"))
        .bearer_auth("synthetic-human")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(!idle.can_send && !idle.can_steer && !idle.can_stop);
    assert_eq!(
        idle.blocked_reason.as_deref(),
        Some("pm_idle_prompt_contract_unavailable")
    );
    let reservation = domain::initial_pm_reservation(&op).unwrap();
    repo.reserve_pm_run(reservation.clone()).await.unwrap();
    repo.prepare_pm_dispatch(PmDispatchIntent {
        session_run_id: reservation.session_run_id,
        origin: "http://127.0.0.1:23810".into(),
        credential_fingerprint: "a".repeat(64),
        request_body:
            json!({"input":"Original task","session_id":reservation.runtime_session_id()})
                .to_string(),
        workflow_assignment: json!({"operation_key":reservation.identity.assignment_operation_key}),
        workflow_origin: "http://workflow.test".into(),
        workflow_credential_fingerprint: "b".repeat(64),
        runtime_context: json!({}),
        submitted: false,
        hermes_run_ref: None,
    })
    .await
    .unwrap();
    assert!(
        repo.claim_pm_submission(reservation.session_run_id)
            .await
            .unwrap()
    );
    repo.record_pm_submission(reservation.session_run_id, "run_answer_fixture".into())
        .await
        .unwrap();
    repo.accept_pm_run(
        reservation.session_run_id,
        "run_answer_fixture".into(),
        reservation.runtime_session_id(),
    )
    .await
    .unwrap();
    // Current assignment is valid, but absent effective PM profile must hold both native controls.
    for operation in ["steer", "stop"] {
        let request = client
            .post(format!(
                "{base}/runs/{}/{operation}",
                reservation.session_run_id
            ))
            .bearer_auth("synthetic-human")
            .header("Idempotency-Key", format!("held-{operation}"));
        let request = if operation == "steer" {
            request.json(&json!({"input":"Owner guidance"}))
        } else {
            request
        };
        assert_eq!(
            request.send().await.unwrap().status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }
    assert!(
        repo.list_runtime_controls(actor.session_id, reservation.session_run_id)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(posts.load(Ordering::SeqCst), 0);
    let response = client
        .post(format!("{base}/clarifications/{question}/answer-commands"))
        .bearer_auth("synthetic-human")
        .json(&original)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let stored: ClarificationAnswerCommand = response.json().await.unwrap();
    assert_eq!(stored.continuation_state, C::Pending);
    assert_eq!(posts.load(Ordering::SeqCst), 0);
    let delivery = format!(
        "{base}/clarification-answer-commands/{}/delivery",
        stored.id
    );
    for _ in 0..2 {
        let response = client
            .post(&delivery)
            .bearer_auth("synthetic-human")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "no-store");
        let command: ClarificationAnswerCommand = response.json().await.unwrap();
        assert_eq!(command.id, stored.id);
        assert_eq!(command.payload_sha256, stored.payload_sha256);
        assert_eq!(command.request.idempotency_key, original.idempotency_key);
        assert_eq!(command.request.text, original.text);
        assert_eq!(command.state, D::Delivered);
        assert_eq!(command.continuation_state, C::Pending);
        let pending: Vec<ClarificationAnswerCommand> = client
            .get(format!("{base}/clarification-answer-commands"))
            .bearer_auth("synthetic-human")
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, stored.id);
        assert_eq!(posts.load(Ordering::SeqCst), 1);
    }
    let unchanged = repo
        .get_clarification_command(&actor, stored.id)
        .await
        .unwrap();
    assert_human_controls_downgrade_refused().await;
    let mut replacement = original.clone();
    replacement.idempotency_key = Uuid::new_v4().to_string();
    assert_eq!(
        client
            .post(format!("{base}/clarifications/{question}/answer-commands"))
            .bearer_auth("synthetic-human")
            .json(&replacement)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(posts.load(Ordering::SeqCst), 1);
    assert!(
        repo.store_clarification_command(&actor, question, replacement)
            .await
            .is_err()
    );
    assert_eq!(
        repo.store_clarification_command(&actor, question, original.clone())
            .await
            .unwrap()
            .id,
        stored.id
    );
    allowed.store(false, Ordering::SeqCst);
    assert_eq!(
        client
            .post(&delivery)
            .bearer_auth("synthetic-human")
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(posts.load(Ordering::SeqCst), 1);
    assert_eq!(
        serde_json::to_value(
            repo.get_clarification_command(&actor, stored.id)
                .await
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(unchanged).unwrap()
    );
    assert_eq!(
        repo.finish_clarification_continuation(
            &actor,
            stored.id,
            domain::PmContinuationOutcome::NotRequired
        )
        .await
        .unwrap()
        .continuation_state,
        C::Pending
    );
    assert_eq!(
        repo.finish_clarification_continuation(
            &actor,
            stored.id,
            domain::PmContinuationOutcome::Pending
        )
        .await
        .unwrap()
        .continuation_state,
        C::Pending
    );
    // Repository persistence of a trusted outcome; exact Workflow proof is Planck's runtime test boundary.
    assert_eq!(
        repo.finish_clarification_continuation(
            &actor,
            stored.id,
            domain::PmContinuationOutcome::Confirmed
        )
        .await
        .unwrap()
        .continuation_state,
        C::Confirmed
    );
    assert!(
        repo.list_pending_clarification_commands(&actor)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(posts.load(Ordering::SeqCst), 1);
}

async fn serve(router: axum::Router) -> (String, super::pm_dispatch::AbortServer) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    (
        url,
        super::pm_dispatch::AbortServer(tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap()
        })),
    )
}

async fn assert_human_controls_downgrade_refused() {
    use migration::MigratorTrait;
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let before = migration::Migrator::get_migration_models(&db)
        .await
        .unwrap()
        .into_iter()
        .map(|row| (row.version, row.applied_at))
        .collect::<Vec<_>>();
    let error = migration::Migrator::down(&db, Some(1))
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("PM custody history prevents human controls downgrade"));
    let after = migration::Migrator::get_migration_models(&db)
        .await
        .unwrap()
        .into_iter()
        .map(|row| (row.version, row.applied_at))
        .collect::<Vec<_>>();
    assert_eq!(before, after);
}

async fn assert_pm_stop_claim_held(repo: &PostgresFleetRepository, run: Uuid) {
    let command = domain::PmToolCommand {
        session_run_id: run,
        key: format!("stop:human-custody:{run}"),
        kind: "stop".into(),
        request: json!({"run_id":"run_human","session_id":"effective-human-session"}),
        result: None,
        attempted: false,
    };
    repo.prepare_pm_tool(command.clone()).await.unwrap();
    assert!(!repo.claim_pm_tool(run, &command.key).await.unwrap());
    let saved = repo.prepare_pm_tool(command).await.unwrap();
    assert!(!saved.attempted);
    assert!(saved.result.is_none());
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn pm_control_migration_rejects_forged_original_dispatch_tuple_without_a_receipt() {
    let (repo, run, scope) = accepted().await;
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    for field in ["hash", "native", "owner", "session"] {
        let owner = if field == "owner" {
            Uuid::new_v4()
        } else {
            scope.owner_user_id
        };
        let native = if field == "native" {
            "run_foreign"
        } else {
            "run_human"
        };
        let session = if field == "session" {
            run.runtime_session_id.clone().unwrap()
        } else {
            scope.record.hermes_session_ref.clone().unwrap()
        };
        let error = db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO runtime_control_commands(id,session_id,session_run_id,agent_id,actor_user_id,operation,idempotency_key,
                payload_sha256,runtime_run_id,runtime_session_id,original_request_sha256,api_origin,credential_fingerprint)
             SELECT $1,$2,$3,$4,$5,'stop',$6,repeat('b',64),$7,$8,
                CASE WHEN $9 THEN repeat('f',64) ELSE encode(sha256(convert_to(intent->>'request_body','UTF8')),'hex') END,
                intent->>'origin',intent->>'credential_fingerprint' FROM pm_dispatch_journal WHERE session_run_id=$3",
            [Uuid::new_v4().into(),run.session_id.into(),run.id.into(),run.agent_id.into(),owner.into(),
                Uuid::new_v4().to_string().into(),native.into(),session.into(),(field=="hash").into()]
        )).await.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("runtime control requires original admitted dispatch custody")
        );
    }
    assert!(
        repo.list_runtime_controls(run.session_id, run.id)
            .await
            .unwrap()
            .is_empty()
    );
    let command = domain::PmToolCommand {
        session_run_id: run.id,
        key: format!("stop:continuation:{0}", run.id),
        kind: "stop".into(),
        request: json!({"run_id":"run_human","session_id":"effective-human-session"}),
        result: None,
        attempted: false,
    };
    repo.prepare_pm_tool(command.clone()).await.unwrap();
    assert!(repo.claim_pm_tool(run.id, &command.key).await.unwrap());
    assert!(
        repo.reserve_pm_runtime_control(
            &run,
            &RuntimeControlActor {
                user_id: scope.owner_user_id,
                idempotency_key: Uuid::new_v4().to_string(),
            },
            RuntimeControlOperation::Stop,
            None,
            &scope,
        )
        .await
        .is_err()
    );
    assert!(
        repo.list_runtime_controls(run.session_id, run.id)
            .await
            .unwrap()
            .is_empty()
    );
}
