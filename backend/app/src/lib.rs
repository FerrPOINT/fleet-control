pub mod auth;
pub mod pm_draft;
pub mod runtime_launch;
pub mod sdlc_workflow;

use async_trait::async_trait;
use domain::{
    Agent, AgentConfig, AgentDirectoryItem, AgentEvent, AgentKind, AgentLogEntry, AgentSession,
    AgentStatus, AgentStorageReport, AssignSessionLeaderRequest, AuditLogEntry, AuthSettings,
    BulkDeploymentRequest, BulkDeploymentResult, CreateAgentRequest, CreateDeploymentJobRequest,
    CreateSessionDelegationRequest, CreateSessionMessageRequest, CreateSessionRequest,
    DeploymentJob, DeploymentJobState, FleetDashboard, HandoffSessionRequest, IntegrationSettings,
    LeaderExecutor, ManagedIntegrationSettings, ManagedPortSettings, ManagedRetentionSettings,
    ManagedSettingsChange, ManagedSettingsSnapshot, ManagedSettingsVersion, MessageDeliveryState,
    MessageKind, PortSettings, PurgeAgentFilesResponse, ResolveRuntimeApprovalRequest,
    RuntimeApprovalRequest, RuntimeOperationResponse, RuntimeRunControlResponse, RuntimeSettings,
    RuntimeTemplate, SessionAgentRun, SessionMessage, SessionParticipant, SessionRunRole,
    SessionRunState, SteerSessionRunRequest, UpdateAgentConfigRequest, UpdateAgentRequest,
    UpdateLeaderExecutorsRequest, UpdateSkillRequest, UpdateUserRoleRequest, UserResponse,
    WorkflowBinding, WorkflowCatalog, WorkflowCatalogEntry, WorkflowNamespaceCatalogEntry,
};
use shared::{AppConfig, AppError, FleetEvent};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RuntimeStatePatch {
    pub status: AgentStatus,
    pub desired_state: domain::DesiredState,
    pub pid: Option<i32>,
    pub health_status: Option<String>,
    pub health_detail: Option<String>,
    pub last_capabilities_json: Option<serde_json::Value>,
    pub startup_command_redacted: Option<String>,
    pub started_at: Option<shared::Timestamp>,
    pub stopped_at: Option<shared::Timestamp>,
}

#[derive(Debug, Clone, Default)]
pub struct SessionListFilter {
    pub agent_id: Option<Uuid>,
    pub user_ids: Vec<Uuid>,
    pub leader_agent_id: Option<Uuid>,
    pub include_all_users: bool,
    pub task_project_access: Option<domain::TaskProjectAccess>,
}

#[derive(Debug, Clone)]
pub struct AuditLogFilter {
    pub actor_user_id: Option<Uuid>,
    pub action: Option<String>,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub date_from: Option<shared::Timestamp>,
    pub date_to: Option<shared::Timestamp>,
    pub limit: u64,
}

#[derive(Debug, Clone)]
pub struct RuntimeApprovalCreate {
    pub session_id: Uuid,
    pub session_run_id: Uuid,
    pub agent_id: Uuid,
    pub runtime_run_id: String,
    pub runtime_approval_id: Option<String>,
    pub prompt: String,
    pub detail: serde_json::Value,
}

impl Default for AuditLogFilter {
    fn default() -> Self {
        Self {
            actor_user_id: None,
            action: None,
            entity_type: None,
            entity_id: None,
            date_from: None,
            date_to: None,
            limit: 100,
        }
    }
}

/// Session observed on a managed runtime (e.g. Java Agent /api/v2/sessions).
#[derive(Debug, Clone)]
pub struct RuntimeSessionSnapshot {
    pub external_id: String,
    pub title: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub updated_at: Option<chrono::DateTime<chrono::FixedOffset>>,
}

/// Private dispatcher input; prompt and credential fingerprint must not enter public/log DTOs.
#[derive(Clone)]
pub struct HermesDispatchDraft {
    pub message_id: Uuid,
    pub session_id: Uuid,
    pub agent_id: Uuid,
    pub run_role: SessionRunRole,
    pub requested_session_id: String,
    pub input: String,
    pub origin: String,
    pub credential_fingerprint: String,
    pub capabilities: serde_json::Value,
}

/// Original exact request, not permission to retry a submitted or accepted POST.
pub struct HermesDispatchIntent {
    pub message_id: Uuid,
    pub run: SessionAgentRun,
    pub request_body: String,
    pub request_hash: String,
    pub idempotency_key: String,
    pub origin: String,
    pub credential_fingerprint: String,
    pub capabilities: serde_json::Value,
    pub state: String,
    pub submission_attempted: bool,
    pub submitted_at: Option<shared::Timestamp>,
    pub recovery_deadline: shared::Timestamp,
    /// Database-clock observation; a recovered mapping rechecks under the journal lock.
    pub recovery_allowed: bool,
}

/// Private original control context. Raw guidance must not enter logs or public receipts.
pub struct RuntimeControlOutcomeIntent {
    pub receipt: domain::RuntimeControlReceipt,
    pub context: serde_json::Value,
}

/// Private delivery witness; not a public DTO, dispatch permission or debug payload.
pub struct ApprovalOutcomeIntent {
    pub decision: domain::ApprovalDecision,
    pub approval: RuntimeApprovalRequest,
    pub context: serde_json::Value,
}

/// Internal terminal proof; never a public request or permission to dispatch.
pub struct HermesTerminalCommit {
    pub message_id: Uuid,
    pub run_id: Uuid,
    pub runtime_run_id: String,
    pub runtime_session_id: String,
    pub state: SessionRunState,
    pub body: Option<String>,
    pub error: Option<String>,
}

#[async_trait]
pub trait FleetRepository: Send + Sync {
    async fn prepare_hermes_dispatch(
        &self,
        _draft: HermesDispatchDraft,
    ) -> Result<HermesDispatchIntent, AppError> {
        Err(AppError::Unavailable(
            "Hermes dispatch journal is unavailable".into(),
        ))
    }
    async fn claim_hermes_submission(
        &self,
        _message_id: Uuid,
        _origin: String,
        _credential_fingerprint: String,
    ) -> Result<Option<HermesDispatchIntent>, AppError> {
        Err(AppError::Unavailable(
            "Hermes dispatch journal is unavailable".into(),
        ))
    }
    async fn get_hermes_dispatch_intent(
        &self,
        _message_id: Uuid,
    ) -> Result<Option<HermesDispatchIntent>, AppError> {
        Err(AppError::Unavailable(
            "Hermes dispatch journal is unavailable".into(),
        ))
    }
    async fn get_hermes_dispatch_intent_for_run(
        &self,
        _run_id: Uuid,
    ) -> Result<Option<HermesDispatchIntent>, AppError> {
        Err(AppError::Unavailable(
            "Hermes dispatch journal is unavailable".into(),
        ))
    }
    async fn reserve_runtime_control(
        &self,
        _run: &SessionAgentRun,
        _actor: &domain::RuntimeControlActor,
        _operation: domain::RuntimeControlOperation,
        _input: Option<&str>,
    ) -> Result<domain::RuntimeControlReservation, AppError> {
        Err(AppError::Unavailable(
            "runtime control journal is unavailable".into(),
        ))
    }
    async fn claim_runtime_control(&self, _id: Uuid) -> Result<bool, AppError> {
        Err(AppError::Unavailable(
            "runtime control journal is unavailable".into(),
        ))
    }
    async fn claim_runtime_control_outcome(
        &self,
        _id: Uuid,
        _context: serde_json::Value,
    ) -> Result<bool, AppError> {
        Err(AppError::Unavailable(
            "runtime control outcome journal is unavailable".into(),
        ))
    }
    async fn get_runtime_control_outcome(
        &self,
        _id: Uuid,
    ) -> Result<Option<RuntimeControlOutcomeIntent>, AppError> {
        Err(AppError::Unavailable(
            "runtime control outcome journal is unavailable".into(),
        ))
    }
    async fn list_runtime_control_outcomes(
        &self,
        _after: Option<Uuid>,
    ) -> Result<Vec<RuntimeControlOutcomeIntent>, AppError> {
        Err(AppError::Unavailable(
            "runtime control outcome journal is unavailable".into(),
        ))
    }
    async fn finish_runtime_control_outcome(
        &self,
        _id: Uuid,
        _context: serde_json::Value,
        _acknowledgement: &str,
    ) -> Result<domain::RuntimeControlReceipt, AppError> {
        Err(AppError::Unavailable(
            "runtime control outcome journal is unavailable".into(),
        ))
    }
    async fn finish_runtime_control(
        &self,
        _id: Uuid,
        _acknowledgement: &str,
    ) -> Result<domain::RuntimeControlReceipt, AppError> {
        Err(AppError::Unavailable(
            "runtime control journal is unavailable".into(),
        ))
    }
    async fn retire_runtime_control(
        &self,
        _id: Uuid,
        _submitted: bool,
    ) -> Result<domain::RuntimeControlReceipt, AppError> {
        Err(AppError::Unavailable(
            "runtime control journal is unavailable".into(),
        ))
    }
    async fn get_runtime_control(
        &self,
        _session: Uuid,
        _id: Uuid,
    ) -> Result<domain::RuntimeControlReceipt, AppError> {
        Err(AppError::Unavailable(
            "runtime control journal is unavailable".into(),
        ))
    }
    async fn list_runtime_controls(
        &self,
        _session: Uuid,
        _run: Uuid,
    ) -> Result<Vec<domain::RuntimeControlReceipt>, AppError> {
        Err(AppError::Unavailable(
            "runtime control journal is unavailable".into(),
        ))
    }
    async fn lookup_runtime_control(
        &self,
        _run: &SessionAgentRun,
        _actor: &domain::RuntimeControlActor,
        _query: &domain::RuntimeControlLookupQuery,
    ) -> Result<domain::RuntimeControlReceipt, AppError> {
        Err(AppError::Unavailable(
            "runtime control journal is unavailable".into(),
        ))
    }
    async fn reconcile_runtime_controls(&self) -> Result<u64, AppError> {
        Err(AppError::Unavailable(
            "runtime control journal is unavailable".into(),
        ))
    }
    /// Initial submissions only; submitted/legacy/task records must never enter this queue.
    async fn list_prepared_hermes_dispatches(
        &self,
        _after: Option<Uuid>,
    ) -> Result<Vec<(SessionMessage, HermesDispatchIntent)>, AppError> {
        Ok(Vec::new())
    }
    async fn reserve_pm_draft_operation(
        &self,
        _operation: domain::PmDraftOperation,
    ) -> Result<domain::PmDraftOperation, AppError> {
        Err(AppError::Unavailable(
            "PM Draft creation is unavailable".into(),
        ))
    }
    async fn read_pm_draft_operation(
        &self,
        _id: Uuid,
        _owner: Uuid,
    ) -> Result<domain::PmDraftOperation, AppError> {
        Err(AppError::Unavailable(
            "PM Draft creation is unavailable".into(),
        ))
    }
    async fn read_pm_draft_operation_by_key(
        &self,
        _owner: Uuid,
        _key: &str,
    ) -> Result<domain::PmDraftOperation, AppError> {
        Err(AppError::Unavailable(
            "PM Draft creation is unavailable".into(),
        ))
    }
    async fn record_pm_draft_proof(
        &self,
        _id: Uuid,
        _owner: Uuid,
        _proof: domain::PmDraftProof,
    ) -> Result<domain::PmDraftOperation, AppError> {
        Err(AppError::Unavailable(
            "PM Draft creation is unavailable".into(),
        ))
    }
    async fn list_session_approvals(
        &self,
        _session_id: Uuid,
    ) -> Result<Vec<RuntimeApprovalRequest>, AppError> {
        Err(AppError::Unavailable(
            "targeted approvals are unavailable".into(),
        ))
    }
    async fn approval_decision(
        &self,
        _session_id: Uuid,
        _approval_id: Uuid,
    ) -> Result<domain::ApprovalDecision, AppError> {
        Err(AppError::Unavailable(
            "targeted approvals are unavailable".into(),
        ))
    }
    async fn reserve_approval_decision(
        &self,
        _session_id: Uuid,
        _approval_id: Uuid,
        _actor: Uuid,
        _req: domain::ApprovalDecisionRequest,
    ) -> Result<domain::ReservedApprovalDecision, AppError> {
        Err(AppError::Unavailable(
            "targeted approvals are unavailable".into(),
        ))
    }
    async fn deliver_approval_decision(
        &self,
        _decision_id: Uuid,
    ) -> Result<domain::ApprovalDecision, AppError> {
        Err(AppError::Unavailable(
            "targeted approvals are unavailable".into(),
        ))
    }
    async fn reserve_original_approval_decision(
        &self,
        _session: Uuid,
        _approval: Uuid,
        _actor: Uuid,
        _req: domain::ApprovalDecisionRequest,
    ) -> Result<domain::ReservedApprovalDecision, AppError> {
        Err(AppError::Unavailable(
            "approval outcome journal is unavailable".into(),
        ))
    }
    async fn claim_approval_outcome(
        &self,
        _id: Uuid,
        _context: serde_json::Value,
    ) -> Result<bool, AppError> {
        Err(AppError::Unavailable(
            "approval outcome journal is unavailable".into(),
        ))
    }
    async fn get_approval_outcome(
        &self,
        _id: Uuid,
    ) -> Result<Option<ApprovalOutcomeIntent>, AppError> {
        Err(AppError::Unavailable(
            "approval outcome journal is unavailable".into(),
        ))
    }
    async fn list_approval_outcomes(
        &self,
        _after: Option<Uuid>,
    ) -> Result<Vec<ApprovalOutcomeIntent>, AppError> {
        Err(AppError::Unavailable(
            "approval outcome journal is unavailable".into(),
        ))
    }
    async fn finish_approval_outcome(
        &self,
        _id: Uuid,
        _context: serde_json::Value,
    ) -> Result<domain::ApprovalDecision, AppError> {
        Err(AppError::Unavailable(
            "approval outcome journal is unavailable".into(),
        ))
    }
    async fn fail_undispatched_approval_decision(
        &self,
        _decision_id: Uuid,
    ) -> Result<domain::ApprovalDecision, AppError> {
        Err(AppError::Unavailable(
            "targeted approvals are unavailable".into(),
        ))
    }
    async fn list_chats_directory(
        &self,
        _filter: domain::ChatsDirectoryFilter,
    ) -> Result<domain::ChatsDirectoryPage, AppError> {
        Err(AppError::Unavailable(
            "chat directory is not available".into(),
        ))
    }
    async fn reserve_pm_run(
        &self,
        _reservation: domain::PmRunReservation,
    ) -> Result<domain::PmRunRecord, AppError> {
        Err(AppError::Unavailable(
            "PM run repository is not available".into(),
        ))
    }
    async fn get_pm_run(&self, _id: Uuid) -> Result<domain::PmRunRecord, AppError> {
        Err(AppError::Unavailable(
            "PM run repository is not available".into(),
        ))
    }
    async fn accept_pm_run(
        &self,
        _id: Uuid,
        _hermes_run_ref: String,
        _hermes_session_ref: String,
    ) -> Result<domain::PmRunRecord, AppError> {
        Err(AppError::Unavailable(
            "PM run repository is not available".into(),
        ))
    }
    async fn observe_pm_run(
        &self,
        _expected: &domain::PmRunRecord,
        _status: domain::PmRuntimeStatus,
        _custody: &dyn PmRuntimeCustody,
    ) -> Result<(), AppError> {
        Err(AppError::Unavailable(
            "PM run repository is not available".into(),
        ))
    }
    async fn has_pending_session_dispatch(&self, _session_id: Uuid) -> Result<bool, AppError> {
        Ok(false)
    }
    async fn get_task_chat_binding(
        &self,
        _session_id: Uuid,
    ) -> Result<Option<domain::TaskChatBinding>, AppError> {
        Err(AppError::Unavailable(
            "task chat repository is not available".into(),
        ))
    }
    async fn bind_task_chat(
        &self,
        _session_id: Uuid,
        _binding: domain::TaskChatBinding,
        _idempotency_key: String,
    ) -> Result<domain::TaskChatBinding, AppError> {
        Err(AppError::Unavailable(
            "task chat repository is not available".into(),
        ))
    }
    async fn create_pm_draft_chat(
        &self,
        _command: domain::CreatePmDraftChat,
        _owner_user_id: Uuid,
    ) -> Result<domain::AgentSession, AppError> {
        Err(AppError::Unavailable(
            "PM Draft chat repository is not available".into(),
        ))
    }
    async fn tracker_event_cursor(&self, _session_id: Uuid) -> Result<i64, AppError> {
        Err(AppError::Unavailable(
            "Tracker inbox is not available".into(),
        ))
    }
    async fn project_tracker_events(
        &self,
        _session_id: Uuid,
        _binding: domain::TaskChatBinding,
        _after: i64,
        _page: domain::TrackerOutboxPage,
    ) -> Result<domain::TrackerProjectionReceipt, AppError> {
        Err(AppError::Unavailable(
            "Tracker inbox is not available".into(),
        ))
    }
    async fn tracker_metadata_cursor(&self, _session_id: Uuid) -> Result<i64, AppError> {
        Err(AppError::Unavailable(
            "Tracker metadata inbox is not available".into(),
        ))
    }
    async fn tracker_projection_targets(
        &self,
        _instance: &str,
        _project_ids: &[Uuid],
        _after_session: Option<Uuid>,
    ) -> Result<Vec<domain::TrackerProjectionTarget>, AppError> {
        Err(AppError::Unavailable(
            "Tracker projection targets are unavailable".into(),
        ))
    }
    async fn project_tracker_metadata(
        &self,
        _session_id: Uuid,
        _binding: domain::TaskChatBinding,
        _after: i64,
        _page: domain::TrackerMetadataPage,
    ) -> Result<domain::TrackerProjectionReceipt, AppError> {
        Err(AppError::Unavailable(
            "Tracker metadata inbox is not available".into(),
        ))
    }
    async fn session_message_history(
        &self,
        _session_id: Uuid,
        _before: Option<Uuid>,
        _limit: u64,
    ) -> Result<domain::MessageHistoryPage, AppError> {
        Err(AppError::Unavailable(
            "message history is not available".into(),
        ))
    }
    async fn list_runtime_templates(&self) -> Result<Vec<RuntimeTemplate>, AppError>;
    async fn ensure_runtime_templates(&self) -> Result<(), AppError>;
    async fn list_agents(&self) -> Result<Vec<Agent>, AppError>;
    async fn list_agent_directory(&self) -> Result<Vec<AgentDirectoryItem>, AppError>;
    async fn list_agents_by_product_role(
        &self,
        product_role: domain::AgentProductRole,
    ) -> Result<Vec<Agent>, AppError>;
    async fn get_agent(&self, id: Uuid) -> Result<Agent, AppError>;
    async fn create_agent(
        &self,
        req: CreateAgentRequest,
        config: &AppConfig,
    ) -> Result<Agent, AppError>;
    async fn update_agent(&self, id: Uuid, req: UpdateAgentRequest) -> Result<Agent, AppError>;
    async fn update_agent_status(&self, id: Uuid, status: AgentStatus) -> Result<Agent, AppError>;
    async fn update_runtime_state(
        &self,
        id: Uuid,
        patch: RuntimeStatePatch,
    ) -> Result<Agent, AppError>;
    async fn archive_agent(&self, id: Uuid) -> Result<Agent, AppError>;
    async fn list_leader_executors(
        &self,
        leader_agent_id: Uuid,
    ) -> Result<Vec<LeaderExecutor>, AppError>;
    async fn replace_leader_executors(
        &self,
        leader_agent_id: Uuid,
        req: UpdateLeaderExecutorsRequest,
        actor_user_id: Uuid,
    ) -> Result<Vec<LeaderExecutor>, AppError>;

    async fn get_agent_config(&self, agent_id: Uuid) -> Result<AgentConfig, AppError>;
    async fn claim_runtime_launch(
        &self,
        _binding: &runtime_launch::RuntimeLaunchBinding,
    ) -> Result<(), AppError> {
        Err(AppError::Unavailable(
            "runtime launch journal is unavailable".into(),
        ))
    }
    async fn get_open_runtime_launch(
        &self,
        _agent_id: Uuid,
    ) -> Result<Option<runtime_launch::RuntimeLaunchRecord>, AppError> {
        Err(AppError::Unavailable(
            "runtime launch journal is unavailable".into(),
        ))
    }
    async fn reserve_controller_recovery(
        &self,
        _request: &runtime_launch::ControllerRecoveryRequest,
    ) -> Result<runtime_launch::ControllerRecoveryRecord, AppError> {
        Err(AppError::Unavailable(
            "controller recovery journal is unavailable".into(),
        ))
    }
    async fn get_runtime_launch(
        &self,
        _launch_id: Uuid,
    ) -> Result<Option<runtime_launch::RuntimeLaunchRecord>, AppError> {
        Err(AppError::Unavailable(
            "runtime launch history is unavailable".into(),
        ))
    }
    async fn read_controller_recovery(
        &self,
        _id: Uuid,
    ) -> Result<Option<runtime_launch::ControllerRecoveryRecord>, AppError> {
        Err(AppError::Unavailable(
            "controller recovery journal is unavailable".into(),
        ))
    }
    async fn heartbeat_controller_recovery(
        &self,
        _id: Uuid,
        _controller: Uuid,
        _version: i64,
    ) -> Result<runtime_launch::ControllerRecoveryRecord, AppError> {
        Err(AppError::Unavailable(
            "controller recovery journal is unavailable".into(),
        ))
    }
    async fn acknowledge_controller_recovery(
        &self,
        _id: Uuid,
        _controller: Uuid,
        _version: i64,
        _native_receipt_sha256: &str,
    ) -> Result<runtime_launch::ControllerRecoveryRecord, AppError> {
        Err(AppError::Unavailable(
            "controller recovery journal is unavailable".into(),
        ))
    }
    async fn record_container_endpoint(
        &self,
        _binding: &runtime_launch::RuntimeLaunchBinding,
        _pid: i32,
        _origin: &str,
    ) -> Result<(), AppError> {
        Err(AppError::Unavailable(
            "runtime endpoint custody is unavailable".into(),
        ))
    }
    async fn retain_controller_recovery_command(
        &self,
        _command: &runtime_launch::ControllerRecoveryCommand,
    ) -> Result<runtime_launch::ControllerRecoveryDelivery, AppError> {
        Err(AppError::Unavailable(
            "controller delivery journal is unavailable".into(),
        ))
    }
    async fn read_controller_recovery_delivery(
        &self,
        _id: Uuid,
    ) -> Result<Option<runtime_launch::ControllerRecoveryDelivery>, AppError> {
        Err(AppError::Unavailable(
            "controller delivery journal is unavailable".into(),
        ))
    }
    async fn current_controller_recovery(
        &self,
        _launch_id: Uuid,
    ) -> Result<Option<runtime_launch::ControllerRecoveryRecord>, AppError> {
        Err(AppError::Unavailable(
            "controller recovery journal is unavailable".into(),
        ))
    }
    async fn claim_controller_recovery_dispatch(
        &self,
        _id: Uuid,
        _controller: Uuid,
    ) -> Result<bool, AppError> {
        Err(AppError::Unavailable(
            "controller delivery journal is unavailable".into(),
        ))
    }
    async fn settle_controller_recovery_outcome(
        &self,
        _id: Uuid,
        _receipt: &serde_json::Value,
    ) -> Result<runtime_launch::ControllerRecoveryRecord, AppError> {
        Err(AppError::Unavailable(
            "controller delivery journal is unavailable".into(),
        ))
    }
    async fn next_container_launch_ordinal(&self, _agent_id: Uuid) -> Result<i64, AppError> {
        Err(AppError::Unavailable(
            "runtime launch history is unavailable".into(),
        ))
    }
    async fn retain_controller_stop(
        &self,
        _command: &runtime_launch::ControllerRecoveryCommand,
    ) -> Result<runtime_launch::ControllerStopDelivery, AppError> {
        Err(AppError::Unavailable(
            "controller stop journal is unavailable".into(),
        ))
    }
    async fn read_controller_stop(
        &self,
        _launch_id: Uuid,
    ) -> Result<Option<runtime_launch::ControllerStopDelivery>, AppError> {
        Err(AppError::Unavailable(
            "controller stop journal is unavailable".into(),
        ))
    }
    async fn claim_controller_stop(
        &self,
        _command: &runtime_launch::ControllerRecoveryCommand,
    ) -> Result<bool, AppError> {
        Err(AppError::Unavailable(
            "controller stop journal is unavailable".into(),
        ))
    }
    async fn settle_controller_stop(
        &self,
        _launch_id: Uuid,
        _outcome: &serde_json::Value,
    ) -> Result<runtime_launch::ControllerStopDelivery, AppError> {
        Err(AppError::Unavailable(
            "controller stop journal is unavailable".into(),
        ))
    }
    async fn claim_container_preparation(
        &self,
        _preparation: &runtime_launch::RuntimeContainerPreparation,
        _configuration: &runtime_launch::RuntimeConfigurationClaim,
    ) -> Result<(), AppError> {
        Err(AppError::Unavailable(
            "container preparation journal is unavailable".into(),
        ))
    }
    async fn has_pending_container_preparation(&self, _agent_id: Uuid) -> Result<bool, AppError> {
        Err(AppError::Unavailable(
            "container preparation journal is unavailable".into(),
        ))
    }
    async fn verify_container_preparation(
        &self,
        _preparation: &runtime_launch::RuntimeContainerPreparation,
        _configuration: &runtime_launch::RuntimeConfigurationClaim,
    ) -> Result<(), AppError> {
        Err(AppError::Unavailable(
            "original container preparation journal is unavailable".into(),
        ))
    }
    async fn observe_runtime_launch(
        &self,
        _binding: &runtime_launch::RuntimeLaunchBinding,
        _state: &str,
        _pid: Option<i32>,
    ) -> Result<(), AppError> {
        Err(AppError::Unavailable(
            "runtime launch journal is unavailable".into(),
        ))
    }
    async fn create_config_revision(
        &self,
        agent_id: Uuid,
        config: UpdateAgentConfigRequest,
        actor: Uuid,
    ) -> Result<domain::AgentConfigRevision, AppError>;
    async fn prepare_base_package_revision(
        &self,
        _agent_id: Uuid,
        _checkout: &str,
        _binding: &domain::SdlcWorkflowBinding,
        _actor: Uuid,
    ) -> Result<domain::AgentConfigRevision, AppError> {
        Err(AppError::validation(
            "Base package preparation is unavailable",
        ))
    }
    async fn list_config_revisions(
        &self,
        agent_id: Uuid,
    ) -> Result<Vec<domain::AgentConfigRevision>, AppError>;
    async fn get_config_revision(
        &self,
        _agent_id: Uuid,
        _revision: i64,
    ) -> Result<domain::AgentConfigRevision, AppError> {
        Err(AppError::Unavailable(
            "configuration revision lookup is unavailable".into(),
        ))
    }
    async fn get_effective_config_revision(
        &self,
        _agent_id: Uuid,
    ) -> Result<Option<domain::AgentConfigRevision>, AppError> {
        Err(AppError::Unavailable(
            "effective configuration head lookup is unavailable".into(),
        ))
    }
    async fn verify_base_package_revision(
        &self,
        _agent_id: Uuid,
        _revision: i64,
        _checkout: &str,
    ) -> Result<(), AppError> {
        Err(AppError::validation(
            "Base package verification is unavailable",
        ))
    }
    async fn validate_config_revision(
        &self,
        agent_id: Uuid,
        revision: i64,
        errors: Vec<String>,
    ) -> Result<domain::AgentConfigRevision, AppError>;
    async fn request_config_activation(
        &self,
        agent_id: Uuid,
        revision: i64,
        actor: Uuid,
    ) -> Result<domain::AgentConfigRevision, AppError>;
    async fn claim_config_activation(
        &self,
    ) -> Result<Option<domain::AgentConfigRevision>, AppError>;
    async fn finish_config_activation(
        &self,
        agent_id: Uuid,
        revision: i64,
        error: Option<String>,
        reconciled: bool,
    ) -> Result<(), AppError>;
    async fn agent_is_draining(&self, agent_id: Uuid) -> Result<bool, AppError>;
    async fn settle_configuration_rollback(
        &self,
        _claim: &runtime_launch::ConfigurationRollbackClaim,
    ) -> Result<(), AppError> {
        Err(AppError::Unavailable(
            "configuration rollback recovery is unavailable".into(),
        ))
    }
    async fn verify_configuration_rollback(
        &self,
        _claim: &runtime_launch::ConfigurationRollbackClaim,
    ) -> Result<(), AppError> {
        Err(AppError::Unavailable(
            "configuration rollback recovery is unavailable".into(),
        ))
    }
    async fn update_agent_config(
        &self,
        agent_id: Uuid,
        req: UpdateAgentConfigRequest,
    ) -> Result<AgentConfig, AppError>;

    async fn list_agent_skills(&self, agent_id: Uuid) -> Result<Vec<domain::AgentSkill>, AppError>;
    async fn update_agent_skill(
        &self,
        agent_id: Uuid,
        name: String,
        req: UpdateSkillRequest,
    ) -> Result<domain::AgentSkill, AppError>;

    async fn list_sessions(&self, filter: SessionListFilter)
    -> Result<Vec<AgentSession>, AppError>;
    async fn sync_runtime_sessions(
        &self,
        agent_id: Uuid,
        sessions: Vec<RuntimeSessionSnapshot>,
    ) -> Result<u64, AppError>;
    async fn get_session(&self, id: Uuid) -> Result<AgentSession, AppError>;
    async fn create_session(
        &self,
        req: CreateSessionRequest,
        user_id: Uuid,
    ) -> Result<AgentSession, AppError>;
    async fn create_session_delegation(
        &self,
        parent_session_id: Uuid,
        req: CreateSessionDelegationRequest,
        user_id: Uuid,
    ) -> Result<AgentSession, AppError>;
    async fn assign_session_leader(
        &self,
        id: Uuid,
        req: AssignSessionLeaderRequest,
        actor_user_id: Uuid,
    ) -> Result<AgentSession, AppError>;
    async fn handoff_session(
        &self,
        id: Uuid,
        req: HandoffSessionRequest,
    ) -> Result<AgentSession, AppError>;
    async fn list_session_messages(&self, id: Uuid) -> Result<Vec<SessionMessage>, AppError>;
    async fn list_session_events(
        &self,
        id: Uuid,
        after: i64,
    ) -> Result<Vec<domain::SessionEvent>, AppError>;
    async fn session_event_cursor(&self, id: Uuid) -> Result<i64, AppError>;
    async fn append_session_event(
        &self,
        id: Uuid,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<(), AppError>;
    async fn claim_message_dispatch(&self) -> Result<Option<SessionMessage>, AppError>;
    async fn claim_controller_message_dispatch(
        &self,
        _controller_id: Uuid,
    ) -> Result<Option<SessionMessage>, AppError> {
        // Repositories without durable launch ownership cannot dispatch managed children.
        Ok(None)
    }
    async fn finish_message_dispatch(
        &self,
        message_id: Uuid,
        uncertain: bool,
        error: Option<String>,
    ) -> Result<(), AppError>;
    async fn list_session_participants(
        &self,
        id: Uuid,
    ) -> Result<Vec<SessionParticipant>, AppError>;
    async fn create_session_message(
        &self,
        id: Uuid,
        req: CreateSessionMessageRequest,
        actor_user_id: Uuid,
    ) -> Result<SessionMessage, AppError>;
    async fn list_session_agent_runs(&self, id: Uuid) -> Result<Vec<SessionAgentRun>, AppError>;
    async fn get_session_agent_run(&self, id: Uuid) -> Result<SessionAgentRun, AppError>;
    async fn prepare_session_agent_run(
        &self,
        session_id: Uuid,
        agent_id: Uuid,
        run_role: SessionRunRole,
        runtime_session_id: String,
    ) -> Result<SessionAgentRun, AppError>;
    async fn update_session_agent_run_dispatch(
        &self,
        id: Uuid,
        runtime_run_id: Option<String>,
        state: SessionRunState,
        last_error: Option<String>,
    ) -> Result<SessionAgentRun, AppError>;
    /// Persist a verified ACK before readback; this never authorizes task dispatch.
    async fn accept_hermes_run(
        &self,
        message_id: Uuid,
        run_id: Uuid,
        runtime_run_id: String,
    ) -> Result<SessionAgentRun, AppError>;
    /// Persist a non-dispatch lookup proof against the original immutable journal.
    async fn accept_recovered_hermes_run(
        &self,
        _message_id: Uuid,
        _run_id: Uuid,
        _runtime_run_id: String,
        _original_capabilities: serde_json::Value,
    ) -> Result<SessionAgentRun, AppError> {
        Err(AppError::Unavailable(
            "Hermes recovery commit is unavailable".into(),
        ))
    }
    async fn pin_hermes_run_session(
        &self,
        run_id: Uuid,
        runtime_run_id: String,
        requested_session_id: String,
        effective_session_id: String,
    ) -> Result<(SessionAgentRun, bool), AppError>;
    /// Commit mirror, delivery and terminal capacity release together; bool means first commit.
    async fn commit_hermes_terminal(
        &self,
        command: HermesTerminalCommit,
    ) -> Result<(SessionAgentRun, Option<SessionMessage>, bool), AppError>;
    async fn list_recoverable_hermes_acceptances(
        &self,
        after: Option<Uuid>,
    ) -> Result<Vec<(SessionMessage, SessionAgentRun)>, AppError>;
    async fn insert_session_message_mirror(
        &self,
        session_id: Uuid,
        author_agent_id: Option<Uuid>,
        body: String,
        message_kind: MessageKind,
        runtime_message_id: Option<String>,
    ) -> Result<SessionMessage, AppError>;
    async fn update_session_message_delivery(
        &self,
        id: Uuid,
        delivery_state: MessageDeliveryState,
        runtime_message_id: Option<String>,
        delivery_error: Option<String>,
    ) -> Result<(), AppError>;
    async fn upsert_runtime_approval_request(
        &self,
        req: RuntimeApprovalCreate,
    ) -> Result<RuntimeApprovalRequest, AppError>;
    /// Observe an exact pending native snapshot, not permission to dispatch a decision.
    async fn recover_hermes_approval(
        &self,
        _req: RuntimeApprovalCreate,
        _native_session_id: String,
        _origin: String,
        _credential_fingerprint: String,
    ) -> Result<(RuntimeApprovalRequest, bool), AppError> {
        Err(AppError::Unavailable(
            "Hermes approval recovery is not implemented".into(),
        ))
    }
    async fn resolve_runtime_approval_request(
        &self,
        id: Uuid,
        req: ResolveRuntimeApprovalRequest,
        actor_user_id: Uuid,
    ) -> Result<RuntimeApprovalRequest, AppError>;
    async fn resolve_runtime_approval_requests_for_run(
        &self,
        session_run_id: Uuid,
        req: ResolveRuntimeApprovalRequest,
        actor_user_id: Uuid,
    ) -> Result<u64, AppError>;

    async fn list_workflow_bindings(&self) -> Result<Vec<WorkflowBinding>, AppError>;
    async fn rebind_workflow_binding(
        &self,
        agent_id: Uuid,
        namespace: WorkflowNamespaceCatalogEntry,
        workflow: WorkflowCatalogEntry,
    ) -> Result<WorkflowBinding, AppError>;

    /// project-workflow sync: refresh binding_status for known bindings
    /// against the live project-workflow namespace/workflow catalog.
    async fn refresh_workflow_bindings(
        &self,
        known_namespaces: Vec<(String, String)>,
        known_workflows: Vec<(String, String)>,
    ) -> Result<u64, AppError>;
    async fn list_events(&self, limit: u64) -> Result<Vec<AgentEvent>, AppError>;
    async fn insert_event(
        &self,
        agent_id: Option<Uuid>,
        event_type: &str,
        message: &str,
        payload: serde_json::Value,
    ) -> Result<AgentEvent, AppError>;
    async fn list_fleet_alerts(
        &self,
        state: Option<&str>,
    ) -> Result<Vec<domain::FleetAlert>, AppError>;
    async fn acknowledge_fleet_alert(
        &self,
        alert_id: Uuid,
        user_id: Uuid,
    ) -> Result<domain::FleetAlert, AppError>;
    async fn insert_fleet_alert(
        &self,
        alert: domain::FleetAlert,
    ) -> Result<domain::FleetAlert, AppError>;

    /// Restarts recorded for the agent within the window (crash-loop input).
    async fn recent_restart_count(
        &self,
        agent_id: Uuid,
        window: chrono::Duration,
    ) -> Result<u32, AppError>;

    async fn resolve_active_alerts_of_kind(
        &self,
        agent_id: Uuid,
        kind: &str,
        resolution_detail: serde_json::Value,
    ) -> Result<u64, AppError>;

    async fn insert_audit(
        &self,
        actor_user_id: Option<Uuid>,
        action: &str,
        entity_type: &str,
        entity_id: Option<String>,
        payload: serde_json::Value,
    ) -> Result<(), AppError>;
    async fn list_audit_log(&self, filter: AuditLogFilter) -> Result<Vec<AuditLogEntry>, AppError>;
    async fn list_logs(
        &self,
        agent_id: Option<Uuid>,
        limit: u64,
    ) -> Result<Vec<AgentLogEntry>, AppError>;
    async fn insert_log(
        &self,
        agent_id: Uuid,
        stream: &str,
        message: &str,
    ) -> Result<AgentLogEntry, AppError>;

    async fn find_user_by_email(&self, email: &str) -> Result<Option<auth::UserRecord>, AppError>;
    async fn find_or_create_central_user(
        &self,
        _sub: &str,
        _email: &str,
        _display_name: &str,
    ) -> Result<auth::UserRecord, AppError> {
        Err(AppError::Unauthorized)
    }
    async fn find_user_by_id(&self, id: Uuid) -> Result<Option<auth::UserRecord>, AppError>;
    async fn list_users(&self) -> Result<Vec<UserResponse>, AppError>;
    async fn update_user_role(
        &self,
        user_id: Uuid,
        req: UpdateUserRoleRequest,
    ) -> Result<UserResponse, AppError>;
    async fn create_user(
        &self,
        req: domain::RegisterRequest,
        password_hash: String,
        is_system_admin: bool,
    ) -> Result<auth::UserRecord, AppError>;
    async fn update_refresh_hash(
        &self,
        user_id: Uuid,
        refresh_hash: Option<String>,
    ) -> Result<(), AppError>;

    async fn list_deployment_jobs(&self, limit: u64) -> Result<Vec<DeploymentJob>, AppError>;
    async fn update_deployment_job_state(
        &self,
        job_id: Uuid,
        state: DeploymentJobState,
        detail_patch: Option<serde_json::Value>,
        last_error: Option<String>,
    ) -> Result<DeploymentJob, AppError>;
    async fn get_deployment_job(&self, id: Uuid) -> Result<DeploymentJob, AppError>;
    async fn create_deployment_job(
        &self,
        req: CreateDeploymentJobRequest,
        requested_by_user_id: Uuid,
    ) -> Result<DeploymentJob, AppError>;
    /// Bulk variant (IMPLEMENTATION_PLAN Phase 3): creates one job per agent
    /// id, skipping archived agents; `rollback` marks runtime_update jobs as
    /// rollback (detail carries `"rollback": true`).
    async fn bulk_create_deployment_jobs(
        &self,
        req: BulkDeploymentRequest,
        requested_by_user_id: Uuid,
    ) -> Result<BulkDeploymentResult, AppError>;
    async fn cancel_deployment_job(
        &self,
        id: Uuid,
        actor_user_id: Uuid,
    ) -> Result<DeploymentJob, AppError>;

    async fn get_runtime_settings(&self, config: &AppConfig) -> Result<RuntimeSettings, AppError>;
    async fn get_port_settings(&self, config: &AppConfig) -> Result<PortSettings, AppError>;
    async fn get_integration_settings(
        &self,
        config: &AppConfig,
    ) -> Result<IntegrationSettings, AppError>;
    async fn get_auth_settings(&self, config: &AppConfig) -> Result<AuthSettings, AppError>;
    async fn get_active_managed_settings(&self)
    -> Result<Option<ManagedSettingsVersion>, AppError>;
    async fn get_managed_settings_version(
        &self,
        version: i64,
    ) -> Result<ManagedSettingsVersion, AppError>;
    async fn list_managed_settings_versions(
        &self,
        limit: u64,
    ) -> Result<Vec<ManagedSettingsVersion>, AppError>;
    async fn activate_managed_settings(
        &self,
        snapshot: ManagedSettingsSnapshot,
        actor_user_id: Uuid,
        expected_active_version: Option<i64>,
        rollback_of_version: Option<i64>,
        audit_action: &str,
    ) -> Result<ManagedSettingsVersion, AppError>;
}

#[async_trait]
pub trait AgentProvisioner: Send + Sync {
    async fn prepare_request_observer_configuration(
        &self,
        _config: &AppConfig,
        request: UpdateAgentConfigRequest,
    ) -> Result<UpdateAgentConfigRequest, AppError> {
        if request.config_json.get("fleet_request_observer").is_some() {
            return Err(AppError::Unavailable(
                "request observer preparation is unavailable".into(),
            ));
        }
        Ok(request)
    }
    /// Fresh readback only; successful verification does not grant dispatch authority.
    async fn verify_effective_configuration(
        &self,
        _agent: &Agent,
        _config: &AppConfig,
        _revision: &domain::AgentConfigRevision,
    ) -> Result<(), AppError> {
        Err(AppError::Unavailable(
            "effective configuration readback is unavailable".into(),
        ))
    }
    async fn provision(&self, agent: &Agent, config: &AppConfig) -> Result<(), AppError>;
    async fn storage_report(
        &self,
        agent: &Agent,
        config: &AppConfig,
    ) -> Result<AgentStorageReport, AppError>;
    async fn purge_files(
        &self,
        agent: &Agent,
        config: &AppConfig,
    ) -> Result<PurgeAgentFilesResponse, AppError>;
}

#[async_trait]
pub trait PmRuntimeCustody: Send + Sync {
    async fn verify(
        &self,
        binding: &domain::PmRuntimeBinding,
        launch: &runtime_launch::RuntimeLaunchBinding,
        pid: i32,
    ) -> Result<(), AppError>;
}

#[async_trait]
pub trait RuntimeSupervisor: Send + Sync {
    async fn read_request_observation(
        &self,
        _agent: &Agent,
        _run: &SessionAgentRun,
    ) -> Result<serde_json::Value, AppError> {
        Err(AppError::Unavailable(
            "native request observation is unavailable".into(),
        ))
    }
    async fn resolve_original_approval(
        &self,
        _agent: &Agent,
        _run: &SessionAgentRun,
        _approval: &RuntimeApprovalRequest,
        _decision: &domain::ApprovalDecision,
    ) -> Result<domain::ApprovalDecision, AppError> {
        Err(AppError::Unavailable(
            "original approval outcomes are unavailable".into(),
        ))
    }
    async fn resolve_targeted_approval(
        &self,
        _agent: &Agent,
        _run: &SessionAgentRun,
        _approval: &RuntimeApprovalRequest,
        _choice: domain::ApprovalChoice,
    ) -> Result<(), AppError> {
        Err(AppError::Unavailable(
            "targeted approvals are unavailable".into(),
        ))
    }
    async fn capture_pm_runtime_binding(
        &self,
        _agent: &Agent,
    ) -> Result<domain::PmRuntimeBinding, AppError> {
        Err(AppError::Unavailable(
            "PM runtime binding is not available".into(),
        ))
    }
    async fn probe_pm_run(
        &self,
        _agent: &Agent,
        _record: &domain::PmRunRecord,
    ) -> Result<domain::PmRuntimeStatus, AppError> {
        Err(AppError::Unavailable(
            "PM runtime readback is not available".into(),
        ))
    }
    async fn start(&self, agent: &Agent) -> Result<RuntimeOperationResponse, AppError>;
    async fn stop(&self, agent: &Agent) -> Result<RuntimeOperationResponse, AppError>;
    async fn restart(&self, agent: &Agent) -> Result<RuntimeOperationResponse, AppError>;
    async fn health(&self, agent: &Agent) -> Result<RuntimeOperationResponse, AppError>;
    async fn send_message(
        &self,
        agent: &Agent,
        session: &AgentSession,
        message: &domain::SessionMessage,
    ) -> Result<RuntimeOperationResponse, AppError>;
    async fn steer_run(
        &self,
        agent: &Agent,
        run: &SessionAgentRun,
        req: SteerSessionRunRequest,
        actor: domain::RuntimeControlActor,
    ) -> Result<RuntimeRunControlResponse, AppError>;
    async fn stop_run(
        &self,
        agent: &Agent,
        run: &SessionAgentRun,
        actor: domain::RuntimeControlActor,
    ) -> Result<RuntimeRunControlResponse, AppError>;
    async fn resolve_approval(
        &self,
        agent: &Agent,
        run: &SessionAgentRun,
        req: ResolveRuntimeApprovalRequest,
    ) -> Result<RuntimeRunControlResponse, AppError>;
    fn command_preview(&self, agent: &Agent) -> String;
}

pub struct AppContext {
    pub config: Arc<AppConfig>,
    pub repo: Arc<dyn FleetRepository>,
    pub provisioner: Arc<dyn AgentProvisioner>,
    pub runtime: Arc<dyn RuntimeSupervisor>,
    pub auth: auth::AuthService,
    pub events: broadcast::Sender<FleetEvent>,
    pub pm_credentials: Option<Arc<dyn pm_draft::PmDraftCredentials>>,
    restart_tx: mpsc::Sender<()>,
}

/// Result of one scheduled stale-folder review pass
/// (docs/IMPLEMENTATION_PLAN.md Phase 3).
#[derive(Debug, Clone, serde::Serialize)]
pub struct RetentionReviewOutcome {
    /// Agents whose archived folders exceeded the stale threshold.
    pub stale_agent_ids: Vec<Uuid>,
    /// The stale threshold (days) used for this pass.
    pub stale_archived_days: u32,
    pub reviewed_at: shared::Timestamp,
}

impl AppContext {
    /// Scheduled stale-folder review: scans archived agents and returns the
    /// ones older than `fleet.retention.stale_archived_days`. Read-only —
    /// physical purge stays a separate explicit operator action.
    pub async fn review_stale_agent_folders(&self) -> Result<RetentionReviewOutcome, AppError> {
        let threshold = i64::from(self.config.fleet.retention.stale_archived_days);
        let agents = self.repo.list_agents().await?;
        let now = chrono::Utc::now();
        let mut stale = Vec::new();
        for agent in agents {
            if agent.status != AgentStatus::Archived {
                continue;
            }
            let ts = chrono::DateTime::parse_from_rfc3339(&agent.updated_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .map_err(|e| AppError::internal(format!("invalid agent timestamp: {e}")))?;
            if (now - ts).num_days() >= threshold {
                stale.push(agent.id);
            }
        }
        Ok(RetentionReviewOutcome {
            stale_agent_ids: stale,
            stale_archived_days: self.config.fleet.retention.stale_archived_days,
            reviewed_at: now.fixed_offset(),
        })
    }

    pub fn new(
        config: Arc<AppConfig>,
        repo: Arc<dyn FleetRepository>,
        provisioner: Arc<dyn AgentProvisioner>,
        runtime: Arc<dyn RuntimeSupervisor>,
        events: broadcast::Sender<FleetEvent>,
        restart_tx: mpsc::Sender<()>,
    ) -> Self {
        let auth = auth::AuthService::new(config.auth.clone());
        Self {
            config,
            repo,
            provisioner,
            runtime,
            auth,
            events,
            pm_credentials: None,
            restart_tx,
        }
    }

    pub fn schedule_restart(&self) {
        let restart_tx = self.restart_tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(350)).await;
            let _ = restart_tx.send(()).await;
        });
    }

    pub fn emit(&self, event: FleetEvent) {
        let _ = self.events.send(event);
    }

    pub async fn dashboard(&self) -> Result<FleetDashboard, AppError> {
        let agents = self.repo.list_agents().await?;
        let recent_events = self.repo.list_events(12).await?;
        let sessions = self
            .repo
            .list_sessions(SessionListFilter {
                include_all_users: true,
                ..SessionListFilter::default()
            })
            .await?;
        Ok(FleetDashboard {
            total_agents: agents.len(),
            leader_agents: agents
                .iter()
                .filter(|agent| agent.product_role == domain::AgentProductRole::Leader)
                .count(),
            executor_agents: agents
                .iter()
                .filter(|agent| agent.product_role == domain::AgentProductRole::Executor)
                .count(),
            running_agents: agents
                .iter()
                .filter(|agent| agent.status == AgentStatus::Running)
                .count(),
            failed_agents: agents
                .iter()
                .filter(|agent| agent.status == AgentStatus::Failed)
                .count(),
            active_sessions: sessions
                .iter()
                .filter(|session| session.state == domain::SessionState::Active)
                .count(),
            private_sessions: sessions
                .iter()
                .filter(|session| session.visibility == domain::SessionVisibility::Private)
                .count(),
            leader_scoped_sessions: sessions
                .iter()
                .filter(|session| session.visibility == domain::SessionVisibility::LeaderScoped)
                .count(),
            agents,
            recent_events,
        })
    }

    pub async fn ensure_seed_agents(&self) -> Result<(), AppError> {
        self.repo.ensure_runtime_templates().await?;
        if !self.repo.list_agents().await?.is_empty() {
            return Ok(());
        }

        let seeds = [
            CreateAgentRequest {
                kind: AgentKind::Hermes,
                product_role: domain::AgentProductRole::Executor,
                role: domain::AgentRole::Developer,
                sdlc_role: Some(domain::SdlcRole::Developer),
                display_name: "Developer Hermes".to_string(),
                description: Some("Primary development workflow agent".to_string()),
                namespace_id: Some("dev".to_string()),
                namespace_name: Some("Development".to_string()),
                workflow_id: Some("workflow-dev".to_string()),
                workflow_name: Some("Developer Workflow".to_string()),
                executor_ids: Vec::new(),
            },
            CreateAgentRequest {
                kind: AgentKind::Hermes,
                product_role: domain::AgentProductRole::Executor,
                role: domain::AgentRole::Tester,
                sdlc_role: Some(domain::SdlcRole::Tester),
                display_name: "Tester Hermes".to_string(),
                description: Some("QA and verification workflow agent".to_string()),
                namespace_id: Some("qa".to_string()),
                namespace_name: Some("Quality Assurance".to_string()),
                workflow_id: Some("workflow-qa".to_string()),
                workflow_name: Some("Tester Workflow".to_string()),
                executor_ids: Vec::new(),
            },
        ];

        for seed in seeds {
            let agent = self.repo.create_agent(seed, &self.config).await?;
            self.provisioner.provision(&agent, &self.config).await?;
            let agent = self
                .repo
                .update_agent_status(agent.id, AgentStatus::Ready)
                .await?;
            self.emit(FleetEvent::AgentCreated {
                agent_id: agent.id.to_string(),
                name: agent.name,
            });
        }
        Ok(())
    }
}

pub fn managed_settings_from_config(config: &AppConfig) -> ManagedSettingsSnapshot {
    ManagedSettingsSnapshot {
        runtime: RuntimeSettings {
            agents_root: config.fleet.agents_root.clone(),
            hermes_source: config.fleet.hermes_source.clone(),
            hermes_command: config.fleet.hermes_command.clone(),
            java_agent_source: config.fleet.java_agent_source.clone(),
            java_agent_command: config.fleet.java_agent_command.clone(),
        },
        ports: ManagedPortSettings {
            agent_port_base: config.fleet.agent_port_base,
            agent_port_stride: config.fleet.agent_port_stride,
        },
        integrations: ManagedIntegrationSettings {
            forge_api_url: config.fleet.forge_api_url.clone(),
            forge_project: config.fleet.forge_project.clone(),
            project_workflow_url: config.fleet.project_workflow_url.clone(),
        },
        auth: AuthSettings {
            mode: config.auth.mode.clone(),
            jwt_issuer: config.auth.jwt_issuer.clone(),
            jwt_audience: config.auth.jwt_audience.clone(),
            access_token_ttl_minutes: config.auth.access_token_ttl_minutes,
            refresh_token_ttl_days: config.auth.refresh_token_ttl_days,
            refresh_cookie_name: config.auth.refresh_cookie_name.clone(),
            refresh_cookie_secure: config.auth.refresh_cookie_secure,
            refresh_cookie_same_site: config.auth.refresh_cookie_same_site.clone(),
            refresh_cookie_domain: config.auth.refresh_cookie_domain.clone(),
            refresh_cookie_path: config.auth.refresh_cookie_path.clone(),
        },
        retention: ManagedRetentionSettings {
            stale_archived_days: config.fleet.retention.stale_archived_days,
            review_interval_secs: config.fleet.retention.review_interval_secs,
        },
    }
}

pub fn normalize_managed_settings(
    mut snapshot: ManagedSettingsSnapshot,
) -> ManagedSettingsSnapshot {
    snapshot.runtime.agents_root = snapshot.runtime.agents_root.trim().to_string();
    snapshot.runtime.hermes_source = snapshot.runtime.hermes_source.trim().to_string();
    snapshot.runtime.hermes_command = snapshot.runtime.hermes_command.trim().to_string();
    snapshot.runtime.java_agent_source = snapshot.runtime.java_agent_source.trim().to_string();
    snapshot.runtime.java_agent_command = snapshot.runtime.java_agent_command.trim().to_string();
    snapshot.integrations.forge_api_url = normalize_optional(snapshot.integrations.forge_api_url);
    snapshot.integrations.forge_project = normalize_optional(snapshot.integrations.forge_project);
    snapshot.integrations.project_workflow_url =
        normalize_optional(snapshot.integrations.project_workflow_url);
    snapshot.auth.mode = snapshot.auth.mode.trim().to_ascii_lowercase();
    snapshot.auth.jwt_issuer = snapshot.auth.jwt_issuer.trim().to_string();
    snapshot.auth.jwt_audience = snapshot.auth.jwt_audience.trim().to_string();
    snapshot.auth.refresh_cookie_name = snapshot.auth.refresh_cookie_name.trim().to_string();
    snapshot.auth.refresh_cookie_same_site =
        snapshot.auth.refresh_cookie_same_site.trim().to_string();
    snapshot.auth.refresh_cookie_domain = normalize_optional(snapshot.auth.refresh_cookie_domain);
    snapshot.auth.refresh_cookie_path = snapshot.auth.refresh_cookie_path.trim().to_string();
    snapshot
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn validate_managed_settings(
    snapshot: &ManagedSettingsSnapshot,
    base_config: &AppConfig,
) -> Result<(), AppError> {
    for (name, value) in [
        ("runtime.agents_root", snapshot.runtime.agents_root.as_str()),
        (
            "runtime.hermes_source",
            snapshot.runtime.hermes_source.as_str(),
        ),
        (
            "runtime.hermes_command",
            snapshot.runtime.hermes_command.as_str(),
        ),
        (
            "runtime.java_agent_source",
            snapshot.runtime.java_agent_source.as_str(),
        ),
        (
            "runtime.java_agent_command",
            snapshot.runtime.java_agent_command.as_str(),
        ),
        ("auth.jwt_issuer", snapshot.auth.jwt_issuer.as_str()),
        ("auth.jwt_audience", snapshot.auth.jwt_audience.as_str()),
        (
            "auth.refresh_cookie_name",
            snapshot.auth.refresh_cookie_name.as_str(),
        ),
    ] {
        if value.is_empty() {
            return Err(AppError::validation(format!("{name} must not be empty")));
        }
    }
    if snapshot.ports.agent_port_base < 1024 {
        return Err(AppError::validation(
            "ports.agent_port_base must be at least 1024",
        ));
    }
    if snapshot.ports.agent_port_stride < 4 {
        return Err(AppError::validation(
            "ports.agent_port_stride must be at least 4",
        ));
    }
    if snapshot.retention.review_interval_secs < 60 {
        return Err(AppError::validation(
            "retention.review_interval_secs must be at least 60",
        ));
    }
    if snapshot.auth.access_token_ttl_minutes == 0 || snapshot.auth.refresh_token_ttl_days == 0 {
        return Err(AppError::validation(
            "auth token TTL values must be positive",
        ));
    }
    if snapshot.auth.mode != "hmac" && snapshot.auth.mode != "oidc" {
        return Err(AppError::validation("auth.mode supports only hmac or oidc"));
    }
    if snapshot.auth.mode == "oidc" && base_config.auth.oidc_issuer_url.trim().is_empty() {
        return Err(AppError::validation(
            "auth.mode=oidc requires startup auth.oidc_issuer_url",
        ));
    }
    if !snapshot.auth.refresh_cookie_path.starts_with('/') {
        return Err(AppError::validation(
            "auth.refresh_cookie_path must start with /",
        ));
    }
    if !matches!(
        snapshot.auth.refresh_cookie_same_site.as_str(),
        "Lax" | "Strict" | "None"
    ) {
        return Err(AppError::validation(
            "auth.refresh_cookie_same_site must be Lax, Strict, or None",
        ));
    }
    if snapshot.auth.refresh_cookie_same_site == "None" && !snapshot.auth.refresh_cookie_secure {
        return Err(AppError::validation(
            "SameSite=None requires a secure refresh cookie",
        ));
    }
    for (name, value) in [
        (
            "integrations.forge_api_url",
            snapshot.integrations.forge_api_url.as_deref(),
        ),
        (
            "integrations.project_workflow_url",
            snapshot.integrations.project_workflow_url.as_deref(),
        ),
    ] {
        if let Some(value) = value {
            let url = reqwest::Url::parse(value)
                .map_err(|_| AppError::validation(format!("{name} must be a valid URL")))?;
            if !matches!(url.scheme(), "http" | "https") {
                return Err(AppError::validation(format!(
                    "{name} must use http or https"
                )));
            }
        }
    }
    Ok(())
}

pub fn apply_managed_settings(
    base_config: &AppConfig,
    snapshot: &ManagedSettingsSnapshot,
) -> AppConfig {
    let mut config = base_config.clone();
    config.fleet.agents_root = snapshot.runtime.agents_root.clone();
    config.fleet.hermes_source = snapshot.runtime.hermes_source.clone();
    config.fleet.hermes_command = snapshot.runtime.hermes_command.clone();
    config.fleet.java_agent_source = snapshot.runtime.java_agent_source.clone();
    config.fleet.java_agent_command = snapshot.runtime.java_agent_command.clone();
    config.fleet.agent_port_base = snapshot.ports.agent_port_base;
    config.fleet.agent_port_stride = snapshot.ports.agent_port_stride;
    config.fleet.forge_api_url = snapshot.integrations.forge_api_url.clone();
    config.fleet.forge_project = snapshot.integrations.forge_project.clone();
    config.fleet.project_workflow_url = snapshot.integrations.project_workflow_url.clone();
    config.auth.mode = snapshot.auth.mode.clone();
    config.auth.jwt_issuer = snapshot.auth.jwt_issuer.clone();
    config.auth.jwt_audience = snapshot.auth.jwt_audience.clone();
    config.auth.access_token_ttl_minutes = snapshot.auth.access_token_ttl_minutes;
    config.auth.refresh_token_ttl_days = snapshot.auth.refresh_token_ttl_days;
    config.auth.refresh_cookie_name = snapshot.auth.refresh_cookie_name.clone();
    config.auth.refresh_cookie_secure = snapshot.auth.refresh_cookie_secure;
    config.auth.refresh_cookie_same_site = snapshot.auth.refresh_cookie_same_site.clone();
    config.auth.refresh_cookie_domain = snapshot.auth.refresh_cookie_domain.clone();
    config.auth.refresh_cookie_path = snapshot.auth.refresh_cookie_path.clone();
    config.fleet.retention.stale_archived_days = snapshot.retention.stale_archived_days;
    config.fleet.retention.review_interval_secs = snapshot.retention.review_interval_secs;
    config
}

pub fn managed_settings_changes(
    current: &ManagedSettingsSnapshot,
    proposed: &ManagedSettingsSnapshot,
) -> Result<Vec<ManagedSettingsChange>, AppError> {
    let current = serde_json::to_value(current).map_err(AppError::internal)?;
    let proposed = serde_json::to_value(proposed).map_err(AppError::internal)?;
    let mut changes = Vec::new();
    collect_settings_changes("", &current, &proposed, &mut changes);
    Ok(changes)
}

fn collect_settings_changes(
    path: &str,
    current: &serde_json::Value,
    proposed: &serde_json::Value,
    changes: &mut Vec<ManagedSettingsChange>,
) {
    match (current, proposed) {
        (serde_json::Value::Object(current), serde_json::Value::Object(proposed)) => {
            let mut keys = current.keys().chain(proposed.keys()).collect::<Vec<_>>();
            keys.sort_unstable();
            keys.dedup();
            for key in keys {
                let child_path = if path.is_empty() {
                    key.to_string()
                } else {
                    format!("{path}.{key}")
                };
                collect_settings_changes(
                    &child_path,
                    current.get(key).unwrap_or(&serde_json::Value::Null),
                    proposed.get(key).unwrap_or(&serde_json::Value::Null),
                    changes,
                );
            }
        }
        _ if current != proposed => changes.push(ManagedSettingsChange {
            path: path.to_string(),
            before: current.clone(),
            after: proposed.clone(),
            requires_restart: true,
        }),
        _ => {}
    }
}

// --- Fleet monitoring alerts (IMPLEMENTATION_PLAN Phase 3) ---

/// Maps a runtime health transition to the alert(s) it should raise.
/// Returns (kind, severity) pairs: `agent_down` on a previously-healthy
/// agent going down, `agent_recovered` auto-resolves active `agent_down`
/// alerts, restart loops raise `agent_restart_loop`.
/// Desired-state reconciliation decision for one agent (Phase 1 runtime
/// reconciler): a failed/stopped agent with desired=running must be
/// restarted; a running agent with desired=stopped must be stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconcileAction {
    Restart,
    Stop,
    HealthCheck,
    None,
}

pub fn reconcile_action(
    status: domain::AgentStatus,
    desired: domain::DesiredState,
) -> ReconcileAction {
    use domain::AgentStatus as S;
    use domain::DesiredState as D;
    match (status, desired) {
        (S::Failed | S::Stopped, D::Running) => ReconcileAction::Restart,
        (S::Running | S::Starting | S::Degraded, _) => ReconcileAction::HealthCheck,
        (_, D::Running) => ReconcileAction::HealthCheck,
        _ => ReconcileAction::None,
    }
}

/// Restart-loop detection input: restarts observed within the sliding window.
#[derive(Debug, Clone, Copy)]
pub struct RestartContext {
    pub recent_restarts: u32,
    pub loop_threshold: u32,
}

/// True when the agent restarted at least `loop_threshold` times inside the
/// observation window (crash loop).
pub fn restart_loop_detected(recent_restarts: u32, loop_threshold: u32) -> bool {
    loop_threshold > 0 && recent_restarts >= loop_threshold
}

fn heartbeat_age(
    last_health_at: Option<chrono::DateTime<chrono::Utc>>,
    current_status: &str,
    observed_at: chrono::DateTime<chrono::Utc>,
) -> Option<chrono::Duration> {
    if current_status != "running" {
        return None;
    }
    last_health_at
        .filter(|timestamp| *timestamp <= observed_at)
        .map(|timestamp| observed_at - timestamp)
}

/// True when a running agent has not reported health within
/// `stale_after_minutes`. Stopped/degraded agents do not owe heartbeats.
pub fn heartbeat_stale(
    last_health_at: Option<chrono::DateTime<chrono::Utc>>,
    current_status: &str,
    stale_after_minutes: i64,
) -> bool {
    heartbeat_age(last_health_at, current_status, chrono::Utc::now())
        .is_some_and(|age| age > chrono::Duration::minutes(stale_after_minutes))
}

/// Extended transition alerts: a crash loop overrides the plain down alert
/// so operators see the loop, not a storm of individual downs.
pub fn health_transition_alerts_ex(
    previous: Option<&str>,
    current: &str,
    restarts: RestartContext,
) -> Vec<(String, String)> {
    let mut out = health_transition_alerts(previous, current);
    if restart_loop_detected(restarts.recent_restarts, restarts.loop_threshold) {
        out.retain(|(kind, _)| kind != "agent_down");
        out.push(("agent_restart_loop".to_string(), "warning".to_string()));
    }
    out
}

pub fn health_transition_alerts(previous: Option<&str>, current: &str) -> Vec<(String, String)> {
    let was_up = matches!(previous, Some("running") | Some("ready"));
    let is_down = matches!(current, "failed" | "stopped" | "degraded");
    let mut out = Vec::new();
    if was_up && is_down {
        out.push(("agent_down".to_string(), "critical".to_string()));
    }
    if let Some(prev) = previous
        && (prev == "failed" || prev == "stopped")
        && (current == "running" || current == "ready")
    {
        out.push(("agent_recovered".to_string(), "info".to_string()));
    }
    out
}

pub const ACTIVE_ALERT_STATES: [&str; 2] = ["open", "acknowledged"];
pub const HEARTBEAT_STALE_ALERT_KIND: &str = "heartbeat_stale";

pub fn alert_state_is_active(state: &str) -> bool {
    ACTIVE_ALERT_STATES.contains(&state)
}

#[async_trait]
pub trait AlertService: Send + Sync {
    /// Records health transitions for an agent: raises `agent_down` /
    /// `agent_restart_loop` alerts and resolves active down-alerts on recovery.
    async fn record_health_transition(
        &self,
        agent_id: Uuid,
        previous_status: Option<String>,
        current_status: &str,
    ) -> Result<(), AppError>;
}

pub struct RepositoryAlertService {
    pub repository: Arc<dyn FleetRepository>,
}

impl RepositoryAlertService {
    /// See [`AlertService::record_health_transition`].
    pub async fn record_health_transition(
        &self,
        agent_id: Uuid,
        previous_status: Option<String>,
        current_status: &str,
    ) -> Result<(), AppError> {
        AlertService::record_health_transition(self, agent_id, previous_status, current_status)
            .await
    }
}

impl RepositoryAlertService {
    /// Scan running agents for stale heartbeats; raises one active
    /// `heartbeat_stale` alert per agent, resolved by a verified fresh heartbeat.
    pub async fn record_heartbeat_freshness(&self) -> Result<(), AppError> {
        let agents = self.repository.list_agents().await?;
        let active_alerts = self
            .repository
            .list_fleet_alerts(None)
            .await?
            .into_iter()
            .filter(|alert| alert_state_is_active(&alert.state))
            .collect::<Vec<_>>();
        for agent in &agents {
            let last_health = agent
                .runtime
                .last_health_at
                .as_deref()
                .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
                .map(|ts| ts.with_timezone(&chrono::Utc));
            let already_active = active_alerts
                .iter()
                .any(|a| a.agent_id == Some(agent.id) && a.kind == HEARTBEAT_STALE_ALERT_KIND);
            let Some(age) = heartbeat_age(last_health, agent.status.as_str(), chrono::Utc::now())
            else {
                continue;
            };
            if age > chrono::Duration::minutes(10) {
                if already_active {
                    continue;
                }
                self.repository
                    .insert_fleet_alert(domain::FleetAlert {
                        id: Uuid::new_v4(),
                        agent_id: Some(agent.id),
                        kind: HEARTBEAT_STALE_ALERT_KIND.to_string(),
                        severity: "warning".to_string(),
                        detail: serde_json::json!({
                            "last_health_at": agent.runtime.last_health_at,
                        }),
                        state: "open".to_string(),
                        opened_at: chrono::Utc::now().to_rfc3339(),
                        resolved_at: None,
                        acknowledged_at: None,
                        acknowledged_by_user_id: None,
                    })
                    .await?;
            } else if already_active {
                self.repository
                    .resolve_active_alerts_of_kind(
                        agent.id,
                        HEARTBEAT_STALE_ALERT_KIND,
                        serde_json::json!({"last_health_at": agent.runtime.last_health_at}),
                    )
                    .await?;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl AlertService for RepositoryAlertService {
    async fn record_health_transition(
        &self,
        agent_id: Uuid,
        previous_status: Option<String>,
        current_status: &str,
    ) -> Result<(), AppError> {
        let previous = previous_status.as_deref();
        let recent_restarts = self
            .repository
            .recent_restart_count(agent_id, chrono::Duration::minutes(15))
            .await
            .unwrap_or(0);
        let restarts = RestartContext {
            recent_restarts,
            loop_threshold: 3,
        };
        for (kind, severity) in health_transition_alerts_ex(previous, current_status, restarts) {
            if kind == "agent_recovered" {
                self.repository
                    .resolve_active_alerts_of_kind(
                        agent_id,
                        "agent_down",
                        serde_json::json!({"recovered_to": current_status}),
                    )
                    .await?;
                self.repository
                    .resolve_active_alerts_of_kind(
                        agent_id,
                        "agent_restart_loop",
                        serde_json::json!({"recovered_to": current_status}),
                    )
                    .await?;
                self.repository
                    .resolve_active_alerts_of_kind(
                        agent_id,
                        HEARTBEAT_STALE_ALERT_KIND,
                        serde_json::json!({"recovered_to": current_status}),
                    )
                    .await?;
                continue;
            }
            self.repository
                .insert_fleet_alert(domain::FleetAlert {
                    id: Uuid::new_v4(),
                    agent_id: Some(agent_id),
                    kind: kind.clone(),
                    severity,
                    detail: serde_json::json!({
                        "previous": previous,
                        "current": current_status,
                    }),
                    state: "open".to_string(),
                    opened_at: chrono::Utc::now().to_rfc3339(),
                    resolved_at: None,
                    acknowledged_at: None,
                    acknowledged_by_user_id: None,
                })
                .await?;
        }
        Ok(())
    }
}

fn catalog_id(value: Option<&serde_json::Value>, field: &str) -> Result<String, AppError> {
    match value {
        Some(serde_json::Value::String(value)) if !value.trim().is_empty() => Ok(value.clone()),
        Some(serde_json::Value::Number(value)) => Ok(value.to_string()),
        _ => Err(AppError::internal(format!(
            "project-workflow {field} is invalid"
        ))),
    }
}

pub fn workflow_catalog_from_payloads(
    namespaces_payload: serde_json::Value,
    workflows_payload: serde_json::Value,
) -> Result<WorkflowCatalog, AppError> {
    let workflows = workflows_payload
        .get("workflows")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| AppError::internal("project-workflow workflows response is invalid"))?
        .iter()
        .map(|item| {
            Ok(WorkflowCatalogEntry {
                id: catalog_id(item.get("id"), "workflow id")?,
                name: item
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .filter(|name| !name.trim().is_empty())
                    .map(str::to_string)
                    .ok_or_else(|| {
                        AppError::internal("project-workflow workflow name is invalid")
                    })?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    let namespaces = namespaces_payload
        .get("namespaces")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| AppError::internal("project-workflow namespaces response is invalid"))?
        .iter()
        .map(|item| {
            Ok(WorkflowNamespaceCatalogEntry {
                id: catalog_id(item.get("id"), "namespace id")?,
                name: item
                    .get("name")
                    .or_else(|| item.get("namespace_name"))
                    .and_then(serde_json::Value::as_str)
                    .filter(|name| !name.trim().is_empty())
                    .map(str::to_string)
                    .ok_or_else(|| {
                        AppError::internal("project-workflow namespace name is invalid")
                    })?,
                workflow_id: catalog_id(item.get("workflow_id"), "namespace workflow id")?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    if namespaces.iter().any(|namespace| {
        !workflows
            .iter()
            .any(|workflow| workflow.id == namespace.workflow_id)
    }) {
        return Err(AppError::validation(
            "project-workflow namespace references an unknown workflow",
        ));
    }
    Ok(WorkflowCatalog {
        namespaces,
        workflows,
    })
}

pub async fn fetch_workflow_catalog(config: &AppConfig) -> Result<WorkflowCatalog, AppError> {
    let base = config
        .fleet
        .project_workflow_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::validation("project-workflow integration is not configured"))?
        .trim_end_matches('/');
    let token = config
        .fleet
        .project_workflow_catalog_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::Unavailable("Project Workflow catalog token is not configured".into())
        })?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|_| AppError::Unavailable("Project Workflow is unavailable".into()))?;
    let response = client
        .get(format!("{base}/internal/runtime/catalog"))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|_| AppError::Unavailable("Project Workflow is unavailable".into()))?;
    if !response.status().is_success() {
        return Err(AppError::Unavailable(
            "Project Workflow catalog is unavailable".into(),
        ));
    }
    let payload: serde_json::Value = response
        .json()
        .await
        .map_err(|_| AppError::Unavailable("Project Workflow catalog is invalid".into()))?;
    workflow_catalog_from_payloads(payload.clone(), payload)
}

pub fn workflow_rebind_selection(
    catalog: &WorkflowCatalog,
    namespace_id: &str,
    workflow_id: &str,
) -> Result<(WorkflowNamespaceCatalogEntry, WorkflowCatalogEntry), AppError> {
    let namespace = catalog
        .namespaces
        .iter()
        .find(|item| item.id == namespace_id)
        .cloned()
        .ok_or_else(|| AppError::validation("project-workflow namespace was not found"))?;
    if namespace.workflow_id != workflow_id {
        return Err(AppError::validation(
            "selected workflow does not belong to the selected namespace",
        ));
    }
    let workflow = catalog
        .workflows
        .iter()
        .find(|item| item.id == workflow_id)
        .cloned()
        .ok_or_else(|| AppError::validation("project-workflow workflow was not found"))?;
    Ok((namespace, workflow))
}

#[cfg(test)]
mod managed_settings_tests {
    use super::*;

    #[test]
    fn managed_settings_are_normalized_and_diffed_by_field() {
        let config = AppConfig::default();
        let current = managed_settings_from_config(&config);
        let mut proposed = current.clone();
        proposed.runtime.hermes_command = "  hermes-next  ".to_string();
        proposed.integrations.forge_api_url = Some("  http://forge:22801/  ".to_string());

        let proposed = normalize_managed_settings(proposed);
        let changes = managed_settings_changes(&current, &proposed).expect("settings diff");

        assert_eq!(proposed.runtime.hermes_command, "hermes-next");
        assert_eq!(
            proposed.integrations.forge_api_url.as_deref(),
            Some("http://forge:22801/")
        );
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().all(|change| change.requires_restart));
        assert!(
            changes
                .iter()
                .any(|change| change.path == "runtime.hermes_command")
        );
    }

    #[test]
    fn managed_settings_reject_invalid_runtime_and_cookie_contracts() {
        let config = AppConfig::default();
        let mut snapshot = managed_settings_from_config(&config);
        snapshot.ports.agent_port_stride = 3;
        assert!(validate_managed_settings(&snapshot, &config).is_err());

        snapshot.ports.agent_port_stride = 10;
        snapshot.auth.refresh_cookie_same_site = "None".to_string();
        snapshot.auth.refresh_cookie_secure = false;
        assert!(validate_managed_settings(&snapshot, &config).is_err());

        snapshot.auth.refresh_cookie_secure = true;
        snapshot.integrations.project_workflow_url = Some("file:///tmp/workflow".to_string());
        assert!(validate_managed_settings(&snapshot, &config).is_err());
    }

    #[test]
    fn applying_managed_settings_preserves_secret_and_infrastructure_configuration() {
        let mut base = AppConfig::default();
        base.database.url = "postgres://db/fleet".to_string();
        base.server.port = 23801;
        base.auth.jwt_secret = "jwt-secret".to_string();
        base.fleet.runtime_token_secret = "runtime-secret".to_string();
        let mut snapshot = managed_settings_from_config(&base);
        snapshot.runtime.hermes_command = "hermes-next".to_string();
        snapshot.ports.agent_port_base = 31000;

        let effective = apply_managed_settings(&base, &snapshot);

        assert_eq!(effective.fleet.hermes_command, "hermes-next");
        assert_eq!(effective.fleet.agent_port_base, 31000);
        assert_eq!(effective.server.port, 23801);
        assert_eq!(effective.database.url, "postgres://db/fleet");
        assert_eq!(effective.auth.jwt_secret, "jwt-secret");
        assert_eq!(effective.fleet.runtime_token_secret, "runtime-secret");
    }
}

#[cfg(test)]
mod alert_tests {
    use super::*;

    #[tokio::test]
    async fn workflow_catalog_requires_a_machine_token() {
        let mut config = AppConfig::default();
        config.fleet.project_workflow_url = Some("http://127.0.0.1:1".into());
        let error = fetch_workflow_catalog(&config).await.unwrap_err();
        assert!(matches!(error, AppError::Unavailable(_)));
    }

    #[test]
    fn workflow_catalog_rejects_namespace_with_unknown_workflow() {
        let error = workflow_catalog_from_payloads(
            serde_json::json!({
                "namespaces": [{"id": 1, "name": "Development", "workflow_id": 99}]
            }),
            serde_json::json!({"workflows": [{"id": 10, "name": "Development workflow"}]}),
        )
        .expect_err("namespace references a missing workflow");

        assert!(error.to_string().contains("unknown workflow"));
    }

    #[test]
    fn workflow_catalog_rejects_workflow_outside_selected_namespace() {
        let catalog = workflow_catalog_from_payloads(
            serde_json::json!({
                "namespaces": [{"id": 1, "name": "Development", "workflow_id": 10}]
            }),
            serde_json::json!({
                "workflows": [
                    {"id": 10, "name": "Development workflow"},
                    {"id": 20, "name": "QA workflow"}
                ]
            }),
        )
        .expect("valid project-workflow catalog");

        let error = workflow_rebind_selection(&catalog, "1", "20")
            .expect_err("workflow belongs to another namespace");
        assert!(error.to_string().contains("does not belong"));
    }

    #[test]
    fn healthy_to_failed_raises_critical_down() {
        let alerts = health_transition_alerts(Some("running"), "failed");
        assert_eq!(
            alerts,
            vec![("agent_down".to_string(), "critical".to_string())]
        );
    }

    #[test]
    fn recovery_resolves_down_alert() {
        let alerts = health_transition_alerts(Some("failed"), "running");
        assert_eq!(
            alerts,
            vec![("agent_recovered".to_string(), "info".to_string())]
        );
    }

    #[test]
    fn acknowledged_alerts_remain_active_until_recovery() {
        assert!(alert_state_is_active("open"));
        assert!(alert_state_is_active("acknowledged"));
        assert!(!alert_state_is_active("resolved"));
    }

    #[test]
    fn reconcile_restarts_failed_agent_with_running_desired_state() {
        use domain::AgentStatus as S;
        use domain::DesiredState as D;
        assert_eq!(
            reconcile_action(S::Failed, D::Running),
            ReconcileAction::Restart
        );
        assert_eq!(
            reconcile_action(S::Stopped, D::Running),
            ReconcileAction::Restart
        );
        // desired=stopped: no restart even after failure
        assert_eq!(
            reconcile_action(S::Failed, D::Stopped),
            ReconcileAction::None
        );
        // healthy running agent: just health checks
        assert_eq!(
            reconcile_action(S::Running, D::Running),
            ReconcileAction::HealthCheck
        );
        assert_eq!(
            reconcile_action(S::Degraded, D::Running),
            ReconcileAction::HealthCheck
        );
        // stopped as desired: nothing to do
        assert_eq!(
            reconcile_action(S::Stopped, D::Stopped),
            ReconcileAction::None
        );
        assert_eq!(
            reconcile_action(S::Ready, D::Stopped),
            ReconcileAction::None
        );
    }

    #[test]
    fn restart_loop_detected_when_restarts_within_window() {
        assert!(restart_loop_detected(3, 3));
        assert!(restart_loop_detected(5, 3));
        assert!(!restart_loop_detected(2, 3));
        assert!(!restart_loop_detected(0, 3));
    }

    #[test]
    fn heartbeat_stale_detected_for_running_desired_agent() {
        use chrono::{Duration, Utc};
        let now = Utc::now();
        assert!(heartbeat_stale(
            Some(now - Duration::minutes(30)),
            "running",
            15
        ));
        assert!(!heartbeat_stale(
            Some(now - Duration::minutes(5)),
            "running",
            15
        ));
        // stopped/degraded agents do not expect fresh heartbeats
        assert!(!heartbeat_stale(
            Some(now - Duration::hours(6)),
            "stopped",
            15
        ));
        assert!(!heartbeat_stale(
            Some(now - Duration::hours(6)),
            "degraded",
            15
        ));
        // missing heartbeat never flags (unknown monitoring state)
        assert!(!heartbeat_stale(None, "running", 15));
    }

    #[test]
    fn heartbeat_freshness_requires_a_running_agent_and_nonfuture_timestamp() {
        use chrono::{Duration, Utc};
        let observed = Utc::now();
        assert_eq!(
            heartbeat_age(Some(observed - Duration::minutes(10)), "running", observed),
            Some(Duration::minutes(10))
        );
        assert_eq!(heartbeat_age(None, "running", observed), None);
        assert_eq!(
            heartbeat_age(Some(observed + Duration::seconds(1)), "running", observed),
            None
        );
        for status in ["stopped", "degraded", "ready", "failed"] {
            assert_eq!(heartbeat_age(Some(observed), status, observed), None);
        }
    }

    #[test]
    fn loop_transition_overrides_down_alert() {
        let alerts = health_transition_alerts_ex(
            Some("running"),
            "failed",
            RestartContext {
                recent_restarts: 3,
                loop_threshold: 3,
            },
        );
        assert!(alerts.contains(&("agent_restart_loop".to_string(), "warning".to_string())));
        assert!(!alerts.contains(&("agent_down".to_string(), "critical".to_string())));
    }

    #[test]
    fn steady_states_raise_nothing() {
        assert!(health_transition_alerts(None, "running").is_empty());
        assert!(health_transition_alerts(Some("running"), "running").is_empty());
        // first observation of a failed agent (no previous) does not alert
        assert!(health_transition_alerts(None, "failed").is_empty());
    }
}
