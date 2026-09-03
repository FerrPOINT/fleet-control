pub mod auth;

use async_trait::async_trait;
use domain::{
    Agent, AgentConfig, AgentDirectoryItem, AgentEvent, AgentKind, AgentLogEntry, AgentSession,
    AgentStatus, AssignSessionLeaderRequest, AuditLogEntry, AuthSettings, CreateAgentRequest,
    CreateDeploymentJobRequest, CreateSessionDelegationRequest, CreateSessionMessageRequest,
    CreateSessionRequest, DeploymentJob, FleetDashboard, HandoffSessionRequest,
    IntegrationSettings, LeaderExecutor, MessageDeliveryState, MessageKind, PortSettings,
    ResolveRuntimeApprovalRequest, RuntimeApprovalRequest, RuntimeOperationResponse,
    RuntimeRunControlResponse, RuntimeSettings, RuntimeTemplate, SessionAgentRun, SessionMessage,
    SessionParticipant, SessionRunRole, SessionRunState, SteerSessionRunRequest,
    UpdateAgentConfigRequest, UpdateAgentRequest, UpdateLeaderExecutorsRequest, UpdateSkillRequest,
    UpdateUserRoleRequest, UserResponse, WorkflowBinding,
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
    async fn list_events(&self, limit: u64) -> Result<Vec<AgentEvent>, AppError>;
    async fn insert_event(
        &self,
        agent_id: Option<Uuid>,
        event_type: &str,
        message: &str,
        payload: serde_json::Value,
    ) -> Result<AgentEvent, AppError>;
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
    async fn get_deployment_job(&self, id: Uuid) -> Result<DeploymentJob, AppError>;
    async fn create_deployment_job(
        &self,
        req: CreateDeploymentJobRequest,
        requested_by_user_id: Uuid,
    ) -> Result<DeploymentJob, AppError>;
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

impl AppContext {
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
