use super::*;
use app::pm_draft::{PmDraftTracker, continue_creation};
use async_trait::async_trait;
use domain::*;
use shared::AppError;
use tokio::sync::Mutex;

#[derive(Default)]
struct RemoteState {
    draft: Option<TrackerCreatedDraft>,
    input: Option<TrackerDraftInputReceipt>,
    reservation: Option<TrackerPmDraftReservation>,
    created: usize,
    reserved: usize,
    lose_creation_response: bool,
    lose_reservation_response: bool,
    stale: bool,
    corrupt_input: bool,
}

#[derive(Default)]
struct Tracker {
    state: Mutex<RemoteState>,
}

#[async_trait]
impl PmDraftTracker for Tracker {
    async fn find_draft(
        &self,
        _: &PmDraftOperation,
    ) -> Result<Option<TrackerCreatedDraft>, AppError> {
        Ok(self.state.lock().await.draft.clone())
    }
    async fn create_draft(&self, op: &PmDraftOperation) -> Result<TrackerCreatedDraft, AppError> {
        let mut state = self.state.lock().await;
        if state.draft.is_none() {
            let task = Uuid::new_v4();
            state.draft = Some(TrackerCreatedDraft {
                tracker_instance_id: op.tracker_instance_id.clone(),
                project_id: op.project_id,
                task_id: task,
                root_task_id: task,
                task_key: "PM-1".into(),
                owner_subject: op.owner_subject.clone(),
                stage: "Draft".into(),
            });
            state.input = Some(TrackerDraftInputReceipt {
                contract_version: 1,
                tracker_instance_id: op.tracker_instance_id.clone(),
                project_id: op.project_id,
                task_id: task,
                root_task_id: task,
                owner_subject: op.owner_subject.clone(),
                input: TrackerDraftInput {
                    snapshot_ref: Uuid::new_v4(),
                    title: op.request.title.clone(),
                    description: op.request.description.clone(),
                    sha256: pm_input_hash(&op.request.title, &op.request.description),
                },
            });
            state.created += 1;
        }
        if std::mem::take(&mut state.lose_creation_response) {
            return Err(AppError::Unavailable(
                "simulated unknown creation acceptance".into(),
            ));
        }
        Ok(state.draft.clone().unwrap())
    }
    async fn original_input(
        &self,
        _: &PmDraftOperation,
    ) -> Result<TrackerDraftInputReceipt, AppError> {
        let state = self.state.lock().await;
        let mut input = state.input.clone().unwrap();
        if state.corrupt_input {
            input.input.description.push_str("changed");
        }
        Ok(input)
    }
    async fn reservation(
        &self,
        op: &PmDraftOperation,
    ) -> Result<TrackerDraftReservationReadback, AppError> {
        let state = self.state.lock().await;
        let operation = state.reservation.clone().map(|result| TrackerDraftReservationOperation {
            idempotency_key: op.reservation_key(),
            request_sha256: pm_canonical_hash(&serde_json::json!({"operation":"reserve_pm_draft","payload":op.reservation_body()})),
            result,
        });
        let mut current = state.reservation.clone();
        if state.stale
            && let Some(current) = &mut current
        {
            current.assignment.assignment_id = Uuid::new_v4();
        }
        Ok(TrackerDraftReservationReadback {
            contract_version: 1,
            binding: op.identity()?,
            owner_version: if current.is_some() { 1 } else { 0 },
            current,
            operation,
        })
    }
    async fn reserve_pm(
        &self,
        op: &PmDraftOperation,
    ) -> Result<TrackerPmDraftReservation, AppError> {
        let mut state = self.state.lock().await;
        if state.reservation.is_none() {
            let assignment_id = Uuid::new_v4();
            let input = state.input.as_ref().unwrap().input.clone();
            state.reservation = Some(TrackerPmDraftReservation {
                contract_version: 1,
                variant: "pm_draft_reserved".into(),
                binding: op.identity()?,
                owner_cas: TrackerOwnerCas {
                    expected_version: 0,
                    version: 1,
                },
                assignment: TrackerDraftAssignment {
                    assignment_id,
                    execution_id: Uuid::new_v4(),
                    agent_id: op.request.agent_id,
                    version: 1,
                    machine_subject: "fleet-test-orchestrator".into(),
                },
                execution: TrackerDraftExecution {
                    ordinal: "41".into(),
                    key: "SDLC-41".into(),
                },
                input: TrackerDraftInputRef {
                    snapshot_ref: input.snapshot_ref,
                    sha256: input.sha256,
                },
                assignment_operation_key: format!("pm-draft:{assignment_id}"),
                admission_state: "reserved".into(),
                dispatch_allowed: false,
            });
            state.reserved += 1;
        }
        if std::mem::take(&mut state.lose_reservation_response) {
            return Err(AppError::Unavailable(
                "simulated unknown reservation acceptance".into(),
            ));
        }
        Ok(state.reservation.clone().unwrap())
    }
}

async fn operation() -> Option<(PostgresFleetRepository, PmDraftOperation)> {
    let (repo, pm) = pm_fixture().await?;
    let session = repo.get_session(pm.session_id).await.unwrap();
    let binding = repo
        .get_task_chat_binding(pm.session_id)
        .await
        .unwrap()
        .unwrap();
    let operation = PmDraftOperation {
        id: Uuid::new_v4(),
        owner_user_id: session.user_id,
        owner_subject: binding.owner_subject,
        tracker_instance_id: binding.tracker_instance_id,
        project_id: binding.project_id,
        request: CreatePmDraftRequest {
            agent_id: binding.agent_id,
            title: "PM creation".into(),
            description: "Original input\nwith UTF-8: проверка".into(),
            idempotency_key: "pm-create".into(),
        },
        draft: None,
        input: None,
        reservation: None,
        session_id: None,
    };
    Some((repo, operation))
}

#[tokio::test]
async fn lost_remote_responses_recover_without_second_draft_reservation_or_run() {
    let Some((repo, candidate)) = operation().await else {
        return;
    };
    let tracker = Tracker::default();
    {
        let mut state = tracker.state.lock().await;
        state.lose_creation_response = true;
        state.lose_reservation_response = true;
    }
    let mut operation = repo
        .reserve_pm_draft_operation(candidate.clone())
        .await
        .unwrap();
    assert!(
        continue_creation(&repo, &tracker, operation.clone())
            .await
            .is_err()
    );
    operation = repo
        .read_pm_draft_operation(operation.id, operation.owner_user_id)
        .await
        .unwrap();
    assert!(operation.draft.is_none());
    assert!(
        continue_creation(&repo, &tracker, operation.clone())
            .await
            .is_err()
    );
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let restarted = PostgresFleetRepository::new(
        sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
            .await
            .unwrap(),
    );
    operation = restarted
        .read_pm_draft_operation(operation.id, operation.owner_user_id)
        .await
        .unwrap();
    assert!(
        operation.draft.is_some() && operation.input.is_some() && operation.reservation.is_none()
    );
    let result = continue_creation(&restarted, &tracker, operation)
        .await
        .unwrap();
    assert!(matches!(
        result.state,
        PmDraftCreationState::AwaitingAdmission
    ));
    assert!(!result.dispatch_allowed);
    let mut same_key = candidate;
    same_key.id = Uuid::new_v4();
    let replay = repo
        .reserve_pm_draft_operation(same_key.clone())
        .await
        .unwrap();
    assert_eq!(replay.id, result.operation_id);
    assert_eq!(
        continue_creation(&repo, &tracker, replay.clone())
            .await
            .unwrap()
            .session_id,
        result.session_id
    );
    same_key.request.description.push('!');
    assert!(matches!(
        repo.reserve_pm_draft_operation(same_key).await,
        Err(AppError::Conflict(_))
    ));
    let state = tracker.state.lock().await;
    assert_eq!((state.created, state.reserved), (1, 1));
    drop(state);
    let session = result.session_id.unwrap();
    assert!(
        repo.list_session_messages(session)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        repo.list_session_agent_runs(session)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(!repo.has_pending_session_dispatch(session).await.unwrap());
    assert!(matches!(
        repo.create_session_message(session, prompt("not-admitted"), replay.owner_user_id)
            .await,
        Err(AppError::Conflict(_))
    ));
    assert!(
        repo.read_pm_draft_operation(result.operation_id, Uuid::new_v4())
            .await
            .is_err()
    );
    assert!(db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE pm_draft_creation_operations SET operation=jsonb_set(operation,'{reservation}','null') WHERE id=$1",
        [result.operation_id.into()])).await.is_err());
}

#[tokio::test]
async fn concurrent_coordinators_share_one_persisted_operation_and_atomic_chat() {
    let Some((repo, candidate)) = operation().await else {
        return;
    };
    let repo = Arc::new(repo);
    let tracker = Arc::new(Tracker::default());
    let mut tasks = Vec::new();
    for _ in 0..6 {
        let repo = repo.clone();
        let tracker = tracker.clone();
        let mut candidate = candidate.clone();
        candidate.id = Uuid::new_v4();
        tasks.push(tokio::spawn(async move {
            let saved = repo.reserve_pm_draft_operation(candidate).await.unwrap();
            continue_creation(repo.as_ref(), tracker.as_ref(), saved)
                .await
                .unwrap()
        }));
    }
    let mut results = Vec::new();
    for task in tasks {
        results.push(task.await.unwrap());
    }
    assert!(
        results
            .iter()
            .all(|result| result.operation_id == results[0].operation_id
                && result.session_id == results[0].session_id)
    );
    let state = tracker.state.lock().await;
    assert_eq!((state.created, state.reserved), (1, 1));
    let session = results[0].session_id.unwrap();
    assert_eq!(
        repo.list_session_participants(session).await.unwrap().len(),
        2
    );
    assert_eq!(
        repo.list_session_events(session, 0)
            .await
            .unwrap()
            .iter()
            .filter(|event| event.event_type == "task.bound")
            .count(),
        1
    );
}

#[tokio::test]
async fn input_integrity_and_stale_current_assignment_block_chat_creation() {
    let Some((repo, candidate)) = operation().await else {
        return;
    };
    let tracker = Tracker::default();
    tracker.state.lock().await.corrupt_input = true;
    let saved = repo.reserve_pm_draft_operation(candidate).await.unwrap();
    assert!(
        continue_creation(&repo, &tracker, saved.clone())
            .await
            .is_err()
    );
    assert_eq!(tracker.state.lock().await.reserved, 0);
    {
        let mut state = tracker.state.lock().await;
        state.corrupt_input = false;
        state.lose_reservation_response = true;
    }
    let saved = repo
        .read_pm_draft_operation(saved.id, saved.owner_user_id)
        .await
        .unwrap();
    assert!(
        continue_creation(&repo, &tracker, saved.clone())
            .await
            .is_err()
    );
    tracker.state.lock().await.stale = true;
    let saved = repo
        .read_pm_draft_operation(saved.id, saved.owner_user_id)
        .await
        .unwrap();
    assert!(matches!(
        continue_creation(&repo, &tracker, saved.clone()).await,
        Err(AppError::Conflict(_))
    ));
    let saved = repo
        .read_pm_draft_operation(saved.id, saved.owner_user_id)
        .await
        .unwrap();
    assert!(saved.session_id.is_none() && saved.reservation.is_none());
    assert!(
        repo.record_pm_draft_proof(
            saved.id,
            saved.owner_user_id,
            PmDraftProof::Chat(Uuid::new_v4())
        )
        .await
        .is_err()
    );
}

#[tokio::test]
async fn public_creation_and_readback_require_human_owner_and_fresh_project_access() {
    use api::middleware::{CurrentUser, VerifiedCentralSubject, VerifiedHumanSession};
    use axum::http::{HeaderMap, StatusCode, Uri, header};
    let Some((repo, candidate)) = operation().await else {
        return;
    };
    let repo = Arc::new(repo);
    let tracker = Tracker::default();
    let saved = repo
        .reserve_pm_draft_operation(candidate.clone())
        .await
        .unwrap();
    let result = continue_creation(repo.as_ref(), &tracker, saved)
        .await
        .unwrap();
    let saved = repo
        .read_pm_draft_operation(result.operation_id, candidate.owner_user_id)
        .await
        .unwrap();
    let readback = tracker.reservation(&saved).await.unwrap();
    let project = saved.project_id;
    let instance = saved.tracker_instance_id.clone();
    let revoked = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let calls = Arc::new(AtomicUsize::new(0));
    let state = (saved.clone(), readback, revoked.clone(), calls.clone());
    let upstream = axum::Router::new().fallback(move |uri: Uri, headers: HeaderMap| {
        let (saved, readback, revoked, calls) = state.clone();
        let instance = instance.clone();
        async move {
            assert_eq!(headers[header::AUTHORIZATION], "Bearer test-only-owner-session");
            calls.fetch_add(1, Ordering::SeqCst);
            if uri.path() == "/api/v1/sdlc/project-access" {
                let projects = if revoked.load(Ordering::SeqCst) { vec![] } else { vec![project] };
                return axum::Json(serde_json::json!({"contract_version":1,"tracker_instance_id":instance,"project_ids":projects}));
            }
            if uri.path().ends_with(&saved.creation_key()) {
                return axum::Json(serde_json::to_value(saved.draft).unwrap());
            }
            if uri.path().ends_with("/pm-draft-input") {
                return axum::Json(serde_json::to_value(saved.input).unwrap());
            }
            assert!(uri.path().ends_with("/pm-draft-assignment"));
            let parsed = reqwest::Url::parse(&format!("http://tracker{uri}")).unwrap();
            assert_eq!(parsed.query_pairs().collect::<Vec<_>>(), vec![("idempotency_key".into(), saved.reservation_key().into())]);
            axum::Json(serde_json::to_value(readback).unwrap())
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let upstream_server =
        tokio::spawn(async move { axum::serve(listener, upstream).await.unwrap() });
    let mut config = AppConfig::default();
    config.tracker.url = url;
    config.tracker.instance_id = saved.tracker_instance_id.clone();
    config.tracker.pm_draft_creation_enabled = true;
    config.tracker.pm_draft_project_ids = vec![project];
    let config = Arc::new(config);
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime = Arc::new(infra::runtime::LocalRuntimeSupervisor::new(
        config.clone(),
        repo.clone(),
        events.clone(),
    ));
    let (restart_tx, _) = tokio::sync::mpsc::channel(1);
    let ctx = Arc::new(app::AppContext::new(
        config,
        repo.clone(),
        Arc::new(infra::FilesystemProvisioner),
        runtime,
        events,
        restart_tx,
    ));
    let routes = axum::Router::new()
        .route(
            "/api/v1/projects/{project_id}/pm-drafts",
            axum::routing::post(api::routes::pm_drafts::create),
        )
        .route(
            "/api/v1/pm-drafts/operations/{operation_id}",
            axum::routing::get(api::routes::pm_drafts::read),
        );
    let identity = CurrentUser {
        id: saved.owner_user_id,
        role: SystemRole::User,
        is_system_admin: false,
    };
    let central = VerifiedCentralSubject(saved.owner_subject.clone());
    let owner = routes
        .clone()
        .layer(axum::Extension(identity.clone()))
        .layer(axum::Extension(central.clone()))
        .layer(axum::Extension(VerifiedHumanSession));
    let machine = routes
        .clone()
        .layer(axum::Extension(identity.clone()))
        .layer(axum::Extension(central));
    let local = routes
        .clone()
        .layer(axum::Extension(identity))
        .layer(axum::Extension(VerifiedHumanSession));
    let other_subject = Uuid::new_v4().to_string();
    let other = repo
        .find_or_create_central_user(
            &other_subject,
            &format!("{}@example.test", Uuid::new_v4()),
            "Other operator",
        )
        .await
        .unwrap();
    let operator = routes
        .layer(axum::Extension(CurrentUser {
            id: other.id,
            role: SystemRole::Admin,
            is_system_admin: true,
        }))
        .layer(axum::Extension(VerifiedCentralSubject(other_subject)))
        .layer(axum::Extension(VerifiedHumanSession));
    let router = axum::Router::new()
        .nest("/owner", owner)
        .nest("/machine", machine)
        .nest("/local", local)
        .nest("/operator", operator)
        .with_state(ctx);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let client = reqwest::Client::new();
    for prefix in ["machine", "local"] {
        let response = client
            .post(format!(
                "{url}/{prefix}/api/v1/projects/{project}/pm-drafts"
            ))
            .bearer_auth("test-only-owner-session")
            .json(&candidate.request)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    let response = client
        .get(format!(
            "{url}/operator/api/v1/pm-drafts/operations/{}",
            saved.id
        ))
        .bearer_auth("test-only-owner-session")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let response = client
        .post(format!("{url}/owner/api/v1/projects/{project}/pm-drafts"))
        .bearer_auth("test-only-owner-session")
        .json(&candidate.request)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let value: serde_json::Value = response.json().await.unwrap();
    assert_eq!(value["operation_id"], saved.id.to_string());
    assert_eq!(value["session_id"], result.session_id.unwrap().to_string());
    assert_eq!(value["state"], "awaiting_admission");
    assert_eq!(value["next_step"], "admission");
    assert_eq!(value["dispatch_allowed"], false);
    assert!(value.get("description").is_none() && value.get("machine_subject").is_none());
    assert_eq!(calls.load(Ordering::SeqCst), 4);
    let response = client
        .get(format!(
            "{url}/owner/api/v1/pm-drafts/operations/{}",
            saved.id
        ))
        .bearer_auth("test-only-owner-session")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    revoked.store(true, Ordering::SeqCst);
    let response = client
        .get(format!(
            "{url}/owner/api/v1/pm-drafts/operations/{}",
            saved.id
        ))
        .bearer_auth("test-only-owner-session")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = client
        .post(format!("{url}/owner/api/v1/projects/{project}/pm-drafts"))
        .bearer_auth("test-only-owner-session")
        .json(&candidate.request)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(calls.load(Ordering::SeqCst), 7);
    assert!(
        repo.list_session_agent_runs(result.session_id.unwrap())
            .await
            .unwrap()
            .is_empty()
    );
    server.abort();
    upstream_server.abort();
    let _ = server.await;
    let _ = upstream_server.await;
}
