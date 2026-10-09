use crate::FleetRepository;
use async_trait::async_trait;
use domain::*;
use shared::AppError;

/// The caller supplies a freshly authorized human gateway, never persisted credentials.
#[async_trait]
pub trait PmDraftTracker: Send + Sync {
    /// Fresh namespace ownership is necessary, but never authorizes Hermes dispatch.
    async fn verify_namespace(
        &self,
        operation: &PmDraftOperation,
        agent: &Agent,
    ) -> Result<(), AppError>;
    async fn find_draft(
        &self,
        operation: &PmDraftOperation,
    ) -> Result<Option<TrackerCreatedDraft>, AppError>;
    async fn create_draft(
        &self,
        operation: &PmDraftOperation,
    ) -> Result<TrackerCreatedDraft, AppError>;
    async fn original_input(
        &self,
        operation: &PmDraftOperation,
    ) -> Result<TrackerDraftInputReceipt, AppError>;
    async fn reservation(
        &self,
        operation: &PmDraftOperation,
    ) -> Result<TrackerDraftReservationReadback, AppError>;
    async fn reserve_pm(
        &self,
        operation: &PmDraftOperation,
    ) -> Result<TrackerPmDraftReservation, AppError>;
}

#[async_trait]
pub trait PmDraftCredentials: Send + Sync {
    /// Persist issuance intent before external mutation; verify the child at Tracker.
    async fn prepare(
        &self,
        repo: &dyn FleetRepository,
        operation: &PmDraftOperation,
    ) -> Result<(), AppError>;
}

pub async fn continue_creation(
    repo: &dyn FleetRepository,
    tracker: &dyn PmDraftTracker,
    operation: PmDraftOperation,
) -> Result<PmDraftCreationResponse, AppError> {
    continue_creation_with_credentials(repo, tracker, operation, None).await
}

pub async fn continue_creation_with_credentials(
    repo: &dyn FleetRepository,
    tracker: &dyn PmDraftTracker,
    mut operation: PmDraftOperation,
    credentials: Option<&dyn PmDraftCredentials>,
) -> Result<PmDraftCreationResponse, AppError> {
    let agent = repo.get_agent(operation.request.agent_id).await?;
    if agent.kind != AgentKind::Hermes
        || agent.sdlc_role != Some(SdlcRole::ProjectManager)
        || agent.status == AgentStatus::Archived
    {
        return Err(AppError::conflict(
            "PM Draft requires a concrete non-archived Hermes PM",
        ));
    }
    tracker.verify_namespace(&operation, &agent).await?;
    // Tracker owns idempotency. Readback is mandatory even when an earlier local write failed.
    let draft = match tracker.find_draft(&operation).await? {
        Some(draft) => draft,
        None if operation.draft.is_none() => tracker.create_draft(&operation).await?,
        None => {
            return Err(AppError::conflict(
                "Tracker Draft receipt is no longer available",
            ));
        }
    };
    operation = repo
        .record_pm_draft_proof(
            operation.id,
            operation.owner_user_id,
            PmDraftProof::Created(draft),
        )
        .await?;
    let input = tracker.original_input(&operation).await?;
    operation = repo
        .record_pm_draft_proof(
            operation.id,
            operation.owner_user_id,
            PmDraftProof::Input(input),
        )
        .await?;
    let mut readback = tracker.reservation(&operation).await?;
    operation.check_current(&readback)?;
    if readback.current.is_none() {
        let reservation = tracker.reserve_pm(&operation).await?;
        operation = repo
            .record_pm_draft_proof(
                operation.id,
                operation.owner_user_id,
                PmDraftProof::Reserved(reservation),
            )
            .await?;
        // A replay of an old reservation is not current assignment authority.
        readback = tracker.reservation(&operation).await?;
        operation.check_current(&readback)?;
    }
    let reservation = readback
        .current
        .ok_or_else(|| AppError::conflict("PM reservation is not current"))?;
    operation = repo
        .record_pm_draft_proof(
            operation.id,
            operation.owner_user_id,
            PmDraftProof::Reserved(reservation),
        )
        .await?;
    let draft = operation
        .draft
        .as_ref()
        .expect("created proof was persisted");
    let session = repo
        .create_pm_draft_chat(
            CreatePmDraftChat {
                binding: TaskChatBinding {
                    tracker_instance_id: operation.tracker_instance_id.clone(),
                    project_id: operation.project_id,
                    task_id: draft.task_id,
                    root_task_id: draft.root_task_id,
                    agent_id: operation.request.agent_id,
                    owner_subject: operation.owner_subject.clone(),
                },
                title: operation.request.title.clone(),
                task_key: draft.task_key.clone(),
                idempotency_key: operation.chat_key(),
            },
            operation.owner_user_id,
        )
        .await?;
    operation = repo
        .record_pm_draft_proof(
            operation.id,
            operation.owner_user_id,
            PmDraftProof::Chat(session.id),
        )
        .await?;
    if let Some(credentials) = credentials {
        // This remains pre-admission. No model, lease claim or workflow mutation is issued.
        credentials.prepare(repo, &operation).await?;
    }
    // There is deliberately no Hermes dispatch here: admission is a separate authority.
    Ok(operation.response())
}
