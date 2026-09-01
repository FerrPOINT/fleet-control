pub mod auth;

use async_trait::async_trait;
use domain::{
    Agent, AgentConfig, AgentEvent, AgentKind, AgentLogEntry, AgentSession, AgentStatus,
    CreateAgentRequest, CreateSessionRequest, FleetDashboard, HandoffSessionRequest,
    RuntimeOperationResponse, RuntimeTemplate, UpdateAgentConfigRequest, UpdateAgentRequest,
    UpdateSkillRequest, UserResponse, WorkflowBinding,
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
    pub started_at: Option<shared::Timestamp>,
    pub stopped_at: Option<shared::Timestamp>,
}

#[derive(Debug, Clone, Default)]
pub struct SessionListFilter {
    pub agent_id: Option<Uuid>,
    pub user_ids: Vec<Uuid>,
}

#[async_trait]
pub trait FleetRepository: Send + Sync {
    async fn list_runtime_templates(&self) -> Result<Vec<RuntimeTemplate>, AppError>;
    async fn ensure_runtime_templates(&self) -> Result<(), AppError>;
    async fn list_agents(&self) -> Result<Vec<Agent>, AppError>;
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
    async fn handoff_session(
        &self,
        id: Uuid,
        req: HandoffSessionRequest,
    ) -> Result<AgentSession, AppError>;

    async fn list_workflow_bindings(&self) -> Result<Vec<WorkflowBinding>, AppError>;
    async fn list_events(&self, limit: u64) -> Result<Vec<AgentEvent>, AppError>;
    async fn insert_event(
        &self,
        agent_id: Option<Uuid>,
        event_type: &str,
        message: &str,
        payload: serde_json::Value,
    ) -> Result<AgentEvent, AppError>;
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
    ) -> Self {
        let (events, _) = broadcast::channel(256);
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
            .list_sessions(SessionListFilter::default())
            .await?;
        Ok(FleetDashboard {
            total_agents: agents.len(),
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
                role: domain::AgentRole::Developer,
                display_name: "Developer Hermes".to_string(),
                description: Some("Primary development workflow agent".to_string()),
                namespace_id: Some("dev".to_string()),
                namespace_name: Some("Development".to_string()),
                workflow_id: Some("workflow-dev".to_string()),
                workflow_name: Some("Developer Workflow".to_string()),
            },
            CreateAgentRequest {
                kind: AgentKind::Hermes,
                role: domain::AgentRole::Tester,
                display_name: "Tester Hermes".to_string(),
                description: Some("QA and verification workflow agent".to_string()),
                namespace_id: Some("qa".to_string()),
                namespace_name: Some("Quality Assurance".to_string()),
                workflow_id: Some("workflow-qa".to_string()),
                workflow_name: Some("Tester Workflow".to_string()),
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
