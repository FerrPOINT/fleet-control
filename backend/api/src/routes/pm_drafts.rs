use super::task_chats::TrackerGateway;
use crate::middleware::{CurrentUser, VerifiedCentralSubject, VerifiedHumanSession};
use app::{AppContext, pm_draft::PmDraftTracker};
use async_trait::async_trait;
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use domain::*;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use shared::AppError;
use std::sync::Arc;
use uuid::Uuid;

struct HumanDraftGateway {
    gateway: TrackerGateway,
    headers: HeaderMap,
}

// Roundtrip equality rejects omitted explicit nulls and noncanonical UUID references.
fn decode<T: DeserializeOwned + Serialize>(value: Value) -> Result<T, AppError> {
    let typed: T = serde_json::from_value(value.clone())
        .map_err(|_| AppError::Unavailable("Tracker PM Draft response is invalid".into()))?;
    if serde_json::to_value(&typed).map_err(AppError::internal)? != value {
        return Err(AppError::Unavailable(
            "Tracker PM Draft response is not canonical".into(),
        ));
    }
    Ok(typed)
}

fn rejected(status: StatusCode) -> AppError {
    match status {
        StatusCode::UNAUTHORIZED => AppError::Unauthorized,
        StatusCode::FORBIDDEN => AppError::Forbidden,
        StatusCode::CONFLICT => {
            AppError::conflict("Tracker PM Draft changed; reconcile before continuing")
        }
        StatusCode::NOT_FOUND => AppError::not_found("Tracker PM Draft", "operation"),
        StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY => {
            AppError::validation("Tracker rejected PM Draft request")
        }
        _ => AppError::Unavailable("Tracker PM Draft is unavailable".into()),
    }
}

impl HumanDraftGateway {
    async fn read<T: DeserializeOwned + Serialize>(
        &self,
        path: &[&str],
        query: Option<(&str, &str)>,
    ) -> Result<T, AppError> {
        let (status, value) = self
            .gateway
            .request_path(path, query, &self.headers, None)
            .await?;
        if status != StatusCode::OK {
            return Err(rejected(status));
        }
        decode(value)
    }
    async fn write<T: DeserializeOwned + Serialize>(
        &self,
        path: &[&str],
        body: Value,
    ) -> Result<T, AppError> {
        let (status, value) = self
            .gateway
            .request_path(path, None, &self.headers, Some(body))
            .await?;
        if status != StatusCode::OK && status != StatusCode::CREATED {
            return Err(rejected(status));
        }
        decode(value)
    }
}

#[async_trait]
impl PmDraftTracker for HumanDraftGateway {
    async fn find_draft(
        &self,
        op: &PmDraftOperation,
    ) -> Result<Option<TrackerCreatedDraft>, AppError> {
        let (status, value) = self
            .gateway
            .request_path(
                &[
                    "api",
                    "v1",
                    "projects",
                    &op.project_id.to_string(),
                    "sdlc",
                    "drafts",
                    "operations",
                    &op.creation_key(),
                ],
                None,
                &self.headers,
                None,
            )
            .await?;
        match status {
            StatusCode::OK => Ok(Some(decode(value)?)),
            StatusCode::NOT_FOUND => Ok(None),
            _ => Err(rejected(status)),
        }
    }
    async fn create_draft(&self, op: &PmDraftOperation) -> Result<TrackerCreatedDraft, AppError> {
        self.write(&["api", "v1", "projects", &op.project_id.to_string(), "sdlc", "drafts"],
            serde_json::json!({"title":op.request.title,"description":op.request.description,"idempotency_key":op.creation_key()})).await
    }
    async fn original_input(
        &self,
        op: &PmDraftOperation,
    ) -> Result<TrackerDraftInputReceipt, AppError> {
        self.read(
            &[
                "api",
                "v1",
                "issues",
                &op.identity()?.task_id.to_string(),
                "sdlc",
                "pm-draft-input",
            ],
            None,
        )
        .await
    }
    async fn reservation(
        &self,
        op: &PmDraftOperation,
    ) -> Result<TrackerDraftReservationReadback, AppError> {
        self.read(
            &[
                "api",
                "v1",
                "issues",
                &op.identity()?.task_id.to_string(),
                "sdlc",
                "pm-draft-assignment",
            ],
            Some(("idempotency_key", &op.reservation_key())),
        )
        .await
    }
    async fn reserve_pm(
        &self,
        op: &PmDraftOperation,
    ) -> Result<TrackerPmDraftReservation, AppError> {
        self.write(
            &[
                "api",
                "v1",
                "issues",
                &op.identity()?.task_id.to_string(),
                "sdlc",
                "pm-draft-assignment",
            ],
            op.reservation_body(),
        )
        .await
    }
}

async fn authorized_project(
    ctx: &AppContext,
    headers: &HeaderMap,
    project: Uuid,
) -> Result<(), AppError> {
    let access = super::project_access::authorized_projects(ctx, headers).await?;
    if project.is_nil() || !access.project_ids.contains(&project) {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

#[utoipa::path(post,path="/api/v1/projects/{project_id}/pm-drafts",tag="task-chats",operation_id="create_pm_draft",
    params(("project_id"=Uuid,Path)),request_body=CreatePmDraftRequest,
    responses((status=202,body=PmDraftCreationResponse),(status=400),(status=401),(status=403),(status=404),(status=409),(status=422),(status=503)))]
pub async fn create(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    subject: Option<Extension<VerifiedCentralSubject>>,
    human: Option<Extension<VerifiedHumanSession>>,
    Path(project): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CreatePmDraftRequest>,
) -> Result<(StatusCode, Json<PmDraftCreationResponse>), AppError> {
    let Extension(_) = human.ok_or(AppError::Unauthorized)?;
    let Extension(subject) = subject.ok_or(AppError::Unauthorized)?;
    if !ctx.config.tracker.pm_draft_creation_enabled
        || !ctx.config.tracker.pm_draft_project_ids.contains(&project)
    {
        return Err(AppError::Unavailable(
            "PM Draft creation is not enabled for this project".into(),
        ));
    }
    request.validate()?;
    authorized_project(&ctx, &headers, project).await?;
    let gateway = HumanDraftGateway {
        gateway: TrackerGateway::configured(&ctx.config.tracker)?,
        headers,
    };
    let operation = ctx
        .repo
        .reserve_pm_draft_operation(PmDraftOperation {
            id: Uuid::new_v4(),
            owner_user_id: user.id,
            owner_subject: subject.0,
            tracker_instance_id: ctx.config.tracker.instance_id.clone(),
            project_id: project,
            request,
            draft: None,
            input: None,
            reservation: None,
            session_id: None,
        })
        .await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(app::pm_draft::continue_creation(ctx.repo.as_ref(), &gateway, operation).await?),
    ))
}

#[utoipa::path(get,path="/api/v1/pm-drafts/operations/{operation_id}",tag="task-chats",operation_id="read_pm_draft_creation",
    params(("operation_id"=Uuid,Path)),responses((status=200,body=PmDraftCreationResponse),(status=401),(status=403),(status=404),(status=503)))]
pub async fn read(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    subject: Option<Extension<VerifiedCentralSubject>>,
    human: Option<Extension<VerifiedHumanSession>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<PmDraftCreationResponse>, AppError> {
    let Extension(_) = human.ok_or(AppError::Unauthorized)?;
    let Extension(subject) = subject.ok_or(AppError::Unauthorized)?;
    let operation = ctx.repo.read_pm_draft_operation(id, user.id).await?;
    if operation.owner_subject != subject.0
        || operation.tracker_instance_id != ctx.config.tracker.instance_id
    {
        return Err(AppError::Forbidden);
    }
    authorized_project(&ctx, &headers, operation.project_id).await?;
    Ok(Json(operation.response()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Uri, header};
    use sha2::{Digest, Sha256};
    use std::sync::atomic::{AtomicUsize, Ordering};

    const RESERVATION: &str = include_str!("../../tests/fixtures/pm-draft/reservation.json");
    const READBACK: &str = include_str!("../../tests/fixtures/pm-draft/readback.json");
    const REQUEST: &str = include_str!("../../tests/fixtures/pm-draft/reserve-request.json");

    #[test]
    fn generated_openapi_operation_ids_are_unique() {
        let spec: Value = serde_json::from_str(&crate::openapi_json()).unwrap();
        let mut seen = std::collections::HashSet::new();
        for path in spec["paths"].as_object().unwrap().values() {
            for (method, operation) in path.as_object().unwrap() {
                if ["get", "post", "put", "patch", "delete", "options", "head"]
                    .contains(&method.as_str())
                {
                    let id = operation["operationId"]
                        .as_str()
                        .expect("operation must have an ID");
                    assert!(seen.insert(id), "duplicate operation ID: {id}");
                }
            }
        }
        assert!(seen.contains("create_pm_draft") && seen.contains("read_pm_draft_creation"));
    }

    #[test]
    fn actual_tracker_http_goldens_preserve_shapes_and_command_envelope_hash() {
        assert_eq!(
            format!("{:x}", Sha256::digest(RESERVATION.as_bytes())),
            "126adefbdc16e13a2eba54e98f76aa03b4ea9133d022fecb87fdf6d31fdd46af"
        );
        assert_eq!(
            format!("{:x}", Sha256::digest(READBACK.as_bytes())),
            "a4b555d66f8817f5a4b5e8bb39df8ceaaf60c885b45e2f4a81e1bb663f74bc91"
        );
        let reservation: TrackerPmDraftReservation =
            decode(serde_json::from_str(RESERVATION).unwrap()).unwrap();
        let readback: TrackerDraftReservationReadback =
            decode(serde_json::from_str(READBACK).unwrap()).unwrap();
        assert_eq!(readback.current.as_ref(), Some(&reservation));
        let request: Value = serde_json::from_str(REQUEST).unwrap();
        let hash = pm_canonical_hash(
            &serde_json::json!({"operation":"reserve_pm_draft","payload":request}),
        );
        assert_eq!(
            hash,
            "5a65ca6c9da22e530ccf7b3a6a408f3af6a736a015d24f74f11bcf8c20aeb9a3"
        );
        assert_eq!(readback.operation.unwrap().request_sha256, hash);
        assert!(!reservation.dispatch_allowed);
        assert_eq!(
            pm_input_hash("Restart-safe Draft", "Exact human request\nSecond line"),
            reservation.input.sha256
        );
    }

    #[test]
    fn reservation_semantics_never_turn_metadata_into_dispatch_authority() {
        let reservation: TrackerPmDraftReservation =
            decode(serde_json::from_str(RESERVATION).unwrap()).unwrap();
        let binding = reservation.binding.clone();
        let mut op = PmDraftOperation {
            id: Uuid::new_v4(),
            owner_user_id: Uuid::new_v4(),
            owner_subject: binding.owner_subject.clone(),
            tracker_instance_id: binding.tracker_instance_id.clone(),
            project_id: binding.project_id,
            request: CreatePmDraftRequest {
                agent_id: reservation.assignment.agent_id,
                title: "Restart-safe Draft".into(),
                description: "Exact human request\nSecond line".into(),
                idempotency_key: "create".into(),
            },
            draft: None,
            input: None,
            reservation: None,
            session_id: None,
        };
        op.apply(PmDraftProof::Created(TrackerCreatedDraft {
            tracker_instance_id: binding.tracker_instance_id.clone(),
            project_id: binding.project_id,
            task_id: binding.task_id,
            root_task_id: binding.root_task_id,
            task_key: "PM-1".into(),
            owner_subject: binding.owner_subject.clone(),
            stage: "Draft".into(),
        }))
        .unwrap();
        op.apply(PmDraftProof::Input(TrackerDraftInputReceipt {
            contract_version: 1,
            tracker_instance_id: binding.tracker_instance_id,
            project_id: binding.project_id,
            task_id: binding.task_id,
            root_task_id: binding.root_task_id,
            owner_subject: binding.owner_subject,
            input: TrackerDraftInput {
                snapshot_ref: reservation.input.snapshot_ref,
                title: op.request.title.clone(),
                description: op.request.description.clone(),
                sha256: reservation.input.sha256.clone(),
            },
        }))
        .unwrap();
        let valid = serde_json::to_value(&reservation).unwrap();
        let changes = [
            ("/dispatch_allowed", Value::Bool(true)),
            ("/admission_state", Value::String("admitted".into())),
            ("/execution/ordinal", Value::String("01".into())),
            ("/execution/ordinal", Value::String("0".into())),
            (
                "/execution/ordinal",
                Value::String("9223372036854775808".into()),
            ),
            ("/assignment/version", Value::from(2)),
            ("/owner_cas/version", Value::from(2)),
            ("/input/sha256", Value::String("a".repeat(64))),
            (
                "/assignment/agent_id",
                Value::String(Uuid::nil().to_string()),
            ),
            (
                "/assignment/execution_id",
                Value::String(Uuid::nil().to_string()),
            ),
        ];
        for (pointer, value) in changes {
            let mut wrong = valid.clone();
            *wrong.pointer_mut(pointer).unwrap() = value;
            let wrong = decode(wrong).unwrap();
            assert!(
                op.clone().apply(PmDraftProof::Reserved(wrong)).is_err(),
                "{pointer}"
            );
        }
        let mut opaque = reservation;
        opaque.assignment.machine_subject = "fleet-orchestrator".into();
        assert!(op.apply(PmDraftProof::Reserved(opaque)).is_ok());
    }

    #[test]
    fn explicit_nulls_canonical_refs_and_unknown_fields_are_checked_before_any_write() {
        let mut empty: Value = serde_json::from_str(READBACK).unwrap();
        empty["current"] = Value::Null;
        empty["operation"] = Value::Null;
        empty["owner_version"] = 0.into();
        assert!(decode::<TrackerDraftReservationReadback>(empty.clone()).is_ok());
        for field in ["current", "operation"] {
            let mut omitted = empty.clone();
            omitted.as_object_mut().unwrap().remove(field);
            assert!(decode::<TrackerDraftReservationReadback>(omitted).is_err());
        }
        let valid: Value = serde_json::from_str(RESERVATION).unwrap();
        for pointer in [
            "/binding/project_id",
            "/binding/task_id",
            "/input/snapshot_ref",
            "/assignment/agent_id",
            "/assignment/assignment_id",
            "/assignment/execution_id",
        ] {
            let mut wrong = valid.clone();
            let value = wrong.pointer_mut(pointer).unwrap();
            *value = Value::String(value.as_str().unwrap().to_uppercase());
            assert!(
                decode::<TrackerPmDraftReservation>(wrong).is_err(),
                "{pointer}"
            );
        }
        let mut unknown = valid.clone();
        unknown["runtime_token"] = "forbidden".into();
        assert!(decode::<TrackerPmDraftReservation>(unknown).is_err());
        let mut opaque = valid;
        opaque["assignment"]["machine_subject"] = "fleet-orchestrator".into();
        assert!(decode::<TrackerPmDraftReservation>(opaque).is_ok());
    }

    #[tokio::test]
    async fn fixed_origin_gateway_is_uncached_and_never_treats_rejection_as_absence() {
        let mode = Arc::new(AtomicUsize::new(0));
        let calls = Arc::new(AtomicUsize::new(0));
        let counters = (mode.clone(), calls.clone());
        let router = axum::Router::new().fallback(move |uri: Uri, headers: HeaderMap| {
            let (mode, calls) = counters.clone();
            async move {
                assert_eq!(headers[header::AUTHORIZATION], "Bearer test-human-bearer");
                assert!(
                    uri.path()
                        .contains("/sdlc/drafts/operations/fleet-pm-create:")
                );
                calls.fetch_add(1, Ordering::SeqCst);
                match mode.load(Ordering::SeqCst) {
                    0 => (StatusCode::NOT_FOUND, HeaderMap::new(), "{}".to_string()),
                    1 => (
                        StatusCode::CONFLICT,
                        HeaderMap::new(),
                        "private upstream detail".to_string(),
                    ),
                    2 => (
                        StatusCode::SERVICE_UNAVAILABLE,
                        HeaderMap::new(),
                        "private upstream detail".to_string(),
                    ),
                    3 => {
                        let mut headers = HeaderMap::new();
                        headers.insert(
                            header::LOCATION,
                            "http://127.0.0.1:1/token-leak".parse().unwrap(),
                        );
                        (StatusCode::TEMPORARY_REDIRECT, headers, String::new())
                    }
                    _ => (StatusCode::OK, HeaderMap::new(), "not-json".to_string()),
                }
            }
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let config = shared::TrackerConfig {
            url: format!("http://{}", listener.local_addr().unwrap()),
            instance_id: "tracker".into(),
            ..Default::default()
        };
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            "Bearer test-human-bearer".parse().unwrap(),
        );
        let gateway = HumanDraftGateway {
            gateway: TrackerGateway::configured(&config).unwrap(),
            headers,
        };
        let op = PmDraftOperation {
            id: Uuid::new_v4(),
            owner_user_id: Uuid::new_v4(),
            owner_subject: Uuid::new_v4().to_string(),
            tracker_instance_id: "tracker".into(),
            project_id: Uuid::new_v4(),
            request: CreatePmDraftRequest {
                agent_id: Uuid::new_v4(),
                title: "Task".into(),
                description: String::new(),
                idempotency_key: "create".into(),
            },
            draft: None,
            input: None,
            reservation: None,
            session_id: None,
        };
        assert!(gateway.find_draft(&op).await.unwrap().is_none());
        assert!(gateway.find_draft(&op).await.unwrap().is_none());
        for value in 1..=4 {
            mode.store(value, Ordering::SeqCst);
            let error = gateway.find_draft(&op).await.unwrap_err();
            assert!(!error.to_string().contains("private upstream detail"));
            if value == 1 {
                assert!(matches!(error, AppError::Conflict(_)));
            } else {
                assert!(matches!(error, AppError::Unavailable(_)));
            }
        }
        assert_eq!(calls.load(Ordering::SeqCst), 6);
        server.abort();
        let _ = server.await;
    }
}
