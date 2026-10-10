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
    namespace_revoked: bool,
    namespace_reads: usize,
    calls: Vec<&'static str>,
    stale_after_reserve: bool,
    machine_subject: Option<String>,
}

#[derive(Default)]
struct Tracker {
    state: Mutex<RemoteState>,
}

#[async_trait]
impl PmDraftTracker for Tracker {
    async fn verify_namespace(&self, op: &PmDraftOperation, agent: &Agent) -> Result<(), AppError> {
        assert_eq!(agent.id, op.request.agent_id);
        let mut state = self.state.lock().await;
        state.namespace_reads += 1;
        state.calls.push("namespace");
        if state.namespace_revoked {
            return Err(AppError::Unavailable(
                "controlled namespace revocation".into(),
            ));
        }
        Ok(())
    }
    async fn find_draft(
        &self,
        _: &PmDraftOperation,
    ) -> Result<Option<TrackerCreatedDraft>, AppError> {
        let mut state = self.state.lock().await;
        state.calls.push("find");
        Ok(state.draft.clone())
    }
    async fn create_draft(&self, op: &PmDraftOperation) -> Result<TrackerCreatedDraft, AppError> {
        let mut state = self.state.lock().await;
        state.calls.push("create");
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
        let mut state = self.state.lock().await;
        state.calls.push("input");
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
        let mut state = self.state.lock().await;
        state.calls.push("read");
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
        state.calls.push("reserve");
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
                    machine_subject: state
                        .machine_subject
                        .clone()
                        .unwrap_or_else(|| "fleet-test-orchestrator".into()),
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
        state.stale |= state.stale_after_reserve;
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
        credentials: None,
    };
    sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap()
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE agents SET namespace_id='7' WHERE id=$1",
            [operation.request.agent_id.into()],
        ))
        .await
        .unwrap();
    Some((repo, operation))
}

pub(super) async fn credential_fixture()
-> Option<(PostgresFleetRepository, PmDraftOperation, String)> {
    let (repo, candidate) = operation().await?;
    let subject = Uuid::new_v4().to_string();
    let tracker = Tracker::default();
    tracker.state.lock().await.machine_subject = Some(subject.clone());
    let operation = repo.reserve_pm_draft_operation(candidate).await.unwrap();
    continue_creation(&repo, &tracker, operation.clone())
        .await
        .unwrap();
    let operation = repo
        .read_pm_draft_operation(operation.id, operation.owner_user_id)
        .await
        .unwrap();
    Some((repo, operation, subject))
}

pub(super) async fn continue_with_credentials(
    repo: &PostgresFleetRepository,
    op: PmDraftOperation,
    credentials: &dyn app::pm_draft::PmDraftCredentials,
) -> Result<PmDraftCreationResponse, AppError> {
    let tracker = Tracker {
        state: Mutex::new(RemoteState {
            draft: op.draft.clone(),
            input: op.input.clone(),
            reservation: op.reservation.clone(),
            ..Default::default()
        }),
    };
    app::pm_draft::continue_creation_with_credentials(repo, &tracker, op, Some(credentials)).await
}

#[tokio::test]
async fn namespace_revocation_blocks_remote_mutations_and_replayed_creation() {
    let Some((repo, candidate)) = operation().await else {
        return;
    };
    let tracker = Tracker::default();
    tracker.state.lock().await.namespace_revoked = true;
    let saved = repo.reserve_pm_draft_operation(candidate).await.unwrap();
    let recovered = repo
        .read_pm_draft_operation_by_key(saved.owner_user_id, &saved.request.idempotency_key)
        .await
        .unwrap();
    assert_eq!(recovered.id, saved.id);
    assert!(recovered.draft.is_none() && recovered.session_id.is_none());
    assert!(matches!(
        repo.read_pm_draft_operation_by_key(Uuid::new_v4(), &saved.request.idempotency_key)
            .await,
        Err(shared::AppError::NotFound { .. })
    ));
    assert!(
        continue_creation(&repo, &tracker, saved.clone())
            .await
            .is_err()
    );
    assert_eq!(tracker.state.lock().await.created, 0);
    assert_eq!(tracker.state.lock().await.calls, ["namespace"]);
    assert!(
        repo.read_pm_draft_operation(saved.id, saved.owner_user_id)
            .await
            .unwrap()
            .draft
            .is_none()
    );
    tracker.state.lock().await.namespace_revoked = false;
    let response = continue_creation(&repo, &tracker, saved.clone())
        .await
        .unwrap();
    let replay = repo
        .read_pm_draft_operation(saved.id, saved.owner_user_id)
        .await
        .unwrap();
    let before_denial = tracker.state.lock().await.calls.len();
    tracker.state.lock().await.namespace_revoked = true;
    assert!(continue_creation(&repo, &tracker, replay).await.is_err());
    let state = tracker.state.lock().await;
    assert_eq!(
        (state.created, state.reserved, state.namespace_reads),
        (1, 1, 3)
    );
    assert_eq!(&state.calls[before_denial..], ["namespace"]);
    assert!(
        repo.list_session_agent_runs(response.session_id.unwrap())
            .await
            .unwrap()
            .is_empty()
    );
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
    let recovered = restarted
        .read_pm_draft_operation_by_key(candidate.owner_user_id, &candidate.request.idempotency_key)
        .await
        .unwrap();
    assert_eq!(recovered.id, operation.id);
    assert!(recovered.draft.is_some() && recovered.reservation.is_none());
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
    assert_eq!(
        state.calls,
        [
            "namespace",
            "find",
            "create",
            "namespace",
            "find",
            "input",
            "read",
            "reserve",
            "namespace",
            "find",
            "input",
            "read",
            "namespace",
            "find",
            "input",
            "read",
        ]
    );
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
    let (repo, candidate) = operation().await.unwrap();
    let tracker = Tracker::default();
    tracker.state.lock().await.stale_after_reserve = true;
    let saved = repo.reserve_pm_draft_operation(candidate).await.unwrap();
    assert!(matches!(
        continue_creation(&repo, &tracker, saved.clone()).await,
        Err(AppError::Conflict(_))
    ));
    let proof = repo
        .read_pm_draft_operation(saved.id, saved.owner_user_id)
        .await
        .unwrap();
    assert!(proof.reservation.is_some() && proof.session_id.is_none());
    assert_eq!(
        tracker.state.lock().await.calls,
        [
            "namespace",
            "find",
            "create",
            "input",
            "read",
            "reserve",
            "read"
        ]
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
            calls.fetch_add(1, Ordering::SeqCst);
            if uri.path() == "/api/pm/namespace-ownership/7" {
                assert_eq!(headers[header::AUTHORIZATION], "Bearer sdlc_pat_test_only_namespace_read_credential");
                assert_eq!(headers[header::CACHE_CONTROL], "no-cache, no-store");
                return axum::Json(serde_json::json!({"ok":true,"result":{
                    "contract_version":1,"ownership_ref":"11111111-1111-4111-8111-111111111111",
                    "namespace_id":7,"tracker_instance_ref":instance,"tracker_project_ref":project,
                    "authority_issuer":"http://authority.example.test",
                    "provisioner_subject":"22222222-2222-4222-8222-222222222222",
                    "created_at":"2026-10-02T00:00:00Z"}}));
            }
            assert_eq!(headers[header::AUTHORIZATION], "Bearer test-only-owner-session");
            if uri.path() == "/api/v1/sdlc/project-directory" {
                let projects = if revoked.load(Ordering::SeqCst) { vec![] } else { vec![serde_json::json!({"id":project,"key":"PM","name":"Owner project"})] };
                return axum::Json(serde_json::json!({"contract_version":1,"tracker_instance_id":instance,"projects":projects,"next_cursor":null}));
            }
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
    config.tracker.url = url.clone();
    config.fleet.project_workflow_url = Some(url);
    config.pm.namespace_read_pat = "sdlc_pat_test_only_namespace_read_credential".into();
    config.pm.namespace_authority_issuer = "http://authority.example.test".into();
    config.pm.namespace_provisioner_subject = "22222222-2222-4222-8222-222222222222".into();
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
            "/api/v1/pm-drafts/projects",
            axum::routing::get(api::routes::pm_drafts::projects),
        )
        .route(
            "/api/v1/projects/{project_id}/pm-drafts",
            axum::routing::post(api::routes::pm_drafts::create),
        )
        .route(
            "/api/v1/pm-drafts/operations/{operation_id}",
            axum::routing::get(api::routes::pm_drafts::read),
        )
        .route(
            "/api/v1/projects/{project_id}/pm-drafts/operation",
            axum::routing::get(api::routes::pm_drafts::read_by_key),
        )
        .route(
            "/api/v1/pm-drafts/operations/{operation_id}/continue",
            axum::routing::post(api::routes::pm_drafts::continue_operation),
        );
    let identity = CurrentUser {
        id: saved.owner_user_id,
        role: SystemRole::User,
        is_system_admin: false,
        central_write: None,
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
            central_write: None,
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
    assert_eq!(calls.load(Ordering::SeqCst), 5);
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
    assert_eq!(calls.load(Ordering::SeqCst), 8);
    assert!(
        repo.list_session_agent_runs(result.session_id.unwrap())
            .await
            .unwrap()
            .is_empty()
    );
    let lookup = format!("{url}/owner/api/v1/projects/{project}/pm-drafts/operation");
    let key = candidate.request.idempotency_key.as_str();
    let response = client
        .get(&lookup)
        .query(&[("idempotency_key", key)])
        .bearer_auth("test-only-owner-session")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    revoked.store(false, Ordering::SeqCst);
    for (prefix, status) in [
        ("owner", StatusCode::OK),
        ("operator", StatusCode::NOT_FOUND),
        ("machine", StatusCode::UNAUTHORIZED),
        ("local", StatusCode::UNAUTHORIZED),
    ] {
        let response = client
            .get(format!(
                "{url}/{prefix}/api/v1/projects/{project}/pm-drafts/operation"
            ))
            .query(&[("idempotency_key", key)])
            .bearer_auth("test-only-owner-session")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), status, "{prefix}");
        if status == StatusCode::OK {
            assert_eq!(response.json::<serde_json::Value>().await.unwrap(), value);
        }
    }
    for (query, status) in [
        (
            vec![("idempotency_key", "")],
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            vec![("idempotency_key", "bad key")],
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            vec![("idempotency_key", key), ("unexpected", "yes")],
            StatusCode::BAD_REQUEST,
        ),
    ] {
        assert_eq!(
            client
                .get(&lookup)
                .query(&query)
                .bearer_auth("test-only-owner-session")
                .send()
                .await
                .unwrap()
                .status(),
            status
        );
    }
    assert_eq!(
        client
            .get(&lookup)
            .query(&[("idempotency_key", "missing")])
            .bearer_auth("test-only-owner-session")
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    let recovered = repo
        .read_pm_draft_operation_by_key(saved.owner_user_id, key)
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(recovered.response()).unwrap(),
        serde_json::to_value(result).unwrap()
    );
    for (prefix, status) in [
        ("owner", StatusCode::ACCEPTED),
        ("operator", StatusCode::NOT_FOUND),
        ("machine", StatusCode::UNAUTHORIZED),
        ("local", StatusCode::UNAUTHORIZED),
    ] {
        let response = client
            .post(format!(
                "{url}/{prefix}/api/v1/pm-drafts/operations/{}/continue",
                saved.id
            ))
            .json(&serde_json::json!({}))
            .bearer_auth("test-only-owner-session")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), status, "{prefix}");
        if status == StatusCode::ACCEPTED {
            assert_eq!(response.json::<serde_json::Value>().await.unwrap(), value);
        }
    }
    let continuation = format!(
        "{url}/owner/api/v1/pm-drafts/operations/{}/continue",
        saved.id
    );
    for body in [
        serde_json::json!({"agent_id":Uuid::new_v4()}),
        serde_json::json!([]),
    ] {
        let before = calls.load(Ordering::SeqCst);
        assert_eq!(
            client
                .post(&continuation)
                .json(&body)
                .bearer_auth("test-only-owner-session")
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(calls.load(Ordering::SeqCst), before);
    }
    revoked.store(true, Ordering::SeqCst);
    assert_eq!(
        client
            .post(&continuation)
            .json(&serde_json::json!({}))
            .bearer_auth("test-only-owner-session")
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let persisted = repo
        .read_pm_draft_operation(saved.id, saved.owner_user_id)
        .await
        .unwrap();
    assert_eq!(serde_json::to_value(persisted.response()).unwrap(), value);
    assert!(
        repo.list_session_agent_runs(persisted.session_id.unwrap())
            .await
            .unwrap()
            .is_empty()
    );
    for expected in [
        vec![],
        vec![serde_json::json!({"id":project,"key":"PM","name":"Owner project"})],
    ] {
        let response = client
            .get(format!("{url}/owner/api/v1/pm-drafts/projects"))
            .bearer_auth("test-only-owner-session")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let page: serde_json::Value = response.json().await.unwrap();
        assert_eq!(page["projects"], serde_json::json!(expected));
        assert_eq!(page["enabled"], true);
        assert_eq!(page["next_cursor"], serde_json::Value::Null);
        assert_eq!(page.as_object().unwrap().len(), 4);
        revoked.store(false, Ordering::SeqCst);
    }
    for prefix in ["machine", "local"] {
        let before = calls.load(Ordering::SeqCst);
        assert_eq!(
            client
                .get(format!("{url}/{prefix}/api/v1/pm-drafts/projects"))
                .bearer_auth("test-only-owner-session")
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(calls.load(Ordering::SeqCst), before);
    }
    for (query, status) in [
        (
            "after=00000000-0000-0000-0000-000000000000",
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        ("after=bad", StatusCode::UNPROCESSABLE_ENTITY),
        ("limit=500", StatusCode::BAD_REQUEST),
        ("unexpected=yes", StatusCode::BAD_REQUEST),
    ] {
        let before = calls.load(Ordering::SeqCst);
        assert_eq!(
            client
                .get(format!("{url}/owner/api/v1/pm-drafts/projects?{query}"))
                .bearer_auth("test-only-owner-session")
                .send()
                .await
                .unwrap()
                .status(),
            status
        );
        assert_eq!(calls.load(Ordering::SeqCst), before);
    }
    server.abort();
    upstream_server.abort();
    let _ = server.await;
    let _ = upstream_server.await;
}
