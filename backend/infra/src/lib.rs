pub mod entities;
pub mod runtime;

use app::{
    AgentProvisioner, AuditLogFilter, FleetRepository, RuntimeApprovalCreate, RuntimeStatePatch,
    SessionListFilter,
};
use async_trait::async_trait;
use domain::{
    Agent, AgentConfig, AgentDirectoryItem, AgentEvent, AgentKind, AgentLogEntry, AgentPaths,
    AgentProductRole, AgentRole, AgentRuntime, AgentSession, AgentStatus,
    AssignSessionLeaderRequest, AuditLogEntry, AuthSettings, CreateAgentRequest,
    CreateDeploymentJobRequest, CreateSessionDelegationRequest, CreateSessionMessageRequest,
    CreateSessionRequest, DeploymentJob, DeploymentJobKind, DeploymentJobState, DesiredState,
    HandoffSessionRequest, IntegrationSettings, LeaderExecutor, MessageAuthorType,
    MessageDeliveryState, MessageKind, PortSettings, PurgeAgentFilesResponse,
    ResolveRuntimeApprovalRequest, RuntimeApprovalRequest, RuntimeApprovalState, RuntimeSettings,
    RuntimeTemplate, SessionAgentRun, SessionMessage, SessionParticipant, SessionParticipantType,
    SessionRole, SessionRunRole, SessionRunState, SessionState, SessionVisibility, SkillState,
    SystemRole, UpdateAgentConfigRequest, UpdateAgentRequest, UpdateLeaderExecutorsRequest,
    UpdateSkillRequest, UpdateUserRoleRequest, UserResponse, WorkflowBinding,
};
use entities::{
    agent, agent_config, agent_event, agent_log, agent_runtime, agent_session, agent_skill,
    audit_log, control_setting, deployment_job, leader_executor, runtime_approval_request,
    runtime_template, session_agent_run, session_message, session_participant, user,
    workflow_binding,
};
use hmac::{Hmac, Mac};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectOptions, ConnectionTrait, Database,
    DatabaseBackend, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder,
    QuerySelect, Statement, TransactionTrait,
};
use sea_orm_migration::MigratorTrait;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use shared::{AppConfig, AppError, DatabaseConfig};
use std::{
    io::ErrorKind,
    path::{Component, Path, PathBuf},
    time::Duration,
};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

pub async fn connect_database(config: DatabaseConfig) -> Result<DatabaseConnection, AppError> {
    if config.url.trim().is_empty() {
        return Err(AppError::validation("database.url must be configured"));
    }
    let mut options = ConnectOptions::new(config.url);
    options
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .connect_timeout(Duration::from_secs(config.connect_timeout_seconds))
        .idle_timeout(Duration::from_secs(config.idle_timeout_seconds));
    Database::connect(options).await.map_err(AppError::database)
}

pub async fn run_migrations(config: DatabaseConfig) -> Result<(), AppError> {
    let db = connect_database(config).await?;
    migration::Migrator::up(&db, None)
        .await
        .map_err(AppError::database)
}

pub struct PostgresFleetRepository {
    db: DatabaseConnection,
}

impl PostgresFleetRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

fn now() -> shared::Timestamp {
    shared::now()
}

fn api_ts(value: shared::Timestamp) -> domain::Timestamp {
    value.to_rfc3339()
}

fn api_ts_opt(value: Option<shared::Timestamp>) -> Option<domain::Timestamp> {
    value.map(api_ts)
}

fn parse_kind(value: &str) -> AgentKind {
    value.parse().unwrap_or(AgentKind::Hermes)
}

fn parse_system_role(value: &str, is_system_admin: bool) -> SystemRole {
    value
        .parse()
        .unwrap_or_else(|_| SystemRole::from_legacy(is_system_admin))
}

fn parse_role(value: &str) -> AgentRole {
    value.parse().unwrap_or(AgentRole::Custom)
}

fn parse_product_role(value: &str) -> AgentProductRole {
    value.parse().unwrap_or(AgentProductRole::Executor)
}

fn parse_status(value: &str) -> AgentStatus {
    value.parse().unwrap_or(AgentStatus::Failed)
}

fn parse_desired(value: &str) -> DesiredState {
    value.parse().unwrap_or(DesiredState::Stopped)
}

fn parse_skill_state(value: &str) -> SkillState {
    value.parse().unwrap_or(SkillState::Missing)
}

fn parse_session_state(value: &str) -> SessionState {
    value.parse().unwrap_or(SessionState::Draft)
}

fn parse_session_visibility(value: &str) -> SessionVisibility {
    value.parse().unwrap_or(SessionVisibility::Private)
}

fn parse_participant_type(value: &str) -> SessionParticipantType {
    value.parse().unwrap_or(SessionParticipantType::User)
}

fn parse_session_role(value: &str) -> SessionRole {
    value.parse().unwrap_or(SessionRole::Observer)
}

fn parse_message_author_type(value: &str) -> MessageAuthorType {
    value.parse().unwrap_or(MessageAuthorType::System)
}

fn parse_message_kind(value: &str) -> MessageKind {
    value.parse().unwrap_or(MessageKind::SystemEvent)
}

fn parse_run_role(value: &str) -> SessionRunRole {
    value.parse().unwrap_or(SessionRunRole::Executor)
}

fn parse_run_state(value: &str) -> SessionRunState {
    value.parse().unwrap_or(SessionRunState::Pending)
}

fn parse_message_delivery_state(value: &str) -> MessageDeliveryState {
    value.parse().unwrap_or(MessageDeliveryState::Mirrored)
}

fn parse_runtime_approval_state(value: &str) -> RuntimeApprovalState {
    value.parse().unwrap_or(RuntimeApprovalState::Pending)
}

fn parse_deployment_job_kind(value: &str) -> DeploymentJobKind {
    value.parse().unwrap_or(DeploymentJobKind::Provision)
}

fn parse_deployment_job_state(value: &str) -> DeploymentJobState {
    value.parse().unwrap_or(DeploymentJobState::Failed)
}

fn agent_directory_item(agent: &Agent) -> AgentDirectoryItem {
    AgentDirectoryItem {
        id: agent.id,
        ordinal: agent.ordinal,
        name: agent.name.clone(),
        kind: agent.kind,
        product_role: agent.product_role,
        role: agent.role,
        status: agent.status,
        display_name: agent.display_name.clone(),
        description: agent.description.clone(),
        namespace_id: agent.namespace_id.clone(),
        workflow_id: agent.workflow_id.clone(),
        runtime_version: agent.runtime_version.clone(),
        dashboard_port: agent.dashboard_port,
        api_port: agent.api_port,
    }
}

fn runtime_paths(root: &str, ordinal: i32) -> AgentPaths {
    let base = Path::new(root).join(format!("agent{ordinal}"));
    AgentPaths {
        runtime: base.join("runtime").to_string_lossy().to_string(),
        config: base.join("config").to_string_lossy().to_string(),
        workspace: base.join("workspace").to_string_lossy().to_string(),
        logs: base.join("logs").to_string_lossy().to_string(),
    }
}

fn ports(config: &AppConfig, ordinal: i32) -> (i32, i32) {
    let base = config.fleet.agent_port_base as i32;
    let stride = config.fleet.agent_port_stride as i32;
    let offset = (ordinal - 1).max(0) * stride;
    (base + offset + 1, base + offset + 2)
}

fn payload_hash(value: &Value) -> Result<String, AppError> {
    let serialized = serde_json::to_vec(value).map_err(AppError::internal)?;
    let mut hasher = Sha256::new();
    hasher.update(serialized);
    Ok(hex::encode(hasher.finalize()))
}

fn redacted_env(kind: AgentKind, agent: &Agent) -> Value {
    match kind {
        AgentKind::Hermes => json!({
            "HERMES_HOME": &agent.paths.config,
            "HERMES_SERVE_HEADLESS": "1",
            "API_SERVER_ENABLED": "true",
            "API_SERVER_KEY": "redacted",
            "cwd": &agent.paths.workspace,
            "secrets": "redacted"
        }),
        AgentKind::JavaAgent => json!({
            "AGENT_SERVER_PORT": agent.api_port,
            "SPRING_CONFIG_ADDITIONAL_LOCATION": &agent.paths.config,
            "cwd": &agent.paths.workspace,
            "secrets": "redacted"
        }),
    }
}

pub fn agent_runtime_token(config: &AppConfig, agent_id: Uuid) -> Result<String, AppError> {
    let mut mac = HmacSha256::new_from_slice(config.fleet.runtime_token_secret.as_bytes())
        .map_err(AppError::internal)?;
    mac.update(b"fleet-control/hermes-api-key/v1/");
    mac.update(agent_id.to_string().as_bytes());
    Ok(format!("fc_{}", hex::encode(mac.finalize().into_bytes())))
}

fn command_preview(kind: AgentKind, config: &AppConfig, agent: &Agent) -> String {
    match kind {
        AgentKind::Hermes => format!(
            "{} serve --host 127.0.0.1 --port {}",
            config.fleet.hermes_command,
            agent.api_port.unwrap_or_default()
        ),
        AgentKind::JavaAgent => format!(
            "{} -jar runtime/backend.jar --server.port={}",
            config.fleet.java_agent_command,
            agent.api_port.unwrap_or_default()
        ),
    }
}

fn agent_from_models(agent: agent::Model, runtime: agent_runtime::Model) -> Agent {
    Agent {
        id: agent.id,
        ordinal: agent.ordinal,
        name: agent.name,
        kind: parse_kind(&agent.kind),
        product_role: parse_product_role(&agent.product_role),
        role: parse_role(&agent.role),
        status: parse_status(&agent.status),
        display_name: agent.display_name,
        description: agent.description,
        namespace_id: agent.namespace_id,
        workflow_id: agent.workflow_id,
        runtime_version: agent.runtime_version,
        dashboard_port: agent.dashboard_port,
        api_port: agent.api_port,
        paths: AgentPaths {
            runtime: agent.runtime_path,
            config: agent.config_path,
            workspace: agent.workspace_path,
            logs: agent.logs_path,
        },
        runtime: AgentRuntime {
            desired_state: parse_desired(&runtime.desired_state),
            pid: runtime.pid,
            health_status: runtime.health_status,
            health_detail: runtime.health_detail,
            command_preview: runtime.command_preview,
            env_preview: runtime.env_preview,
            last_capabilities_json: runtime.last_capabilities_json,
            startup_command_redacted: runtime.startup_command_redacted,
            started_at: api_ts_opt(runtime.started_at),
            stopped_at: api_ts_opt(runtime.stopped_at),
            last_health_at: api_ts_opt(runtime.last_health_at),
        },
        created_at: api_ts(agent.created_at),
        updated_at: api_ts(agent.updated_at),
    }
}

async fn load_agent(db: &DatabaseConnection, id: Uuid) -> Result<Agent, AppError> {
    let agent = agent::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("agent", id))?;
    let runtime = agent_runtime::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("agent_runtime", id))?;
    Ok(agent_from_models(agent, runtime))
}

async fn load_agent_row<C>(db: &C, id: Uuid) -> Result<agent::Model, AppError>
where
    C: ConnectionTrait,
{
    agent::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("agent", id))
}

async fn ensure_agent_product_role<C>(
    db: &C,
    id: Uuid,
    expected: AgentProductRole,
    label: &str,
) -> Result<agent::Model, AppError>
where
    C: ConnectionTrait,
{
    let row = load_agent_row(db, id).await?;
    let actual = parse_product_role(&row.product_role);
    if actual != expected {
        return Err(AppError::validation(format!(
            "{label} must be an {}",
            expected.as_str()
        )));
    }
    Ok(row)
}

fn user_response(model: user::Model) -> UserResponse {
    let system_role = parse_system_role(&model.system_role, model.is_system_admin);
    UserResponse {
        id: model.id,
        email: model.email,
        username: model.username,
        display_name: model.display_name,
        system_role,
        is_system_admin: model.is_system_admin,
        is_active: model.is_active,
    }
}

fn user_record(model: user::Model) -> app::auth::UserRecord {
    let system_role = parse_system_role(&model.system_role, model.is_system_admin);
    app::auth::UserRecord {
        id: model.id,
        email: model.email,
        username: model.username,
        display_name: model.display_name,
        password_hash: model.password_hash,
        refresh_token_hash: model.refresh_token_hash,
        system_role,
        is_system_admin: model.is_system_admin,
        is_active: model.is_active,
    }
}

fn audit_entry(row: audit_log::Model) -> AuditLogEntry {
    AuditLogEntry {
        id: row.id,
        actor_user_id: row.actor_user_id,
        action: row.action,
        entity_type: row.entity_type,
        entity_id: row.entity_id,
        payload: row.payload,
        created_at: api_ts(row.created_at),
    }
}

fn deployment_job_from_model(row: deployment_job::Model) -> DeploymentJob {
    DeploymentJob {
        id: row.id,
        job_kind: parse_deployment_job_kind(&row.job_kind),
        state: parse_deployment_job_state(&row.state),
        agent_id: row.agent_id,
        runtime_kind: row.runtime_kind.as_deref().map(parse_kind),
        requested_by_user_id: row.requested_by_user_id,
        title: row.title,
        detail: row.detail,
        last_error: row.last_error,
        created_at: api_ts(row.created_at),
        updated_at: api_ts(row.updated_at),
    }
}

fn runtime_settings_from_config(config: &AppConfig) -> RuntimeSettings {
    RuntimeSettings {
        agents_root: config.fleet.agents_root.clone(),
        hermes_source: config.fleet.hermes_source.clone(),
        hermes_command: config.fleet.hermes_command.clone(),
        java_agent_source: config.fleet.java_agent_source.clone(),
        java_agent_command: config.fleet.java_agent_command.clone(),
    }
}

fn port_settings_from_config(config: &AppConfig) -> PortSettings {
    PortSettings {
        backend_port: config.server.port,
        frontend_port: 23802,
        agent_port_base: config.fleet.agent_port_base,
        agent_port_stride: config.fleet.agent_port_stride,
    }
}

fn auth_settings_from_config(config: &AppConfig) -> AuthSettings {
    AuthSettings {
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
    }
}

fn default_integration_settings() -> IntegrationSettings {
    IntegrationSettings {
        project_workflow_url: None,
        project_workflow_status: "not_connected".to_string(),
        github_remote: Some("https://github.com/FerrPOINT/fleet-control".to_string()),
    }
}

async fn load_setting<T>(
    db: &DatabaseConnection,
    key: &str,
    default_value: T,
) -> Result<T, AppError>
where
    T: DeserializeOwned,
{
    let Some(row) = control_setting::Entity::find_by_id(key.to_string())
        .one(db)
        .await
        .map_err(AppError::database)?
    else {
        return Ok(default_value);
    };
    serde_json::from_value(row.value_json).map_err(AppError::internal)
}

async fn save_setting<T>(
    db: &DatabaseConnection,
    key: &str,
    value: T,
    actor_user_id: Uuid,
) -> Result<T, AppError>
where
    T: Clone + Serialize,
{
    let ts = now();
    let value_json = serde_json::to_value(value.clone()).map_err(AppError::internal)?;
    if let Some(row) = control_setting::Entity::find_by_id(key.to_string())
        .one(db)
        .await
        .map_err(AppError::database)?
    {
        let mut model = row.into_active_model();
        model.value_json = Set(redact_json(value_json));
        model.updated_by_user_id = Set(Some(actor_user_id));
        model.updated_at = Set(ts);
        model.update(db).await.map_err(AppError::database)?;
    } else {
        control_setting::Entity::insert(control_setting::ActiveModel {
            key: Set(key.to_string()),
            value_json: Set(redact_json(value_json)),
            updated_by_user_id: Set(Some(actor_user_id)),
            updated_at: Set(ts),
        })
        .exec(db)
        .await
        .map_err(AppError::database)?;
    }
    Ok(value)
}

#[async_trait]
impl FleetRepository for PostgresFleetRepository {
    async fn list_runtime_templates(&self) -> Result<Vec<RuntimeTemplate>, AppError> {
        runtime_template::Entity::find()
            .order_by_asc(runtime_template::Column::Kind)
            .all(&self.db)
            .await
            .map_err(AppError::database)
            .map(|rows| {
                rows.into_iter()
                    .map(|row| RuntimeTemplate {
                        kind: parse_kind(&row.kind),
                        display_name: row.display_name,
                        implemented: row.implemented,
                        enabled: row.enabled,
                        description: row.description,
                        capabilities: row.capabilities,
                    })
                    .collect()
            })
    }

    async fn ensure_runtime_templates(&self) -> Result<(), AppError> {
        let templates = [
            RuntimeTemplate {
                kind: AgentKind::Hermes,
                display_name: "Hermes".to_string(),
                implemented: true,
                enabled: true,
                description: "Python Hermes runtime with isolated HERMES_HOME and workspace"
                    .to_string(),
                capabilities: json!({
                    "provision": true,
                    "process_control": true,
                    "skills": true,
                    "soul": true,
                    "sessions": "/v1/runs",
                    "chat": "/v1/runs",
                    "stream": "/v1/runs/{run_id}/events",
                    "steer": "/v1/runs/{run_id}/steer",
                    "stop": "/v1/runs/{run_id}/stop",
                    "approval": "/v1/runs/{run_id}/approval",
                    "health": "/health",
                    "capabilities": "/v1/capabilities"
                }),
            },
            RuntimeTemplate {
                kind: AgentKind::JavaAgent,
                display_name: "Java Agent".to_string(),
                implemented: false,
                enabled: true,
                description: "Spring Boot Java Agent runtime contract reserved for phase 2"
                    .to_string(),
                capabilities: json!({
                    "provision": false,
                    "process_control": false,
                    "skills": "contract",
                    "sessions": "/api/v2/sessions",
                    "health": "/actuator/health",
                    "chat": "/api/v1/agent/chat/stream",
                    "openai_compatible": "/v1/*"
                }),
            },
        ];
        for template in templates {
            if runtime_template::Entity::find_by_id(template.kind.as_str().to_string())
                .one(&self.db)
                .await
                .map_err(AppError::database)?
                .is_some()
            {
                continue;
            }
            runtime_template::Entity::insert(runtime_template::ActiveModel {
                kind: Set(template.kind.as_str().to_string()),
                display_name: Set(template.display_name),
                implemented: Set(template.implemented),
                enabled: Set(template.enabled),
                description: Set(template.description),
                capabilities: Set(template.capabilities),
                updated_at: Set(now()),
            })
            .exec(&self.db)
            .await
            .map_err(AppError::database)?;
        }
        Ok(())
    }

    async fn list_agents(&self) -> Result<Vec<Agent>, AppError> {
        let agents = agent::Entity::find()
            .filter(agent::Column::Status.ne(AgentStatus::Archived.as_str()))
            .order_by_asc(agent::Column::Ordinal)
            .all(&self.db)
            .await
            .map_err(AppError::database)?;
        let mut result = Vec::with_capacity(agents.len());
        for row in agents {
            result.push(load_agent(&self.db, row.id).await?);
        }
        Ok(result)
    }

    async fn list_agent_directory(&self) -> Result<Vec<AgentDirectoryItem>, AppError> {
        Ok(self
            .list_agents()
            .await?
            .iter()
            .map(agent_directory_item)
            .collect())
    }

    async fn list_agents_by_product_role(
        &self,
        product_role: AgentProductRole,
    ) -> Result<Vec<Agent>, AppError> {
        let agents = agent::Entity::find()
            .filter(agent::Column::ProductRole.eq(product_role.as_str()))
            .filter(agent::Column::Status.ne(AgentStatus::Archived.as_str()))
            .order_by_asc(agent::Column::Ordinal)
            .all(&self.db)
            .await
            .map_err(AppError::database)?;
        let mut result = Vec::with_capacity(agents.len());
        for row in agents {
            result.push(load_agent(&self.db, row.id).await?);
        }
        Ok(result)
    }

    async fn get_agent(&self, id: Uuid) -> Result<Agent, AppError> {
        load_agent(&self.db, id).await
    }

    async fn create_agent(
        &self,
        req: CreateAgentRequest,
        config: &AppConfig,
    ) -> Result<Agent, AppError> {
        let product_role = req.product_role;
        let executor_ids = req.executor_ids.clone();
        let template = runtime_template::Entity::find_by_id(req.kind.as_str().to_string())
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::validation("unknown runtime template"))?;
        if !template.implemented {
            return Err(AppError::validation(
                "Java Agent runtime is modeled but provisioning is planned for phase 2",
            ));
        }

        let txn = self.db.begin().await.map_err(AppError::database)?;
        let ordinal = txn
            .query_one(Statement::from_string(
                DatabaseBackend::Postgres,
                "SELECT nextval('agent_ordinal_seq')::int AS ordinal".to_string(),
            ))
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::internal("agent ordinal sequence returned no value"))?
            .try_get::<i32>("", "ordinal")
            .map_err(AppError::database)?;
        let id = Uuid::new_v4();
        let name = format!("agent{ordinal}");
        let paths = runtime_paths(&config.fleet.agents_root, ordinal);
        let (api_port, dashboard_port) = ports(config, ordinal);
        let created_at = now();

        let preview_agent = Agent {
            id,
            ordinal,
            name: name.clone(),
            kind: req.kind,
            product_role: req.product_role,
            role: req.role,
            status: AgentStatus::Provisioning,
            display_name: req.display_name.clone(),
            description: req.description.clone(),
            namespace_id: req.namespace_id.clone(),
            workflow_id: req.workflow_id.clone(),
            runtime_version: None,
            dashboard_port: Some(dashboard_port),
            api_port: Some(api_port),
            paths: paths.clone(),
            runtime: AgentRuntime {
                desired_state: DesiredState::Stopped,
                pid: None,
                health_status: None,
                health_detail: None,
                command_preview: String::new(),
                env_preview: json!({}),
                last_capabilities_json: json!({}),
                startup_command_redacted: None,
                started_at: None,
                stopped_at: None,
                last_health_at: None,
            },
            created_at: api_ts(created_at),
            updated_at: api_ts(created_at),
        };

        agent::Entity::insert(agent::ActiveModel {
            id: Set(id),
            ordinal: Set(ordinal),
            name: Set(name.clone()),
            kind: Set(req.kind.as_str().to_string()),
            product_role: Set(req.product_role.as_str().to_string()),
            role: Set(req.role.as_str().to_string()),
            status: Set(AgentStatus::Provisioning.as_str().to_string()),
            display_name: Set(req.display_name),
            description: Set(req.description),
            namespace_id: Set(req.namespace_id.clone()),
            workflow_id: Set(req.workflow_id.clone()),
            runtime_version: Set(None),
            dashboard_port: Set(Some(dashboard_port)),
            api_port: Set(Some(api_port)),
            runtime_path: Set(paths.runtime),
            config_path: Set(paths.config),
            workspace_path: Set(paths.workspace),
            logs_path: Set(paths.logs),
            created_at: Set(created_at),
            updated_at: Set(created_at),
            archived_at: Set(None),
        })
        .exec(&txn)
        .await
        .map_err(AppError::database)?;

        agent_runtime::Entity::insert(agent_runtime::ActiveModel {
            agent_id: Set(id),
            desired_state: Set(DesiredState::Stopped.as_str().to_string()),
            pid: Set(None),
            health_status: Set(Some("not_started".to_string())),
            health_detail: Set(Some("Provisioned but not running".to_string())),
            command_preview: Set(command_preview(req.kind, config, &preview_agent)),
            env_preview: Set(redacted_env(req.kind, &preview_agent)),
            last_capabilities_json: Set(json!({})),
            startup_command_redacted: Set(Some(command_preview(req.kind, config, &preview_agent))),
            started_at: Set(None),
            stopped_at: Set(None),
            last_health_at: Set(None),
        })
        .exec(&txn)
        .await
        .map_err(AppError::database)?;

        agent_config::Entity::insert(agent_config::ActiveModel {
            agent_id: Set(id),
            config_json: Set(json!({
                "agent": name,
                "kind": req.kind.as_str(),
                "terminal": { "cwd": preview_agent.paths.workspace },
                "namespace_id": req.namespace_id,
                "workflow_id": req.workflow_id
            })),
            soul_md: Set(format!(
                "# {}\n\nYou are a managed {} agent in Fleet Control.\n",
                preview_agent.display_name,
                req.kind.as_str()
            )),
            env_json: Set(redacted_env(req.kind, &preview_agent)),
            updated_at: Set(created_at),
        })
        .exec(&txn)
        .await
        .map_err(AppError::database)?;

        workflow_binding::Entity::insert(workflow_binding::ActiveModel {
            id: Set(Uuid::new_v4()),
            agent_id: Set(id),
            namespace_id: Set(req.namespace_id),
            namespace_name: Set(req.namespace_name),
            workflow_id: Set(req.workflow_id),
            workflow_name: Set(req.workflow_name),
            binding_status: Set("pending".to_string()),
            created_at: Set(created_at),
            updated_at: Set(created_at),
        })
        .exec(&txn)
        .await
        .map_err(AppError::database)?;

        for (name, title) in default_skills(req.role) {
            agent_skill::Entity::insert(agent_skill::ActiveModel {
                id: Set(Uuid::new_v4()),
                agent_id: Set(id),
                name: Set(name.to_string()),
                title: Set(title.to_string()),
                state: Set(SkillState::Enabled.as_str().to_string()),
                source: Set("seed".to_string()),
                content: Set(None),
                updated_at: Set(created_at),
            })
            .exec(&txn)
            .await
            .map_err(AppError::database)?;
        }

        if product_role == AgentProductRole::Leader {
            for executor_id in executor_ids {
                ensure_agent_product_role(
                    &txn,
                    executor_id,
                    AgentProductRole::Executor,
                    "leader executor",
                )
                .await?;
                leader_executor::Entity::insert(leader_executor::ActiveModel {
                    leader_agent_id: Set(id),
                    executor_agent_id: Set(executor_id),
                    created_by_user_id: Set(None),
                    created_at: Set(created_at),
                })
                .exec(&txn)
                .await
                .map_err(AppError::database)?;
            }
        }

        txn.commit().await.map_err(AppError::database)?;
        self.get_agent(id).await
    }

    async fn update_agent(&self, id: Uuid, req: UpdateAgentRequest) -> Result<Agent, AppError> {
        let next_product_role = req.product_role;
        let next_executor_ids = req.executor_ids.clone();
        let mut model = agent::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("agent", id))?
            .into_active_model();
        if let Some(product_role) = next_product_role {
            model.product_role = Set(product_role.as_str().to_string());
        }
        if let Some(role) = req.role {
            model.role = Set(role.as_str().to_string());
        }
        if let Some(display_name) = req.display_name {
            model.display_name = Set(display_name);
        }
        if let Some(description) = req.description {
            model.description = Set(Some(description));
        }
        if let Some(namespace_id) = req.namespace_id {
            model.namespace_id = Set(Some(namespace_id));
        }
        if let Some(workflow_id) = req.workflow_id {
            model.workflow_id = Set(Some(workflow_id));
        }
        model.updated_at = Set(now());
        model.update(&self.db).await.map_err(AppError::database)?;
        if let Some(executor_ids) = next_executor_ids {
            self.replace_leader_executors(
                id,
                UpdateLeaderExecutorsRequest { executor_ids },
                Uuid::nil(),
            )
            .await?;
        } else if next_product_role == Some(AgentProductRole::Executor) {
            leader_executor::Entity::delete_many()
                .filter(leader_executor::Column::LeaderAgentId.eq(id))
                .exec(&self.db)
                .await
                .map_err(AppError::database)?;
        }
        self.get_agent(id).await
    }

    async fn update_agent_status(&self, id: Uuid, status: AgentStatus) -> Result<Agent, AppError> {
        let mut model = agent::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("agent", id))?
            .into_active_model();
        model.status = Set(status.as_str().to_string());
        model.updated_at = Set(now());
        model.update(&self.db).await.map_err(AppError::database)?;
        self.get_agent(id).await
    }

    async fn update_runtime_state(
        &self,
        id: Uuid,
        patch: RuntimeStatePatch,
    ) -> Result<Agent, AppError> {
        let txn = self.db.begin().await.map_err(AppError::database)?;
        let ts = now();
        let mut agent_model = agent::Entity::find_by_id(id)
            .one(&txn)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("agent", id))?
            .into_active_model();
        agent_model.status = Set(patch.status.as_str().to_string());
        agent_model.updated_at = Set(ts);
        agent_model.update(&txn).await.map_err(AppError::database)?;

        let mut runtime_model = agent_runtime::Entity::find_by_id(id)
            .one(&txn)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("agent_runtime", id))?
            .into_active_model();
        runtime_model.desired_state = Set(patch.desired_state.as_str().to_string());
        runtime_model.pid = Set(patch.pid);
        runtime_model.health_status = Set(patch.health_status);
        runtime_model.health_detail = Set(patch.health_detail);
        if let Some(capabilities) = patch.last_capabilities_json {
            runtime_model.last_capabilities_json = Set(capabilities);
        }
        if patch.startup_command_redacted.is_some() {
            runtime_model.startup_command_redacted = Set(patch.startup_command_redacted);
        }
        runtime_model.started_at = Set(patch.started_at);
        runtime_model.stopped_at = Set(patch.stopped_at);
        runtime_model.last_health_at = Set(Some(ts));
        runtime_model
            .update(&txn)
            .await
            .map_err(AppError::database)?;
        txn.commit().await.map_err(AppError::database)?;
        self.get_agent(id).await
    }

    async fn archive_agent(&self, id: Uuid) -> Result<Agent, AppError> {
        let mut model = agent::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("agent", id))?
            .into_active_model();
        let ts = now();
        model.status = Set(AgentStatus::Archived.as_str().to_string());
        model.updated_at = Set(ts);
        model.archived_at = Set(Some(ts));
        model.update(&self.db).await.map_err(AppError::database)?;
        self.get_agent(id).await
    }

    async fn list_leader_executors(
        &self,
        leader_agent_id: Uuid,
    ) -> Result<Vec<LeaderExecutor>, AppError> {
        ensure_agent_product_role(
            &self.db,
            leader_agent_id,
            AgentProductRole::Leader,
            "leader",
        )
        .await?;
        let rows = leader_executor::Entity::find()
            .filter(leader_executor::Column::LeaderAgentId.eq(leader_agent_id))
            .order_by_asc(leader_executor::Column::CreatedAt)
            .all(&self.db)
            .await
            .map_err(AppError::database)?;
        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            let executor = load_agent_row(&self.db, row.executor_agent_id).await?;
            result.push(LeaderExecutor {
                leader_agent_id: row.leader_agent_id,
                executor_agent_id: row.executor_agent_id,
                executor_name: executor.name,
                executor_display_name: executor.display_name,
                executor_profile: parse_role(&executor.role),
                namespace_id: executor.namespace_id,
                workflow_id: executor.workflow_id,
                created_by_user_id: row.created_by_user_id,
                created_at: api_ts(row.created_at),
            });
        }
        Ok(result)
    }

    async fn replace_leader_executors(
        &self,
        leader_agent_id: Uuid,
        req: UpdateLeaderExecutorsRequest,
        actor_user_id: Uuid,
    ) -> Result<Vec<LeaderExecutor>, AppError> {
        let txn = self.db.begin().await.map_err(AppError::database)?;
        ensure_agent_product_role(&txn, leader_agent_id, AgentProductRole::Leader, "leader")
            .await?;
        for executor_id in &req.executor_ids {
            if *executor_id == leader_agent_id {
                return Err(AppError::validation("leader cannot manage itself"));
            }
            ensure_agent_product_role(
                &txn,
                *executor_id,
                AgentProductRole::Executor,
                "leader executor",
            )
            .await?;
        }
        leader_executor::Entity::delete_many()
            .filter(leader_executor::Column::LeaderAgentId.eq(leader_agent_id))
            .exec(&txn)
            .await
            .map_err(AppError::database)?;
        let ts = now();
        for executor_id in req.executor_ids {
            leader_executor::Entity::insert(leader_executor::ActiveModel {
                leader_agent_id: Set(leader_agent_id),
                executor_agent_id: Set(executor_id),
                created_by_user_id: Set((actor_user_id != Uuid::nil()).then_some(actor_user_id)),
                created_at: Set(ts),
            })
            .exec(&txn)
            .await
            .map_err(AppError::database)?;
        }
        txn.commit().await.map_err(AppError::database)?;
        self.list_leader_executors(leader_agent_id).await
    }

    async fn get_agent_config(&self, agent_id: Uuid) -> Result<AgentConfig, AppError> {
        agent_config::Entity::find_by_id(agent_id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .map(|row| AgentConfig {
                agent_id: row.agent_id,
                config_json: row.config_json,
                soul_md: row.soul_md,
                env_json: row.env_json,
                updated_at: api_ts(row.updated_at),
            })
            .ok_or_else(|| AppError::not_found("agent_config", agent_id))
    }

    async fn update_agent_config(
        &self,
        agent_id: Uuid,
        req: UpdateAgentConfigRequest,
    ) -> Result<AgentConfig, AppError> {
        let mut model = agent_config::Entity::find_by_id(agent_id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("agent_config", agent_id))?
            .into_active_model();
        model.config_json = Set(req.config_json);
        model.soul_md = Set(req.soul_md);
        model.env_json = Set(redact_json(req.env_json));
        model.updated_at = Set(now());
        model.update(&self.db).await.map_err(AppError::database)?;
        self.get_agent_config(agent_id).await
    }

    async fn list_agent_skills(&self, agent_id: Uuid) -> Result<Vec<domain::AgentSkill>, AppError> {
        agent_skill::Entity::find()
            .filter(agent_skill::Column::AgentId.eq(agent_id))
            .order_by_asc(agent_skill::Column::Name)
            .all(&self.db)
            .await
            .map_err(AppError::database)
            .map(|rows| {
                rows.into_iter()
                    .map(|row| domain::AgentSkill {
                        id: row.id,
                        agent_id: row.agent_id,
                        name: row.name,
                        title: row.title,
                        state: parse_skill_state(&row.state),
                        source: row.source,
                        content: row.content,
                        updated_at: api_ts(row.updated_at),
                    })
                    .collect()
            })
    }

    async fn update_agent_skill(
        &self,
        agent_id: Uuid,
        name: String,
        req: UpdateSkillRequest,
    ) -> Result<domain::AgentSkill, AppError> {
        let existing = agent_skill::Entity::find()
            .filter(agent_skill::Column::AgentId.eq(agent_id))
            .filter(agent_skill::Column::Name.eq(name.clone()))
            .one(&self.db)
            .await
            .map_err(AppError::database)?;
        let updated_at = now();
        match existing {
            Some(row) => {
                let mut model = row.into_active_model();
                model.state = Set(req.state.as_str().to_string());
                model.content = Set(req.content);
                model.updated_at = Set(updated_at);
                model.update(&self.db).await.map_err(AppError::database)?;
            }
            None => {
                agent_skill::Entity::insert(agent_skill::ActiveModel {
                    id: Set(Uuid::new_v4()),
                    agent_id: Set(agent_id),
                    title: Set(titleize(&name)),
                    name: Set(name.clone()),
                    state: Set(req.state.as_str().to_string()),
                    source: Set("manual".to_string()),
                    content: Set(req.content),
                    updated_at: Set(updated_at),
                })
                .exec(&self.db)
                .await
                .map_err(AppError::database)?;
            }
        }
        self.list_agent_skills(agent_id)
            .await?
            .into_iter()
            .find(|skill| skill.name == name)
            .ok_or_else(|| AppError::not_found("agent_skill", name))
    }

    async fn list_sessions(
        &self,
        filter: SessionListFilter,
    ) -> Result<Vec<AgentSession>, AppError> {
        let mut query =
            agent_session::Entity::find().order_by_desc(agent_session::Column::UpdatedAt);
        if let Some(agent_id) = filter.agent_id {
            query = query.filter(agent_session::Column::AgentId.eq(agent_id));
        }
        if let Some(leader_agent_id) = filter.leader_agent_id {
            query = query.filter(agent_session::Column::LeaderAgentId.eq(leader_agent_id));
        }
        if !filter.user_ids.is_empty() {
            query = query.filter(agent_session::Column::UserId.is_in(filter.user_ids));
        } else if !filter.include_all_users {
            return Ok(Vec::new());
        }
        let sessions = query
            .limit(200)
            .all(&self.db)
            .await
            .map_err(AppError::database)?;
        let mut result = Vec::with_capacity(sessions.len());
        for row in sessions {
            let agent = agent::Entity::find_by_id(row.agent_id)
                .one(&self.db)
                .await
                .map_err(AppError::database)?
                .ok_or_else(|| AppError::not_found("agent", row.agent_id))?;
            let leader = match row.leader_agent_id {
                Some(leader_agent_id) => agent::Entity::find_by_id(leader_agent_id)
                    .one(&self.db)
                    .await
                    .map_err(AppError::database)?,
                None => None,
            };
            let user = user::Entity::find_by_id(row.user_id)
                .one(&self.db)
                .await
                .map_err(AppError::database)?
                .ok_or_else(|| AppError::not_found("user", row.user_id))?;
            result.push(session_from_model(row, agent, leader, user));
        }
        Ok(result)
    }

    async fn get_session(&self, id: Uuid) -> Result<AgentSession, AppError> {
        let row = agent_session::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("agent_session", id))?;
        let agent = agent::Entity::find_by_id(row.agent_id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("agent", row.agent_id))?;
        let leader = match row.leader_agent_id {
            Some(leader_agent_id) => agent::Entity::find_by_id(leader_agent_id)
                .one(&self.db)
                .await
                .map_err(AppError::database)?,
            None => None,
        };
        let user = user::Entity::find_by_id(row.user_id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("user", row.user_id))?;
        Ok(session_from_model(row, agent, leader, user))
    }

    async fn create_session(
        &self,
        req: CreateSessionRequest,
        user_id: Uuid,
    ) -> Result<AgentSession, AppError> {
        let idempotency_key = req
            .idempotency_key
            .as_ref()
            .map(|key| key.trim().to_string())
            .filter(|key| !key.is_empty());
        let idempotency_payload_hash = match idempotency_key.as_ref() {
            Some(_) => Some(payload_hash(
                &serde_json::to_value(&req).map_err(AppError::internal)?,
            )?),
            None => None,
        };
        if let Some(key) = idempotency_key.as_ref()
            && let Some(existing) = agent_session::Entity::find()
                .filter(agent_session::Column::UserId.eq(user_id))
                .filter(agent_session::Column::IdempotencyKey.eq(key))
                .one(&self.db)
                .await
                .map_err(AppError::database)?
        {
            if existing.idempotency_payload_hash == idempotency_payload_hash {
                return self.get_session(existing.id).await;
            }
            return Err(AppError::conflict(
                "idempotency_key was already used with a different session payload",
            ));
        }
        let primary_agent_id = selected_primary_agent_id(&req)?;
        let agent = load_agent_row(&self.db, primary_agent_id).await?;
        user::Entity::find_by_id(user_id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("user", user_id))?;
        let parent = match req.parent_session_id {
            Some(parent_session_id) => Some(
                agent_session::Entity::find_by_id(parent_session_id)
                    .one(&self.db)
                    .await
                    .map_err(AppError::database)?
                    .ok_or_else(|| AppError::not_found("parent_session", parent_session_id))?,
            ),
            None => None,
        };
        let primary_product_role = parse_product_role(&agent.product_role);
        let leader_agent_id = req
            .leader_agent_id
            .or_else(|| parent.as_ref().and_then(|session| session.leader_agent_id))
            .or_else(|| {
                (primary_product_role == AgentProductRole::Leader).then_some(primary_agent_id)
            });

        if let Some(leader_id) = leader_agent_id {
            ensure_agent_product_role(&self.db, leader_id, AgentProductRole::Leader, "leader")
                .await?;
            if primary_product_role == AgentProductRole::Leader && leader_id != primary_agent_id {
                return Err(AppError::validation(
                    "leader chat must use the same primary and leader agent",
                ));
            }
            if primary_product_role == AgentProductRole::Executor {
                let allowed = leader_executor::Entity::find_by_id((leader_id, primary_agent_id))
                    .one(&self.db)
                    .await
                    .map_err(AppError::database)?
                    .is_some();
                if !allowed {
                    return Err(AppError::validation(
                        "selected leader does not manage this executor",
                    ));
                }
            }
        }

        let id = Uuid::new_v4();
        let ts = now();
        let visibility = if leader_agent_id.is_some() {
            SessionVisibility::LeaderScoped
        } else {
            SessionVisibility::Private
        };
        let txn = self.db.begin().await.map_err(AppError::database)?;
        agent_session::Entity::insert(agent_session::ActiveModel {
            id: Set(id),
            agent_id: Set(primary_agent_id),
            user_id: Set(user_id),
            leader_agent_id: Set(leader_agent_id),
            parent_session_id: Set(req.parent_session_id),
            created_by_leader_agent_id: Set(parent
                .as_ref()
                .and_then(|session| session.leader_agent_id)),
            visibility: Set(visibility.as_str().to_string()),
            title: Set(req.title),
            task_key: Set(req.task_key),
            state: Set(SessionState::Draft.as_str().to_string()),
            namespace_id: Set(req.namespace_id.or(agent.namespace_id.clone())),
            external_session_id: Set(None),
            last_message_preview: Set(Some("Session created in Fleet Control".to_string())),
            idempotency_key: Set(idempotency_key.clone()),
            idempotency_payload_hash: Set(idempotency_payload_hash.clone()),
            created_at: Set(ts),
            updated_at: Set(ts),
        })
        .exec(&txn)
        .await
        .map_err(AppError::database)?;
        session_participant::Entity::insert(session_participant::ActiveModel {
            id: Set(Uuid::new_v4()),
            session_id: Set(id),
            participant_type: Set(SessionParticipantType::User.as_str().to_string()),
            user_id: Set(Some(user_id)),
            agent_id: Set(None),
            session_role: Set(SessionRole::Owner.as_str().to_string()),
            created_at: Set(ts),
        })
        .exec(&txn)
        .await
        .map_err(AppError::database)?;
        session_participant::Entity::insert(session_participant::ActiveModel {
            id: Set(Uuid::new_v4()),
            session_id: Set(id),
            participant_type: Set(SessionParticipantType::Agent.as_str().to_string()),
            user_id: Set(None),
            agent_id: Set(Some(primary_agent_id)),
            session_role: Set(SessionRole::Primary.as_str().to_string()),
            created_at: Set(ts),
        })
        .exec(&txn)
        .await
        .map_err(AppError::database)?;
        if let Some(leader_id) = leader_agent_id {
            session_participant::Entity::insert(session_participant::ActiveModel {
                id: Set(Uuid::new_v4()),
                session_id: Set(id),
                participant_type: Set(SessionParticipantType::Agent.as_str().to_string()),
                user_id: Set(None),
                agent_id: Set(Some(leader_id)),
                session_role: Set(SessionRole::Leader.as_str().to_string()),
                created_at: Set(ts),
            })
            .exec(&txn)
            .await
            .map_err(AppError::database)?;
        }
        let primary_run_role = if primary_product_role == AgentProductRole::Leader {
            SessionRunRole::Leader
        } else {
            SessionRunRole::Primary
        };
        let id = Uuid::new_v4();
        session_agent_run::Entity::insert(session_agent_run::ActiveModel {
            id: Set(id),
            session_id: Set(id),
            agent_id: Set(primary_agent_id),
            runtime_session_id: Set(None),
            runtime_run_id: Set(None),
            run_role: Set(primary_run_role.as_str().to_string()),
            state: Set(SessionRunState::Pending.as_str().to_string()),
            last_error: Set(None),
            last_event_at: Set(None),
            model: Set(None),
            provider: Set(None),
            model_options: Set(json!({})),
            created_at: Set(ts),
            updated_at: Set(ts),
        })
        .exec(&txn)
        .await
        .map_err(AppError::database)?;
        session_message::Entity::insert(session_message::ActiveModel {
            id: Set(Uuid::new_v4()),
            session_id: Set(id),
            author_type: Set(MessageAuthorType::System.as_str().to_string()),
            author_user_id: Set(None),
            author_agent_id: Set(None),
            body: Set("Session created in Fleet Control".to_string()),
            message_kind: Set(MessageKind::SystemEvent.as_str().to_string()),
            runtime_message_id: Set(None),
            idempotency_key: Set(None),
            idempotency_payload_hash: Set(None),
            created_by_user_id: Set(Some(user_id)),
            delivery_state: Set(MessageDeliveryState::Mirrored.as_str().to_string()),
            delivery_error: Set(None),
            created_at: Set(ts),
        })
        .exec(&txn)
        .await
        .map_err(AppError::database)?;
        txn.commit().await.map_err(AppError::database)?;
        self.get_session(id).await
    }

    async fn create_session_delegation(
        &self,
        parent_session_id: Uuid,
        req: CreateSessionDelegationRequest,
        user_id: Uuid,
    ) -> Result<AgentSession, AppError> {
        let parent = self.get_session(parent_session_id).await?;
        if parent.user_id != user_id {
            return Err(AppError::Forbidden);
        }
        if parent.visibility != SessionVisibility::LeaderScoped {
            return Err(AppError::validation(
                "delegations can be created only from a leader-scoped session",
            ));
        }
        let leader_agent_id = parent.leader_agent_id.ok_or_else(|| {
            AppError::validation("leader-scoped parent session is missing leader_agent_id")
        })?;
        ensure_agent_product_role(
            &self.db,
            req.executor_agent_id,
            AgentProductRole::Executor,
            "executor",
        )
        .await?;
        let allowed = leader_executor::Entity::find_by_id((leader_agent_id, req.executor_agent_id))
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .is_some();
        if !allowed {
            return Err(AppError::validation(
                "parent leader does not manage selected executor",
            ));
        }
        let initial_message = req.initial_message.clone();
        let idempotency_key = req.idempotency_key.clone();

        let child = self
            .create_session(
                CreateSessionRequest {
                    primary_agent_id: Some(req.executor_agent_id),
                    agent_id: None,
                    title: req.title,
                    task_key: req.task_key,
                    leader_agent_id: Some(leader_agent_id),
                    parent_session_id: Some(parent_session_id),
                    namespace_id: None,
                    idempotency_key: idempotency_key.clone(),
                },
                user_id,
            )
            .await?;

        if let Some(initial_message) = initial_message
            .as_ref()
            .map(|body| body.trim().to_string())
            .filter(|body| !body.is_empty())
        {
            let message_key = idempotency_key
                .as_ref()
                .map(|key| format!("delegation:{key}:initial_message"));
            let _ = self
                .create_session_message(
                    child.id,
                    CreateSessionMessageRequest {
                        body: initial_message,
                        author_agent_id: Some(leader_agent_id),
                        message_kind: Some(MessageKind::UserPrompt),
                        runtime_message_id: None,
                        idempotency_key: message_key,
                    },
                    user_id,
                )
                .await?;
        }

        self.get_session(child.id).await
    }

    async fn assign_session_leader(
        &self,
        id: Uuid,
        req: AssignSessionLeaderRequest,
        actor_user_id: Uuid,
    ) -> Result<AgentSession, AppError> {
        let session = agent_session::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("agent_session", id))?;
        let primary = load_agent_row(&self.db, session.agent_id).await?;
        let primary_product_role = parse_product_role(&primary.product_role);
        if let Some(leader_id) = req.leader_agent_id {
            ensure_agent_product_role(&self.db, leader_id, AgentProductRole::Leader, "leader")
                .await?;
            if primary_product_role == AgentProductRole::Leader && leader_id != primary.id {
                return Err(AppError::validation(
                    "leader chat must use the same primary and leader agent",
                ));
            }
            if primary_product_role == AgentProductRole::Executor {
                let allowed = leader_executor::Entity::find_by_id((leader_id, primary.id))
                    .one(&self.db)
                    .await
                    .map_err(AppError::database)?
                    .is_some();
                if !allowed {
                    return Err(AppError::validation(
                        "selected leader does not manage this executor",
                    ));
                }
            }
        }

        let txn = self.db.begin().await.map_err(AppError::database)?;
        let ts = now();
        let mut model = session.into_active_model();
        model.leader_agent_id = Set(req.leader_agent_id);
        model.visibility = Set(req
            .leader_agent_id
            .map(|_| SessionVisibility::LeaderScoped)
            .unwrap_or(SessionVisibility::Private)
            .as_str()
            .to_string());
        model.last_message_preview = Set(Some(match req.leader_agent_id {
            Some(leader_id) => format!("Leader selected: {leader_id}"),
            None => "Leader removed; session is private".to_string(),
        }));
        model.updated_at = Set(ts);
        model.update(&txn).await.map_err(AppError::database)?;

        session_participant::Entity::delete_many()
            .filter(session_participant::Column::SessionId.eq(id))
            .filter(session_participant::Column::SessionRole.eq(SessionRole::Leader.as_str()))
            .exec(&txn)
            .await
            .map_err(AppError::database)?;
        session_agent_run::Entity::delete_many()
            .filter(session_agent_run::Column::SessionId.eq(id))
            .filter(session_agent_run::Column::RunRole.eq(SessionRunRole::Leader.as_str()))
            .exec(&txn)
            .await
            .map_err(AppError::database)?;

        if let Some(leader_id) = req.leader_agent_id {
            session_participant::Entity::insert(session_participant::ActiveModel {
                id: Set(Uuid::new_v4()),
                session_id: Set(id),
                participant_type: Set(SessionParticipantType::Agent.as_str().to_string()),
                user_id: Set(None),
                agent_id: Set(Some(leader_id)),
                session_role: Set(SessionRole::Leader.as_str().to_string()),
                created_at: Set(ts),
            })
            .exec(&txn)
            .await
            .map_err(AppError::database)?;
            session_agent_run::Entity::insert(session_agent_run::ActiveModel {
                id: Set(Uuid::new_v4()),
                session_id: Set(id),
                agent_id: Set(leader_id),
                runtime_session_id: Set(None),
                runtime_run_id: Set(None),
                run_role: Set(SessionRunRole::Leader.as_str().to_string()),
                state: Set(SessionRunState::Pending.as_str().to_string()),
                last_error: Set(None),
                last_event_at: Set(None),
                model: Set(None),
                provider: Set(None),
                model_options: Set(json!({})),
                created_at: Set(ts),
                updated_at: Set(ts),
            })
            .exec(&txn)
            .await
            .map_err(AppError::database)?;
        }

        session_message::Entity::insert(session_message::ActiveModel {
            id: Set(Uuid::new_v4()),
            session_id: Set(id),
            author_type: Set(MessageAuthorType::System.as_str().to_string()),
            author_user_id: Set(None),
            author_agent_id: Set(None),
            body: Set(match req.leader_agent_id {
                Some(leader_id) => format!("Leader selected by {actor_user_id}: {leader_id}"),
                None => format!("Leader removed by {actor_user_id}"),
            }),
            message_kind: Set(MessageKind::Control.as_str().to_string()),
            runtime_message_id: Set(None),
            idempotency_key: Set(None),
            idempotency_payload_hash: Set(None),
            created_by_user_id: Set(Some(actor_user_id)),
            delivery_state: Set(MessageDeliveryState::Mirrored.as_str().to_string()),
            delivery_error: Set(None),
            created_at: Set(ts),
        })
        .exec(&txn)
        .await
        .map_err(AppError::database)?;

        txn.commit().await.map_err(AppError::database)?;
        self.get_session(id).await
    }

    async fn handoff_session(
        &self,
        id: Uuid,
        req: HandoffSessionRequest,
    ) -> Result<AgentSession, AppError> {
        let target = load_agent_row(&self.db, req.target_agent_id).await?;
        let session = agent_session::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("agent_session", id))?;
        if let Some(leader_id) = session.leader_agent_id {
            let target_product_role = parse_product_role(&target.product_role);
            if target_product_role == AgentProductRole::Executor {
                let allowed = leader_executor::Entity::find_by_id((leader_id, target.id))
                    .one(&self.db)
                    .await
                    .map_err(AppError::database)?
                    .is_some();
                if !allowed {
                    return Err(AppError::validation(
                        "selected leader does not manage handoff target",
                    ));
                }
            }
        }
        let ts = now();
        let txn = self.db.begin().await.map_err(AppError::database)?;
        let mut model = session.into_active_model();
        model.agent_id = Set(req.target_agent_id);
        model.state = Set(SessionState::HandoffRequested.as_str().to_string());
        model.namespace_id = Set(target.namespace_id);
        model.last_message_preview = Set(Some(format!("Handoff requested to {}", target.name)));
        model.updated_at = Set(ts);
        model.update(&txn).await.map_err(AppError::database)?;

        session_participant::Entity::delete_many()
            .filter(session_participant::Column::SessionId.eq(id))
            .filter(session_participant::Column::SessionRole.eq(SessionRole::Primary.as_str()))
            .exec(&txn)
            .await
            .map_err(AppError::database)?;
        session_participant::Entity::insert(session_participant::ActiveModel {
            id: Set(Uuid::new_v4()),
            session_id: Set(id),
            participant_type: Set(SessionParticipantType::Agent.as_str().to_string()),
            user_id: Set(None),
            agent_id: Set(Some(req.target_agent_id)),
            session_role: Set(SessionRole::Primary.as_str().to_string()),
            created_at: Set(ts),
        })
        .exec(&txn)
        .await
        .map_err(AppError::database)?;
        let id = Uuid::new_v4();
        session_agent_run::Entity::insert(session_agent_run::ActiveModel {
            id: Set(id),
            session_id: Set(id),
            agent_id: Set(req.target_agent_id),
            runtime_session_id: Set(None),
            runtime_run_id: Set(None),
            run_role: Set(SessionRunRole::Primary.as_str().to_string()),
            state: Set(SessionRunState::Pending.as_str().to_string()),
            last_error: Set(None),
            last_event_at: Set(None),
            model: Set(None),
            provider: Set(None),
            model_options: Set(json!({})),
            created_at: Set(ts),
            updated_at: Set(ts),
        })
        .exec(&txn)
        .await
        .map_err(AppError::database)?;
        session_message::Entity::insert(session_message::ActiveModel {
            id: Set(Uuid::new_v4()),
            session_id: Set(id),
            author_type: Set(MessageAuthorType::System.as_str().to_string()),
            author_user_id: Set(None),
            author_agent_id: Set(None),
            body: Set(format!("Handoff requested to {}", req.target_agent_id)),
            message_kind: Set(MessageKind::Control.as_str().to_string()),
            runtime_message_id: Set(None),
            idempotency_key: Set(None),
            idempotency_payload_hash: Set(None),
            created_by_user_id: Set(None),
            delivery_state: Set(MessageDeliveryState::Mirrored.as_str().to_string()),
            delivery_error: Set(None),
            created_at: Set(ts),
        })
        .exec(&txn)
        .await
        .map_err(AppError::database)?;
        txn.commit().await.map_err(AppError::database)?;
        self.get_session(id).await
    }

    async fn list_session_messages(&self, id: Uuid) -> Result<Vec<SessionMessage>, AppError> {
        let rows = session_message::Entity::find()
            .filter(session_message::Column::SessionId.eq(id))
            .order_by_asc(session_message::Column::CreatedAt)
            .order_by_asc(session_message::Column::Id)
            .limit(500)
            .all(&self.db)
            .await
            .map_err(AppError::database)?;
        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            let author_type = parse_message_author_type(&row.author_type);
            let author_user = match row.author_user_id {
                Some(user_id) => user::Entity::find_by_id(user_id)
                    .one(&self.db)
                    .await
                    .map_err(AppError::database)?,
                None => None,
            };
            let author_agent = match row.author_agent_id {
                Some(agent_id) => agent::Entity::find_by_id(agent_id)
                    .one(&self.db)
                    .await
                    .map_err(AppError::database)?,
                None => None,
            };
            result.push(SessionMessage {
                id: row.id,
                session_id: row.session_id,
                author_type,
                author_user_id: row.author_user_id,
                author_agent_id: row.author_agent_id,
                author_display_name: message_author_display_name(
                    author_type,
                    author_user.as_ref(),
                    author_agent.as_ref(),
                ),
                body: row.body,
                message_kind: parse_message_kind(&row.message_kind),
                runtime_message_id: row.runtime_message_id,
                delivery_state: parse_message_delivery_state(&row.delivery_state),
                delivery_error: row.delivery_error,
                replayed: false,
                created_at: api_ts(row.created_at),
            });
        }
        Ok(result)
    }

    async fn list_session_participants(
        &self,
        id: Uuid,
    ) -> Result<Vec<SessionParticipant>, AppError> {
        let rows = session_participant::Entity::find()
            .filter(session_participant::Column::SessionId.eq(id))
            .order_by_asc(session_participant::Column::CreatedAt)
            .all(&self.db)
            .await
            .map_err(AppError::database)?;
        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            let participant_type = parse_participant_type(&row.participant_type);
            let participant_user = match row.user_id {
                Some(user_id) => user::Entity::find_by_id(user_id)
                    .one(&self.db)
                    .await
                    .map_err(AppError::database)?,
                None => None,
            };
            let participant_agent = match row.agent_id {
                Some(agent_id) => agent::Entity::find_by_id(agent_id)
                    .one(&self.db)
                    .await
                    .map_err(AppError::database)?,
                None => None,
            };
            result.push(SessionParticipant {
                id: row.id,
                session_id: row.session_id,
                participant_type,
                user_id: row.user_id,
                agent_id: row.agent_id,
                session_role: parse_session_role(&row.session_role),
                display_name: participant_display_name(
                    participant_type,
                    participant_user.as_ref(),
                    participant_agent.as_ref(),
                ),
                created_at: api_ts(row.created_at),
            });
        }
        Ok(result)
    }

    async fn create_session_message(
        &self,
        id: Uuid,
        req: CreateSessionMessageRequest,
        actor_user_id: Uuid,
    ) -> Result<SessionMessage, AppError> {
        let body = req.body.trim().to_string();
        if body.is_empty() {
            return Err(AppError::validation("message body is required"));
        }
        let idempotency_key = req
            .idempotency_key
            .as_ref()
            .map(|key| key.trim().to_string())
            .filter(|key| !key.is_empty());
        let idempotency_payload_hash = match idempotency_key.as_ref() {
            Some(_) => Some(payload_hash(
                &serde_json::to_value(&req).map_err(AppError::internal)?,
            )?),
            None => None,
        };
        let session = agent_session::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("agent_session", id))?;
        if actor_user_id != session.user_id {
            return Err(AppError::Forbidden);
        }
        if let Some(key) = idempotency_key.as_ref()
            && let Some(existing) = session_message::Entity::find()
                .filter(session_message::Column::SessionId.eq(id))
                .filter(session_message::Column::CreatedByUserId.eq(actor_user_id))
                .filter(session_message::Column::IdempotencyKey.eq(key))
                .one(&self.db)
                .await
                .map_err(AppError::database)?
        {
            if existing.idempotency_payload_hash == idempotency_payload_hash {
                let mut message = self
                    .list_session_messages(id)
                    .await?
                    .into_iter()
                    .find(|message| message.id == existing.id)
                    .ok_or_else(|| AppError::not_found("session_message", existing.id))?;
                message.replayed = true;
                return Ok(message);
            }
            return Err(AppError::conflict(
                "idempotency_key was already used with a different message payload",
            ));
        }

        let (author_type, author_user_id, author_agent_id, message_kind) =
            if let Some(agent_id) = req.author_agent_id {
                if Some(agent_id) != session.leader_agent_id && agent_id != session.agent_id {
                    return Err(AppError::validation(
                        "agent author must be the session primary agent or selected leader",
                    ));
                }
                (
                    MessageAuthorType::Agent,
                    None,
                    Some(agent_id),
                    req.message_kind.unwrap_or(MessageKind::AssistantMessage),
                )
            } else {
                (
                    MessageAuthorType::User,
                    Some(actor_user_id),
                    None,
                    req.message_kind.unwrap_or(MessageKind::UserPrompt),
                )
            };
        let message_id = Uuid::new_v4();
        let ts = now();
        let txn = self.db.begin().await.map_err(AppError::database)?;
        session_message::Entity::insert(session_message::ActiveModel {
            id: Set(message_id),
            session_id: Set(id),
            author_type: Set(author_type.as_str().to_string()),
            author_user_id: Set(author_user_id),
            author_agent_id: Set(author_agent_id),
            body: Set(body.clone()),
            message_kind: Set(message_kind.as_str().to_string()),
            runtime_message_id: Set(req.runtime_message_id),
            idempotency_key: Set(idempotency_key),
            idempotency_payload_hash: Set(idempotency_payload_hash),
            created_by_user_id: Set(Some(actor_user_id)),
            delivery_state: Set(match message_kind {
                MessageKind::UserPrompt | MessageKind::Control => {
                    MessageDeliveryState::Pending.as_str().to_string()
                }
                _ => MessageDeliveryState::Mirrored.as_str().to_string(),
            }),
            delivery_error: Set(None),
            created_at: Set(ts),
        })
        .exec(&txn)
        .await
        .map_err(AppError::database)?;
        let mut session_model = session.into_active_model();
        session_model.last_message_preview = Set(Some(body.chars().take(180).collect()));
        session_model.updated_at = Set(ts);
        if message_kind == MessageKind::UserPrompt {
            session_model.state = Set(SessionState::Active.as_str().to_string());
        }
        session_model
            .update(&txn)
            .await
            .map_err(AppError::database)?;
        txn.commit().await.map_err(AppError::database)?;
        self.list_session_messages(id)
            .await?
            .into_iter()
            .find(|message| message.id == message_id)
            .ok_or_else(|| AppError::not_found("session_message", message_id))
    }

    async fn list_session_agent_runs(&self, id: Uuid) -> Result<Vec<SessionAgentRun>, AppError> {
        let rows = session_agent_run::Entity::find()
            .filter(session_agent_run::Column::SessionId.eq(id))
            .order_by_asc(session_agent_run::Column::CreatedAt)
            .all(&self.db)
            .await
            .map_err(AppError::database)?;
        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            result.push(session_run_from_model(&self.db, row).await?);
        }
        Ok(result)
    }

    async fn get_session_agent_run(&self, id: Uuid) -> Result<SessionAgentRun, AppError> {
        let row = session_agent_run::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("session_agent_run", id))?;
        session_run_from_model(&self.db, row).await
    }

    async fn prepare_session_agent_run(
        &self,
        session_id: Uuid,
        agent_id: Uuid,
        run_role: SessionRunRole,
        runtime_session_id: String,
    ) -> Result<SessionAgentRun, AppError> {
        load_agent_row(&self.db, agent_id).await?;
        let ts = now();
        if let Some(row) = session_agent_run::Entity::find()
            .filter(session_agent_run::Column::SessionId.eq(session_id))
            .filter(session_agent_run::Column::AgentId.eq(agent_id))
            .filter(session_agent_run::Column::State.eq(SessionRunState::Pending.as_str()))
            .filter(session_agent_run::Column::RuntimeRunId.is_null())
            .order_by_asc(session_agent_run::Column::CreatedAt)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
        {
            let mut model = row.into_active_model();
            model.runtime_session_id = Set(Some(runtime_session_id));
            model.run_role = Set(run_role.as_str().to_string());
            model.updated_at = Set(ts);
            let updated = model.update(&self.db).await.map_err(AppError::database)?;
            return session_run_from_model(&self.db, updated).await;
        }

        let id = Uuid::new_v4();
        session_agent_run::Entity::insert(session_agent_run::ActiveModel {
            id: Set(id),
            session_id: Set(session_id),
            agent_id: Set(agent_id),
            runtime_session_id: Set(Some(runtime_session_id)),
            runtime_run_id: Set(None),
            run_role: Set(run_role.as_str().to_string()),
            state: Set(SessionRunState::Pending.as_str().to_string()),
            last_error: Set(None),
            last_event_at: Set(None),
            model: Set(None),
            provider: Set(None),
            model_options: Set(json!({})),
            created_at: Set(ts),
            updated_at: Set(ts),
        })
        .exec(&self.db)
        .await
        .map_err(AppError::database)?;

        let row = session_agent_run::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::internal("session run insert returned no row"))?;
        session_run_from_model(&self.db, row).await
    }

    async fn update_session_agent_run_dispatch(
        &self,
        id: Uuid,
        runtime_run_id: Option<String>,
        state: SessionRunState,
        last_error: Option<String>,
    ) -> Result<SessionAgentRun, AppError> {
        let mut model = session_agent_run::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("session_agent_run", id))?
            .into_active_model();
        if runtime_run_id.is_some() {
            model.runtime_run_id = Set(runtime_run_id);
        }
        model.state = Set(state.as_str().to_string());
        model.last_error = Set(last_error.map(|error| redact_text(&error)));
        model.last_event_at = Set(Some(now()));
        model.updated_at = Set(now());
        let updated = model.update(&self.db).await.map_err(AppError::database)?;
        session_run_from_model(&self.db, updated).await
    }

    async fn insert_session_message_mirror(
        &self,
        session_id: Uuid,
        author_agent_id: Option<Uuid>,
        body: String,
        message_kind: MessageKind,
        runtime_message_id: Option<String>,
    ) -> Result<SessionMessage, AppError> {
        let body = body.trim().to_string();
        if body.is_empty() {
            return Err(AppError::validation("message body is required"));
        }
        let id = Uuid::new_v4();
        let ts = now();
        let author_type = if author_agent_id.is_some() {
            MessageAuthorType::Agent
        } else {
            MessageAuthorType::System
        };
        let txn = self.db.begin().await.map_err(AppError::database)?;
        session_message::Entity::insert(session_message::ActiveModel {
            id: Set(id),
            session_id: Set(session_id),
            author_type: Set(author_type.as_str().to_string()),
            author_user_id: Set(None),
            author_agent_id: Set(author_agent_id),
            body: Set(body.clone()),
            message_kind: Set(message_kind.as_str().to_string()),
            runtime_message_id: Set(runtime_message_id),
            idempotency_key: Set(None),
            idempotency_payload_hash: Set(None),
            created_by_user_id: Set(None),
            delivery_state: Set(MessageDeliveryState::Mirrored.as_str().to_string()),
            delivery_error: Set(None),
            created_at: Set(ts),
        })
        .exec(&txn)
        .await
        .map_err(AppError::database)?;
        let mut session = agent_session::Entity::find_by_id(session_id)
            .one(&txn)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("agent_session", session_id))?
            .into_active_model();
        session.last_message_preview = Set(Some(body.chars().take(180).collect()));
        session.updated_at = Set(ts);
        session.update(&txn).await.map_err(AppError::database)?;
        txn.commit().await.map_err(AppError::database)?;
        self.list_session_messages(session_id)
            .await?
            .into_iter()
            .find(|message| message.id == id)
            .ok_or_else(|| AppError::not_found("session_message", id))
    }

    async fn update_session_message_delivery(
        &self,
        id: Uuid,
        delivery_state: MessageDeliveryState,
        runtime_message_id: Option<String>,
        delivery_error: Option<String>,
    ) -> Result<(), AppError> {
        let mut model = session_message::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("session_message", id))?
            .into_active_model();
        model.delivery_state = Set(delivery_state.as_str().to_string());
        if runtime_message_id.is_some() {
            model.runtime_message_id = Set(runtime_message_id);
        }
        model.delivery_error = Set(delivery_error.map(|error| redact_text(&error)));
        model.update(&self.db).await.map_err(AppError::database)?;
        Ok(())
    }

    async fn upsert_runtime_approval_request(
        &self,
        req: RuntimeApprovalCreate,
    ) -> Result<RuntimeApprovalRequest, AppError> {
        if let Some(runtime_approval_id) = req.runtime_approval_id.as_ref()
            && let Some(existing) = runtime_approval_request::Entity::find()
                .filter(runtime_approval_request::Column::SessionRunId.eq(req.session_run_id))
                .filter(runtime_approval_request::Column::RuntimeApprovalId.eq(runtime_approval_id))
                .one(&self.db)
                .await
                .map_err(AppError::database)?
        {
            return Ok(runtime_approval_from_model(existing));
        }
        let id = Uuid::new_v4();
        runtime_approval_request::Entity::insert(runtime_approval_request::ActiveModel {
            id: Set(id),
            session_id: Set(req.session_id),
            session_run_id: Set(req.session_run_id),
            agent_id: Set(req.agent_id),
            runtime_run_id: Set(req.runtime_run_id),
            runtime_approval_id: Set(req.runtime_approval_id),
            prompt: Set(req.prompt),
            detail: Set(redact_json(req.detail)),
            state: Set(RuntimeApprovalState::Pending.as_str().to_string()),
            resolved_by_user_id: Set(None),
            resolved_at: Set(None),
            created_at: Set(now()),
        })
        .exec(&self.db)
        .await
        .map_err(AppError::database)?;
        let row = runtime_approval_request::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("runtime_approval_request", id))?;
        Ok(runtime_approval_from_model(row))
    }

    async fn resolve_runtime_approval_request(
        &self,
        id: Uuid,
        req: ResolveRuntimeApprovalRequest,
        actor_user_id: Uuid,
    ) -> Result<RuntimeApprovalRequest, AppError> {
        let state = match req.choice.as_str() {
            "always" | "approve" | "approved" => RuntimeApprovalState::Approved,
            "deny" | "denied" | "reject" => RuntimeApprovalState::Denied,
            "cancel" | "cancelled" => RuntimeApprovalState::Cancelled,
            _ => return Err(AppError::validation("unknown approval choice")),
        };
        let mut model = runtime_approval_request::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("runtime_approval_request", id))?
            .into_active_model();
        model.state = Set(state.as_str().to_string());
        model.resolved_by_user_id = Set(Some(actor_user_id));
        model.resolved_at = Set(Some(now()));
        let updated = model.update(&self.db).await.map_err(AppError::database)?;
        Ok(runtime_approval_from_model(updated))
    }

    async fn resolve_runtime_approval_requests_for_run(
        &self,
        session_run_id: Uuid,
        req: ResolveRuntimeApprovalRequest,
        actor_user_id: Uuid,
    ) -> Result<u64, AppError> {
        let state = match req.choice.as_str() {
            "always" | "approve" | "approved" => RuntimeApprovalState::Approved,
            "deny" | "denied" | "reject" => RuntimeApprovalState::Denied,
            "cancel" | "cancelled" => RuntimeApprovalState::Cancelled,
            _ => return Err(AppError::validation("unknown approval choice")),
        };
        let result = runtime_approval_request::Entity::update_many()
            .set(runtime_approval_request::ActiveModel {
                state: Set(state.as_str().to_string()),
                resolved_by_user_id: Set(Some(actor_user_id)),
                resolved_at: Set(Some(now())),
                ..Default::default()
            })
            .filter(runtime_approval_request::Column::SessionRunId.eq(session_run_id))
            .filter(
                runtime_approval_request::Column::State
                    .eq(RuntimeApprovalState::Pending.as_str().to_string()),
            )
            .exec(&self.db)
            .await
            .map_err(AppError::database)?;
        Ok(result.rows_affected)
    }

    async fn list_workflow_bindings(&self) -> Result<Vec<WorkflowBinding>, AppError> {
        workflow_binding::Entity::find()
            .order_by_asc(workflow_binding::Column::NamespaceId)
            .all(&self.db)
            .await
            .map_err(AppError::database)
            .map(|rows| {
                rows.into_iter()
                    .map(|row| WorkflowBinding {
                        id: row.id,
                        agent_id: row.agent_id,
                        namespace_id: row.namespace_id,
                        namespace_name: row.namespace_name,
                        workflow_id: row.workflow_id,
                        workflow_name: row.workflow_name,
                        binding_status: row.binding_status,
                        created_at: api_ts(row.created_at),
                        updated_at: api_ts(row.updated_at),
                    })
                    .collect()
            })
    }

    async fn list_events(&self, limit: u64) -> Result<Vec<AgentEvent>, AppError> {
        agent_event::Entity::find()
            .order_by_desc(agent_event::Column::CreatedAt)
            .limit(limit)
            .all(&self.db)
            .await
            .map_err(AppError::database)
            .map(|rows| {
                rows.into_iter()
                    .map(|row| AgentEvent {
                        id: row.id,
                        agent_id: row.agent_id,
                        event_type: row.event_type,
                        message: row.message,
                        payload: row.payload,
                        created_at: api_ts(row.created_at),
                    })
                    .collect()
            })
    }

    async fn insert_event(
        &self,
        agent_id: Option<Uuid>,
        event_type: &str,
        message: &str,
        payload: Value,
    ) -> Result<AgentEvent, AppError> {
        let id = Uuid::new_v4();
        let ts = now();
        agent_event::Entity::insert(agent_event::ActiveModel {
            id: Set(id),
            agent_id: Set(agent_id),
            event_type: Set(event_type.to_string()),
            message: Set(message.to_string()),
            payload: Set(payload),
            created_at: Set(ts),
        })
        .exec(&self.db)
        .await
        .map_err(AppError::database)?;
        self.list_events(1)
            .await?
            .into_iter()
            .find(|event| event.id == id)
            .ok_or_else(|| AppError::not_found("agent_event", id))
    }

    async fn insert_audit(
        &self,
        actor_user_id: Option<Uuid>,
        action: &str,
        entity_type: &str,
        entity_id: Option<String>,
        payload: Value,
    ) -> Result<(), AppError> {
        audit_log::Entity::insert(audit_log::ActiveModel {
            id: Set(Uuid::new_v4()),
            actor_user_id: Set(actor_user_id),
            action: Set(action.to_string()),
            entity_type: Set(entity_type.to_string()),
            entity_id: Set(entity_id),
            payload: Set(redact_json(payload)),
            created_at: Set(now()),
        })
        .exec(&self.db)
        .await
        .map_err(AppError::database)?;
        Ok(())
    }

    async fn list_audit_log(&self, filter: AuditLogFilter) -> Result<Vec<AuditLogEntry>, AppError> {
        let mut query = audit_log::Entity::find().order_by_desc(audit_log::Column::CreatedAt);
        if let Some(actor_user_id) = filter.actor_user_id {
            query = query.filter(audit_log::Column::ActorUserId.eq(actor_user_id));
        }
        if let Some(action) = filter.action {
            query = query.filter(audit_log::Column::Action.eq(action));
        }
        if let Some(entity_type) = filter.entity_type {
            query = query.filter(audit_log::Column::EntityType.eq(entity_type));
        }
        if let Some(entity_id) = filter.entity_id {
            query = query.filter(audit_log::Column::EntityId.eq(entity_id));
        }
        if let Some(date_from) = filter.date_from {
            query = query.filter(audit_log::Column::CreatedAt.gte(date_from));
        }
        if let Some(date_to) = filter.date_to {
            query = query.filter(audit_log::Column::CreatedAt.lte(date_to));
        }
        query
            .limit(filter.limit.clamp(1, 500))
            .all(&self.db)
            .await
            .map_err(AppError::database)
            .map(|rows| rows.into_iter().map(audit_entry).collect())
    }

    async fn list_logs(
        &self,
        agent_id: Option<Uuid>,
        limit: u64,
    ) -> Result<Vec<AgentLogEntry>, AppError> {
        let mut query = agent_log::Entity::find().order_by_desc(agent_log::Column::CreatedAt);
        if let Some(agent_id) = agent_id {
            query = query.filter(agent_log::Column::AgentId.eq(agent_id));
        }
        query
            .limit(limit)
            .all(&self.db)
            .await
            .map_err(AppError::database)
            .map(|rows| {
                rows.into_iter()
                    .map(|row| AgentLogEntry {
                        id: row.id,
                        agent_id: row.agent_id,
                        stream: row.stream,
                        message: row.message,
                        created_at: api_ts(row.created_at),
                    })
                    .collect()
            })
    }

    async fn insert_log(
        &self,
        agent_id: Uuid,
        stream: &str,
        message: &str,
    ) -> Result<AgentLogEntry, AppError> {
        let id = Uuid::new_v4();
        let ts = now();
        agent_log::Entity::insert(agent_log::ActiveModel {
            id: Set(id),
            agent_id: Set(agent_id),
            stream: Set(stream.to_string()),
            message: Set(redact_text(message)),
            created_at: Set(ts),
        })
        .exec(&self.db)
        .await
        .map_err(AppError::database)?;
        self.list_logs(Some(agent_id), 1)
            .await?
            .into_iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| AppError::not_found("agent_log", id))
    }

    async fn find_user_by_email(
        &self,
        email: &str,
    ) -> Result<Option<app::auth::UserRecord>, AppError> {
        user::Entity::find()
            .filter(user::Column::Email.eq(email))
            .one(&self.db)
            .await
            .map_err(AppError::database)
            .map(|row| row.map(user_record))
    }

    async fn find_user_by_id(&self, id: Uuid) -> Result<Option<app::auth::UserRecord>, AppError> {
        user::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(AppError::database)
            .map(|row| row.map(user_record))
    }

    async fn list_users(&self) -> Result<Vec<UserResponse>, AppError> {
        user::Entity::find()
            .order_by_asc(user::Column::Email)
            .all(&self.db)
            .await
            .map_err(AppError::database)
            .map(|rows| rows.into_iter().map(user_response).collect())
    }

    async fn update_user_role(
        &self,
        user_id: Uuid,
        req: UpdateUserRoleRequest,
    ) -> Result<UserResponse, AppError> {
        let mut model = user::Entity::find_by_id(user_id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("user", user_id))?
            .into_active_model();
        model.system_role = Set(req.role.as_str().to_string());
        model.is_system_admin = Set(req.role.is_admin());
        model.updated_at = Set(now());
        let updated = model.update(&self.db).await.map_err(AppError::database)?;
        Ok(user_response(updated))
    }

    async fn create_user(
        &self,
        req: domain::RegisterRequest,
        password_hash: String,
        is_system_admin: bool,
    ) -> Result<app::auth::UserRecord, AppError> {
        let id = Uuid::new_v4();
        let ts = now();
        user::Entity::insert(user::ActiveModel {
            id: Set(id),
            email: Set(req.email),
            username: Set(req.username),
            display_name: Set(req.display_name),
            password_hash: Set(password_hash),
            refresh_token_hash: Set(None),
            system_role: Set(if is_system_admin {
                SystemRole::Admin.as_str().to_string()
            } else {
                SystemRole::User.as_str().to_string()
            }),
            is_system_admin: Set(is_system_admin),
            is_active: Set(true),
            created_at: Set(ts),
            updated_at: Set(ts),
        })
        .exec(&self.db)
        .await
        .map_err(AppError::database)?;
        self.find_user_by_id(id)
            .await?
            .ok_or_else(|| AppError::not_found("user", id))
    }

    async fn update_refresh_hash(
        &self,
        user_id: Uuid,
        refresh_hash: Option<String>,
    ) -> Result<(), AppError> {
        let mut model = user::Entity::find_by_id(user_id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("user", user_id))?
            .into_active_model();
        model.refresh_token_hash = Set(refresh_hash);
        model.updated_at = Set(now());
        model.update(&self.db).await.map_err(AppError::database)?;
        Ok(())
    }

    async fn list_deployment_jobs(&self, limit: u64) -> Result<Vec<DeploymentJob>, AppError> {
        deployment_job::Entity::find()
            .order_by_desc(deployment_job::Column::CreatedAt)
            .limit(limit.clamp(1, 500))
            .all(&self.db)
            .await
            .map_err(AppError::database)
            .map(|rows| rows.into_iter().map(deployment_job_from_model).collect())
    }

    async fn get_deployment_job(&self, id: Uuid) -> Result<DeploymentJob, AppError> {
        deployment_job::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .map(deployment_job_from_model)
            .ok_or_else(|| AppError::not_found("deployment_job", id))
    }

    async fn create_deployment_job(
        &self,
        req: CreateDeploymentJobRequest,
        requested_by_user_id: Uuid,
    ) -> Result<DeploymentJob, AppError> {
        if req.title.trim().is_empty() {
            return Err(AppError::validation("deployment job title is required"));
        }
        if let Some(agent_id) = req.agent_id {
            load_agent_row(&self.db, agent_id).await?;
        }
        let id = Uuid::new_v4();
        let ts = now();
        deployment_job::Entity::insert(deployment_job::ActiveModel {
            id: Set(id),
            job_kind: Set(req.job_kind.as_str().to_string()),
            state: Set(DeploymentJobState::Queued.as_str().to_string()),
            agent_id: Set(req.agent_id),
            runtime_kind: Set(req.runtime_kind.map(|kind| kind.as_str().to_string())),
            requested_by_user_id: Set(Some(requested_by_user_id)),
            title: Set(req.title.trim().to_string()),
            detail: Set(redact_json(req.detail.unwrap_or_else(|| json!({})))),
            last_error: Set(None),
            created_at: Set(ts),
            updated_at: Set(ts),
        })
        .exec(&self.db)
        .await
        .map_err(AppError::database)?;
        self.get_deployment_job(id).await
    }

    async fn cancel_deployment_job(
        &self,
        id: Uuid,
        _actor_user_id: Uuid,
    ) -> Result<DeploymentJob, AppError> {
        let row = deployment_job::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("deployment_job", id))?;
        let state = parse_deployment_job_state(&row.state);
        if matches!(
            state,
            DeploymentJobState::Completed
                | DeploymentJobState::Failed
                | DeploymentJobState::Cancelled
        ) {
            return Err(AppError::conflict("deployment job is already terminal"));
        }
        let mut model = row.into_active_model();
        model.state = Set(DeploymentJobState::Cancelled.as_str().to_string());
        model.updated_at = Set(now());
        let updated = model.update(&self.db).await.map_err(AppError::database)?;
        Ok(deployment_job_from_model(updated))
    }

    async fn get_runtime_settings(&self, config: &AppConfig) -> Result<RuntimeSettings, AppError> {
        load_setting(&self.db, "runtime", runtime_settings_from_config(config)).await
    }

    async fn update_runtime_settings(
        &self,
        req: RuntimeSettings,
        actor_user_id: Uuid,
    ) -> Result<RuntimeSettings, AppError> {
        if req.agents_root.trim().is_empty()
            || req.hermes_command.trim().is_empty()
            || req.java_agent_command.trim().is_empty()
        {
            return Err(AppError::validation(
                "runtime roots and commands must not be empty",
            ));
        }
        save_setting(&self.db, "runtime", req, actor_user_id).await
    }

    async fn get_port_settings(&self, config: &AppConfig) -> Result<PortSettings, AppError> {
        load_setting(&self.db, "ports", port_settings_from_config(config)).await
    }

    async fn update_port_settings(
        &self,
        req: PortSettings,
        actor_user_id: Uuid,
    ) -> Result<PortSettings, AppError> {
        if req.agent_port_stride < 4 {
            return Err(AppError::validation("agent_port_stride must be at least 4"));
        }
        save_setting(&self.db, "ports", req, actor_user_id).await
    }

    async fn get_integration_settings(&self) -> Result<IntegrationSettings, AppError> {
        load_setting(&self.db, "integrations", default_integration_settings()).await
    }

    async fn update_integration_settings(
        &self,
        req: IntegrationSettings,
        actor_user_id: Uuid,
    ) -> Result<IntegrationSettings, AppError> {
        save_setting(&self.db, "integrations", req, actor_user_id).await
    }

    async fn get_auth_settings(&self, config: &AppConfig) -> Result<AuthSettings, AppError> {
        load_setting(&self.db, "auth", auth_settings_from_config(config)).await
    }

    async fn update_auth_settings(
        &self,
        mut req: AuthSettings,
        actor_user_id: Uuid,
    ) -> Result<AuthSettings, AppError> {
        req.mode = req.mode.trim().to_ascii_lowercase();
        req.jwt_issuer = req.jwt_issuer.trim().to_string();
        req.jwt_audience = req.jwt_audience.trim().to_string();
        if req.access_token_ttl_minutes == 0 || req.refresh_token_ttl_days == 0 {
            return Err(AppError::validation(
                "auth TTL values must be greater than zero",
            ));
        }
        if req.mode != "hmac" {
            return Err(AppError::validation(
                "auth mode currently supports only hmac; oidc is phase 2",
            ));
        }
        if req.jwt_issuer.trim().is_empty() {
            return Err(AppError::validation("jwt_issuer must not be empty"));
        }
        if req.jwt_audience.trim().is_empty() {
            return Err(AppError::validation("jwt_audience must not be empty"));
        }
        save_setting(&self.db, "auth", req, actor_user_id).await
    }
}

fn session_from_model(
    row: agent_session::Model,
    agent: agent::Model,
    leader: Option<agent::Model>,
    user: user::Model,
) -> AgentSession {
    let agent_name = agent.name.clone();
    AgentSession {
        id: row.id,
        agent_id: row.agent_id,
        primary_agent_id: row.agent_id,
        agent_name,
        primary_agent_name: agent.name,
        user_id: row.user_id,
        user_email: user.email,
        user_username: user.username,
        user_display_name: user.display_name,
        leader_agent_id: row.leader_agent_id,
        leader_agent_name: leader.map(|agent| agent.name),
        parent_session_id: row.parent_session_id,
        created_by_leader_agent_id: row.created_by_leader_agent_id,
        visibility: parse_session_visibility(&row.visibility),
        title: row.title,
        task_key: row.task_key,
        state: parse_session_state(&row.state),
        namespace_id: row.namespace_id,
        external_session_id: row.external_session_id,
        last_message_preview: row.last_message_preview,
        created_at: api_ts(row.created_at),
        updated_at: api_ts(row.updated_at),
    }
}

fn selected_primary_agent_id(req: &CreateSessionRequest) -> Result<Uuid, AppError> {
    req.primary_agent_id
        .or(req.agent_id)
        .ok_or_else(|| AppError::validation("primary_agent_id is required"))
}

fn participant_display_name(
    participant_type: SessionParticipantType,
    user: Option<&user::Model>,
    agent: Option<&agent::Model>,
) -> String {
    match participant_type {
        SessionParticipantType::User => user
            .map(|user| user.display_name.clone())
            .unwrap_or_else(|| "Unknown user".to_string()),
        SessionParticipantType::Agent => agent
            .map(|agent| agent.display_name.clone())
            .unwrap_or_else(|| "Unknown agent".to_string()),
    }
}

fn message_author_display_name(
    author_type: MessageAuthorType,
    user: Option<&user::Model>,
    agent: Option<&agent::Model>,
) -> String {
    match author_type {
        MessageAuthorType::User => user
            .map(|user| user.display_name.clone())
            .unwrap_or_else(|| "Unknown user".to_string()),
        MessageAuthorType::Agent => agent
            .map(|agent| agent.display_name.clone())
            .unwrap_or_else(|| "Unknown agent".to_string()),
        MessageAuthorType::System => "Fleet Control".to_string(),
    }
}

async fn session_run_from_model(
    db: &DatabaseConnection,
    row: session_agent_run::Model,
) -> Result<SessionAgentRun, AppError> {
    let agent = load_agent_row(db, row.agent_id).await?;
    Ok(SessionAgentRun {
        id: row.id,
        session_id: row.session_id,
        agent_id: row.agent_id,
        agent_name: agent.name,
        runtime_session_id: row.runtime_session_id,
        runtime_run_id: row.runtime_run_id,
        run_role: parse_run_role(&row.run_role),
        state: parse_run_state(&row.state),
        last_error: row.last_error,
        last_event_at: api_ts_opt(row.last_event_at),
        model: row.model,
        provider: row.provider,
        model_options: row.model_options,
        created_at: api_ts(row.created_at),
        updated_at: api_ts(row.updated_at),
    })
}

fn runtime_approval_from_model(row: runtime_approval_request::Model) -> RuntimeApprovalRequest {
    RuntimeApprovalRequest {
        id: row.id,
        session_id: row.session_id,
        session_run_id: row.session_run_id,
        agent_id: row.agent_id,
        runtime_run_id: row.runtime_run_id,
        runtime_approval_id: row.runtime_approval_id,
        prompt: row.prompt,
        detail: row.detail,
        state: parse_runtime_approval_state(&row.state),
        resolved_by_user_id: row.resolved_by_user_id,
        resolved_at: api_ts_opt(row.resolved_at),
        created_at: api_ts(row.created_at),
    }
}

fn default_skills(role: AgentRole) -> Vec<(&'static str, &'static str)> {
    match role {
        AgentRole::Developer => vec![
            ("development", "Development"),
            ("project-workflow", "Project Workflow"),
            ("gh-commit-pr", "GitHub Commit and PR"),
        ],
        AgentRole::Tester => vec![
            ("audit-web-system", "Web System Audit"),
            ("project-workflow", "Project Workflow"),
            ("browser-control", "Browser Control"),
        ],
        AgentRole::ItLead => vec![
            ("project-workflow", "Project Workflow"),
            ("development", "Development"),
            ("audit-web-system", "Web System Audit"),
        ],
        AgentRole::Custom => vec![("project-workflow", "Project Workflow")],
    }
}

fn titleize(name: &str) -> String {
    name.split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn redact_json(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| {
                    let lower = key.to_ascii_lowercase();
                    if lower.contains("token")
                        || lower.contains("secret")
                        || lower.contains("password")
                        || lower.contains("key")
                    {
                        (key, Value::String("redacted".to_string()))
                    } else {
                        (key, redact_json(value))
                    }
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(redact_json).collect()),
        other => other,
    }
}

fn redact_text(value: &str) -> String {
    let mut output = value.to_string();
    for marker in ["token=", "password=", "secret=", "api_key=", "apikey="] {
        if let Some(pos) = output.to_ascii_lowercase().find(marker) {
            output.truncate(pos + marker.len());
            output.push_str("redacted");
        }
    }
    output
}

#[derive(Clone, Default)]
pub struct FilesystemProvisioner;

#[async_trait]
impl AgentProvisioner for FilesystemProvisioner {
    async fn provision(&self, agent: &Agent, config: &AppConfig) -> Result<(), AppError> {
        if agent.kind == AgentKind::JavaAgent {
            return Err(AppError::validation(
                "Java Agent runtime provisioning is planned for phase 2",
            ));
        }
        let root = PathBuf::from(&config.fleet.agents_root);
        let agent_root = safe_agent_root(&root, &agent.name)?;
        tokio::fs::create_dir_all(&agent_root)
            .await
            .map_err(AppError::internal)?;
        let marker_path = agent_root.join(".fleet-agent.json");
        if tokio::fs::try_exists(&marker_path)
            .await
            .map_err(AppError::internal)?
        {
            let marker = tokio::fs::read_to_string(&marker_path)
                .await
                .map_err(AppError::internal)?;
            let marker: Value = serde_json::from_str(&marker).map_err(AppError::internal)?;
            let marker_id = marker
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| AppError::validation("agent marker is missing id"))?;
            if marker_id != agent.id.to_string() {
                return Err(AppError::conflict(format!(
                    "agent directory {} belongs to another agent",
                    agent.name
                )));
            }
        }
        for path in [
            &agent.paths.runtime,
            &agent.paths.config,
            &agent.paths.workspace,
            &agent.paths.logs,
        ] {
            let path = PathBuf::from(path);
            ensure_inside(&root, &path)?;
            tokio::fs::create_dir_all(path)
                .await
                .map_err(AppError::internal)?;
        }

        write_if_missing(
            marker_path,
            serde_json::to_string_pretty(&json!({
                "id": agent.id,
                "ordinal": agent.ordinal,
                "name": agent.name,
                "kind": agent.kind.as_str()
            }))
            .map_err(AppError::internal)?,
        )
        .await?;

        provision_hermes(agent, config).await
    }

    async fn purge_files(
        &self,
        agent: &Agent,
        config: &AppConfig,
    ) -> Result<PurgeAgentFilesResponse, AppError> {
        if agent.status != AgentStatus::Archived {
            return Err(AppError::validation(
                "agent files can only be purged after archive",
            ));
        }

        let root = PathBuf::from(&config.fleet.agents_root);
        let agent_root = safe_agent_root(&root, &agent.name)?;
        let purged_path = normalize_path(&agent_root)?.to_string_lossy().to_string();
        let metadata = match tokio::fs::symlink_metadata(&agent_root).await {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(PurgeAgentFilesResponse {
                    agent_id: agent.id,
                    agent_name: agent.name.clone(),
                    purged_path,
                    files_deleted: false,
                    marker_verified: false,
                    message: "agent folder is already absent".to_string(),
                });
            }
            Err(error) => return Err(AppError::internal(error)),
        };
        if metadata.file_type().is_symlink() {
            return Err(AppError::validation(
                "agent folder symlinks cannot be purged",
            ));
        }
        if !metadata.is_dir() {
            return Err(AppError::validation("agent root is not a directory"));
        }

        let marker_path = agent_root.join(".fleet-agent.json");
        let marker_metadata = tokio::fs::symlink_metadata(&marker_path)
            .await
            .map_err(|error| {
                if error.kind() == ErrorKind::NotFound {
                    AppError::validation("agent marker is missing; refusing to purge files")
                } else {
                    AppError::internal(error)
                }
            })?;
        if marker_metadata.file_type().is_symlink() {
            return Err(AppError::validation(
                "agent marker symlinks cannot be purged",
            ));
        }

        let marker = tokio::fs::read_to_string(&marker_path)
            .await
            .map_err(AppError::internal)?;
        let marker: Value = serde_json::from_str(&marker).map_err(AppError::internal)?;
        let marker_id = marker
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::validation("agent marker is missing id"))?;
        if marker_id != agent.id.to_string() {
            return Err(AppError::conflict(format!(
                "agent directory {} belongs to another agent",
                agent.name
            )));
        }
        if let Some(marker_name) = marker.get("name").and_then(Value::as_str)
            && marker_name != agent.name
        {
            return Err(AppError::conflict(format!(
                "agent directory marker name {} does not match {}",
                marker_name, agent.name
            )));
        }

        tokio::fs::remove_dir_all(&agent_root)
            .await
            .map_err(AppError::internal)?;
        Ok(PurgeAgentFilesResponse {
            agent_id: agent.id,
            agent_name: agent.name.clone(),
            purged_path,
            files_deleted: true,
            marker_verified: true,
            message: "agent files purged".to_string(),
        })
    }
}

async fn provision_hermes(agent: &Agent, config: &AppConfig) -> Result<(), AppError> {
    let config_path = PathBuf::from(&agent.paths.config);
    let runtime_path = PathBuf::from(&agent.paths.runtime);
    let token = agent_runtime_token(config, agent.id)?;
    tokio::fs::create_dir_all(config_path.join("skills"))
        .await
        .map_err(AppError::internal)?;
    tokio::fs::create_dir_all(config_path.join("sessions"))
        .await
        .map_err(AppError::internal)?;
    write_if_missing(
        config_path.join("config.yaml"),
        format!(
            "profile: {}\nruntime: hermes\nterminal:\n  cwd: {}\nfleet_control:\n  agent_id: {}\n  api_port: {}\n  dashboard_port: {}\n",
            agent.name,
            agent.paths.workspace.replace('\\', "/"),
            agent.id,
            agent.api_port.unwrap_or_default(),
            agent.dashboard_port.unwrap_or_default()
        ),
    )
    .await?;
    write_if_missing(
        config_path.join("SOUL.md"),
        format!(
            "# {}\n\nYou are {} managed by Fleet Control.\n",
            agent.display_name, agent.name
        ),
    )
    .await?;
    write_managed(
        config_path.join(".env"),
        format!(
            "# Managed by Fleet Control. Secrets are redacted in API responses.\nHERMES_HOME={}\nHERMES_SERVE_HEADLESS=1\nAPI_SERVER_ENABLED=true\nAPI_SERVER_KEY={}\nAPI_SERVER_CORS_ORIGINS={}\n",
            agent.paths.config.replace('\\', "/"),
            token,
            config.server.cors_allowed_origins.join(",")
        ),
    )
    .await?;
    write_if_missing(
        runtime_path.join("source.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "hermes",
            "source": config.fleet.hermes_source,
            "materialization": "source_reference"
        }))
        .map_err(AppError::internal)?,
    )
    .await
}

async fn write_if_missing(path: PathBuf, content: String) -> Result<(), AppError> {
    if tokio::fs::try_exists(&path)
        .await
        .map_err(AppError::internal)?
    {
        return Ok(());
    }
    tokio::fs::write(path, content)
        .await
        .map_err(AppError::internal)
}

async fn write_managed(path: PathBuf, content: String) -> Result<(), AppError> {
    tokio::fs::write(path, content)
        .await
        .map_err(AppError::internal)
}

fn safe_agent_root(root: &Path, name: &str) -> Result<PathBuf, AppError> {
    if !name.starts_with("agent") || !name.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return Err(AppError::validation("invalid agent directory name"));
    }
    let path = root.join(name);
    ensure_inside(root, &path)?;
    Ok(path)
}

fn ensure_inside(root: &Path, path: &Path) -> Result<(), AppError> {
    let root = normalize_path(root)?;
    let candidate = if path.is_absolute() {
        normalize_path(path)?
    } else {
        normalize_path(&root.join(path))?
    };
    if candidate.starts_with(&root) {
        return Ok(());
    }
    Err(AppError::validation("path must stay inside agents root"))
}

fn normalize_path(path: &Path) -> Result<PathBuf, AppError> {
    let base = if path.is_absolute() {
        PathBuf::new()
    } else {
        std::env::current_dir().map_err(AppError::internal)?
    };
    let mut normalized = base;
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir => {
                return Err(AppError::validation("path traversal is not allowed"));
            }
        }
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_purge_root() -> PathBuf {
        std::env::temp_dir().join(format!("fleet-control-purge-{}", Uuid::new_v4()))
    }

    fn test_config(root: &Path) -> AppConfig {
        let mut config = AppConfig::default();
        config.auth.jwt_secret = "test-jwt-secret".to_string();
        config.fleet.runtime_token_secret = "test-runtime-secret".to_string();
        config.fleet.agents_root = root.to_string_lossy().to_string();
        config
    }

    fn test_agent(root: &Path, id: Uuid, status: AgentStatus) -> Agent {
        let paths = runtime_paths(&root.to_string_lossy(), 1);
        Agent {
            id,
            ordinal: 1,
            name: "agent1".to_string(),
            kind: AgentKind::Hermes,
            product_role: AgentProductRole::Executor,
            role: AgentRole::Developer,
            status,
            display_name: "Developer Hermes".to_string(),
            description: None,
            namespace_id: Some("dev".to_string()),
            workflow_id: Some("workflow-dev".to_string()),
            runtime_version: None,
            dashboard_port: Some(29001),
            api_port: Some(29002),
            paths,
            runtime: AgentRuntime {
                desired_state: DesiredState::Stopped,
                pid: None,
                health_status: None,
                health_detail: None,
                command_preview: String::new(),
                env_preview: json!({}),
                last_capabilities_json: json!({}),
                startup_command_redacted: None,
                started_at: None,
                stopped_at: None,
                last_health_at: None,
            },
            created_at: shared::now().to_rfc3339(),
            updated_at: shared::now().to_rfc3339(),
        }
    }

    async fn write_marker(agent_root: &Path, agent_id: Uuid, name: &str) {
        tokio::fs::create_dir_all(agent_root.join("config"))
            .await
            .expect("agent config dir");
        tokio::fs::write(
            agent_root.join(".fleet-agent.json"),
            serde_json::to_string_pretty(&json!({
                "id": agent_id,
                "ordinal": 1,
                "name": name,
                "kind": "hermes"
            }))
            .expect("marker json"),
        )
        .await
        .expect("agent marker");
    }

    #[test]
    fn selected_primary_agent_prefers_new_field() {
        let primary_agent_id = Uuid::new_v4();
        let legacy_agent_id = Uuid::new_v4();
        let req = CreateSessionRequest {
            primary_agent_id: Some(primary_agent_id),
            agent_id: Some(legacy_agent_id),
            title: "Session".to_string(),
            task_key: None,
            leader_agent_id: None,
            parent_session_id: None,
            namespace_id: None,
            idempotency_key: None,
        };

        assert_eq!(
            selected_primary_agent_id(&req).expect("agent id"),
            primary_agent_id
        );
    }

    #[test]
    fn selected_primary_agent_accepts_legacy_agent_id() {
        let legacy_agent_id = Uuid::new_v4();
        let req = CreateSessionRequest {
            primary_agent_id: None,
            agent_id: Some(legacy_agent_id),
            title: "Session".to_string(),
            task_key: None,
            leader_agent_id: None,
            parent_session_id: None,
            namespace_id: None,
            idempotency_key: None,
        };

        assert_eq!(
            selected_primary_agent_id(&req).expect("agent id"),
            legacy_agent_id
        );
    }

    #[test]
    fn path_guard_rejects_absolute_path_outside_root() {
        let root = std::env::temp_dir().join("fleet-control-root");
        let outside = std::env::temp_dir()
            .join("fleet-control-other")
            .join("agent1");
        let err = ensure_inside(&root, &outside).expect_err("outside path must be rejected");

        assert!(
            err.to_string()
                .contains("path must stay inside agents root")
        );
    }

    #[test]
    fn path_guard_rejects_parent_segments() {
        let root = std::env::temp_dir().join("fleet-control-root");
        let path = Path::new("agent1").join("..").join("agent2");
        let err = ensure_inside(&root, &path).expect_err("parent segment must be rejected");

        assert!(err.to_string().contains("path traversal is not allowed"));
    }

    #[test]
    fn agent_runtime_token_is_stable_and_agent_scoped() {
        let mut config = AppConfig::default();
        config.fleet.runtime_token_secret = "test-runtime-secret".to_string();
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();

        let first_token = agent_runtime_token(&config, first).expect("first token");
        let replayed_token = agent_runtime_token(&config, first).expect("replayed token");
        let second_token = agent_runtime_token(&config, second).expect("second token");

        assert_eq!(first_token, replayed_token);
        assert_ne!(first_token, second_token);
        assert!(first_token.starts_with("fc_"));
    }

    #[tokio::test]
    async fn purge_files_deletes_archived_marked_agent_root() {
        let root = temp_purge_root();
        let agent_id = Uuid::new_v4();
        let agent_root = root.join("agent1");
        let config = test_config(&root);
        let agent = test_agent(&root, agent_id, AgentStatus::Archived);
        write_marker(&agent_root, agent_id, "agent1").await;
        tokio::fs::write(agent_root.join("config").join("SOUL.md"), "test")
            .await
            .expect("managed file");

        let provisioner = FilesystemProvisioner;
        let response = provisioner
            .purge_files(&agent, &config)
            .await
            .expect("purge files");

        assert!(response.files_deleted);
        assert!(response.marker_verified);
        assert!(!tokio::fs::try_exists(&agent_root).await.expect("exists"));
        let _ = tokio::fs::remove_dir_all(&root).await;
    }

    #[tokio::test]
    async fn purge_files_rejects_foreign_marker() {
        let root = temp_purge_root();
        let agent_id = Uuid::new_v4();
        let agent_root = root.join("agent1");
        let config = test_config(&root);
        let agent = test_agent(&root, agent_id, AgentStatus::Archived);
        write_marker(&agent_root, Uuid::new_v4(), "agent1").await;

        let provisioner = FilesystemProvisioner;
        let err = provisioner
            .purge_files(&agent, &config)
            .await
            .expect_err("foreign marker must be rejected");

        assert!(err.to_string().contains("belongs to another agent"));
        assert!(tokio::fs::try_exists(&agent_root).await.expect("exists"));
        let _ = tokio::fs::remove_dir_all(&root).await;
    }

    #[tokio::test]
    async fn purge_files_rejects_missing_marker() {
        let root = temp_purge_root();
        let agent_id = Uuid::new_v4();
        let agent_root = root.join("agent1");
        let config = test_config(&root);
        let agent = test_agent(&root, agent_id, AgentStatus::Archived);
        tokio::fs::create_dir_all(agent_root.join("config"))
            .await
            .expect("agent config dir");

        let provisioner = FilesystemProvisioner;
        let err = provisioner
            .purge_files(&agent, &config)
            .await
            .expect_err("missing marker must be rejected");

        assert!(err.to_string().contains("marker is missing"));
        assert!(tokio::fs::try_exists(&agent_root).await.expect("exists"));
        let _ = tokio::fs::remove_dir_all(&root).await;
    }

    #[tokio::test]
    async fn purge_files_requires_archived_agent() {
        let root = temp_purge_root();
        let config = test_config(&root);
        let agent = test_agent(&root, Uuid::new_v4(), AgentStatus::Ready);

        let provisioner = FilesystemProvisioner;
        let err = provisioner
            .purge_files(&agent, &config)
            .await
            .expect_err("ready agent cannot be purged");

        assert!(err.to_string().contains("after archive"));
    }
}
