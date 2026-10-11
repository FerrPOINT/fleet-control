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
    let requested_session = run.runtime_session_id.clone().unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE session_agent_runs SET runtime_session_id=$2 WHERE id=$1",
        [run.id.into(), "foreign-requested-session".into()],
    ))
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
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE session_agent_runs SET runtime_session_id=$2 WHERE id=$1",
        [run.id.into(), requested_session.into()],
    ))
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
    repo.reconcile_runtime_controls().await.unwrap();
    assert_eq!(
        serde_json::to_value(
            repo.get_runtime_control(run.session_id, observed.id)
                .await
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(observed).unwrap()
    );
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
    let binding = repo
        .get_task_chat_binding(op.session_id.unwrap())
        .await
        .unwrap()
        .expect("persisted task-chat binding");
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
    let idle: serde_json::Value = client
        .get(format!("{base}/chat-controls"))
        .bearer_auth("synthetic-human")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        idle,
        json!({
            "can_send": false,
            "can_steer": false,
            "can_stop": false,
            "active_run_id": null,
            "blocked_reason": "pm_idle_prompt_contract_unavailable"
        })
    );
    let reservation = domain::initial_pm_reservation(&op).unwrap();
    repo.reserve_pm_run(reservation.clone()).await.unwrap();
    let native_posts = Arc::new(AtomicUsize::new(0));
    let native = Router::new().route(
        "/v1/runs/{run}/stop",
        post({
            let posts = native_posts.clone();
            move || {
                let posts = posts.clone();
                async move {
                    posts.fetch_add(1, Ordering::SeqCst);
                    StatusCode::SERVICE_UNAVAILABLE
                }
            }
        }),
    );
    let (native_url, _native) = serve(native).await;
    repo.prepare_pm_dispatch(PmDispatchIntent {
        session_run_id: reservation.session_run_id,
        origin: native_url,
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
    let run = repo
        .get_session_agent_run(reservation.session_run_id)
        .await
        .unwrap();
    let scope = PmHumanControlScope {
        record: repo.get_pm_run(run.id).await.unwrap(),
        intent: repo.get_pm_dispatch(run.id).await.unwrap().unwrap(),
        owner_subject: actor.subject.clone(),
        owner_user_id: actor.user_id,
    };
    let stop_actor = RuntimeControlActor {
        user_id: actor.user_id,
        idempotency_key: Uuid::new_v4().to_string(),
    };
    let stop = repo
        .reserve_pm_runtime_control(
            &run,
            &stop_actor,
            RuntimeControlOperation::Stop,
            None,
            &scope,
        )
        .await
        .unwrap();
    assert!(
        repo.claim_pm_runtime_control(stop.receipt.id, &scope)
            .await
            .unwrap()
    );
    let stop = repo
        .finish_runtime_control(stop.receipt.id, "stopping", None)
        .await
        .unwrap();
    let replay_url = format!("{base}/runs/{}/stop", run.id);
    for _ in 0..2 {
        let response = client
            .post(&replay_url)
            .bearer_auth("synthetic-human")
            .header("Idempotency-Key", &stop_actor.idempotency_key)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let response: domain::RuntimeRunControlResponse = response.json().await.unwrap();
        assert!(response.accepted);
        assert_eq!(response.state, SessionRunState::Stopping);
        assert_eq!(
            serde_json::to_value(response.command.unwrap()).unwrap(),
            serde_json::to_value(&stop).unwrap()
        );
    }
    assert_eq!(native_posts.load(Ordering::SeqCst), 0);
    assert_eq!(
        client
            .post(format!("{base}/runs/{}/steer", run.id))
            .bearer_auth("synthetic-human")
            .header("Idempotency-Key", &stop_actor.idempotency_key)
            .json(&json!({"input":"different operation and body"}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        repo.list_runtime_controls(actor.session_id, run.id)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(native_posts.load(Ordering::SeqCst), 0);
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
            .post(&replay_url)
            .bearer_auth("synthetic-human")
            .header("Idempotency-Key", &stop_actor.idempotency_key)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(native_posts.load(Ordering::SeqCst), 0);
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
    let human_controls = migration::Migrator::migrations()
        .into_iter()
        .find(|item| item.name() == "m20261010_000023_pm_human_controls")
        .expect("PM human controls migration must remain registered");
    let before = migration::Migrator::get_migration_models(&db)
        .await
        .unwrap()
        .into_iter()
        .map(|row| (row.version, row.applied_at))
        .collect::<Vec<_>>();
    let error = human_controls
        .down(&migration::SchemaManager::new(&db))
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

// Synthetic pre-023 history uses the real 024 ACK repair, not untouched 022 schema.
// The ledger stays at 022 until the normal upgrade; no continuation receipt column exists.
// HTTP acceptance is not simulated here: this exercises only the migration's
// interpretation of durable producer-shaped repository history in an owned schema.
#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn legacy_pm_answer_backfill_requires_source_answer_and_completed_gated_tool_history() {
    use domain::{ClarificationAnswerRequest, ClarificationCommandActor, PmRunReservation};
    use migration::MigratorTrait;
    use sea_orm::{ConnectOptions, Database};
    for case in [
        "completed_a_then_checkpoint_b",
        "native_ack_only",
        "missing_source_answer",
        "source_answer_mismatch",
        "checkpoint_mismatch",
        "identity_mismatch",
        "fence_mismatch",
        "run_custody_mismatch",
        "unfinished_tool",
        "options_permuted",
        "options_different",
        "options_duplicate",
        "malformed_answer_id",
        "nil_answer_id",
    ] {
        let url =
            std::env::var("FLEET_TEST_DATABASE_URL").expect("isolated PostgreSQL is required");
        let admin = Database::connect(&url).await.unwrap();
        let schema = format!("fleet_pm_human_{}", Uuid::new_v4().simple());
        admin
            .execute_unprepared(&format!("CREATE SCHEMA {schema}"))
            .await
            .unwrap();
        let connect = || {
            let mut options = ConnectOptions::new(url.clone());
            options.max_connections(2).set_schema_search_path(&schema);
            Database::connect(options)
        };
        let db = connect().await.unwrap();
        let migrations = migration::Migrator::migrations();
        let legacy_prefix = migrations
            .iter()
            .position(|item| item.name() == "m20261010_000022_pm_dispatch")
            .expect("PM dispatch migration must remain registered")
            + 1;
        migration::Migrator::up(&db, Some(u32::try_from(legacy_prefix).unwrap()))
            .await
            .unwrap();
        let versions = migration::Migrator::get_migration_models(&db)
            .await
            .unwrap();
        assert_eq!(
            versions.last().unwrap().version,
            "m20261010_000022_pm_dispatch"
        );
        let before_ack_repair = versions
            .into_iter()
            .map(|row| (row.version, row.applied_at))
            .collect::<Vec<_>>();
        // Apply only the registered constraint repair before seeding native ACK custody.
        // Normal Migrator::up below still applies 023 and records idempotent 024 itself.
        let ack_repair = migrations
            .iter()
            .find(|item| item.name() == "m20261010_000024_pm_ack_bounds")
            .expect("PM ACK bounds migration must remain registered");
        ack_repair
            .up(&migration::SchemaManager::new(&db))
            .await
            .unwrap();
        assert_eq!(
            migration::Migrator::get_migration_models(&db)
                .await
                .unwrap()
                .into_iter()
                .map(|row| (row.version, row.applied_at))
                .collect::<Vec<_>>(),
            before_ack_repair
        );
        let columns = db.query_one(Statement::from_string(DatabaseBackend::Postgres,
            "SELECT EXISTS(SELECT 1 FROM information_schema.columns WHERE table_schema=current_schema()
                AND table_name='clarification_answer_commands' AND column_name='continuation_state') AS present".to_owned())).await.unwrap().unwrap();
        assert!(!columns.try_get::<bool>("", "present").unwrap());
        let repo = PostgresFleetRepository::new(connect().await.unwrap());
        let subject = Uuid::new_v4().to_string();
        let owner = repo
            .find_or_create_central_user(&subject, &format!("{subject}@example.test"), "Owner")
            .await
            .unwrap();
        let agent_id = agent(&repo).await;
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE agents SET sdlc_role='project_manager' WHERE id=$1",
            [agent_id.into()],
        ))
        .await
        .unwrap();
        let session = repo
            .create_session(chat(agent_id, "legacy-pm"), owner.id)
            .await
            .unwrap();
        let binding = domain::TaskChatBinding {
            tracker_instance_id: "legacy-pm-tracker".into(),
            project_id: Uuid::new_v4(),
            task_id: Uuid::new_v4(),
            root_task_id: Uuid::new_v4(),
            agent_id,
            owner_subject: subject.clone(),
        };
        repo.bind_task_chat(session.id, binding.clone(), "legacy-binding".into())
            .await
            .unwrap();
        let actor = ClarificationCommandActor {
            session_id: session.id,
            user_id: owner.id,
            subject: subject.clone(),
            binding: binding.clone(),
        };
        let command = Uuid::new_v4();
        let question = Uuid::new_v4();
        let checkpoint = Uuid::new_v4();
        let request = ClarificationAnswerRequest {
            expected_question_version: 1,
            requirement_revision: 2,
            selected_option_ids: if case.starts_with("options_") {
                vec![Uuid::from_u128(1), Uuid::from_u128(2)]
            } else {
                vec![]
            },
            text: Some("Original A answer".into()),
            comment: None,
            idempotency_key: "original-a".into(),
        };
        let body = domain::canonical_answer_request(request.clone()).unwrap();
        let mut answer = json!({"id":Uuid::new_v4(),"question_id":question,"question_version":1,
            "requirement_revision":2,"selected_option_ids":request.selected_option_ids,"text":request.text,"comment":null,
            "author_subject":subject,"created_at":"2026-10-10T00:00:00Z"});
        match case {
            "options_permuted" => {
                answer["selected_option_ids"] = json!([Uuid::from_u128(2), Uuid::from_u128(1)]);
            }
            "options_different" => {
                answer["selected_option_ids"] = json!([Uuid::from_u128(1), Uuid::from_u128(3)]);
            }
            "options_duplicate" => {
                answer["selected_option_ids"] =
                    json!([Uuid::from_u128(1), Uuid::from_u128(1), Uuid::from_u128(2)]);
            }
            "malformed_answer_id" => answer["id"] = json!("garbage"),
            "nil_answer_id" => answer["id"] = json!(Uuid::nil()),
            _ => {}
        }
        db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO clarification_answer_commands(id,session_id,question_id,actor_user_id,owner_subject,binding,idempotency_key,request_body,payload_sha256)
             VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)",
            [command.into(),session.id.into(),question.into(),owner.id.into(),subject.clone().into(),
                serde_json::to_value(&binding).unwrap().into(),request.idempotency_key.clone().into(),body.clone().into(),domain::answer_payload_hash(&body).into()])).await.unwrap();
        let attempt = Uuid::new_v4();
        db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "UPDATE clarification_answer_commands SET state='delivering',attempt_id=$2,lease_until=clock_timestamp()+interval '30 seconds' WHERE id=$1",
            [command.into(),attempt.into()])).await.unwrap();
        db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "UPDATE clarification_answer_commands SET state='delivered',lease_until=NULL,answer=$2 WHERE id=$1",
            [command.into(),answer.clone().into()])).await.unwrap();
        let original = PmRunReservation {
            session_id: session.id,
            session_run_id: Uuid::new_v4(),
            identity: domain::PmExecutionIdentity {
                task: "SDLC-42".into(),
                execution_ref: Uuid::new_v4().to_string(),
                tracker_instance_ref: binding.tracker_instance_id.clone(),
                tracker_project_ref: binding.project_id.to_string(),
                task_ref: binding.task_id.to_string(),
                root_ref: binding.root_task_id.to_string(),
                agent_ref: agent_id.to_string(),
                assignment_operation_key: "assign-legacy".into(),
                assignment_ref: Uuid::new_v4().to_string(),
                assignment_revision: 1,
            },
            binding_ref: "original-binding".into(),
            dispatch_operation_key: "original-dispatch".into(),
            checkpoint_ref: None,
            fence: 1,
        };
        accepted_legacy_run(&repo, &original, "run_original").await;
        let mut original_answer = answer.clone();
        if case == "source_answer_mismatch" {
            original_answer["id"] = json!(Uuid::new_v4());
        }
        if case == "missing_source_answer" {
            original_answer = Value::Null;
        }
        let corrupt_answer_id = matches!(case, "malformed_answer_id" | "nil_answer_id");
        if corrupt_answer_id {
            original_answer["id"] = json!(Uuid::new_v4());
        }
        let (original_request, mut original_question) = legacy_question(
            &original,
            question,
            if case == "checkpoint_mismatch" {
                Uuid::new_v4()
            } else {
                checkpoint
            },
            original_answer,
        );
        // Corrupt legacy JSON after constructing the valid typed producer fixture.
        if corrupt_answer_id {
            original_question["answer"] = answer.clone();
        }
        let original_key = original_request["idempotency_key"]
            .as_str()
            .unwrap()
            .to_owned();
        complete_legacy_tool(
            &repo,
            &db,
            original.session_run_id,
            &original_key,
            original_request,
            Some(original_question),
        )
        .await;
        repo.observe_pm_run(original.session_run_id, domain::PmRuntimeStatus::Completed)
            .await
            .unwrap();
        let mut resumed = PmRunReservation {
            session_run_id: command,
            binding_ref: format!("fleet:pm:{command}"),
            dispatch_operation_key: format!("fleet-pm-resume:{command}"),
            checkpoint_ref: Some(checkpoint.to_string()),
            fence: 2,
            ..original.clone()
        };
        if case == "identity_mismatch" {
            resumed.identity.execution_ref = Uuid::new_v4().to_string();
        }
        if case == "fence_mismatch" {
            resumed.fence += 1;
        }
        accepted_legacy_run(&repo, &resumed, "run_continuation_a").await;
        if case == "run_custody_mismatch" {
            db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE session_agent_runs SET runtime_session_id=$2 WHERE id=$1",
                [command.into(), "foreign-requested-session".into()],
            ))
            .await
            .unwrap();
        }
        let next_question = Uuid::new_v4();
        if case != "native_ack_only" {
            let (next_request, next_result) =
                legacy_question(&resumed, next_question, Uuid::new_v4(), Value::Null);
            let next_key = next_request["idempotency_key"].as_str().unwrap().to_owned();
            complete_legacy_tool(
                &repo,
                &db,
                command,
                &next_key,
                next_request,
                (case != "unfinished_tool").then_some(next_result),
            )
            .await;
        }
        // The next question belongs to checkpoint B; the original answer A must not be resumed again.
        let before = legacy_history_snapshot(&db).await;
        let before_versions = migration::Migrator::get_migration_models(&db)
            .await
            .unwrap()
            .into_iter()
            .map(|r| (r.version, r.applied_at))
            .collect::<Vec<_>>();
        if matches!(case, "completed_a_then_checkpoint_b" | "options_permuted") {
            migration::Migrator::up(&db, None).await.unwrap();
            assert_eq!(legacy_history_snapshot(&db).await, before);
            let saved = repo
                .get_clarification_command(&actor, command)
                .await
                .unwrap();
            assert_eq!(
                saved.continuation_state,
                domain::ClarificationContinuationState::Confirmed
            );
            assert_eq!(saved.id, command);
            assert_eq!(saved.request.idempotency_key, request.idempotency_key);
            assert_eq!(serde_json::to_value(saved.answer).unwrap(), answer);
            assert!(
                repo.list_pending_clarification_commands(&actor)
                    .await
                    .unwrap()
                    .is_empty()
            );
            let mut next = request.clone();
            next.idempotency_key = "new-b".into();
            repo.store_clarification_command(&actor, next_question, next)
                .await
                .unwrap();
            let after_versions = migration::Migrator::get_migration_models(&db)
                .await
                .unwrap()
                .into_iter()
                .map(|r| (r.version, r.applied_at))
                .collect::<Vec<_>>();
            migration::Migrator::up(&db, None).await.unwrap();
            assert_eq!(
                migration::Migrator::get_migration_models(&db)
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|r| (r.version, r.applied_at))
                    .collect::<Vec<_>>(),
                after_versions
            );
            assert_eq!(
                repo.get_clarification_command(&actor, command)
                    .await
                    .unwrap()
                    .continuation_state,
                domain::ClarificationContinuationState::Confirmed
            );
        } else {
            for _ in 0..2 {
                let error = migration::Migrator::up(&db, None)
                    .await
                    .unwrap_err()
                    .to_string();
                assert!(
                    error.contains(
                        "legacy PM answer continuation requires authoritative reconciliation"
                    ),
                    "{case}"
                );
                assert_eq!(legacy_history_snapshot(&db).await, before);
                assert_eq!(
                    migration::Migrator::get_migration_models(&db)
                        .await
                        .unwrap()
                        .into_iter()
                        .map(|r| (r.version, r.applied_at))
                        .collect::<Vec<_>>(),
                    before_versions
                );
                let columns = db.query_one(Statement::from_string(DatabaseBackend::Postgres,
                    "SELECT EXISTS(SELECT 1 FROM information_schema.columns WHERE table_schema=current_schema()
                        AND table_name='clarification_answer_commands' AND column_name='continuation_state') AS present".to_owned())).await.unwrap().unwrap();
                assert!(!columns.try_get::<bool>("", "present").unwrap());
            }
        }
        drop(repo);
        db.close().await.unwrap();
        admin
            .execute_unprepared(&format!("DROP SCHEMA {schema} CASCADE"))
            .await
            .unwrap();
        admin.close().await.unwrap();
    }
}

async fn accepted_legacy_run(
    repo: &PostgresFleetRepository,
    reservation: &domain::PmRunReservation,
    native: &str,
) {
    repo.reserve_pm_run(reservation.clone()).await.unwrap();
    repo.prepare_pm_dispatch(PmDispatchIntent {
        session_run_id: reservation.session_run_id, origin: "http://127.0.0.1:23810".into(),
        credential_fingerprint: "a".repeat(64),
        request_body: json!({"input":"Producer-shaped legacy fixture","session_id":reservation.runtime_session_id()}).to_string(),
        workflow_assignment: json!({"operation_key":reservation.identity.assignment_operation_key}),
        workflow_origin: "http://workflow.test".into(), workflow_credential_fingerprint: "b".repeat(64),
        runtime_context: json!({}), submitted: false, hermes_run_ref: None,
    }).await.unwrap();
    assert!(
        repo.claim_pm_submission(reservation.session_run_id)
            .await
            .unwrap()
    );
    repo.record_pm_submission(reservation.session_run_id, native.into())
        .await
        .unwrap();
    repo.accept_pm_run(
        reservation.session_run_id,
        native.into(),
        "effective-legacy-session".into(),
    )
    .await
    .unwrap();
    repo.observe_pm_run(reservation.session_run_id, domain::PmRuntimeStatus::Running)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn pm_stop_custody_migration_preserves_original_function_and_holds_unsafe_downgrade() {
    use domain::{PmGuidancePermit, PmRunReservation};
    use migration::MigratorTrait;
    use sea_orm::{ConnectOptions, Database};
    for legacy in [false, true] {
        let prefix = if legacy { 28 } else { 25 };
        let url =
            std::env::var("FLEET_TEST_DATABASE_URL").expect("isolated PostgreSQL is required");
        let admin = Database::connect(&url).await.unwrap();
        let schema = format!("fleet_pm_stop_{}", Uuid::new_v4().simple());
        admin
            .execute_unprepared(&format!("CREATE SCHEMA {schema}"))
            .await
            .unwrap();
        let mut options = ConnectOptions::new(url);
        options.max_connections(2).set_schema_search_path(&schema);
        let db = Database::connect(options.clone()).await.unwrap();
        if legacy {
            migration::LegacyMigrator::up(&db, Some(prefix))
                .await
                .unwrap();
        } else {
            migration::Migrator::up(&db, Some(prefix)).await.unwrap();
        }
        let versions = migration::Migrator::get_migration_models(&db)
            .await
            .unwrap()
            .into_iter()
            .map(|r| (r.version, r.applied_at))
            .collect::<Vec<_>>();
        assert_eq!(versions.len(), prefix as usize);
        assert_eq!(versions.last().unwrap().0, "m20261010_000024_pm_ack_bounds");
        let definition_sql = "SELECT p.oid::bigint AS id,p.prosrc AS body,
        pg_get_functiondef(p.oid) AS definition,pg_get_triggerdef(t.oid) AS trigger_definition
        FROM pg_proc p JOIN pg_trigger t ON t.tgfoid=p.oid
        WHERE t.tgrelid='runtime_control_commands'::regclass AND t.tgname='runtime_control_custody'";
        let original = db
            .query_one(Statement::from_string(
                DatabaseBackend::Postgres,
                definition_sql.to_owned(),
            ))
            .await
            .unwrap()
            .unwrap();
        let original_id: i64 = original.try_get("", "id").unwrap();
        let original_body: String = original.try_get("", "body").unwrap();
        let repo_db = Database::connect(options).await.unwrap();
        let repo_pool = repo_db.get_postgres_connection_pool().clone();
        let repo = PostgresFleetRepository::new(repo_db);
        let subject = Uuid::new_v4().to_string();
        let owner = repo
            .find_or_create_central_user(&subject, &format!("{subject}@example.test"), "Owner")
            .await
            .unwrap();
        let agent_id = agent(&repo).await;
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE agents SET sdlc_role='project_manager' WHERE id=$1",
            [agent_id.into()],
        ))
        .await
        .unwrap();
        let session = repo
            .create_session(chat(agent_id, "pm-stop-migration"), owner.id)
            .await
            .unwrap();
        let binding = domain::TaskChatBinding {
            tracker_instance_id: "stop-migration-tracker".into(),
            project_id: Uuid::new_v4(),
            task_id: Uuid::new_v4(),
            root_task_id: Uuid::new_v4(),
            agent_id,
            owner_subject: subject.clone(),
        };
        repo.bind_task_chat(session.id, binding.clone(), "stop-migration-binding".into())
            .await
            .unwrap();
        let reservation = PmRunReservation {
            session_id: session.id,
            session_run_id: Uuid::new_v4(),
            identity: domain::PmExecutionIdentity {
                task: "SDLC-42".into(),
                execution_ref: Uuid::new_v4().to_string(),
                tracker_instance_ref: binding.tracker_instance_id,
                tracker_project_ref: binding.project_id.to_string(),
                task_ref: binding.task_id.to_string(),
                root_ref: binding.root_task_id.to_string(),
                agent_ref: agent_id.to_string(),
                assignment_operation_key: "assign-stop-migration".into(),
                assignment_ref: Uuid::new_v4().to_string(),
                assignment_revision: 1,
            },
            binding_ref: "stop-migration-binding".into(),
            dispatch_operation_key: "stop-migration-dispatch".into(),
            checkpoint_ref: None,
            fence: 1,
        };
        accepted_legacy_run(&repo, &reservation, "run_stop_migration").await;
        let guidance = json!({"input":"Initial guidance with unknown native ACK"}).to_string();
        assert!(matches!(
            repo.claim_pm_guidance(reservation.session_run_id, guidance.clone())
                .await
                .unwrap(),
            PmGuidancePermit::Claimed
        ));
        assert!(matches!(
            repo.claim_pm_guidance(reservation.session_run_id, guidance)
                .await
                .unwrap(),
            PmGuidancePermit::Unknown
        ));
        let custody = legacy_history_snapshot(&db).await;
        let old_stop_error = db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO runtime_control_commands(id,session_id,session_run_id,agent_id,actor_user_id,operation,idempotency_key,
            payload_sha256,runtime_run_id,runtime_session_id,original_request_sha256,api_origin,credential_fingerprint)
         SELECT $1,$2,$3,$4,$5,'stop',$6,repeat('b',64),'run_stop_migration','effective-legacy-session',
            encode(sha256(convert_to(intent->>'request_body','UTF8')),'hex'),
            intent->>'origin',intent->>'credential_fingerprint' FROM pm_dispatch_journal WHERE session_run_id=$3",
        [Uuid::new_v4().into(),session.id.into(),reservation.session_run_id.into(),agent_id.into(),
            owner.id.into(),Uuid::new_v4().to_string().into()])).await.unwrap_err();
        assert!(
            old_stop_error
                .to_string()
                .contains("runtime control requires original admitted dispatch custody")
        );
        assert!(
            repo.list_runtime_controls(session.id, reservation.session_run_id)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(legacy_history_snapshot(&db).await, custody);
        let old_prerequisite =
            "(b.reservation->>'checkpoint_ref' IS NOT NULL OR j.guidance_delivered)";
        let new_prerequisite = "(NEW.operation='stop' OR b.reservation->>'checkpoint_ref' IS NOT NULL OR j.guidance_delivered)";
        assert_eq!(original_body.matches(old_prerequisite).count(), 1);
        // An empty control ledger can restore the exact function OID/body and trigger definition.
        for _ in 0..2 {
            migration::Migrator::up(&db, Some(1)).await.unwrap();
            let upgraded = migration::Migrator::get_migration_models(&db)
                .await
                .unwrap()
                .into_iter()
                .map(|r| (r.version, r.applied_at))
                .collect::<Vec<_>>();
            assert_eq!(&upgraded[..prefix as usize], &versions);
            assert_eq!(upgraded.len(), prefix as usize + 1);
            assert_eq!(
                upgraded[prefix as usize].0,
                "m20261011_000025_pm_stop_custody"
            );
            let active = db
                .query_one(Statement::from_string(
                    DatabaseBackend::Postgres,
                    definition_sql.to_owned(),
                ))
                .await
                .unwrap()
                .unwrap();
            assert_eq!(
                active.try_get::<String>("", "body").unwrap(),
                original_body.replace(old_prerequisite, new_prerequisite)
            );
            let saved = db.query_one(Statement::from_string(DatabaseBackend::Postgres,
            "SELECT oid::bigint AS id,prosrc AS body FROM pg_proc WHERE oid='admit_runtime_control_custody_v23()'::regprocedure".to_owned()))
            .await.unwrap().unwrap();
            assert_eq!(saved.try_get::<i64>("", "id").unwrap(), original_id);
            assert_eq!(saved.try_get::<String>("", "body").unwrap(), original_body);
            assert_eq!(legacy_history_snapshot(&db).await, custody);
            migration::Migrator::down(&db, Some(1)).await.unwrap();
            let restored = db
                .query_one(Statement::from_string(
                    DatabaseBackend::Postgres,
                    definition_sql.to_owned(),
                ))
                .await
                .unwrap()
                .unwrap();
            for column in ["body", "definition", "trigger_definition"] {
                assert_eq!(
                    restored.try_get::<String>("", column).unwrap(),
                    original.try_get::<String>("", column).unwrap()
                );
            }
            assert_eq!(restored.try_get::<i64>("", "id").unwrap(), original_id);
            assert_eq!(
                migration::Migrator::get_migration_models(&db)
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|r| (r.version, r.applied_at))
                    .collect::<Vec<_>>(),
                versions
            );
            assert_eq!(legacy_history_snapshot(&db).await, custody);
        }
        migration::Migrator::up(&db, Some(1)).await.unwrap();
        // Direct INSERTs exercise the DB trigger, independently of repository admission.
        for case in [
            "steer",
            "foreign_owner",
            "foreign_run",
            "foreign_session",
            "foreign_hash",
        ] {
            let error = db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO runtime_control_commands(id,session_id,session_run_id,agent_id,actor_user_id,operation,idempotency_key,
                payload_sha256,runtime_run_id,runtime_session_id,original_request_sha256,api_origin,credential_fingerprint)
             SELECT $1,$2,$3,$4,$5,$6,$7,repeat('b',64),$8,$9,
                CASE WHEN $10 THEN repeat('f',64) ELSE encode(sha256(convert_to(intent->>'request_body','UTF8')),'hex') END,
                intent->>'origin',intent->>'credential_fingerprint' FROM pm_dispatch_journal WHERE session_run_id=$3",
            [Uuid::new_v4().into(),session.id.into(),reservation.session_run_id.into(),agent_id.into(),
                (if case=="foreign_owner" {Uuid::new_v4()} else {owner.id}).into(),
                (if case=="steer" {"steer"} else {"stop"}).into(),Uuid::new_v4().to_string().into(),
                (if case=="foreign_run" {"run_foreign"} else {"run_stop_migration"}).into(),
                (if case=="foreign_session" {"foreign-session"} else {"effective-legacy-session"}).into(),
                (case=="foreign_hash").into()])).await.unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("runtime control requires original admitted dispatch custody"),
                "{case}"
            );
        }
        let run = repo
            .get_session_agent_run(reservation.session_run_id)
            .await
            .unwrap();
        let scope = PmHumanControlScope {
            record: repo.get_pm_run(run.id).await.unwrap(),
            intent: repo.get_pm_dispatch(run.id).await.unwrap().unwrap(),
            owner_user_id: owner.id,
            owner_subject: subject,
        };
        let actor = RuntimeControlActor {
            user_id: owner.id,
            idempotency_key: "original-stop".into(),
        };
        let stop = repo
            .reserve_pm_runtime_control(&run, &actor, RuntimeControlOperation::Stop, None, &scope)
            .await
            .unwrap();
        assert!(stop.dispatch);
        assert!(
            repo.claim_pm_runtime_control(stop.receipt.id, &scope)
                .await
                .unwrap()
        );
        assert!(
            !repo
                .claim_pm_runtime_control(stop.receipt.id, &scope)
                .await
                .unwrap()
        );
        repo.retire_runtime_control(stop.receipt.id, true)
            .await
            .unwrap();
        let before = migration::Migrator::get_migration_models(&db)
            .await
            .unwrap()
            .into_iter()
            .map(|r| (r.version, r.applied_at))
            .collect::<Vec<_>>();
        let active = db
            .query_one(Statement::from_string(
                DatabaseBackend::Postgres,
                definition_sql.to_owned(),
            ))
            .await
            .unwrap()
            .unwrap();
        let data = legacy_history_snapshot(&db).await;
        let control_sql =
            "SELECT to_jsonb(c) AS record FROM runtime_control_commands c WHERE id=$1";
        let original_control: Value = db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                control_sql,
                [stop.receipt.id.into()],
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "record")
            .unwrap();
        for _ in 0..2 {
            let error = migration::Migrator::down(&db, Some(1))
                .await
                .unwrap_err()
                .to_string();
            assert!(error.contains("PM stop custody prevents guidance prerequisite downgrade"));
            assert_eq!(
                migration::Migrator::get_migration_models(&db)
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|r| (r.version, r.applied_at))
                    .collect::<Vec<_>>(),
                before
            );
            assert_eq!(legacy_history_snapshot(&db).await, data);
            let unchanged = db
                .query_one(Statement::from_string(
                    DatabaseBackend::Postgres,
                    definition_sql.to_owned(),
                ))
                .await
                .unwrap()
                .unwrap();
            for column in ["body", "definition", "trigger_definition"] {
                assert_eq!(
                    unchanged.try_get::<String>("", column).unwrap(),
                    active.try_get::<String>("", column).unwrap()
                );
            }
            let current_control: Value = db
                .query_one(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    control_sql,
                    [stop.receipt.id.into()],
                ))
                .await
                .unwrap()
                .unwrap()
                .try_get("", "record")
                .unwrap();
            assert_eq!(current_control, original_control);
            let receipt = repo
                .get_runtime_control(session.id, stop.receipt.id)
                .await
                .unwrap();
            assert_eq!(receipt.state, RuntimeControlState::Uncertain);
            assert_eq!(
                repo.list_runtime_controls(session.id, run.id)
                    .await
                    .unwrap()
                    .len(),
                1
            );
        }
        drop(repo);
        repo_pool.close().await;
        db.close().await.unwrap();
        admin
            .execute_unprepared(&format!("DROP SCHEMA {schema} CASCADE"))
            .await
            .unwrap();
        admin.close().await.unwrap();
    }
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn pm_stop_drain_migration_preserves_v25_function_and_holds_retained_stop_history() {
    use domain::{PmGuidancePermit, PmRunReservation};
    use migration::MigratorTrait;
    use sea_orm::{ConnectOptions, Database};
    for legacy in [false, true] {
        let prefix = if legacy { 29 } else { 26 };
        let url =
            std::env::var("FLEET_TEST_DATABASE_URL").expect("isolated PostgreSQL is required");
        let admin = Database::connect(&url).await.unwrap();
        let schema = format!("fleet_pm_drain_{}", Uuid::new_v4().simple());
        admin
            .execute_unprepared(&format!("CREATE SCHEMA {schema}"))
            .await
            .unwrap();
        let mut options = ConnectOptions::new(url);
        options.max_connections(2).set_schema_search_path(&schema);
        let db = Database::connect(options.clone()).await.unwrap();
        if legacy {
            migration::LegacyMigrator::up(&db, Some(prefix))
                .await
                .unwrap();
        } else {
            migration::Migrator::up(&db, Some(prefix)).await.unwrap();
        }
        let versions = migration::Migrator::get_migration_models(&db)
            .await
            .unwrap()
            .into_iter()
            .map(|r| (r.version, r.applied_at))
            .collect::<Vec<_>>();
        assert_eq!(versions.len(), prefix as usize);
        assert_eq!(
            versions.last().unwrap().0,
            "m20261011_000025_pm_stop_custody"
        );
        let definition_sql = "SELECT p.oid::bigint AS id,p.prosrc AS body,
        pg_get_functiondef(p.oid) AS definition,pg_get_triggerdef(t.oid) AS trigger_definition
        FROM pg_proc p JOIN pg_trigger t ON t.tgfoid=p.oid
        WHERE t.tgrelid='runtime_control_commands'::regclass AND t.tgname='runtime_control_custody'";
        let original = db
            .query_one(Statement::from_string(
                DatabaseBackend::Postgres,
                definition_sql.to_owned(),
            ))
            .await
            .unwrap()
            .unwrap();
        let original_id: i64 = original.try_get("", "id").unwrap();
        let original_body: String = original.try_get("", "body").unwrap();
        let repo_db = Database::connect(options).await.unwrap();
        let repo_pool = repo_db.get_postgres_connection_pool().clone();
        let repo = PostgresFleetRepository::new(repo_db);
        let subject = Uuid::new_v4().to_string();
        let owner = repo
            .find_or_create_central_user(&subject, &format!("{subject}@example.test"), "Owner")
            .await
            .unwrap();
        let agent_id = agent(&repo).await;
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE agents SET sdlc_role='project_manager' WHERE id=$1",
            [agent_id.into()],
        ))
        .await
        .unwrap();
        let session = repo
            .create_session(chat(agent_id, "pm-stop-migration"), owner.id)
            .await
            .unwrap();
        let binding = domain::TaskChatBinding {
            tracker_instance_id: "stop-migration-tracker".into(),
            project_id: Uuid::new_v4(),
            task_id: Uuid::new_v4(),
            root_task_id: Uuid::new_v4(),
            agent_id,
            owner_subject: subject.clone(),
        };
        repo.bind_task_chat(session.id, binding.clone(), "stop-migration-binding".into())
            .await
            .unwrap();
        let reservation = PmRunReservation {
            session_id: session.id,
            session_run_id: Uuid::new_v4(),
            identity: domain::PmExecutionIdentity {
                task: "SDLC-42".into(),
                execution_ref: Uuid::new_v4().to_string(),
                tracker_instance_ref: binding.tracker_instance_id,
                tracker_project_ref: binding.project_id.to_string(),
                task_ref: binding.task_id.to_string(),
                root_ref: binding.root_task_id.to_string(),
                agent_ref: agent_id.to_string(),
                assignment_operation_key: "assign-stop-migration".into(),
                assignment_ref: Uuid::new_v4().to_string(),
                assignment_revision: 1,
            },
            binding_ref: "stop-migration-binding".into(),
            dispatch_operation_key: "stop-migration-dispatch".into(),
            checkpoint_ref: None,
            fence: 1,
        };
        accepted_legacy_run(&repo, &reservation, "run_stop_migration").await;
        let guidance = json!({"input":"Verified initial guidance"}).to_string();
        assert!(matches!(
            repo.claim_pm_guidance(reservation.session_run_id, guidance)
                .await
                .unwrap(),
            PmGuidancePermit::Claimed
        ));
        repo.finish_pm_guidance(reservation.session_run_id)
            .await
            .unwrap();
        let draft = repo
            .create_config_revision(agent_id, configuration(), owner.id)
            .await
            .unwrap();
        repo.validate_config_revision(agent_id, draft.revision, vec![])
            .await
            .unwrap();
        repo.request_config_activation(agent_id, draft.revision, owner.id)
            .await
            .unwrap();
        assert!(repo.agent_is_draining(agent_id).await.unwrap());
        assert!(repo.claim_config_activation().await.unwrap().is_none());
        let control_id = Uuid::new_v4();
        let insert = |operation: &str| {
            Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO runtime_control_commands(id,session_id,session_run_id,agent_id,actor_user_id,operation,idempotency_key,
                payload_sha256,runtime_run_id,runtime_session_id,original_request_sha256,api_origin,credential_fingerprint)
             SELECT $1,$2,$3,$4,$5,$6,$7,repeat('b',64),'run_stop_migration','effective-legacy-session',
                encode(sha256(convert_to(intent->>'request_body','UTF8')),'hex'),
                intent->>'origin',intent->>'credential_fingerprint' FROM pm_dispatch_journal WHERE session_run_id=$3",
            [control_id.into(),session.id.into(),reservation.session_run_id.into(),agent_id.into(),
                owner.id.into(),operation.into(),"drain-stop".into()])
        };
        // Delivered guidance isolates the drain predicate in the original025 trigger.
        let error = db.execute(insert("stop")).await.unwrap_err().to_string();
        assert!(error.contains("runtime control requires original admitted dispatch custody"));
        assert!(
            repo.list_runtime_controls(session.id, reservation.session_run_id)
                .await
                .unwrap()
                .is_empty()
        );
        let custody = legacy_history_snapshot(&db).await;
        let old_predicate =
            "NOT EXISTS(SELECT 1 FROM agent_config_heads WHERE agent_id=b.agent_id AND draining)";
        let new_predicate = "(NEW.operation='stop' OR NOT EXISTS(SELECT 1 FROM agent_config_heads WHERE agent_id=b.agent_id AND draining))";
        assert_eq!(original_body.matches(old_predicate).count(), 1);
        for _ in 0..2 {
            migration::Migrator::up(&db, Some(1)).await.unwrap();
            let upgraded = migration::Migrator::get_migration_models(&db)
                .await
                .unwrap()
                .into_iter()
                .map(|r| (r.version, r.applied_at))
                .collect::<Vec<_>>();
            assert_eq!(&upgraded[..prefix as usize], &versions);
            assert_eq!(upgraded.len(), prefix as usize + 1);
            assert_eq!(upgraded.last().unwrap().0, "m20261011_000026_pm_stop_drain");
            let active = db
                .query_one(Statement::from_string(
                    DatabaseBackend::Postgres,
                    definition_sql.to_owned(),
                ))
                .await
                .unwrap()
                .unwrap();
            assert_eq!(
                active.try_get::<String>("", "body").unwrap(),
                original_body.replace(old_predicate, new_predicate)
            );
            let saved = db.query_one(Statement::from_string(DatabaseBackend::Postgres,
                "SELECT oid::bigint AS id,prosrc AS body FROM pg_proc WHERE oid='admit_runtime_control_custody_v25()'::regprocedure".to_owned())).await.unwrap().unwrap();
            assert_eq!(saved.try_get::<i64>("", "id").unwrap(), original_id);
            assert_eq!(saved.try_get::<String>("", "body").unwrap(), original_body);
            migration::Migrator::down(&db, Some(1)).await.unwrap();
            let restored = db
                .query_one(Statement::from_string(
                    DatabaseBackend::Postgres,
                    definition_sql.to_owned(),
                ))
                .await
                .unwrap()
                .unwrap();
            assert_eq!(restored.try_get::<i64>("", "id").unwrap(), original_id);
            for column in ["body", "definition", "trigger_definition"] {
                assert_eq!(
                    restored.try_get::<String>("", column).unwrap(),
                    original.try_get::<String>("", column).unwrap()
                );
            }
            assert_eq!(
                migration::Migrator::get_migration_models(&db)
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|r| (r.version, r.applied_at))
                    .collect::<Vec<_>>(),
                versions
            );
            assert_eq!(legacy_history_snapshot(&db).await, custody);
            assert!(db.execute(insert("stop")).await.is_err());
        }
        migration::Migrator::up(&db, Some(1)).await.unwrap();
        assert!(
            db.execute(insert("steer"))
                .await
                .unwrap_err()
                .to_string()
                .contains("runtime control requires original admitted dispatch custody")
        );
        db.execute(insert("stop")).await.unwrap();
        assert_eq!(
            repo.list_runtime_controls(session.id, reservation.session_run_id)
                .await
                .unwrap()
                .len(),
            1
        );
        // Clearing drain after terminal proof cannot make retained Stop history safe to downgrade.
        repo.observe_pm_run(reservation.session_run_id, domain::PmRuntimeStatus::Stopped)
            .await
            .unwrap();
        let claimed = repo.claim_config_activation().await.unwrap().unwrap();
        assert_eq!(claimed.agent_id, agent_id);
        assert_eq!(claimed.revision, draft.revision);
        repo.finish_config_activation(agent_id, draft.revision, None, true)
            .await
            .unwrap();
        assert!(!repo.agent_is_draining(agent_id).await.unwrap());
        let before = migration::Migrator::get_migration_models(&db)
            .await
            .unwrap()
            .into_iter()
            .map(|r| (r.version, r.applied_at))
            .collect::<Vec<_>>();
        let data = legacy_history_snapshot(&db).await;
        let controls = serde_json::to_value(
            repo.list_runtime_controls(session.id, reservation.session_run_id)
                .await
                .unwrap(),
        )
        .unwrap();
        let active = db
            .query_one(Statement::from_string(
                DatabaseBackend::Postgres,
                definition_sql.to_owned(),
            ))
            .await
            .unwrap()
            .unwrap();
        let error = migration::Migrator::down(&db, Some(1))
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("PM stop custody prevents drain prerequisite downgrade"));
        assert_eq!(
            migration::Migrator::get_migration_models(&db)
                .await
                .unwrap()
                .into_iter()
                .map(|r| (r.version, r.applied_at))
                .collect::<Vec<_>>(),
            before
        );
        assert_eq!(legacy_history_snapshot(&db).await, data);
        assert_eq!(
            serde_json::to_value(
                repo.list_runtime_controls(session.id, reservation.session_run_id)
                    .await
                    .unwrap()
            )
            .unwrap(),
            controls
        );
        let unchanged = db
            .query_one(Statement::from_string(
                DatabaseBackend::Postgres,
                definition_sql.to_owned(),
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            unchanged.try_get::<i64>("", "id").unwrap(),
            active.try_get::<i64>("", "id").unwrap()
        );
        for column in ["body", "definition", "trigger_definition"] {
            assert_eq!(
                unchanged.try_get::<String>("", column).unwrap(),
                active.try_get::<String>("", column).unwrap()
            );
        }
        drop(repo);
        repo_pool.close().await;
        db.close().await.unwrap();
        admin
            .execute_unprepared(&format!("DROP SCHEMA {schema} CASCADE"))
            .await
            .unwrap();
        admin.close().await.unwrap();
    }
}

async fn complete_legacy_tool(
    repo: &PostgresFleetRepository,
    db: &sea_orm::DatabaseConnection,
    run: Uuid,
    key: &str,
    request: Value,
    result: Option<Value>,
) {
    repo.prepare_pm_tool(domain::PmToolCommand {
        session_run_id: run,
        key: key.into(),
        kind: "question".into(),
        request,
        result: None,
        attempted: false,
    })
    .await
    .unwrap();
    // Historical pre-023 publication custody predates instruction admission. Do not fabricate
    // an instruction receipt or use today's claim path to manufacture that old history.
    assert_eq!(db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE pm_tool_commands SET attempted=true WHERE session_run_id=$1 AND operation_key=$2 AND NOT attempted",
        [run.into(),key.into()])).await.unwrap().rows_affected(),1);
    if let Some(result) = result {
        repo.finish_pm_tool(run, key, result).await.unwrap();
    }
}

async fn legacy_history_snapshot(db: &sea_orm::DatabaseConnection) -> Vec<Value> {
    db.query_all(Statement::from_string(DatabaseBackend::Postgres,
        "SELECT snapshot FROM (SELECT to_jsonb(c)-'continuation_state' AS snapshot FROM clarification_answer_commands c
         UNION ALL SELECT to_jsonb(b) FROM pm_run_bindings b
         UNION ALL SELECT to_jsonb(j) FROM pm_dispatch_journal j
         UNION ALL SELECT to_jsonb(t) FROM pm_tool_commands t) history ORDER BY snapshot::text".to_owned())).await.unwrap()
        .into_iter().map(|r| r.try_get("", "snapshot").unwrap()).collect()
}

fn legacy_question(
    reservation: &domain::PmRunReservation,
    id: Uuid,
    checkpoint: Uuid,
    answer: Value,
) -> (Value, Value) {
    let identity = &reservation.identity;
    let request = json!({"fence":{"assignment_id":identity.assignment_ref,"execution_id":identity.execution_ref,
        "agent_id":identity.agent_ref,"assignment_version":identity.assignment_revision},
        "request_id":Uuid::new_v4(),"question_id":id,"expected_question_version":null,"requirement_revision":2,
        "checkpoint_id":checkpoint,"requirement_reference":null,"text":"Scoped question","rationale":"Owner input",
        "required":true,"mode":"text","options":[],"recommended_option_id":null,"idempotency_key":format!("question:{id}")});
    let result = json!({"id":id,"request_id":request["request_id"],"task_id":identity.task_ref,"root_task_id":identity.root_ref,
        "version":1,"requirement_revision":2,"requirement_reference":null,"assignment_id":identity.assignment_ref,
        "execution_id":identity.execution_ref,"agent_id":identity.agent_ref,"assignment_version":identity.assignment_revision,
        "checkpoint_id":checkpoint,"text":"Scoped question","rationale":"Owner input","required":true,"mode":"text",
        "options":[],"recommended_option_id":null,"state":if answer.is_null() {"open"} else {"answered"},"answer":answer,
        "author_subject":"fixture-machine","created_at":"2026-10-10T00:00:00Z"});
    serde_json::from_value::<domain::PmPublishQuestion>(request.clone()).unwrap();
    serde_json::from_value::<domain::TrackerQuestion>(result.clone()).unwrap();
    (request, result)
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
