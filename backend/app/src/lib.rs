pub mod auth;

use async_trait::async_trait;
use domain::{
    Agent, AgentConfig, AgentDirectoryItem, AgentEvent, AgentKind, AgentLogEntry, AgentSession,
    AgentStatus, AgentStorageReport, AssignSessionLeaderRequest, AuditLogEntry, AuthSettings,
    BulkDeploymentRequest, BulkDeploymentResult, CreateAgentRequest, CreateDeploymentJobRequest,
    CreateSessionDelegationRequest, CreateSessionMessageRequest, CreateSessionRequest,
    DeploymentJob, DeploymentJobState, FleetDashboard, HandoffSessionRequest, IntegrationSettings,
    LeaderExecutor, MessageDeliveryState, MessageKind, PortSettings, PurgeAgentFilesResponse,
    ResolveRuntimeApprovalRequest, RuntimeApprovalRequest, RuntimeOperationResponse,
    RuntimeRunControlResponse, RuntimeSettings, RuntimeTemplate, SessionAgentRun, SessionMessage,
    SessionParticipant, SessionRunRole, SessionRunState, SteerSessionRunRequest,
    UpdateAgentConfigRequest, UpdateAgentRequest, UpdateLeaderExecutorsRequest, UpdateSkillRequest,
    UpdateUserRoleRequest, UserResponse, WorkflowBinding, WorkflowCatalog, WorkflowCatalogEntry,
    WorkflowNamespaceCatalogEntry,
};
use shared::{AppConfig, AppError, FleetEvent};
use std::sync::Arc;
use tokio::sync::broadcast;
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

#[async_trait]
pub trait FleetRepository: Send + Sync {
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

    async fn resolve_open_alerts_of_kind(
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
    async fn update_runtime_settings(
        &self,
        req: RuntimeSettings,
        actor_user_id: Uuid,
    ) -> Result<RuntimeSettings, AppError>;
    async fn get_port_settings(&self, config: &AppConfig) -> Result<PortSettings, AppError>;
    async fn update_port_settings(
        &self,
        req: PortSettings,
        actor_user_id: Uuid,
    ) -> Result<PortSettings, AppError>;
    async fn get_integration_settings(&self) -> Result<IntegrationSettings, AppError>;
    async fn update_integration_settings(
        &self,
        req: IntegrationSettings,
        actor_user_id: Uuid,
    ) -> Result<IntegrationSettings, AppError>;
    async fn get_auth_settings(&self, config: &AppConfig) -> Result<AuthSettings, AppError>;
    async fn update_auth_settings(
        &self,
        req: AuthSettings,
        actor_user_id: Uuid,
    ) -> Result<AuthSettings, AppError>;
}

#[async_trait]
pub trait AgentProvisioner: Send + Sync {
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
pub trait RuntimeSupervisor: Send + Sync {
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
    ) -> Result<RuntimeRunControlResponse, AppError>;
    async fn stop_run(
        &self,
        agent: &Agent,
        run: &SessionAgentRun,
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
    ) -> Self {
        let auth = auth::AuthService::new(config.auth.clone());
        Self {
            config,
            repo,
            provisioner,
            runtime,
            auth,
            events,
        }
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

// --- Fleet monitoring alerts (IMPLEMENTATION_PLAN Phase 3) ---

/// Maps a runtime health transition to the alert(s) it should raise.
/// Returns (kind, severity) pairs: `agent_down` on a previously-healthy
/// agent going down, `agent_recovered` auto-resolves open `agent_down`
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

/// True when a running-desired agent has not reported health within
/// `stale_after_minutes`. Stopped/degraded agents do not owe heartbeats.
pub fn heartbeat_stale(
    last_health_at: Option<chrono::DateTime<chrono::Utc>>,
    current_status: &str,
    stale_after_minutes: i64,
) -> bool {
    if current_status != "running" {
        return false;
    }
    match last_health_at {
        Some(ts) => chrono::Utc::now() - ts > chrono::Duration::minutes(stale_after_minutes),
        None => false,
    }
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
    if let Some(prev) = previous {
        if (prev == "failed" || prev == "stopped") && (current == "running" || current == "ready") {
            out.push(("agent_recovered".to_string(), "info".to_string()));
        }
    }
    out
}

#[async_trait]
pub trait AlertService: Send + Sync {
    /// Records health transitions for an agent: raises `agent_down` /
    /// `agent_restart_loop` alerts and resolves open down-alerts on recovery.
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
    /// Scan running agents for stale heartbeats; raises one open
    /// `agent_heartbeat_stale` alert per agent (auto-resolved on recovery).
    pub async fn record_heartbeat_freshness(&self) -> Result<(), AppError> {
        let agents = self.repository.list_agents().await?;
        for agent in &agents {
            let last_health = agent
                .runtime
                .last_health_at
                .as_deref()
                .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
                .map(|ts| ts.with_timezone(&chrono::Utc));
            let stale = heartbeat_stale(last_health, agent.status.as_str(), 10);
            if stale {
                let already_open = self
                    .repository
                    .list_fleet_alerts(Some("open"))
                    .await?
                    .iter()
                    .any(|a| a.agent_id == Some(agent.id) && a.kind == "agent_heartbeat_stale");
                if already_open {
                    continue;
                }
                self.repository
                    .insert_fleet_alert(domain::FleetAlert {
                        id: Uuid::new_v4(),
                        agent_id: Some(agent.id),
                        kind: "agent_heartbeat_stale".to_string(),
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
                    .resolve_open_alerts_of_kind(
                        agent_id,
                        "agent_down",
                        serde_json::json!({"recovered_to": current_status}),
                    )
                    .await?;
                self.repository
                    .resolve_open_alerts_of_kind(
                        agent_id,
                        "agent_restart_loop",
                        serde_json::json!({"recovered_to": current_status}),
                    )
                    .await?;
                self.repository
                    .resolve_open_alerts_of_kind(
                        agent_id,
                        "agent_heartbeat_stale",
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
