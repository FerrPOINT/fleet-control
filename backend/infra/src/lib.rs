mod approval_decisions;
pub mod base_package;
mod chats_directory;
mod clarification_commands;
mod config_revisions;
mod container_activation;
mod container_preparation;
mod container_recovery;
mod container_runtime;
mod effective_configuration;
pub mod entities;
mod hermes_approval_recovery;
mod hermes_dispatch_journal;
mod pm_controls;
pub mod pm_credentials;
mod pm_draft;
mod pm_execution;
mod pm_tool_config;
pub mod runtime;
mod runtime_acceptance;
mod runtime_controls;
mod task_chats;
pub mod tracker_event_poller;
mod tracker_events;

use app::{
    AgentProvisioner, AuditLogFilter, FleetRepository, RuntimeApprovalCreate,
    RuntimeSessionSnapshot, RuntimeStatePatch, SessionListFilter,
};
use async_trait::async_trait;
use domain::{
    Agent, AgentConfig, AgentDirectoryItem, AgentEvent, AgentKind, AgentLogEntry, AgentPaths,
    AgentProductRole, AgentRetentionReport, AgentRole, AgentRuntime, AgentSession, AgentStatus,
    AgentStorageArea, AgentStorageReport, AssignSessionLeaderRequest, AuditLogEntry, AuthSettings,
    BulkDeploymentRequest, BulkDeploymentResult, CreateAgentRequest, CreateDeploymentJobRequest,
    CreateSessionDelegationRequest, CreateSessionMessageRequest, CreateSessionRequest,
    DeploymentJob, DeploymentJobKind, DeploymentJobState, DesiredState, HandoffSessionRequest,
    IntegrationSettings, LeaderExecutor, ManagedSettingsSnapshot, ManagedSettingsVersion,
    MessageAuthorType, MessageDeliveryState, MessageKind, PortSettings, PurgeAgentFilesResponse,
    ResolveRuntimeApprovalRequest, RuntimeApprovalRequest, RuntimeApprovalState, RuntimeSettings,
    RuntimeTemplate, SdlcRole, SessionAgentRun, SessionMessage, SessionParticipant,
    SessionParticipantType, SessionRole, SessionRunRole, SessionRunState, SessionState,
    SessionVisibility, SkillState, SystemRole, UpdateAgentConfigRequest, UpdateAgentRequest,
    UpdateLeaderExecutorsRequest, UpdateSkillRequest, UpdateUserRoleRequest, UserResponse,
    WorkflowBinding,
};
use entities::{
    agent, agent_config, agent_event, agent_log, agent_runtime, agent_session, agent_skill,
    audit_log, deployment_job, fleet_alerts, leader_executor, managed_settings_version,
    runtime_approval_request, runtime_template, session_agent_run, session_message,
    session_participant, user, workflow_binding,
};
use hmac::{Hmac, Mac};
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ActiveValue::Set, ColumnTrait, ConnectOptions,
    ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, EntityTrait, IntoActiveModel,
    QueryFilter, QueryOrder, QuerySelect, Statement, TransactionTrait,
};
use sea_orm_migration::MigratorTrait;
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

fn parse_ts(value: &str) -> chrono::DateTime<chrono::FixedOffset> {
    chrono::DateTime::parse_from_rfc3339(value)
        .unwrap_or_else(|_| chrono::Utc::now().fixed_offset())
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

fn workflow_config_with_binding(mut config: Value, namespace_id: &str, workflow_id: &str) -> Value {
    if let Some(values) = config.as_object_mut() {
        values.insert("namespace_id".to_string(), json!(namespace_id));
        values.insert("workflow_id".to_string(), json!(workflow_id));
    }
    config
}

fn workflow_binding_from_row(row: workflow_binding::Model) -> WorkflowBinding {
    WorkflowBinding {
        id: row.id,
        agent_id: row.agent_id,
        namespace_id: row.namespace_id,
        namespace_name: row.namespace_name,
        workflow_id: row.workflow_id,
        workflow_name: row.workflow_name,
        binding_status: row.binding_status,
        created_at: api_ts(row.created_at),
        updated_at: api_ts(row.updated_at),
    }
}

fn workflow_binding_status(
    namespace_id: Option<&str>,
    namespace_name: Option<&str>,
    workflow_id: Option<&str>,
    workflow_name: Option<&str>,
    known_namespaces: &[(String, String)],
    known_workflows: &[(String, String)],
) -> &'static str {
    let namespace_id = namespace_id.filter(|value| !value.trim().is_empty());
    let namespace_name = namespace_name.filter(|value| !value.trim().is_empty());
    let workflow_id = workflow_id.filter(|value| !value.trim().is_empty());
    let workflow_name = workflow_name.filter(|value| !value.trim().is_empty());
    if namespace_id.is_none()
        && namespace_name.is_none()
        && workflow_id.is_none()
        && workflow_name.is_none()
    {
        return "unbound";
    }
    let namespace_live = known_namespaces.iter().any(|(id, name)| {
        Some(id.as_str()) == namespace_id && Some(name.as_str()) == namespace_name
    });
    let workflow_live = known_workflows
        .iter()
        .any(|(id, name)| Some(id.as_str()) == workflow_id && Some(name.as_str()) == workflow_name);
    if namespace_live && workflow_live {
        "connected"
    } else {
        "stale"
    }
}

fn agent_directory_item(agent: &Agent) -> AgentDirectoryItem {
    AgentDirectoryItem {
        id: agent.id,
        ordinal: agent.ordinal,
        name: agent.name.clone(),
        kind: agent.kind,
        product_role: agent.product_role,
        role: agent.role,
        sdlc_role: agent.sdlc_role,
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
        sdlc_role: agent
            .sdlc_role
            .as_deref()
            .and_then(|value| value.parse().ok()),
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

fn managed_settings_entry(
    row: managed_settings_version::Model,
) -> Result<ManagedSettingsVersion, AppError> {
    Ok(ManagedSettingsVersion {
        id: row.id,
        version: row.version,
        snapshot: serde_json::from_value(row.snapshot).map_err(AppError::internal)?,
        created_by_user_id: row.created_by_user_id,
        rollback_of_version: row.rollback_of_version,
        created_at: api_ts(row.created_at),
        is_active: row.is_active,
    })
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

fn integration_settings_from_config(config: &AppConfig) -> IntegrationSettings {
    let project_workflow_url = config
        .fleet
        .project_workflow_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let configured = project_workflow_url.is_some();
    IntegrationSettings {
        project_workflow_url,
        project_workflow_status: if configured {
            "configured"
        } else {
            "not_configured"
        }
        .to_string(),
        github_remote: None,
    }
}

/// Shared validation for bulk deployment requests (unit-tested).
fn validate_bulk_deployment_request(req: &BulkDeploymentRequest) -> Result<bool, AppError> {
    if matches!(
        req.job_kind,
        DeploymentJobKind::ProductDeploy | DeploymentJobKind::ProductRollback
    ) {
        return Err(AppError::validation(
            "product deployments require a single idempotent request",
        ));
    }
    if req.title.trim().is_empty() {
        return Err(AppError::validation("deployment job title is required"));
    }
    if req.agent_ids.is_empty() {
        return Err(AppError::validation("agent_ids must not be empty"));
    }
    if req.agent_ids.len() > 100 {
        return Err(AppError::validation(
            "agent_ids is limited to 100 per bulk request",
        ));
    }
    let rollback = req.rollback && req.job_kind == DeploymentJobKind::RuntimeUpdate;
    if req.rollback && !rollback {
        return Err(AppError::validation(
            "rollback is only valid for runtime_update jobs",
        ));
    }
    Ok(rollback)
}

fn fleet_alert_to_domain(row: fleet_alerts::Model) -> domain::FleetAlert {
    domain::FleetAlert {
        id: row.id,
        agent_id: row.agent_id,
        kind: row.kind,
        severity: row.severity,
        detail: row.detail,
        state: row.state,
        opened_at: row.opened_at.to_rfc3339(),
        resolved_at: row.resolved_at.map(|t| t.to_rfc3339()),
        acknowledged_at: row.acknowledged_at.map(|t| t.to_rfc3339()),
        acknowledged_by_user_id: row.acknowledged_by_user_id,
    }
}

#[async_trait]
impl FleetRepository for PostgresFleetRepository {
    async fn get_accepted_hermes_context(
        &self,
        run_id: Uuid,
    ) -> Result<Option<app::HermesDispatchIntent>, AppError> {
        hermes_approval_recovery::context(self, run_id).await
    }

    async fn list_recoverable_hermes_acceptances(
        &self,
        after: Option<Uuid>,
    ) -> Result<Vec<(SessionMessage, SessionAgentRun)>, AppError> {
        hermes_approval_recovery::queue(self, after).await
    }

    async fn recover_hermes_approval(
        &self,
        req: RuntimeApprovalCreate,
        native_session_id: String,
        origin: String,
        credential_fingerprint: String,
    ) -> Result<(RuntimeApprovalRequest, bool), AppError> {
        hermes_approval_recovery::record(
            self,
            req,
            native_session_id,
            origin,
            credential_fingerprint,
        )
        .await
    }

    async fn get_container_activation(
        &self,
        agent: Uuid,
        revision: i64,
    ) -> Result<Option<app::container_activation::Activation>, AppError> {
        container_activation::get(self, agent, revision).await
    }
    async fn container_generation_intent_hash(
        &self,
        agent: Uuid,
        generation: Uuid,
    ) -> Result<Option<String>, AppError> {
        container_activation::generation_intent_hash(self, agent, generation).await
    }
    async fn pending_container_activations(
        &self,
        after_agent: Option<Uuid>,
    ) -> Result<Vec<domain::AgentConfigRevision>, AppError> {
        container_activation::pending(self, after_agent).await
    }
    async fn hold_container_activation(
        &self,
        revision: &domain::AgentConfigRevision,
        activation: Option<&app::container_activation::Activation>,
        launch: &app::container_runtime::ContainerLaunch,
        hold: &app::container_activation::RecoveryHold,
    ) -> Result<(), AppError> {
        container_activation::hold(self, revision, activation, launch, hold).await
    }
    async fn claim_container_activation(
        &self,
        claim: &app::container_activation::Claim,
    ) -> Result<app::container_activation::Activation, AppError> {
        container_activation::claim(self, claim).await
    }
    async fn open_container_activation(
        &self,
        agent: Uuid,
    ) -> Result<Option<app::container_activation::Activation>, AppError> {
        container_activation::open(self, agent).await
    }
    async fn claim_recovered_container_activation(
        &self,
        claim: &app::container_activation::Claim,
        proof: &app::container_activation::RecoveredProof,
    ) -> Result<app::container_activation::Activation, AppError> {
        container_activation::claim_proved(self, claim, Some(proof)).await
    }
    async fn container_activation_for_launch(
        &self,
        launch: &app::container_runtime::ContainerLaunch,
    ) -> Result<Option<app::container_activation::Activation>, AppError> {
        container_activation::for_launch(self, launch).await
    }
    async fn authorize_recovered_activation(
        &self,
        record: &app::container_activation::Activation,
        proof: &app::container_activation::RecoveredProof,
    ) -> Result<(), AppError> {
        container_activation::authorize(self, record, proof).await
    }
    async fn advance_recovered_activation(
        &self,
        previous: &app::container_activation::Activation,
        next: &app::container_activation::Activation,
        proof: &app::container_activation::RecoveredProof,
    ) -> Result<(), AppError> {
        container_activation::advance_proved(self, previous, next, Some(proof)).await
    }
    async fn advance_container_activation(
        &self,
        previous: &app::container_activation::Activation,
        next: &app::container_activation::Activation,
    ) -> Result<(), AppError> {
        container_activation::advance(self, previous, next).await
    }
    async fn get_container_preparation(
        &self,
        agent: Uuid,
    ) -> Result<Option<app::container_runtime::ContainerPreparation>, AppError> {
        container_preparation::get(self, agent).await
    }
    async fn claim_container_preparation(
        &self,
        claim: &app::container_runtime::ContainerPreparationClaim,
    ) -> Result<app::container_runtime::ContainerPreparation, AppError> {
        container_preparation::claim(self, claim).await
    }
    async fn claim_container_preparation_delivery(
        &self,
        claim: &app::container_runtime::ContainerPreparationClaim,
    ) -> Result<bool, AppError> {
        container_preparation::delivery(self, claim).await
    }
    async fn acknowledge_container_preparation(
        &self,
        claim: &app::container_runtime::ContainerPreparationClaim,
        receipt: &app::container_runtime::PreparedContainer,
    ) -> Result<(), AppError> {
        container_preparation::acknowledge(self, claim, receipt).await
    }
    async fn get_container_recovery(
        &self,
        generation: Uuid,
    ) -> Result<Option<app::container_runtime::ContainerRecovery>, AppError> {
        container_recovery::get(self, generation).await
    }
    async fn claim_container_recovery(
        &self,
        launch: &app::container_runtime::ContainerLaunch,
        command: &app::container_runtime::ContainerRecoveryCommand,
    ) -> Result<(), AppError> {
        container_recovery::claim(self, launch, command).await
    }
    async fn acknowledge_container_recovery(
        &self,
        command: &app::container_runtime::ContainerRecoveryCommand,
        receipt: Value,
    ) -> Result<(), AppError> {
        container_recovery::acknowledge(self, command, receipt, false).await
    }
    async fn claim_container_lease(
        &self,
        previous: &app::container_runtime::ContainerRecovery,
        command: &app::container_runtime::ContainerRecoveryCommand,
    ) -> Result<(), AppError> {
        container_recovery::renew(self, previous, command).await
    }
    async fn acknowledge_container_lease(
        &self,
        command: &app::container_runtime::ContainerRecoveryCommand,
        receipt: Value,
    ) -> Result<(), AppError> {
        container_recovery::acknowledge(self, command, receipt, true).await
    }
    async fn advance_recovered_container(
        &self,
        launch: &app::container_runtime::ContainerLaunch,
        command: &app::container_runtime::ContainerRecoveryCommand,
        state: &str,
    ) -> Result<(), AppError> {
        container_runtime::advance_owned(
            self,
            launch,
            state,
            launch.snapshot.clone(),
            launch.origin.clone(),
            Some(command),
        )
        .await
    }
    async fn get_container_configuration(
        &self,
        agent: Uuid,
    ) -> Result<Option<domain::AgentConfigRevision>, AppError> {
        config_revisions::effective(self, agent).await
    }
    async fn get_container_launch(
        &self,
        agent: Uuid,
    ) -> Result<Option<app::container_runtime::ContainerLaunch>, AppError> {
        container_runtime::get(self, agent).await
    }
    async fn claim_container_launch(
        &self,
        launch: &app::container_runtime::ContainerLaunch,
    ) -> Result<(), AppError> {
        container_runtime::claim(self, launch).await
    }
    async fn advance_container_launch(
        &self,
        launch: &app::container_runtime::ContainerLaunch,
        state: &str,
        snapshot: Option<Value>,
        origin: Option<String>,
    ) -> Result<(), AppError> {
        container_runtime::advance(self, launch, state, snapshot, origin).await
    }
    async fn get_hermes_run_intent(
        &self,
        run: Uuid,
    ) -> Result<Option<app::HermesDispatchIntent>, AppError> {
        let row = self
            .db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT message_id FROM hermes_dispatch_journal WHERE run_id=$1",
                [run.into()],
            ))
            .await
            .map_err(AppError::database)?;
        match row {
            Some(row) => {
                hermes_dispatch_journal::get(
                    self,
                    row.try_get("", "message_id").map_err(AppError::database)?,
                )
                .await
            }
            None => Ok(None),
        }
    }
    async fn list_session_approvals(
        &self,
        session: Uuid,
    ) -> Result<Vec<RuntimeApprovalRequest>, AppError> {
        approval_decisions::list(self, session).await
    }
    async fn approval_decision(
        &self,
        session: Uuid,
        approval: Uuid,
    ) -> Result<domain::ApprovalDecision, AppError> {
        approval_decisions::get(self, session, approval).await
    }
    async fn reserve_approval_decision(
        &self,
        session: Uuid,
        approval: Uuid,
        actor: Uuid,
        req: domain::ApprovalDecisionRequest,
    ) -> Result<domain::ReservedApprovalDecision, AppError> {
        approval_decisions::reserve(self, session, approval, actor, req).await
    }
    async fn deliver_approval_decision(
        &self,
        id: Uuid,
    ) -> Result<domain::ApprovalDecision, AppError> {
        approval_decisions::deliver(self, id).await
    }
    async fn fail_undispatched_approval_decision(
        &self,
        id: Uuid,
    ) -> Result<domain::ApprovalDecision, AppError> {
        approval_decisions::fail_undispatched(self, id).await
    }
    async fn list_chats_directory(
        &self,
        filter: domain::ChatsDirectoryFilter,
    ) -> Result<domain::ChatsDirectoryPage, AppError> {
        self.chats_directory(filter).await
    }
    async fn reserve_pm_run(
        &self,
        reservation: domain::PmRunReservation,
    ) -> Result<domain::PmRunRecord, AppError> {
        pm_execution::reserve(self, reservation).await
    }
    async fn get_pm_run(&self, id: Uuid) -> Result<domain::PmRunRecord, AppError> {
        pm_execution::get(self, id).await
    }
    async fn get_pm_tool(
        &self,
        run: Uuid,
        key: &str,
    ) -> Result<Option<domain::PmToolCommand>, AppError> {
        pm_execution::get_tool(self, run, key).await
    }

    async fn has_pm_run_custody(&self, session: Uuid) -> Result<bool, AppError> {
        self.db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT EXISTS(SELECT 1 FROM pm_run_bindings WHERE session_id=$1) AS custody",
                [session.into()],
            ))
            .await
            .map_err(pm_execution::dispatch_error_db)?
            .ok_or_else(|| AppError::internal("missing PM custody lookup"))?
            .try_get("", "custody")
            .map_err(pm_execution::dispatch_error_db)
    }

    async fn read_pm_operation_for_session(
        &self,
        session: Uuid,
        owner: Uuid,
    ) -> Result<domain::PmDraftOperation, AppError> {
        let row = self.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT id FROM pm_draft_creation_operations WHERE owner_user_id=$1 AND operation->>'session_id'=$2 LIMIT 1",
            [owner.into(), session.to_string().into()])).await.map_err(pm_execution::dispatch_error_db)?
            .ok_or_else(|| AppError::not_found("PM operation", session))?;
        self.read_pm_creation(
            row.try_get("", "id")
                .map_err(pm_execution::dispatch_error_db)?,
            owner,
        )
        .await
    }
    async fn prepare_pm_tool(
        &self,
        command: domain::PmToolCommand,
    ) -> Result<domain::PmToolCommand, AppError> {
        pm_execution::prepare_tool(self, command)
            .await
            .map_err(pm_execution::dispatch_error)
    }
    async fn claim_pm_tool(&self, run: Uuid, key: &str) -> Result<bool, AppError> {
        pm_execution::claim_tool(self, run, key)
            .await
            .map_err(pm_execution::dispatch_error)
    }
    async fn finish_pm_tool(&self, run: Uuid, key: &str, result: Value) -> Result<(), AppError> {
        pm_execution::finish_tool(self, run, key, result)
            .await
            .map_err(pm_execution::dispatch_error)
    }
    async fn prepare_pm_dispatch(
        &self,
        intent: domain::PmDispatchIntent,
    ) -> Result<domain::PmDispatchIntent, AppError> {
        pm_execution::prepare_dispatch(self, intent)
            .await
            .map_err(pm_execution::dispatch_error)
    }
    async fn claim_pm_submission(&self, id: Uuid) -> Result<bool, AppError> {
        pm_execution::claim_submission(self, id)
            .await
            .map_err(pm_execution::dispatch_error)
    }
    async fn get_pm_dispatch(
        &self,
        id: Uuid,
    ) -> Result<Option<domain::PmDispatchIntent>, AppError> {
        pm_execution::get_dispatch(self, id)
            .await
            .map_err(pm_execution::dispatch_error)
    }
    async fn record_pm_submission(&self, id: Uuid, run_ref: String) -> Result<(), AppError> {
        pm_execution::record_submission(self, id, run_ref)
            .await
            .map_err(pm_execution::dispatch_error)
    }
    async fn claim_pm_guidance(
        &self,
        id: Uuid,
        body: String,
    ) -> Result<domain::PmGuidancePermit, AppError> {
        pm_execution::claim_guidance(self, id, body)
            .await
            .map_err(pm_execution::dispatch_error)
    }
    async fn finish_pm_guidance(&self, id: Uuid) -> Result<(), AppError> {
        pm_execution::finish_guidance(self, id)
            .await
            .map_err(pm_execution::dispatch_error)
    }
    async fn accept_pm_run(
        &self,
        id: Uuid,
        hermes_run_ref: String,
        hermes_session_ref: String,
    ) -> Result<domain::PmRunRecord, AppError> {
        pm_execution::accept(self, id, hermes_run_ref, hermes_session_ref).await
    }
    async fn observe_pm_run(
        &self,
        id: Uuid,
        status: domain::PmRuntimeStatus,
    ) -> Result<(), AppError> {
        pm_execution::observe(self, id, status).await
    }
    async fn has_pending_session_dispatch(&self, id: Uuid) -> Result<bool, AppError> {
        let row=self.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT EXISTS(SELECT 1 FROM message_dispatch_outbox o JOIN session_messages m ON m.id=o.message_id WHERE m.session_id=$1 AND o.state IN ('pending','dispatching','uncertain')) AS pending",[id.into()]))
            .await.map_err(AppError::database)?.ok_or_else(|| AppError::internal("missing dispatch observation"))?;
        row.try_get("", "pending").map_err(AppError::database)
    }
    async fn get_task_chat_binding(
        &self,
        session_id: Uuid,
    ) -> Result<Option<domain::TaskChatBinding>, AppError> {
        self.task_binding(session_id).await
    }
    async fn tracker_event_cursor(&self, session_id: Uuid) -> Result<i64, AppError> {
        self.source_event_cursor(session_id, "legacy_full_v1").await
    }
    async fn project_tracker_events(
        &self,
        session_id: Uuid,
        binding: domain::TaskChatBinding,
        after: i64,
        page: domain::TrackerOutboxPage,
    ) -> Result<domain::TrackerProjectionReceipt, AppError> {
        self.persist_tracker_page(session_id, binding, after, page)
            .await
    }
    async fn tracker_metadata_cursor(&self, session_id: Uuid) -> Result<i64, AppError> {
        self.source_event_cursor(session_id, "metadata_v1").await
    }
    async fn tracker_projection_targets(
        &self,
        instance: &str,
        project_ids: &[Uuid],
        after_session: Option<Uuid>,
    ) -> Result<Vec<domain::TrackerProjectionTarget>, AppError> {
        self.metadata_targets(instance, project_ids, after_session)
            .await
    }
    async fn project_tracker_metadata(
        &self,
        session_id: Uuid,
        binding: domain::TaskChatBinding,
        after: i64,
        page: domain::TrackerMetadataPage,
    ) -> Result<domain::TrackerProjectionReceipt, AppError> {
        self.persist_tracker_metadata(session_id, binding, after, page)
            .await
    }
    async fn bind_task_chat(
        &self,
        session_id: Uuid,
        binding: domain::TaskChatBinding,
        key: String,
    ) -> Result<domain::TaskChatBinding, AppError> {
        self.persist_task_binding(session_id, binding, key).await
    }
    async fn store_clarification_command(
        &self,
        actor: &domain::ClarificationCommandActor,
        question: Uuid,
        request: domain::ClarificationAnswerRequest,
    ) -> Result<domain::ClarificationAnswerCommand, AppError> {
        clarification_commands::store(self, actor, question, request).await
    }
    async fn get_clarification_command(
        &self,
        actor: &domain::ClarificationCommandActor,
        id: Uuid,
    ) -> Result<domain::ClarificationAnswerCommand, AppError> {
        clarification_commands::get(self, actor, id).await
    }
    async fn list_pending_clarification_commands(
        &self,
        actor: &domain::ClarificationCommandActor,
    ) -> Result<Vec<domain::ClarificationAnswerCommand>, AppError> {
        clarification_commands::list(self, actor).await
    }
    async fn claim_clarification_delivery(
        &self,
        actor: &domain::ClarificationCommandActor,
        id: Uuid,
    ) -> Result<domain::ClarificationDeliveryPermit, AppError> {
        clarification_commands::claim(self, actor, id).await
    }

    async fn finish_clarification_continuation(
        &self,
        actor: &domain::ClarificationCommandActor,
        id: Uuid,
        outcome: domain::PmContinuationOutcome,
    ) -> Result<domain::ClarificationAnswerCommand, AppError> {
        clarification_commands::finish_continuation(self, actor, id, outcome).await
    }
    async fn finish_clarification_delivery(
        &self,
        actor: &domain::ClarificationCommandActor,
        id: Uuid,
        attempt: Uuid,
        outcome: domain::ClarificationDeliveryOutcome,
    ) -> Result<domain::ClarificationAnswerCommand, AppError> {
        clarification_commands::finish(self, actor, id, attempt, outcome).await
    }
    async fn create_pm_draft_chat(
        &self,
        command: domain::CreatePmDraftChat,
        owner_user_id: Uuid,
    ) -> Result<AgentSession, AppError> {
        self.persist_pm_draft_chat(command, owner_user_id).await
    }
    async fn get_hermes_dispatch_intent_for_run(
        &self,
        run_id: Uuid,
    ) -> Result<Option<app::HermesDispatchIntent>, AppError> {
        hermes_dispatch_journal::get_for_run(self, run_id).await
    }

    async fn reserve_runtime_control(
        &self,
        run: &SessionAgentRun,
        actor: &domain::RuntimeControlActor,
        operation: domain::RuntimeControlOperation,
        input: Option<&str>,
    ) -> Result<domain::RuntimeControlReservation, AppError> {
        runtime_controls::reserve(self, run, actor, operation, input).await
    }

    async fn reserve_pm_runtime_control(
        &self,
        run: &SessionAgentRun,
        actor: &domain::RuntimeControlActor,
        op: domain::RuntimeControlOperation,
        input: Option<&str>,
        scope: &domain::PmHumanControlScope,
    ) -> Result<domain::RuntimeControlReservation, AppError> {
        runtime_controls::reserve_scoped(self, run, actor, op, input, Some(scope)).await
    }
    async fn check_pm_runtime_control(
        &self,
        run: &SessionAgentRun,
        owner: Uuid,
        op: domain::RuntimeControlOperation,
        scope: &domain::PmHumanControlScope,
    ) -> Result<(), AppError> {
        runtime_controls::check_pm(self, run, owner, op, scope).await
    }
    async fn claim_pm_runtime_control(
        &self,
        id: Uuid,
        scope: &domain::PmHumanControlScope,
    ) -> Result<bool, AppError> {
        runtime_controls::claim_scoped(self, id, Some(scope)).await
    }

    async fn claim_runtime_control(&self, id: Uuid) -> Result<bool, AppError> {
        runtime_controls::claim(self, id).await
    }

    async fn finish_runtime_control(
        &self,
        id: Uuid,
        acknowledgement: &str,
        input: Option<&str>,
    ) -> Result<domain::RuntimeControlReceipt, AppError> {
        runtime_controls::finish(self, id, acknowledgement, input).await
    }

    async fn retire_runtime_control(
        &self,
        id: Uuid,
        submitted: bool,
    ) -> Result<domain::RuntimeControlReceipt, AppError> {
        runtime_controls::retire(self, id, submitted).await
    }

    async fn get_runtime_control(
        &self,
        session: Uuid,
        id: Uuid,
    ) -> Result<domain::RuntimeControlReceipt, AppError> {
        runtime_controls::get(self, session, id).await
    }

    async fn list_runtime_controls(
        &self,
        session: Uuid,
        run: Uuid,
    ) -> Result<Vec<domain::RuntimeControlReceipt>, AppError> {
        runtime_controls::list(self, session, run).await
    }

    async fn find_runtime_control_by_key(
        &self,
        session: Uuid,
        run: Uuid,
        actor: &domain::RuntimeControlActor,
    ) -> Result<Option<domain::RuntimeControlReceipt>, AppError> {
        runtime_controls::find_by_key(self, session, run, actor).await
    }

    async fn reconcile_runtime_controls(&self) -> Result<u64, AppError> {
        runtime_controls::reconcile(self).await
    }

    async fn reserve_pm_draft_operation(
        &self,
        operation: domain::PmDraftOperation,
    ) -> Result<domain::PmDraftOperation, AppError> {
        self.reserve_pm_creation(operation).await
    }
    async fn read_pm_draft_operation(
        &self,
        id: Uuid,
        owner: Uuid,
    ) -> Result<domain::PmDraftOperation, AppError> {
        self.read_pm_creation(id, owner).await
    }
    async fn read_pm_draft_operation_by_key(
        &self,
        owner: Uuid,
        key: &str,
    ) -> Result<domain::PmDraftOperation, AppError> {
        self.read_pm_creation_by_key(owner, key).await
    }
    async fn record_pm_draft_proof(
        &self,
        id: Uuid,
        owner: Uuid,
        proof: domain::PmDraftProof,
    ) -> Result<domain::PmDraftOperation, AppError> {
        self.persist_pm_creation_proof(id, owner, proof).await
    }
    async fn session_message_history(
        &self,
        session_id: Uuid,
        before: Option<Uuid>,
        limit: u64,
    ) -> Result<domain::MessageHistoryPage, AppError> {
        self.paged_message_history(session_id, before, limit).await
    }
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
                implemented: true,
                enabled: true,
                description: "Spring Boot Java Agent runtime (process control + actuator health)"
                    .to_string(),
                capabilities: json!({
                    "provision": "jar:runtime/backend.jar",
                    "process_control": true,
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
                // Keep the seeded catalog in sync with code (implemented
                // flags evolve as adapters land).
                runtime_template::Entity::update(runtime_template::ActiveModel {
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
        let rows = agent::Entity::find()
            .order_by_asc(agent::Column::Ordinal)
            .all(&self.db)
            .await
            .map_err(AppError::database)?;
        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            result.push(agent_directory_item(&load_agent(&self.db, row.id).await?));
        }
        Ok(result)
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
            sdlc_role: req.sdlc_role,
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
            sdlc_role: Set(req.sdlc_role.map(|role| role.as_str().to_string())),
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
        let txn = self.db.begin().await.map_err(AppError::database)?;
        txn.query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id FROM agents WHERE id = $1 FOR UPDATE",
            [id.into()],
        ))
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("agent", id))?;
        let draining = txn
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT draining FROM agent_config_heads WHERE agent_id = $1",
                [id.into()],
            ))
            .await
            .map_err(AppError::database)?
            .map(|row| {
                row.try_get::<bool>("", "draining")
                    .map_err(AppError::database)
            })
            .transpose()?
            .unwrap_or(false);
        if draining {
            return Err(AppError::conflict("agent configuration is draining"));
        }
        let current = agent::Entity::find_by_id(id)
            .one(&txn)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("agent", id))?;
        if req
            .product_role
            .is_some_and(|value| value.as_str() != current.product_role)
            || req.role.is_some_and(|value| value.as_str() != current.role)
            || req
                .sdlc_role
                .is_some_and(|value| Some(value.as_str()) != current.sdlc_role.as_deref())
            || req
                .namespace_id
                .as_deref()
                .is_some_and(|value| Some(value) != current.namespace_id.as_deref())
            || req
                .workflow_id
                .as_deref()
                .is_some_and(|value| Some(value) != current.workflow_id.as_deref())
        {
            config_revisions::guard_identity_change(&txn, id).await?;
        }
        let mut model = current.into_active_model();
        if let Some(product_role) = next_product_role {
            model.product_role = Set(product_role.as_str().to_string());
        }
        if let Some(role) = req.role {
            model.role = Set(role.as_str().to_string());
        }
        if let Some(role) = req.sdlc_role {
            model.sdlc_role = Set(Some(role.as_str().to_string()));
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
        model.update(&txn).await.map_err(AppError::database)?;
        txn.commit().await.map_err(AppError::database)?;
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
                config_json: redact_configuration_json(row.config_json),
                soul_md: row.soul_md,
                env_json: redact_configuration_json(row.env_json),
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
        model.env_json = Set(req.env_json);
        model.updated_at = Set(now());
        model.update(&self.db).await.map_err(AppError::database)?;
        self.get_agent_config(agent_id).await
    }

    async fn create_config_revision(
        &self,
        id: Uuid,
        config: UpdateAgentConfigRequest,
        actor: Uuid,
    ) -> Result<domain::AgentConfigRevision, AppError> {
        config_revisions::create(self, id, config, actor).await
    }
    async fn list_config_revisions(
        &self,
        id: Uuid,
    ) -> Result<Vec<domain::AgentConfigRevision>, AppError> {
        config_revisions::list(self, id).await
    }
    async fn get_effective_config_revision(
        &self,
        id: Uuid,
    ) -> Result<Option<domain::AgentConfigRevision>, AppError> {
        config_revisions::effective(self, id).await
    }
    async fn get_config_revision(
        &self,
        id: Uuid,
        revision: i64,
    ) -> Result<domain::AgentConfigRevision, AppError> {
        self.get_agent(id).await?;
        config_revisions::get(self, id, revision).await
    }
    async fn prepare_base_package_revision(
        &self,
        id: Uuid,
        checkout: &str,
        binding: &domain::SdlcWorkflowBinding,
        actor: Uuid,
    ) -> Result<domain::AgentConfigRevision, AppError> {
        let agent = self.get_agent(id).await?;
        let role = agent
            .sdlc_role
            .ok_or_else(|| AppError::validation("SDLC role is required"))?;
        let revisions = self.list_config_revisions(id).await?;
        let desired = revisions.into_iter().find(|revision| revision.is_desired);
        let expected = desired.as_ref().map(|revision| revision.revision);
        let snapshot = match desired {
            Some(revision) => revision.snapshot,
            None => {
                let config = self.get_agent_config(id).await?;
                domain::AgentConfigurationSnapshot {
                    config: UpdateAgentConfigRequest {
                        config_json: config.config_json,
                        soul_md: config.soul_md,
                        env_json: config.env_json,
                    },
                    skills: self.list_agent_skills(id).await?,
                }
            }
        };
        let package =
            base_package::VerifiedRolePackage::read(std::path::Path::new(checkout), role).await?;
        let snapshot = package.prepare_snapshot(&agent, binding, snapshot)?;
        config_revisions::create_snapshot(self, id, snapshot, actor, Some(expected)).await
    }
    async fn validate_config_revision(
        &self,
        id: Uuid,
        revision: i64,
        errors: Vec<String>,
    ) -> Result<domain::AgentConfigRevision, AppError> {
        config_revisions::validate(self, id, revision, errors).await
    }
    async fn verify_base_package_revision(
        &self,
        id: Uuid,
        revision: i64,
        checkout: &str,
    ) -> Result<(), AppError> {
        let agent = self.get_agent(id).await?;
        let role = agent
            .sdlc_role
            .ok_or_else(|| AppError::validation("SDLC role is required"))?;
        let revision = config_revisions::get(self, id, revision).await?;
        let package =
            base_package::VerifiedRolePackage::read(std::path::Path::new(checkout), role).await?;
        package.verify_snapshot(&agent, &revision.snapshot)
    }
    async fn request_config_activation(
        &self,
        id: Uuid,
        revision: i64,
        actor: Uuid,
    ) -> Result<domain::AgentConfigRevision, AppError> {
        config_revisions::activate(self, id, revision, actor).await
    }
    async fn claim_config_activation(
        &self,
    ) -> Result<Option<domain::AgentConfigRevision>, AppError> {
        config_revisions::claim(self).await
    }
    async fn finish_config_activation(
        &self,
        id: Uuid,
        revision: i64,
        error: Option<String>,
        reconciled: bool,
    ) -> Result<(), AppError> {
        config_revisions::finish(self, id, revision, error, reconciled).await
    }
    async fn agent_is_draining(&self, id: Uuid) -> Result<bool, AppError> {
        config_revisions::draining(self, id).await
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
        let (instance, projects) = match filter.task_project_access {
            Some(scope) => (Some(scope.tracker_instance_id), scope.project_ids),
            None => (None, Vec::new()),
        };
        let scope_values: [sea_orm::Value; 2] = [
            instance.into(),
            serde_json::to_value(projects)
                .map_err(AppError::internal)?
                .into(),
        ];
        query = query.filter(sea_orm::sea_query::Expr::cust_with_values(
            "(NOT EXISTS(SELECT 1 FROM task_chat_bindings b WHERE b.session_id=agent_sessions.id)
             OR EXISTS(SELECT 1 FROM task_chat_bindings b WHERE b.session_id=agent_sessions.id
                AND b.tracker_instance_id=$1 AND b.project_id IN
                    (SELECT value::uuid FROM jsonb_array_elements_text($2::jsonb))))",
            scope_values,
        ));
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
        if let Some(owner) = filter.private_user_id {
            query = query.filter(
                sea_orm::Condition::any()
                    .add(
                        agent_session::Column::Visibility
                            .eq(SessionVisibility::LeaderScoped.as_str()),
                    )
                    .add(agent_session::Column::UserId.eq(owner)),
            );
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
        let delivery = self
            .db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT EXISTS(SELECT 1 FROM session_messages WHERE session_id=$1 \
             AND delivery_state IN ('pending','dispatched')) AS pending_delivery, \
             EXISTS(SELECT 1 FROM task_chat_bindings WHERE session_id=$1) AS task_bound",
                [id.into()],
            ))
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::internal("delivery projection returned no row"))?;
        let mut session = session_from_model(row, agent, leader, user);
        session.pending_delivery = Some(
            delivery
                .try_get("", "pending_delivery")
                .map_err(AppError::database)?,
        );
        session.task_bound = Some(
            delivery
                .try_get("", "task_bound")
                .map_err(AppError::database)?,
        );
        Ok(session)
    }

    async fn sync_runtime_sessions(
        &self,
        agent_id: Uuid,
        sessions: Vec<RuntimeSessionSnapshot>,
    ) -> Result<u64, AppError> {
        let agent = agent::Entity::find_by_id(agent_id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("agent", agent_id))?;
        // Service user that owns runtime-imported sessions.
        let system_user = user::Entity::find()
            .filter(user::Column::Email.eq("runtime@fleet-control.local"))
            .one(&self.db)
            .await
            .map_err(AppError::database)?;
        let system_user_id = match system_user {
            Some(u) => u.id,
            None => {
                let id = Uuid::new_v4();
                user::Entity::insert(user::ActiveModel {
                    central_sub: sea_orm::ActiveValue::NotSet,
                    id: Set(id),
                    email: Set("runtime@fleet-control.local".to_string()),
                    username: Set("runtime-sync".to_string()),
                    display_name: Set("Runtime Sync".to_string()),
                    password_hash: Set(String::new()),
                    refresh_token_hash: Set(None),
                    is_system_admin: Set(false),
                    is_active: Set(true),
                    system_role: Set("user".to_string()),
                    created_at: Set(now()),
                    updated_at: Set(now()),
                })
                .exec(&self.db)
                .await
                .map_err(AppError::database)?;
                id
            }
        };
        let mut applied = 0u64;
        for snapshot in sessions {
            let existing = agent_session::Entity::find()
                .filter(agent_session::Column::ExternalSessionId.eq(&snapshot.external_id))
                .one(&self.db)
                .await
                .map_err(AppError::database)?;
            match existing {
                Some(model) => {
                    let mut patch = agent_session::ActiveModel {
                        id: Set(model.id),
                        ..Default::default()
                    };
                    if let Some(title) = snapshot.title.clone() {
                        patch.title = Set(title);
                    }
                    if let Some(updated) = snapshot.updated_at {
                        patch.updated_at = Set(updated);
                    }
                    patch.update(&self.db).await.map_err(AppError::database)?;
                }
                None => {
                    agent_session::Entity::insert(agent_session::ActiveModel {
                        id: Set(Uuid::new_v4()),
                        agent_id: Set(agent.id),
                        user_id: Set(system_user_id),
                        leader_agent_id: Set(None),
                        parent_session_id: Set(None),
                        created_by_leader_agent_id: Set(None),
                        visibility: Set(SessionVisibility::Private.as_str().to_string()),
                        title: Set(snapshot
                            .title
                            .clone()
                            .unwrap_or_else(|| "Imported runtime session".to_string())),
                        task_key: Set(None),
                        state: Set(SessionState::Active.as_str().to_string()),
                        namespace_id: Set(agent.namespace_id.clone()),
                        external_session_id: Set(Some(snapshot.external_id.clone())),
                        last_message_preview: Set(Some(
                            "Imported from runtime /api/v2/sessions".to_string(),
                        )),
                        idempotency_key: Set(None),
                        idempotency_payload_hash: Set(None),
                        created_at: Set(snapshot.created_at.unwrap_or_else(now)),
                        updated_at: Set(snapshot.updated_at.unwrap_or_else(now)),
                    })
                    .exec(&self.db)
                    .await
                    .map_err(AppError::database)?;
                }
            }
            applied += 1;
        }
        Ok(applied)
    }

    async fn create_session(
        &self,
        req: CreateSessionRequest,
        user_id: Uuid,
    ) -> Result<AgentSession, AppError> {
        let txn = self.db.begin().await.map_err(AppError::database)?;
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
        if let Some(key) = idempotency_key.as_ref() {
            txn.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
                [format!("session:{user_id}:{key}").into()],
            ))
            .await
            .map_err(AppError::database)?;
        }
        if let Some(key) = idempotency_key.as_ref()
            && let Some(existing) = agent_session::Entity::find()
                .filter(agent_session::Column::UserId.eq(user_id))
                .filter(agent_session::Column::IdempotencyKey.eq(key))
                .one(&txn)
                .await
                .map_err(AppError::database)?
        {
            if existing.idempotency_payload_hash == idempotency_payload_hash {
                txn.commit().await.map_err(AppError::database)?;
                return self.get_session(existing.id).await;
            }
            return Err(AppError::conflict(
                "idempotency_key was already used with a different session payload",
            ));
        }
        let primary_agent_id = selected_primary_agent_id(&req)?;
        let agent = load_agent_row(&txn, primary_agent_id).await?;
        if agent.status == AgentStatus::Archived.as_str() {
            return Err(AppError::conflict(
                "cannot create a chat with an archived agent",
            ));
        }
        user::Entity::find_by_id(user_id)
            .one(&txn)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("user", user_id))?;
        let parent = match req.parent_session_id {
            Some(parent_session_id) => Some(
                agent_session::Entity::find_by_id(parent_session_id)
                    .one(&txn)
                    .await
                    .map_err(AppError::database)?
                    .ok_or_else(|| AppError::not_found("parent_session", parent_session_id))?,
            ),
            None => None,
        };
        let primary_product_role = parse_product_role(&agent.product_role);
        if parent
            .as_ref()
            .is_some_and(|parent| parent.user_id != user_id)
        {
            return Err(AppError::Forbidden);
        }
        let leader_agent_id = req
            .leader_agent_id
            .or_else(|| parent.as_ref().and_then(|session| session.leader_agent_id))
            .or_else(|| {
                (primary_product_role == AgentProductRole::Leader).then_some(primary_agent_id)
            });

        if let Some(leader_id) = leader_agent_id {
            ensure_agent_product_role(&txn, leader_id, AgentProductRole::Leader, "leader").await?;
            if primary_product_role == AgentProductRole::Leader && leader_id != primary_agent_id {
                return Err(AppError::validation(
                    "leader chat must use the same primary and leader agent",
                ));
            }
            if primary_product_role == AgentProductRole::Executor {
                let allowed = leader_executor::Entity::find_by_id((leader_id, primary_agent_id))
                    .one(&txn)
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

        let session_id = Uuid::new_v4();
        let ts = now();
        let visibility = if leader_agent_id.is_some() {
            SessionVisibility::LeaderScoped
        } else {
            SessionVisibility::Private
        };
        agent_session::Entity::insert(agent_session::ActiveModel {
            id: Set(session_id),
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
            session_id: Set(session_id),
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
            session_id: Set(session_id),
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
                session_id: Set(session_id),
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
        session_agent_run::Entity::insert(pending_session_run(
            session_id,
            primary_agent_id,
            primary_run_role,
            ts,
        ))
        .exec(&txn)
        .await
        .map_err(AppError::database)?;
        session_message::Entity::insert(session_message::ActiveModel {
            id: Set(Uuid::new_v4()),
            session_id: Set(session_id),
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
        self.get_session(session_id).await
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
            session_agent_run::Entity::insert(pending_session_run(
                id,
                leader_id,
                SessionRunRole::Leader,
                ts,
            ))
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

    async fn list_session_events(
        &self,
        id: Uuid,
        after: i64,
    ) -> Result<Vec<domain::SessionEvent>, AppError> {
        let rows = self
            .db
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT session_id, sequence, event_type, payload, created_at FROM session_events
             WHERE session_id = $1 AND sequence > $2 ORDER BY sequence LIMIT 100",
                [id.into(), after.into()],
            ))
            .await
            .map_err(AppError::database)?;
        rows.into_iter()
            .map(|row| {
                Ok(domain::SessionEvent {
                    session_id: row.try_get("", "session_id").map_err(AppError::database)?,
                    sequence: row.try_get("", "sequence").map_err(AppError::database)?,
                    event_type: row.try_get("", "event_type").map_err(AppError::database)?,
                    payload: row.try_get("", "payload").map_err(AppError::database)?,
                    created_at: api_ts(row.try_get("", "created_at").map_err(AppError::database)?),
                })
            })
            .collect()
    }

    async fn append_session_event(
        &self,
        id: Uuid,
        event_type: &str,
        payload: Value,
    ) -> Result<(), AppError> {
        let statement = Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "WITH cursor AS (
                INSERT INTO session_event_cursors(session_id, sequence) VALUES ($1, 1)
                ON CONFLICT(session_id) DO UPDATE SET sequence = session_event_cursors.sequence + 1
                RETURNING sequence)
             INSERT INTO session_events(session_id, sequence, event_type, payload)
                SELECT $1, sequence, $2, $3 FROM cursor",
            [
                id.into(),
                event_type.into(),
                redact_json(payload.clone()).into(),
            ],
        );
        if event_type == "session_run_delta" {
            let run_id = payload
                .get("run_id")
                .and_then(Value::as_str)
                .and_then(|value| Uuid::parse_str(value).ok())
                .ok_or_else(|| AppError::validation("delta requires a concrete run"))?;
            let txn = self.db.begin().await.map_err(AppError::database)?;
            txn.query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT id FROM agent_sessions WHERE id=$1 FOR NO KEY UPDATE",
                [id.into()],
            ))
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("agent_session", id))?;
            let run = session_agent_run::Entity::find_by_id(run_id)
                .lock_exclusive()
                .one(&txn)
                .await
                .map_err(AppError::database)?
                .ok_or_else(|| AppError::not_found("session_agent_run", run_id))?;
            if run.session_id != id
                || payload.get("session_id").and_then(Value::as_str)
                    != Some(id.to_string().as_str())
                || run.runtime_run_id.is_none()
                || payload.get("runtime_run_id").and_then(Value::as_str)
                    != run.runtime_run_id.as_deref()
            {
                return Err(AppError::conflict("delta run identity changed"));
            }
            if matches!(run.state.as_str(), "completed" | "failed" | "cancelled") {
                return Ok(());
            }
            if !matches!(run.state.as_str(), "running" | "waiting" | "stopping") {
                return Err(AppError::conflict("delta requires a pinned active run"));
            }
            txn.execute(statement).await.map_err(AppError::database)?;
            txn.commit().await.map_err(AppError::database)?;
        } else {
            self.db
                .execute(statement)
                .await
                .map_err(AppError::database)?;
        }
        Ok(())
    }

    async fn session_event_cursor(&self, id: Uuid) -> Result<i64, AppError> {
        let row = self
            .db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT sequence FROM session_event_cursors WHERE session_id = $1",
                [id.into()],
            ))
            .await
            .map_err(AppError::database)?;
        row.map(|row| row.try_get("", "sequence").map_err(AppError::database))
            .transpose()
            .map(|value| value.unwrap_or(0))
    }

    async fn list_session_messages(&self, id: Uuid) -> Result<Vec<SessionMessage>, AppError> {
        let rows = session_message::Entity::find()
            .filter(session_message::Column::SessionId.eq(id))
            .order_by_asc(sea_orm::sea_query::Expr::col(
                sea_orm::sea_query::Alias::new("append_sequence"),
            ))
            .limit(500)
            .all(&self.db)
            .await
            .map_err(AppError::database)?;
        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            result.push(session_message_from_model(&self.db, row).await?);
        }
        Ok(result)
    }

    async fn claim_message_dispatch(&self) -> Result<Option<SessionMessage>, AppError> {
        let row = self.db.query_one(Statement::from_string(DatabaseBackend::Postgres,
            "WITH candidate AS (
                SELECT o.message_id FROM message_dispatch_outbox o
                JOIN agents a ON a.id = o.agent_id
                JOIN session_messages m ON m.id = o.message_id
                JOIN agent_sessions s ON s.id = m.session_id
                WHERE o.state = 'pending' AND a.status = 'running' AND a.kind = 'hermes'
                  AND NOT EXISTS (SELECT 1 FROM task_chat_bindings b WHERE b.session_id = s.id)
                  AND NOT EXISTS (SELECT 1 FROM agent_config_heads h WHERE h.agent_id = a.id AND h.draining)
                  AND NOT EXISTS (SELECT 1 FROM message_dispatch_outbox busy WHERE busy.agent_id = a.id AND busy.state IN ('dispatching','uncertain'))
                  AND NOT EXISTS (SELECT 1 FROM session_agent_runs r WHERE r.agent_id = a.id
                      AND r.state IN ('pending','running','waiting','stopping') AND r.runtime_session_id IS NOT NULL)
                ORDER BY o.created_at, o.message_id FOR UPDATE OF a, o, s SKIP LOCKED LIMIT 1)
             UPDATE message_dispatch_outbox o SET state = 'dispatching', updated_at = now()
                FROM candidate WHERE o.message_id = candidate.message_id RETURNING o.message_id".to_string()))
            .await.map_err(AppError::database)?;
        let Some(row) = row else {
            return Ok(None);
        };
        let id: Uuid = row.try_get("", "message_id").map_err(AppError::database)?;
        let message = session_message::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("session_message", id))?;
        let body = message.body.clone();
        let mut result = session_message_from_model(&self.db, message).await?;
        // Runtime dispatch receives the original prompt; public transcript reads are redacted.
        result.body = body;
        Ok(Some(result))
    }

    async fn finish_message_dispatch(
        &self,
        message_id: Uuid,
        uncertain: bool,
        error: Option<String>,
    ) -> Result<(), AppError> {
        let state = if uncertain {
            "uncertain"
        } else if error.is_some() {
            "failed"
        } else {
            "dispatched"
        };
        self.db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "UPDATE message_dispatch_outbox SET state = $2, last_error = $3, updated_at = now() WHERE message_id = $1 AND state = 'dispatching'",
            [message_id.into(), state.into(), error.map(|error| redact_text(&error)).into()]))
            .await.map_err(AppError::database)?;
        Ok(())
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
        let txn = self.db.begin().await.map_err(AppError::database)?;
        let session = agent_session::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("agent_session", id))?;
        let binding = txn
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT session_id FROM task_chat_bindings WHERE session_id=$1",
                [id.into()],
            ))
            .await
            .map_err(AppError::database)?;
        if binding.is_some() {
            return Err(AppError::conflict(
                "task-bound messages require a verified workflow assignment",
            ));
        }
        let actor = user::Entity::find_by_id(actor_user_id)
            .one(&txn)
            .await
            .map_err(AppError::database)?
            .ok_or(AppError::Unauthorized)?;
        let can_write_other = if actor.central_sub.is_some() {
            session.visibility == SessionVisibility::LeaderScoped.as_str()
        } else {
            parse_system_role(&actor.system_role, actor.is_system_admin).can_operate_fleet()
        };
        if !actor.is_active || (actor_user_id != session.user_id && !can_write_other) {
            return Err(AppError::Forbidden);
        }
        if let Some(key) = idempotency_key.as_ref()
            && let Some(existing) = session_message::Entity::find()
                .filter(session_message::Column::SessionId.eq(id))
                .filter(session_message::Column::CreatedByUserId.eq(actor_user_id))
                .filter(session_message::Column::IdempotencyKey.eq(key))
                .one(&txn)
                .await
                .map_err(AppError::database)?
        {
            if existing.idempotency_payload_hash == idempotency_payload_hash {
                txn.commit().await.map_err(AppError::database)?;
                let mut message = session_message_receipt(&self.db, existing).await?;
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
        let row = session_message::Entity::insert(session_message::ActiveModel {
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
        .exec_with_returning(&txn)
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
        session_message_receipt(&self.db, row).await
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
        let txn = self.db.begin().await.map_err(AppError::database)?;
        let agent = agent::Entity::find_by_id(agent_id)
            .lock_exclusive()
            .one(&txn)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("agent", agent_id))?;
        if agent.status != AgentStatus::Running.as_str() {
            return Err(AppError::conflict("agent runtime is not running"));
        }
        let active = session_agent_run::Entity::find()
            .filter(session_agent_run::Column::AgentId.eq(agent_id))
            .filter(
                session_agent_run::Column::State
                    .is_in(["pending", "running", "waiting", "stopping"]),
            )
            .filter(session_agent_run::Column::RuntimeSessionId.is_not_null())
            .one(&txn)
            .await
            .map_err(AppError::database)?;
        if active.is_some() {
            return Err(AppError::conflict(
                "agent already has an active or unresolved run; use steer or reconcile it",
            ));
        }
        let ts = now();
        if let Some(row) = session_agent_run::Entity::find()
            .filter(session_agent_run::Column::SessionId.eq(session_id))
            .filter(session_agent_run::Column::AgentId.eq(agent_id))
            .filter(session_agent_run::Column::State.eq(SessionRunState::Pending.as_str()))
            .filter(session_agent_run::Column::RuntimeRunId.is_null())
            .filter(session_agent_run::Column::RuntimeSessionId.is_null())
            .order_by_asc(session_agent_run::Column::CreatedAt)
            .one(&txn)
            .await
            .map_err(AppError::database)?
        {
            let mut model = row.into_active_model();
            model.runtime_session_id = Set(Some(runtime_session_id));
            model.run_role = Set(run_role.as_str().to_string());
            model.updated_at = Set(ts);
            let updated = model.update(&txn).await.map_err(AppError::database)?;
            txn.commit().await.map_err(AppError::database)?;
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
        .exec(&txn)
        .await
        .map_err(AppError::database)?;

        let row = session_agent_run::Entity::find_by_id(id)
            .one(&txn)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::internal("session run insert returned no row"))?;
        txn.commit().await.map_err(AppError::database)?;
        session_run_from_model(&self.db, row).await
    }

    async fn prepare_hermes_dispatch(
        &self,
        draft: app::HermesDispatchDraft,
    ) -> Result<app::HermesDispatchIntent, AppError> {
        hermes_dispatch_journal::prepare(self, draft).await
    }

    async fn claim_hermes_submission(
        &self,
        message_id: Uuid,
        origin: String,
        credential_fingerprint: String,
    ) -> Result<Option<app::HermesDispatchIntent>, AppError> {
        hermes_dispatch_journal::claim(self, message_id, origin, credential_fingerprint).await
    }

    async fn get_hermes_dispatch_intent(
        &self,
        message_id: Uuid,
    ) -> Result<Option<app::HermesDispatchIntent>, AppError> {
        hermes_dispatch_journal::get(self, message_id).await
    }

    async fn update_session_agent_run_dispatch(
        &self,
        id: Uuid,
        runtime_run_id: Option<String>,
        state: SessionRunState,
        last_error: Option<String>,
    ) -> Result<SessionAgentRun, AppError> {
        let txn = self.db.begin().await.map_err(AppError::database)?;
        // Match readback's lock order; late SSE/error updates cannot regress verified PM proof.
        let pm = pm_execution::locked(&txn, id).await?;
        let row = session_agent_run::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("session_agent_run", id))?;
        if row
            .runtime_run_id
            .as_ref()
            .is_some_and(|old| runtime_run_id.as_ref().is_some_and(|new| new != old))
        {
            return Err(AppError::conflict("runtime run identity is immutable"));
        }
        if pm.is_none() && matches!(row.state.as_str(), "completed" | "failed" | "cancelled") {
            txn.commit().await.map_err(AppError::database)?;
            return session_run_from_model(&self.db, row).await;
        }
        let mut model = row.into_active_model();
        if let Some(pm) = &pm
            && runtime_run_id
                .as_ref()
                .is_some_and(|id| pm.hermes_run_ref.as_ref() != Some(id))
        {
            return Err(AppError::conflict(
                "PM runtime mapping must use its immutable acceptance receipt",
            ));
        }
        if pm
            .as_ref()
            .is_some_and(|record| record.terminal_status.is_none())
            && matches!(
                state,
                SessionRunState::Completed | SessionRunState::Failed | SessionRunState::Cancelled
            )
        {
            return Err(AppError::conflict(
                "PM capacity requires verified terminal proof",
            ));
        }
        if runtime_run_id.is_some() {
            model.runtime_run_id = Set(runtime_run_id);
        }
        if let Some(terminal) = pm.and_then(|record| record.terminal_status) {
            model.state = Set(pm_execution::visible_terminal(terminal).into());
        } else {
            model.state = Set(state.as_str().to_string());
            model.last_error = Set(last_error.map(|error| redact_text(&error)));
        }
        model.last_event_at = Set(Some(now()));
        model.updated_at = Set(now());
        let updated = model.update(&txn).await.map_err(AppError::database)?;
        txn.commit().await.map_err(AppError::database)?;
        session_run_from_model(&self.db, updated).await
    }

    async fn accept_hermes_run(
        &self,
        message_id: Uuid,
        run_id: Uuid,
        runtime_run_id: String,
    ) -> Result<SessionAgentRun, AppError> {
        runtime_acceptance::accept(self, message_id, run_id, runtime_run_id, None).await
    }

    async fn accept_recovered_hermes_run(
        &self,
        message_id: Uuid,
        run_id: Uuid,
        runtime_run_id: String,
        original_capabilities: Value,
    ) -> Result<SessionAgentRun, AppError> {
        runtime_acceptance::accept(
            self,
            message_id,
            run_id,
            runtime_run_id,
            Some(original_capabilities),
        )
        .await
    }

    async fn pin_hermes_run_session(
        &self,
        run_id: Uuid,
        runtime_run_id: String,
        requested_session_id: String,
        effective_session_id: String,
    ) -> Result<(SessionAgentRun, bool), AppError> {
        runtime_acceptance::pin_session(
            self,
            run_id,
            runtime_run_id,
            requested_session_id,
            effective_session_id,
        )
        .await
    }

    async fn commit_hermes_terminal(
        &self,
        command: app::HermesTerminalCommit,
    ) -> Result<(SessionAgentRun, Option<SessionMessage>, bool), AppError> {
        runtime_acceptance::terminal(self, command).await
    }

    async fn commit_pm_terminal(
        &self,
        command: app::HermesTerminalCommit,
        status: domain::PmRuntimeStatus,
    ) -> Result<(SessionAgentRun, Option<SessionMessage>, bool), AppError> {
        pm_execution::terminal(self, command, status).await
    }

    async fn pm_stream_context(&self, run: Uuid) -> Result<(domain::PmRunRecord, bool), AppError> {
        pm_execution::stream_context(&self.db, run).await
    }

    async fn list_recoverable_pm_streams(
        &self,
        after: Option<Uuid>,
    ) -> Result<Vec<Uuid>, AppError> {
        pm_execution::stream_queue(self, after).await
    }

    async fn insert_session_message_mirror(
        &self,
        session_id: Uuid,
        author_agent_id: Option<Uuid>,
        body: String,
        message_kind: MessageKind,
        runtime_message_id: Option<String>,
    ) -> Result<SessionMessage, AppError> {
        let body = redact_text(body.trim());
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
        txn.query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id FROM agent_sessions WHERE id=$1 FOR NO KEY UPDATE",
            [session_id.into()],
        ))
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("agent_session", session_id))?;
        if message_kind == MessageKind::ToolEvent
            && let (Some(agent_id), Some(native_id)) =
                (author_agent_id, runtime_message_id.as_deref())
        {
            let terminal = session_agent_run::Entity::find()
                .filter(session_agent_run::Column::SessionId.eq(session_id))
                .filter(session_agent_run::Column::AgentId.eq(agent_id))
                .filter(session_agent_run::Column::RuntimeRunId.eq(native_id))
                .lock_exclusive()
                .all(&txn)
                .await
                .map_err(AppError::database)?;
            if terminal
                .iter()
                .any(|run| matches!(run.state.as_str(), "completed" | "failed" | "cancelled"))
            {
                return Err(AppError::conflict(
                    "terminal run cannot append a tool event",
                ));
            }
        }
        if message_kind == MessageKind::AssistantMessage
            && let Some(runtime_id) = runtime_message_id.as_ref()
        {
            let mut existing = session_message::Entity::find()
                .filter(session_message::Column::SessionId.eq(session_id))
                .filter(
                    session_message::Column::MessageKind.eq(MessageKind::AssistantMessage.as_str()),
                )
                .filter(session_message::Column::RuntimeMessageId.eq(runtime_id));
            existing = match author_agent_id {
                Some(id) => existing.filter(session_message::Column::AuthorAgentId.eq(id)),
                None => existing.filter(session_message::Column::AuthorAgentId.is_null()),
            };
            if let Some(row) = existing.one(&txn).await.map_err(AppError::database)? {
                txn.commit().await.map_err(AppError::database)?;
                return session_message_from_model(&self.db, row).await;
            }
        }
        let row = session_message::Entity::insert(session_message::ActiveModel {
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
        .exec_with_returning(&txn)
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
        session_message_from_model(&self.db, row).await
    }

    async fn update_session_message_delivery(
        &self,
        id: Uuid,
        delivery_state: MessageDeliveryState,
        runtime_message_id: Option<String>,
        delivery_error: Option<String>,
    ) -> Result<(), AppError> {
        let txn = self.db.begin().await.map_err(AppError::database)?;
        // Serialize with dispatch/terminal writers before locking the child message.
        let session_id: Uuid = txn
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT s.id FROM agent_sessions s JOIN session_messages m ON m.session_id=s.id
                 WHERE m.id=$1 FOR NO KEY UPDATE OF s",
                [id.into()],
            ))
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("session_message", id))?
            .try_get("", "id")
            .map_err(AppError::database)?;
        let row = session_message::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("session_message", id))?;
        if row.session_id != session_id {
            return Err(AppError::conflict("message session identity changed"));
        }
        if let Some(previous) = &row.runtime_message_id {
            if runtime_message_id
                .as_ref()
                .is_some_and(|next| next != previous)
            {
                return Err(AppError::conflict("runtime message identity is immutable"));
            }
            // A post-commit failure cannot erase an ACK, nor reopen terminal delivery.
            if runtime_message_id.is_none()
                || matches!(row.delivery_state.as_str(), "completed" | "failed")
            {
                txn.commit().await.map_err(AppError::database)?;
                return Ok(());
            }
            if delivery_state == MessageDeliveryState::Pending {
                return Err(AppError::conflict(
                    "acknowledged delivery cannot become pending",
                ));
            }
        }
        // Submission also locks this message before advancing the journal. Classify
        // an unacknowledged error here, not from a stale pre-transaction read.
        let delivery_state = if delivery_state == MessageDeliveryState::Failed
            && runtime_message_id.is_none()
        {
            let journal = txn
                .query_one(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    "SELECT state FROM hermes_dispatch_journal WHERE message_id=$1",
                    [id.into()],
                ))
                .await
                .map_err(|_| AppError::Database("Hermes delivery classification failed".into()))?;
            let submitted = journal
                .map(|row| row.try_get::<String>("", "state"))
                .transpose()
                .map_err(|_| AppError::Database("Hermes delivery classification failed".into()))?
                .is_some_and(|state| matches!(state.as_str(), "submitted" | "accepted"));
            if submitted {
                MessageDeliveryState::Pending
            } else {
                delivery_state
            }
        } else {
            delivery_state
        };
        let mut model = row.into_active_model();
        model.delivery_state = Set(delivery_state.as_str().to_string());
        if runtime_message_id.is_some() {
            model.runtime_message_id = Set(runtime_message_id);
        }
        model.delivery_error = Set(delivery_error.map(|error| redact_text(&error)));
        model.update(&txn).await.map_err(AppError::database)?;
        txn.commit().await.map_err(AppError::database)?;
        Ok(())
    }

    async fn upsert_runtime_approval_request(
        &self,
        req: RuntimeApprovalCreate,
    ) -> Result<RuntimeApprovalRequest, AppError> {
        let id = Uuid::new_v4();
        let txn = self.db.begin().await.map_err(AppError::database)?;
        txn.query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id FROM agent_sessions WHERE id=$1 FOR NO KEY UPDATE",
            [req.session_id.into()],
        ))
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("agent_session", req.session_id))?;
        let run = session_agent_run::Entity::find_by_id(req.session_run_id)
            .lock_exclusive()
            .one(&txn)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("session_agent_run", req.session_run_id))?;
        if run.session_id != req.session_id
            || run.agent_id != req.agent_id
            || run.runtime_run_id.as_deref() != Some(req.runtime_run_id.as_str())
            || matches!(run.state.as_str(), "completed" | "failed" | "cancelled")
        {
            return Err(AppError::conflict(
                "approval requires its active runtime run",
            ));
        }
        txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO runtime_approval_requests(id,session_id,session_run_id,agent_id,runtime_run_id,runtime_approval_id,prompt,detail,state,created_at)
             VALUES($1,$2,$3,$4,$5,$6,$7,$8,'pending',now())
             ON CONFLICT(session_run_id,runtime_approval_id) WHERE runtime_approval_id IS NOT NULL DO NOTHING",
            [id.into(),req.session_id.into(),req.session_run_id.into(),req.agent_id.into(),req.runtime_run_id.clone().into(),req.runtime_approval_id.clone().into(),redact_text(&req.prompt).into(),redact_json(req.detail).into()]))
            .await.map_err(AppError::database)?;
        let query = match req.runtime_approval_id {
            Some(request_id) => runtime_approval_request::Entity::find()
                .filter(runtime_approval_request::Column::SessionRunId.eq(req.session_run_id))
                .filter(runtime_approval_request::Column::RuntimeApprovalId.eq(request_id)),
            None => runtime_approval_request::Entity::find_by_id(id),
        };
        let row = query
            .one(&txn)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("runtime_approval_request", id))?;
        if row.session_id != req.session_id
            || row.agent_id != req.agent_id
            || row.runtime_run_id != req.runtime_run_id
        {
            return Err(AppError::conflict("runtime approval identity changed"));
        }
        txn.commit().await.map_err(AppError::database)?;
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

    async fn refresh_workflow_bindings(
        &self,
        known_namespaces: Vec<(String, String)>,
        known_workflows: Vec<(String, String)>,
    ) -> Result<u64, AppError> {
        let bindings = workflow_binding::Entity::find()
            .all(&self.db)
            .await
            .map_err(AppError::database)?;
        let mut updated = 0u64;
        for row in bindings {
            let target = workflow_binding_status(
                row.namespace_id.as_deref(),
                row.namespace_name.as_deref(),
                row.workflow_id.as_deref(),
                row.workflow_name.as_deref(),
                &known_namespaces,
                &known_workflows,
            );
            if row.binding_status != target {
                let mut model = row.into_active_model();
                model.binding_status = Set(target.to_string());
                model.updated_at = Set(chrono::Utc::now().into());
                model.update(&self.db).await.map_err(AppError::database)?;
                updated += 1;
            }
        }
        Ok(updated)
    }

    async fn rebind_workflow_binding(
        &self,
        agent_id: Uuid,
        namespace: domain::WorkflowNamespaceCatalogEntry,
        workflow: domain::WorkflowCatalogEntry,
    ) -> Result<WorkflowBinding, AppError> {
        let txn = self.db.begin().await.map_err(AppError::database)?;
        let timestamp = now();
        txn.query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id FROM agents WHERE id=$1 AND archived_at IS NULL FOR UPDATE",
            [agent_id.into()],
        ))
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("agent", agent_id))?;
        config_revisions::guard_identity_change(&txn, agent_id).await?;
        let updated_agents = agent::Entity::update_many()
            .set(agent::ActiveModel {
                namespace_id: Set(Some(namespace.id.clone())),
                workflow_id: Set(Some(workflow.id.clone())),
                updated_at: Set(timestamp),
                ..Default::default()
            })
            .filter(agent::Column::Id.eq(agent_id))
            .exec(&txn)
            .await
            .map_err(AppError::database)?;
        if updated_agents.rows_affected != 1 {
            return Err(AppError::not_found("agent", agent_id));
        }
        let binding = workflow_binding::Entity::find()
            .filter(workflow_binding::Column::AgentId.eq(agent_id))
            .one(&txn)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("workflow_binding", agent_id))?;
        let mut binding = binding.into_active_model();
        binding.namespace_id = Set(Some(namespace.id));
        binding.namespace_name = Set(Some(namespace.name));
        binding.workflow_id = Set(Some(workflow.id));
        binding.workflow_name = Set(Some(workflow.name));
        binding.binding_status = Set("connected".to_string());
        binding.updated_at = Set(timestamp);
        let binding = binding.update(&txn).await.map_err(AppError::database)?;
        if let Some(config_row) = agent_config::Entity::find_by_id(agent_id)
            .one(&txn)
            .await
            .map_err(AppError::database)?
        {
            let mut config = config_row.into_active_model();
            config.config_json = Set(workflow_config_with_binding(
                config.config_json.clone().unwrap(),
                binding.namespace_id.as_deref().unwrap_or_default(),
                binding.workflow_id.as_deref().unwrap_or_default(),
            ));
            config.updated_at = Set(timestamp);
            config.update(&txn).await.map_err(AppError::database)?;
        }
        txn.commit().await.map_err(AppError::database)?;
        Ok(workflow_binding_from_row(binding))
    }

    async fn list_workflow_bindings(&self) -> Result<Vec<WorkflowBinding>, AppError> {
        workflow_binding::Entity::find()
            .order_by_asc(workflow_binding::Column::NamespaceId)
            .all(&self.db)
            .await
            .map_err(AppError::database)
            .map(|rows| rows.into_iter().map(workflow_binding_from_row).collect())
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

    async fn list_fleet_alerts(
        &self,
        state: Option<&str>,
    ) -> Result<Vec<domain::FleetAlert>, AppError> {
        let mut query = fleet_alerts::Entity::find().order_by_desc(fleet_alerts::Column::OpenedAt);
        if let Some(state) = state {
            query = query.filter(fleet_alerts::Column::State.eq(state));
        }
        let rows = query.all(&self.db).await.map_err(AppError::database)?;
        Ok(rows.into_iter().map(fleet_alert_to_domain).collect())
    }

    async fn acknowledge_fleet_alert(
        &self,
        alert_id: Uuid,
        user_id: Uuid,
    ) -> Result<domain::FleetAlert, AppError> {
        let updated = fleet_alerts::Entity::update_many()
            .col_expr(
                fleet_alerts::Column::State,
                sea_orm::sea_query::Expr::value("acknowledged"),
            )
            .col_expr(
                fleet_alerts::Column::AcknowledgedAt,
                sea_orm::sea_query::Expr::value(chrono::Utc::now().fixed_offset()),
            )
            .col_expr(
                fleet_alerts::Column::AcknowledgedByUserId,
                sea_orm::sea_query::Expr::value(user_id),
            )
            .filter(fleet_alerts::Column::Id.eq(alert_id))
            .filter(fleet_alerts::Column::State.eq("open"))
            .exec(&self.db)
            .await
            .map_err(AppError::database)?;
        if updated.rows_affected == 0 {
            return Err(AppError::not_found("fleet_alert(open)", alert_id));
        }
        self.list_fleet_alerts(None)
            .await?
            .into_iter()
            .find(|alert| alert.id == alert_id)
            .ok_or_else(|| AppError::not_found("fleet_alert", alert_id))
    }

    async fn insert_fleet_alert(
        &self,
        alert: domain::FleetAlert,
    ) -> Result<domain::FleetAlert, AppError> {
        let transaction = self.db.begin().await.map_err(AppError::database)?;
        if alert.kind == app::HEARTBEAT_STALE_ALERT_KIND {
            let agent_id = alert
                .agent_id
                .ok_or_else(|| AppError::validation("heartbeat alert requires an agent"))?;
            // Serialize incident creation across reconciler instances, without a new migration.
            agent::Entity::find_by_id(agent_id)
                .lock_exclusive()
                .one(&transaction)
                .await
                .map_err(AppError::database)?
                .ok_or_else(|| AppError::not_found("agent", agent_id))?;
            if let Some(existing) = fleet_alerts::Entity::find()
                .filter(fleet_alerts::Column::AgentId.eq(agent_id))
                .filter(fleet_alerts::Column::Kind.eq(app::HEARTBEAT_STALE_ALERT_KIND))
                .filter(fleet_alerts::Column::State.is_in(app::ACTIVE_ALERT_STATES))
                .order_by_desc(fleet_alerts::Column::OpenedAt)
                .one(&transaction)
                .await
                .map_err(AppError::database)?
            {
                transaction.commit().await.map_err(AppError::database)?;
                return Ok(fleet_alert_to_domain(existing));
            }
        }
        fleet_alerts::Entity::insert(fleet_alerts::ActiveModel {
            id: Set(alert.id),
            agent_id: Set(alert.agent_id),
            kind: Set(alert.kind.clone()),
            severity: Set(alert.severity.clone()),
            detail: Set(alert.detail.clone()),
            state: Set(alert.state.clone()),
            opened_at: Set(parse_ts(&alert.opened_at)),
            resolved_at: Set(alert.resolved_at.as_deref().map(parse_ts)),
            acknowledged_at: Set(alert.acknowledged_at.as_deref().map(parse_ts)),
            acknowledged_by_user_id: Set(alert.acknowledged_by_user_id),
        })
        .exec(&transaction)
        .await
        .map_err(AppError::database)?;
        transaction.commit().await.map_err(AppError::database)?;
        Ok(alert)
    }

    async fn recent_restart_count(
        &self,
        agent_id: Uuid,
        window: chrono::Duration,
    ) -> Result<u32, AppError> {
        use sea_orm::{ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter};
        let since = (chrono::Utc::now() - window).fixed_offset();
        let count = crate::entities::agent_event::Entity::find()
            .filter(
                Condition::all()
                    .add(crate::entities::agent_event::Column::AgentId.eq(agent_id))
                    .add(crate::entities::agent_event::Column::EventType.eq("agent.restart"))
                    .add(crate::entities::agent_event::Column::CreatedAt.gte(since)),
            )
            .count(&self.db)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(count as u32)
    }

    async fn resolve_active_alerts_of_kind(
        &self,
        agent_id: Uuid,
        kind: &str,
        resolution_detail: Value,
    ) -> Result<u64, AppError> {
        let transaction = self.db.begin().await.map_err(AppError::database)?;
        let updated = fleet_alerts::Entity::update_many()
            .col_expr(
                fleet_alerts::Column::State,
                sea_orm::sea_query::Expr::value("resolved"),
            )
            .col_expr(
                fleet_alerts::Column::ResolvedAt,
                sea_orm::sea_query::Expr::value(chrono::Utc::now().fixed_offset()),
            )
            .filter(fleet_alerts::Column::AgentId.eq(agent_id))
            .filter(fleet_alerts::Column::Kind.eq(kind))
            .filter(fleet_alerts::Column::State.is_in(app::ACTIVE_ALERT_STATES))
            .exec(&transaction)
            .await
            .map_err(AppError::database)?;
        if updated.rows_affected > 0 {
            audit_log::Entity::insert(audit_log::ActiveModel {
                id: Set(Uuid::new_v4()),
                actor_user_id: Set(None),
                action: Set("fleet_alert.resolved".to_string()),
                entity_type: Set("fleet_alert".to_string()),
                entity_id: Set(Some(agent_id.to_string())),
                payload: Set(redact_json(resolution_detail)),
                created_at: Set(now()),
            })
            .exec(&transaction)
            .await
            .map_err(AppError::database)?;
        }
        transaction.commit().await.map_err(AppError::database)?;
        Ok(updated.rows_affected)
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
        let row = agent_log::Entity::insert(agent_log::ActiveModel {
            id: Set(id),
            agent_id: Set(agent_id),
            stream: Set(stream.to_string()),
            message: Set(redact_text(message)),
            created_at: Set(ts),
        })
        .exec_with_returning(&self.db)
        .await
        .map_err(AppError::database)?;
        Ok(AgentLogEntry {
            id: row.id,
            agent_id: row.agent_id,
            stream: row.stream,
            message: row.message,
            created_at: api_ts(row.created_at),
        })
    }

    async fn find_user_by_email(
        &self,
        email: &str,
    ) -> Result<Option<app::auth::UserRecord>, AppError> {
        user::Entity::find()
            .filter(user::Column::Email.eq(email))
            .filter(user::Column::CentralSub.is_null())
            .one(&self.db)
            .await
            .map_err(AppError::database)
            .map(|row| row.map(user_record))
    }

    async fn find_or_create_central_user(
        &self,
        sub: &str,
        email: &str,
        display_name: &str,
    ) -> Result<app::auth::UserRecord, AppError> {
        if sub.trim().is_empty() || email.trim().is_empty() || display_name.trim().is_empty() {
            return Err(AppError::Unauthorized);
        }
        // Verified, unchanged profiles must not queue behind user write locks.
        if let Some(model) = user::Entity::find()
            .filter(user::Column::CentralSub.eq(sub.trim()))
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            && model.is_active
            && model.display_name == display_name.trim()
        {
            return Ok(user_record(model));
        }
        let id = Uuid::new_v4();
        self.db
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "INSERT INTO users (id, email, username, display_name, password_hash, central_sub, \
               system_role, is_system_admin, is_active, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, '!', $5, 'user', false, true, now(), now()) \
             ON CONFLICT (central_sub) WHERE central_sub IS NOT NULL DO UPDATE \
             SET display_name = EXCLUDED.display_name, updated_at = now() \
             WHERE users.is_active AND users.display_name IS DISTINCT FROM EXCLUDED.display_name",
                [
                    id.into(),
                    email.trim().to_lowercase().into(),
                    format!("central-{}", id.simple()).into(),
                    display_name.trim().into(),
                    sub.trim().into(),
                ],
            ))
            .await
            .map_err(AppError::database)?;
        let model = user::Entity::find()
            .filter(user::Column::CentralSub.eq(sub.trim()))
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or(AppError::Unauthorized)?;
        if !model.is_active {
            return Err(AppError::Unauthorized);
        }
        Ok(user_record(model))
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
            central_sub: sea_orm::ActiveValue::NotSet,
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
        let product_job = matches!(
            req.job_kind,
            DeploymentJobKind::ProductDeploy | DeploymentJobKind::ProductRollback
        );
        let detail = if product_job {
            if req.agent_id.is_some() || req.runtime_kind.is_some() || req.detail.is_some() {
                return Err(AppError::validation(
                    "product jobs cannot target agent runtimes",
                ));
            }
            if req.environment.as_deref() != Some("demo") || req.idempotency_key.is_none() {
                return Err(AppError::validation(
                    "product jobs require environment demo and idempotency_key",
                ));
            }
            match req.job_kind {
                DeploymentJobKind::ProductDeploy => {
                    let sha = req.commit_sha.as_deref().unwrap_or_default();
                    if sha.len() != 40
                        || !sha.bytes().all(|byte| byte.is_ascii_hexdigit())
                        || req.previous_release_id.is_some()
                    {
                        return Err(AppError::validation(
                            "product_deploy requires exact commit_sha only",
                        ));
                    }
                }
                DeploymentJobKind::ProductRollback => {
                    if req.previous_release_id.is_none() || req.commit_sha.is_some() {
                        return Err(AppError::validation(
                            "product_rollback requires previous_release_id only",
                        ));
                    }
                }
                _ => unreachable!(),
            }
            json!({
                "environment": "demo",
                "commit_sha": req.commit_sha,
                "previous_release_id": req.previous_release_id,
                "idempotency_key": req.idempotency_key,
            })
        } else {
            if req.environment.is_some()
                || req.commit_sha.is_some()
                || req.previous_release_id.is_some()
                || req.idempotency_key.is_some()
            {
                return Err(AppError::validation(
                    "product deployment fields require a product job kind",
                ));
            }
            redact_json(req.detail.unwrap_or_else(|| json!({})))
        };
        if let Some(key) = req.idempotency_key
            && let Some(existing) = deployment_job::Entity::find()
                .filter(deployment_job::Column::IdempotencyKey.eq(key))
                .one(&self.db)
                .await
                .map_err(AppError::database)?
        {
            if existing.job_kind != req.job_kind.as_str() || existing.detail != detail {
                return Err(AppError::conflict(
                    "idempotency key belongs to another request",
                ));
            }
            return Ok(deployment_job_from_model(existing));
        }
        if let Some(agent_id) = req.agent_id {
            load_agent_row(&self.db, agent_id).await?;
        }
        let id = Uuid::new_v4();
        let ts = now();
        let inserted = deployment_job::Entity::insert(deployment_job::ActiveModel {
            id: Set(id),
            job_kind: Set(req.job_kind.as_str().to_string()),
            state: Set(DeploymentJobState::Queued.as_str().to_string()),
            agent_id: Set(req.agent_id),
            runtime_kind: Set(req.runtime_kind.map(|kind| kind.as_str().to_string())),
            requested_by_user_id: Set(Some(requested_by_user_id)),
            title: Set(req.title.trim().to_string()),
            detail: Set(detail.clone()),
            idempotency_key: Set(req.idempotency_key),
            last_error: Set(None),
            created_at: Set(ts),
            updated_at: Set(ts),
        })
        .exec(&self.db)
        .await;
        if let Err(error) = inserted {
            if let Some(key) = req.idempotency_key
                && let Some(existing) = deployment_job::Entity::find()
                    .filter(deployment_job::Column::IdempotencyKey.eq(key))
                    .one(&self.db)
                    .await
                    .map_err(AppError::database)?
            {
                if existing.job_kind == req.job_kind.as_str() && existing.detail == detail {
                    return Ok(deployment_job_from_model(existing));
                }
                return Err(AppError::conflict(
                    "idempotency key belongs to another request",
                ));
            }
            return Err(AppError::database(error));
        }
        self.get_deployment_job(id).await
    }

    async fn bulk_create_deployment_jobs(
        &self,
        req: BulkDeploymentRequest,
        requested_by_user_id: Uuid,
    ) -> Result<BulkDeploymentResult, AppError> {
        let rollback = validate_bulk_deployment_request(&req)?;
        let mut detail = req.detail.unwrap_or_else(|| json!({}));
        if rollback && let Some(object) = detail.as_object_mut() {
            object.insert("rollback".to_string(), json!(true));
        }
        let detail = redact_json(detail);
        let ts = now();
        let mut jobs = Vec::with_capacity(req.agent_ids.len());
        let mut skipped = 0usize;
        for agent_id in &req.agent_ids {
            if load_agent_row(&self.db, *agent_id).await.is_err() {
                skipped += 1;
                continue;
            }
            let id = Uuid::new_v4();
            deployment_job::Entity::insert(deployment_job::ActiveModel {
                id: Set(id),
                job_kind: Set(req.job_kind.as_str().to_string()),
                state: Set(DeploymentJobState::Queued.as_str().to_string()),
                agent_id: Set(Some(*agent_id)),
                runtime_kind: Set(req.runtime_kind.map(|kind| kind.as_str().to_string())),
                requested_by_user_id: Set(Some(requested_by_user_id)),
                title: Set(format!(
                    "{}{}",
                    req.title.trim(),
                    if rollback { " (rollback)" } else { "" }
                )),
                detail: Set(detail.clone()),
                idempotency_key: Set(None),
                last_error: Set(None),
                created_at: Set(ts),
                updated_at: Set(ts),
            })
            .exec(&self.db)
            .await
            .map_err(AppError::database)?;
            jobs.push(self.get_deployment_job(id).await?);
        }
        let created = jobs.len();
        Ok(BulkDeploymentResult {
            jobs,
            created,
            skipped,
        })
    }

    async fn update_deployment_job_state(
        &self,
        job_id: Uuid,
        state: DeploymentJobState,
        detail_patch: Option<Value>,
        last_error: Option<String>,
    ) -> Result<DeploymentJob, AppError> {
        let row = deployment_job::Entity::find_by_id(job_id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("deployment_job", job_id))?;
        if matches!(
            parse_deployment_job_kind(&row.job_kind),
            DeploymentJobKind::ProductDeploy | DeploymentJobKind::ProductRollback
        ) {
            self.db
                .execute(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    "UPDATE deployment_jobs \
                     SET state = $2, detail = detail || $3::jsonb, last_error = $4, updated_at = now() \
                     WHERE id = $1 AND (state = 'running' OR (state = 'queued' AND $2 IN ('running', 'failed')))",
                    vec![
                        job_id.into(),
                        state.as_str().to_owned().into(),
                        detail_patch.unwrap_or_else(|| json!({})).to_string().into(),
                        last_error.into(),
                    ],
                ))
                .await
                .map_err(AppError::database)?;
            return self.get_deployment_job(job_id).await;
        }
        let detail_base = row.detail.clone();
        let mut model = row.into_active_model();
        model.state = Set(state.as_str().to_string());
        if let Some(patch) = detail_patch {
            let mut detail = detail_base;
            if let (Some(dst), Some(src)) = (detail.as_object_mut(), patch.as_object()) {
                for (k, v) in src {
                    dst.insert(k.clone(), v.clone());
                }
            }
            model.detail = Set(detail);
        }
        model.last_error = Set(last_error);
        model.updated_at = Set(now());
        let updated = model.update(&self.db).await.map_err(AppError::database)?;
        Ok(deployment_job_from_model(updated))
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
        let product_job = matches!(
            parse_deployment_job_kind(&row.job_kind),
            DeploymentJobKind::ProductDeploy | DeploymentJobKind::ProductRollback
        );
        let mut model = row.into_active_model();
        if product_job {
            model.state = Set(DeploymentJobState::Failed.as_str().to_string());
            model.last_error = Set(Some("cancelled by operator".to_string()));
        } else {
            model.state = Set(DeploymentJobState::Cancelled.as_str().to_string());
        }
        model.updated_at = Set(now());
        let updated = model.update(&self.db).await.map_err(AppError::database)?;
        Ok(deployment_job_from_model(updated))
    }

    async fn get_runtime_settings(&self, config: &AppConfig) -> Result<RuntimeSettings, AppError> {
        Ok(runtime_settings_from_config(config))
    }

    async fn get_port_settings(&self, config: &AppConfig) -> Result<PortSettings, AppError> {
        Ok(port_settings_from_config(config))
    }

    async fn get_integration_settings(
        &self,
        config: &AppConfig,
    ) -> Result<IntegrationSettings, AppError> {
        Ok(integration_settings_from_config(config))
    }

    async fn get_auth_settings(&self, config: &AppConfig) -> Result<AuthSettings, AppError> {
        Ok(auth_settings_from_config(config))
    }

    async fn get_active_managed_settings(
        &self,
    ) -> Result<Option<ManagedSettingsVersion>, AppError> {
        managed_settings_version::Entity::find()
            .filter(managed_settings_version::Column::IsActive.eq(true))
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .map(managed_settings_entry)
            .transpose()
    }

    async fn get_managed_settings_version(
        &self,
        version: i64,
    ) -> Result<ManagedSettingsVersion, AppError> {
        let row = managed_settings_version::Entity::find()
            .filter(managed_settings_version::Column::Version.eq(version))
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("managed_settings_version", version))?;
        managed_settings_entry(row)
    }

    async fn list_managed_settings_versions(
        &self,
        limit: u64,
    ) -> Result<Vec<ManagedSettingsVersion>, AppError> {
        let rows = managed_settings_version::Entity::find()
            .order_by_desc(managed_settings_version::Column::Version)
            .limit(limit.clamp(1, 100))
            .all(&self.db)
            .await
            .map_err(AppError::database)?;
        rows.into_iter().map(managed_settings_entry).collect()
    }

    async fn activate_managed_settings(
        &self,
        snapshot: ManagedSettingsSnapshot,
        actor_user_id: Uuid,
        expected_active_version: Option<i64>,
        rollback_of_version: Option<i64>,
        audit_action: &str,
    ) -> Result<ManagedSettingsVersion, AppError> {
        let txn = self.db.begin().await.map_err(AppError::database)?;
        txn.execute(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT pg_advisory_xact_lock(238010008)".to_string(),
        ))
        .await
        .map_err(AppError::database)?;

        let active = managed_settings_version::Entity::find()
            .filter(managed_settings_version::Column::IsActive.eq(true))
            .one(&txn)
            .await
            .map_err(AppError::database)?;
        let actual_active_version = active.as_ref().map(|row| row.version);
        if actual_active_version != expected_active_version {
            return Err(AppError::conflict(format!(
                "managed settings changed concurrently: expected version {expected_active_version:?}, active version is {actual_active_version:?}"
            )));
        }

        if let Some(active) = active {
            let mut model = active.into_active_model();
            model.is_active = Set(false);
            model.update(&txn).await.map_err(AppError::database)?;
        }

        let row = managed_settings_version::ActiveModel {
            id: Set(Uuid::new_v4()),
            version: NotSet,
            snapshot: Set(serde_json::to_value(&snapshot).map_err(AppError::internal)?),
            created_by_user_id: Set(Some(actor_user_id)),
            rollback_of_version: Set(rollback_of_version),
            created_at: Set(now()),
            is_active: Set(true),
        }
        .insert(&txn)
        .await
        .map_err(AppError::database)?;

        audit_log::ActiveModel {
            id: Set(Uuid::new_v4()),
            actor_user_id: Set(Some(actor_user_id)),
            action: Set(audit_action.to_string()),
            entity_type: Set("managed_settings_version".to_string()),
            entity_id: Set(Some(row.version.to_string())),
            payload: Set(json!({
                "version": row.version,
                "previous_version": expected_active_version,
                "rollback_of_version": rollback_of_version,
                "snapshot": snapshot,
            })),
            created_at: Set(now()),
        }
        .insert(&txn)
        .await
        .map_err(AppError::database)?;

        txn.commit().await.map_err(AppError::database)?;
        managed_settings_entry(row)
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
        last_message_preview: row.last_message_preview.map(|value| redact_text(&value)),
        pending_delivery: None,
        task_bound: None,
        created_at: api_ts(row.created_at),
        updated_at: api_ts(row.updated_at),
    }
}

fn selected_primary_agent_id(req: &CreateSessionRequest) -> Result<Uuid, AppError> {
    req.primary_agent_id
        .or(req.agent_id)
        .ok_or_else(|| AppError::validation("primary_agent_id is required"))
}

fn pending_session_run(
    session_id: Uuid,
    agent_id: Uuid,
    run_role: SessionRunRole,
    timestamp: shared::Timestamp,
) -> session_agent_run::ActiveModel {
    session_agent_run::ActiveModel {
        id: Set(Uuid::new_v4()),
        session_id: Set(session_id),
        agent_id: Set(agent_id),
        runtime_session_id: Set(None),
        runtime_run_id: Set(None),
        run_role: Set(run_role.as_str().to_string()),
        state: Set(SessionRunState::Pending.as_str().to_string()),
        last_error: Set(None),
        last_event_at: Set(None),
        model: Set(None),
        provider: Set(None),
        model_options: Set(json!({})),
        created_at: Set(timestamp),
        updated_at: Set(timestamp),
    }
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

async fn session_message_from_model(
    db: &DatabaseConnection,
    row: session_message::Model,
) -> Result<SessionMessage, AppError> {
    let author_type = parse_message_author_type(&row.author_type);
    let author_user = match row.author_user_id {
        Some(user_id) => user::Entity::find_by_id(user_id)
            .one(db)
            .await
            .map_err(AppError::database)?,
        None => None,
    };
    let author_agent = match row.author_agent_id {
        Some(agent_id) => agent::Entity::find_by_id(agent_id)
            .one(db)
            .await
            .map_err(AppError::database)?,
        None => None,
    };
    Ok(SessionMessage {
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
        body: redact_text(&row.body),
        message_kind: parse_message_kind(&row.message_kind),
        runtime_message_id: row.runtime_message_id,
        delivery_state: parse_message_delivery_state(&row.delivery_state),
        delivery_error: row.delivery_error.map(|error| redact_text(&error)),
        replayed: false,
        request_payload_hash: None,
        created_at: api_ts(row.created_at),
    })
}

async fn session_message_receipt(
    db: &DatabaseConnection,
    row: session_message::Model,
) -> Result<SessionMessage, AppError> {
    let request_payload_hash = row.idempotency_payload_hash.clone();
    let mut message = session_message_from_model(db, row).await?;
    message.request_payload_hash = request_payload_hash;
    Ok(message)
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

fn redact_configuration_json(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| {
                    let secret = domain::is_configuration_secret_name(&key);
                    let safe_reference = value.as_object().is_some_and(|object| {
                        object.len() == 1
                            && object
                                .get("secret_ref")
                                .and_then(Value::as_str)
                                .is_some_and(|reference| {
                                    !reference.is_empty()
                                        && reference.len() <= 128
                                        && reference.bytes().all(|ch| {
                                            ch.is_ascii_uppercase()
                                                || ch.is_ascii_digit()
                                                || ch == b'_'
                                        })
                                })
                    });
                    let value = if secret && !safe_reference {
                        Value::String("redacted".into())
                    } else if safe_reference {
                        value
                    } else {
                        redact_configuration_json(value)
                    };
                    (key, value)
                })
                .collect(),
        ),
        Value::Array(items) => {
            Value::Array(items.into_iter().map(redact_configuration_json).collect())
        }
        Value::String(value) => Value::String(redact_text(&value)),
        other => other,
    }
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
        Value::String(value) => Value::String(redact_text(&value)),
        other => other,
    }
}

fn redact_text(value: &str) -> String {
    let mut output = value.to_string();
    for (name, secret) in std::env::vars() {
        if (name.starts_with("FLEET_CONTROL_SECRET__")
            || name == "FLEET_CONTROL_RUNTIME__TOKEN_SECRET"
            || name == "FLEET_CONTROL_FLEET__RUNTIME_TOKEN_SECRET")
            && secret.len() >= 4
        {
            output = output.replace(&secret, "redacted");
        }
    }
    for marker in ["token=", "password=", "secret=", "api_key=", "apikey="] {
        if let Some(pos) = output.to_ascii_lowercase().find(marker) {
            output.truncate(pos + marker.len());
            output.push_str("redacted");
        }
    }
    let mut offset = 0;
    while let Some(start) = output[offset..].find("fc_").map(|start| start + offset) {
        let length = output[start + 3..]
            .bytes()
            .take_while(u8::is_ascii_hexdigit)
            .count();
        if length < 32 {
            offset = start + 3;
            continue;
        }
        output.replace_range(start..start + 3 + length, "redacted");
        offset = start + "redacted".len();
    }
    output
}

fn redact_stream_text(value: &str, secrets: &[String]) -> String {
    let mut end = value.len();
    // Withhold a suffix that might be the beginning of a credential split across SSE frames.
    for secret in secrets.iter().filter(|secret| secret.len() >= 4) {
        for (offset, _) in secret.char_indices().skip(1) {
            if value.ends_with(&secret[..offset]) {
                end = end.min(value.len() - offset);
            }
        }
    }
    let mut output = redact_text(&value[..end]);
    for secret in secrets {
        output = output.replace(secret, "redacted");
    }
    output
}

#[derive(Clone, Default)]
pub struct FilesystemProvisioner;

#[async_trait]
impl AgentProvisioner for FilesystemProvisioner {
    async fn verify_effective_configuration(
        &self,
        agent: &Agent,
        config: &AppConfig,
        revision: &domain::AgentConfigRevision,
    ) -> Result<(), AppError> {
        effective_configuration::verify(agent, config, revision).await
    }
    async fn provision(&self, agent: &Agent, config: &AppConfig) -> Result<(), AppError> {
        if !matches!(
            agent.status,
            AgentStatus::Provisioning | AgentStatus::Ready | AgentStatus::Stopped
        ) || agent.runtime.pid.is_some()
        {
            return Err(AppError::conflict(
                "stop and reconcile the runtime before provisioning",
            ));
        }
        // Java agents share the same layout; the runtime jar is expected at
        // agents/agentN/runtime/backend.jar and is copied by the build
        // pipeline (java_agent_source), not provisioned here. Missing jar is
        // reported at start time by the supervisor.

        let root = PathBuf::from(&config.fleet.agents_root);
        let agent_root = safe_agent_root(&root, &agent.name)?;
        reject_symlink_components(&root, &agent_root).await?;
        if tokio::fs::try_exists(&agent_root)
            .await
            .map_err(AppError::internal)?
            && !tokio::fs::try_exists(agent_root.join(".fleet-agent.json"))
                .await
                .map_err(AppError::internal)?
            && tokio::fs::read_dir(&agent_root)
                .await
                .map_err(AppError::internal)?
                .next_entry()
                .await
                .map_err(AppError::internal)?
                .is_some()
        {
            return Err(AppError::conflict(
                "refusing to adopt a nonempty directory without an agent marker",
            ));
        }
        tokio::fs::create_dir_all(&agent_root)
            .await
            .map_err(AppError::internal)?;
        let marker_path = agent_root.join(".fleet-agent.json");
        reject_symlink_components(&root, &marker_path).await?;
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
            reject_symlink_components(&root, &path).await?;
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

    async fn storage_report(
        &self,
        agent: &Agent,
        config: &AppConfig,
    ) -> Result<AgentStorageReport, AppError> {
        inspect_agent_storage(agent, config).await
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

async fn inspect_agent_storage(
    agent: &Agent,
    config: &AppConfig,
) -> Result<AgentStorageReport, AppError> {
    let root = PathBuf::from(&config.fleet.agents_root);
    let agent_root = safe_agent_root(&root, &agent.name)?;
    let root_path = normalize_path(&agent_root)?.to_string_lossy().to_string();
    let root_exists = tokio::fs::symlink_metadata(&agent_root)
        .await
        .map(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
        .unwrap_or(false);
    let (marker_present, marker_verified) = inspect_agent_marker(&agent_root, agent).await?;
    let areas = vec![
        scan_storage_area(&root, "runtime", agent_root.join("runtime")).await?,
        scan_storage_area(&root, "config", agent_root.join("config")).await?,
        scan_storage_area(&root, "workspace", agent_root.join("workspace")).await?,
        scan_storage_area(&root, "logs", agent_root.join("logs")).await?,
    ];
    let total_bytes = areas.iter().map(|area| area.bytes).sum();
    let total_files = areas.iter().map(|area| area.files).sum();
    let total_directories = areas.iter().map(|area| area.directories).sum();
    let total_symlinks = areas.iter().map(|area| area.symlinks).sum();
    let purge_eligible = agent.status == AgentStatus::Archived && root_exists && marker_verified;
    let archived_days =
        (agent.status == AgentStatus::Archived).then(|| days_since(&agent.updated_at));
    let stale = archived_days
        .map(|days| days >= i64::from(config.fleet.retention.stale_archived_days))
        .unwrap_or(false);

    Ok(AgentStorageReport {
        agent_id: agent.id,
        agent_name: agent.name.clone(),
        root_path,
        root_exists,
        marker_present,
        marker_verified,
        total_bytes,
        total_files,
        total_directories,
        total_symlinks,
        areas,
        retention: AgentRetentionReport {
            archived: agent.status == AgentStatus::Archived,
            archived_since: (agent.status == AgentStatus::Archived)
                .then(|| agent.updated_at.clone()),
            purge_eligible,
            stale,
            archived_days,
            retention_hint: retention_hint(
                agent,
                root_exists,
                marker_verified,
                stale,
                &config.fleet.retention,
            ),
        },
    })
}

async fn inspect_agent_marker(agent_root: &Path, agent: &Agent) -> Result<(bool, bool), AppError> {
    let marker_path = agent_root.join(".fleet-agent.json");
    let metadata = match tokio::fs::symlink_metadata(&marker_path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok((false, false)),
        Err(error) => return Err(AppError::internal(error)),
    };
    if metadata.file_type().is_symlink() {
        return Ok((true, false));
    }
    let Ok(marker) = tokio::fs::read_to_string(&marker_path).await else {
        return Ok((true, false));
    };
    let Ok(marker) = serde_json::from_str::<Value>(&marker) else {
        return Ok((true, false));
    };
    let marker_id_matches = marker
        .get("id")
        .and_then(Value::as_str)
        .is_some_and(|id| id == agent.id.to_string());
    let marker_name_matches = marker
        .get("name")
        .and_then(Value::as_str)
        .is_none_or(|name| name == agent.name);
    Ok((true, marker_id_matches && marker_name_matches))
}

#[derive(Default)]
struct StorageCounters {
    bytes: u64,
    files: u64,
    directories: u64,
    symlinks: u64,
    last_modified_at: Option<String>,
}

async fn scan_storage_area(
    root: &Path,
    name: &str,
    path: PathBuf,
) -> Result<AgentStorageArea, AppError> {
    ensure_inside(root, &path)?;
    let normalized = normalize_path(&path)?.to_string_lossy().to_string();
    let metadata = match tokio::fs::symlink_metadata(&path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Ok(AgentStorageArea {
                name: name.to_string(),
                path: normalized,
                exists: false,
                is_directory: false,
                bytes: 0,
                files: 0,
                directories: 0,
                symlinks: 0,
                last_modified_at: None,
            });
        }
        Err(error) => return Err(AppError::internal(error)),
    };

    let mut counters = StorageCounters::default();
    update_last_modified(&mut counters, &metadata);
    if metadata.file_type().is_symlink() {
        counters.symlinks = 1;
    } else if metadata.is_file() {
        counters.bytes = metadata.len();
        counters.files = 1;
    } else if metadata.is_dir() {
        scan_directory(root, &path, &mut counters).await?;
    }

    Ok(AgentStorageArea {
        name: name.to_string(),
        path: normalized,
        exists: true,
        is_directory: metadata.is_dir() && !metadata.file_type().is_symlink(),
        bytes: counters.bytes,
        files: counters.files,
        directories: counters.directories,
        symlinks: counters.symlinks,
        last_modified_at: counters.last_modified_at,
    })
}

async fn scan_directory(
    root: &Path,
    start: &Path,
    counters: &mut StorageCounters,
) -> Result<(), AppError> {
    let mut stack = vec![start.to_path_buf()];
    while let Some(dir) = stack.pop() {
        ensure_inside(root, &dir)?;
        let mut entries = tokio::fs::read_dir(&dir)
            .await
            .map_err(AppError::internal)?;
        while let Some(entry) = entries.next_entry().await.map_err(AppError::internal)? {
            let path = entry.path();
            ensure_inside(root, &path)?;
            let metadata = tokio::fs::symlink_metadata(&path)
                .await
                .map_err(AppError::internal)?;
            update_last_modified(counters, &metadata);
            if metadata.file_type().is_symlink() {
                counters.symlinks += 1;
            } else if metadata.is_dir() {
                counters.directories += 1;
                stack.push(path);
            } else if metadata.is_file() {
                counters.files += 1;
                counters.bytes += metadata.len();
            }
        }
    }
    Ok(())
}

fn update_last_modified(counters: &mut StorageCounters, metadata: &std::fs::Metadata) {
    let Some(modified) = metadata
        .modified()
        .ok()
        .map(chrono::DateTime::<chrono::Utc>::from)
        .map(|timestamp| timestamp.to_rfc3339())
    else {
        return;
    };
    if counters
        .last_modified_at
        .as_ref()
        .is_none_or(|current| modified > *current)
    {
        counters.last_modified_at = Some(modified);
    }
}

fn days_since(timestamp: &str) -> i64 {
    let now = chrono::Utc::now();
    let ts = chrono::DateTime::parse_from_rfc3339(timestamp)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or(now);
    (now - ts).num_days()
}

fn retention_hint(
    agent: &Agent,
    root_exists: bool,
    marker_verified: bool,
    stale: bool,
    retention: &shared::RetentionConfig,
) -> String {
    if !root_exists {
        return "agent folder is already absent".to_string();
    }
    if !marker_verified {
        return "folder marker must match this agent before purge is allowed".to_string();
    }
    if agent.status == AgentStatus::Archived {
        if stale {
            return format!(
                "archived agent is stale (over {} days); review and purge it",
                retention.stale_archived_days
            );
        }
        return "archived agent files can be purged explicitly by an operator".to_string();
    }
    "archive the agent before physical purge".to_string()
}

async fn provision_hermes(agent: &Agent, config: &AppConfig) -> Result<(), AppError> {
    let config_path = PathBuf::from(&agent.paths.config);
    let runtime_path = PathBuf::from(&agent.paths.runtime);
    for file in ["config.yaml", "SOUL.md", ".env", "skills", "sessions"] {
        reject_symlink_components(
            Path::new(&config.fleet.agents_root),
            &config_path.join(file),
        )
        .await?;
    }
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
            hermes_workspace(agent, config).replace('\\', "/"),
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
    write_if_missing(
        config_path.join(".env"),
        provisioned_hermes_env(agent, config)?,
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

fn hermes_workspace<'a>(agent: &'a Agent, config: &AppConfig) -> &'a str {
    if agent.kind == AgentKind::Hermes && config.fleet.container_control.is_some() {
        "/workspace"
    } else {
        &agent.paths.workspace
    }
}

fn hermes_home<'a>(agent: &'a Agent, config: &AppConfig) -> &'a str {
    if agent.kind == AgentKind::Hermes && config.fleet.container_control.is_some() {
        "/config"
    } else {
        &agent.paths.config
    }
}

fn provisioned_hermes_env(agent: &Agent, config: &AppConfig) -> Result<String, AppError> {
    Ok(format!(
        "# Managed by Fleet Control. Secrets are redacted in API responses.\nHERMES_HOME={}\nHERMES_SERVE_HEADLESS=1\nAPI_SERVER_ENABLED=true\nAPI_SERVER_KEY={}\nAPI_SERVER_CORS_ORIGINS={}\n",
        hermes_home(agent, config).replace('\\', "/"),
        agent_runtime_token(config, agent.id)?,
        config.server.cors_allowed_origins.join(",")
    ))
}

async fn reject_symlink_components(root: &Path, path: &Path) -> Result<(), AppError> {
    let root = normalize_path(root)?;
    let path = normalize_path(path)?;
    let suffix = path
        .strip_prefix(&root)
        .map_err(|_| AppError::validation("path escapes agents root"))?;
    let mut current = root;
    for component in std::iter::once(None).chain(suffix.components().map(Some)) {
        if let Some(component) = component {
            current.push(component.as_os_str());
        }
        match tokio::fs::symlink_metadata(&current).await {
            Ok(metadata) => {
                #[cfg(windows)]
                let reparse_point = {
                    use std::os::windows::fs::MetadataExt;
                    metadata.file_attributes() & 0x400 != 0
                };
                #[cfg(not(windows))]
                let reparse_point = false;
                if metadata.file_type().is_symlink() || reparse_point {
                    return Err(AppError::validation(
                        "managed paths cannot traverse symlinks or junctions",
                    ));
                }
            }
            Err(error) if error.kind() == ErrorKind::NotFound => break,
            Err(error) => return Err(AppError::internal(error)),
        }
    }
    Ok(())
}

pub(crate) async fn configuration_files(
    agent: &Agent,
    config: &AppConfig,
    revision: &domain::AgentConfigRevision,
) -> Result<Vec<(PathBuf, String)>, AppError> {
    let root = Path::new(&config.fleet.agents_root);
    let agent_root = safe_agent_root(root, &agent.name)?;
    reject_symlink_components(root, &agent_root).await?;
    let (marker, verified) = inspect_agent_marker(&agent_root, agent).await?;
    if !marker || !verified {
        return Err(AppError::validation(
            "provisioned agent marker must match before activation",
        ));
    }
    let expected = agent_root.join("config");
    if normalize_path(Path::new(&agent.paths.config))? != normalize_path(&expected)? {
        return Err(AppError::validation(
            "agent config path does not match its isolated layout",
        ));
    }
    let mut content = revision.snapshot.config.config_json.clone();
    pm_tool_config::configure(&mut content, agent, config)?;
    if config.pm.dispatch.enabled && agent.sdlc_role == Some(SdlcRole::ProjectManager) {
        pm_tool_config::reject_server_secrets(&content.to_string(), config)?;
        pm_tool_config::reject_server_secrets(&revision.snapshot.config.soul_md, config)?;
    }
    if content
        .get("terminal")
        .is_some_and(|value| !value.is_object())
    {
        return Err(AppError::validation(
            "terminal configuration must be an object",
        ));
    }
    content["terminal"]["cwd"] = json!(hermes_workspace(agent, config));
    let mut env = format!(
        "HERMES_HOME={}\nHERMES_SERVE_HEADLESS=1\nAPI_SERVER_ENABLED=true\nAPI_SERVER_KEY={}\n",
        serde_json::to_string(hermes_home(agent, config)).map_err(AppError::internal)?,
        agent_runtime_token(config, agent.id)?
    );
    if let Some(values) = revision.snapshot.config.env_json.as_object() {
        for (key, value) in values {
            if agent.kind == AgentKind::Hermes
                && config.fleet.container_control.is_some()
                && (key.starts_with("DOCKER_")
                    || key == "CONTAINER_HOST"
                    || matches!(
                        key.as_str(),
                        "HOME" | "API_SERVER_HOST" | "API_SERVER_PORT" | "API_SERVER_CORS_ORIGINS"
                    ))
            {
                return Err(AppError::validation(
                    "Docker runtime environment authority is reserved",
                ));
            }
            if matches!(
                key.as_str(),
                "HERMES_HOME" | "HERMES_SERVE_HEADLESS" | "API_SERVER_ENABLED" | "API_SERVER_KEY"
            ) {
                continue;
            }
            let value = if let Some(reference) = value.get("secret_ref").and_then(Value::as_str) {
                std::env::var(format!("FLEET_CONTROL_SECRET__{reference}")).map_err(|_| {
                    AppError::validation(format!("secret_ref for {key} is unavailable"))
                })?
            } else {
                value.as_str().unwrap_or_default().to_string()
            };
            if value == "redacted" {
                return Err(AppError::validation(format!(
                    "masked value for {key} cannot replace a secret reference"
                )));
            }
            if config.pm.dispatch.enabled && agent.sdlc_role == Some(SdlcRole::ProjectManager) {
                pm_tool_config::reject_server_secrets(&value, config)?;
            }
            env.push_str(&format!(
                "{key}={}\n",
                serde_json::to_string(&value).map_err(AppError::internal)?
            ));
        }
    }
    let mut files = vec![
        (
            expected.join("config.yaml"),
            serde_json::to_string_pretty(&content).map_err(AppError::internal)?,
        ),
        (
            expected.join("SOUL.md"),
            revision.snapshot.config.soul_md.clone(),
        ),
        (expected.join(".env"), env),
    ];
    for skill in &revision.snapshot.skills {
        if skill.name.is_empty()
            || !skill
                .name
                .bytes()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, b'-' | b'_'))
        {
            return Err(AppError::validation(
                "skill name is not a safe directory name",
            ));
        }
        let dir = expected.join("skills").join(&skill.name);
        reject_symlink_components(root, &dir).await?;
        let path = dir.join("SKILL.md");
        let body = if skill.state == SkillState::Enabled {
            skill
                .content
                .clone()
                .filter(|content| !content.trim().is_empty())
                .ok_or_else(|| {
                    AppError::validation(format!(
                        "enabled skill {} has no installed content",
                        skill.name
                    ))
                })?
        } else {
            String::new()
        };
        files.push((path, body));
    }
    let hashes: serde_json::Map<String, Value> = files
        .iter()
        .map(|(path, body)| {
            (
                path.strip_prefix(&expected)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .into_owned(),
                if body.is_empty() && path.file_name().is_some_and(|name| name == "SKILL.md") {
                    Value::Null
                } else {
                    json!(hex::encode(Sha256::digest(body.as_bytes())))
                },
            )
        })
        .collect();
    files.push((
        expected.join(".fleet-config-revision.json"),
        serde_json::json!({"agent_id":agent.id,"revision":revision.revision,"hashes":hashes})
            .to_string(),
    ));
    for (path, _) in &files {
        reject_symlink_components(root, path).await?;
    }
    Ok(files)
}

pub(crate) async fn write_configuration_file(path: &Path, body: &[u8]) -> Result<(), AppError> {
    use tokio::io::AsyncWriteExt;
    let temporary = path.with_file_name(format!(".fleet-next-{}", Uuid::new_v4()));
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(&temporary).await.map_err(AppError::internal)?;
    file.write_all(body).await.map_err(AppError::internal)?;
    file.sync_all().await.map_err(AppError::internal)?;
    drop(file);
    tokio::fs::rename(&temporary, path)
        .await
        .map_err(AppError::internal)
}

async fn write_if_missing(path: PathBuf, content: String) -> Result<(), AppError> {
    use tokio::io::AsyncWriteExt;
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = match options.open(path).await {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::AlreadyExists => return Ok(()),
        Err(error) => return Err(AppError::internal(error)),
    };
    file.write_all(content.as_bytes())
        .await
        .map_err(AppError::internal)?;
    file.sync_all().await.map_err(AppError::internal)
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

    #[test]
    fn docker_bootstrap_projection_preserves_process_and_java_paths() {
        let root = Path::new("/agents");
        let mut config = test_config(root);
        let mut agent = test_agent(root, Uuid::new_v4(), AgentStatus::Ready);
        assert_eq!(hermes_home(&agent, &config), agent.paths.config);
        assert_eq!(hermes_workspace(&agent, &config), agent.paths.workspace);
        config.fleet.container_control=Some(serde_json::from_value(json!({
            "python":"python3","base_root":"/base","context":"desktop-linux","controller_root":"/private"
        })).unwrap());
        assert_eq!(hermes_home(&agent, &config), "/config");
        assert_eq!(hermes_workspace(&agent, &config), "/workspace");
        assert!(
            provisioned_hermes_env(&agent, &config)
                .unwrap()
                .contains("\nHERMES_HOME=/config\n")
        );
        agent.kind = AgentKind::JavaAgent;
        assert_eq!(hermes_home(&agent, &config), agent.paths.config);
        assert_eq!(hermes_workspace(&agent, &config), agent.paths.workspace);
    }

    #[test]
    fn configuration_reads_mask_legacy_secrets_and_preserve_safe_references() {
        let value = redact_configuration_json(json!({
            "OPENAI_API_KEY": "legacy-credential",
            "GITHUB_TOKEN": {"secret_ref": "GITHUB"},
            "INVALID_TOKEN": {"secret_ref": "GITHUB", "plaintext": "credential"},
            "INVALID_API_KEY": {"secret_ref": "not a reference"},
            "CREDENTIAL": "old-credential",
            "PRIVATE_KEY": "old-private-key",
            "TASK_KEY": "FC-001",
            "model": {"provider": "openai", "api_key": "old-model-secret"},
            "entries": [{"password": "old-password"}],
            "ORDINARY_VALUE": "preserved"
        }));
        assert_eq!(value["OPENAI_API_KEY"], "redacted");
        assert_eq!(value["GITHUB_TOKEN"], json!({"secret_ref": "GITHUB"}));
        assert_eq!(value["INVALID_TOKEN"], "redacted");
        assert_eq!(value["INVALID_API_KEY"], "redacted");
        assert_eq!(value["CREDENTIAL"], "redacted");
        assert_eq!(value["PRIVATE_KEY"], "redacted");
        assert_eq!(value["TASK_KEY"], "FC-001");
        assert_eq!(value["model"]["provider"], "openai");
        assert_eq!(value["model"]["api_key"], "redacted");
        assert_eq!(value["entries"][0]["password"], "redacted");
        assert_eq!(value["ORDINARY_VALUE"], "preserved");
    }

    #[tokio::test]
    async fn provisioning_preserves_effective_env_and_restricts_new_files() {
        let root = temp_purge_root();
        let agent = test_agent(&root, Uuid::new_v4(), AgentStatus::Ready);
        let config = test_config(&root);
        FilesystemProvisioner
            .provision(&agent, &config)
            .await
            .unwrap();
        let env = Path::new(&agent.paths.config).join(".env");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                tokio::fs::metadata(&env)
                    .await
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        tokio::fs::write(&env, "EFFECTIVE_CONFIG=preserved\n")
            .await
            .unwrap();
        FilesystemProvisioner
            .provision(&agent, &config)
            .await
            .unwrap();
        assert_eq!(
            tokio::fs::read_to_string(&env).await.unwrap(),
            "EFFECTIVE_CONFIG=preserved\n"
        );
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn provisioning_rejects_active_runtime_without_creating_files() {
        let root = temp_purge_root();
        let agent = test_agent(&root, Uuid::new_v4(), AgentStatus::Running);
        assert!(
            FilesystemProvisioner
                .provision(&agent, &test_config(&root))
                .await
                .is_err()
        );
        assert!(!root.exists());
    }

    #[tokio::test]
    async fn provisioning_does_not_adopt_nonempty_unmarked_folder() {
        let root = temp_purge_root();
        let foreign = root.join("agent1");
        tokio::fs::create_dir_all(&foreign).await.unwrap();
        let file = foreign.join("foreign.txt");
        tokio::fs::write(&file, "preserved").await.unwrap();
        let agent = test_agent(&root, Uuid::new_v4(), AgentStatus::Ready);
        assert!(
            FilesystemProvisioner
                .provision(&agent, &test_config(&root))
                .await
                .is_err()
        );
        assert_eq!(tokio::fs::read_to_string(file).await.unwrap(), "preserved");
        assert!(!foreign.join(".fleet-agent.json").exists());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

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

    pub(super) fn test_agent(root: &Path, id: Uuid, status: AgentStatus) -> Agent {
        let paths = runtime_paths(&root.to_string_lossy(), 1);
        Agent {
            id,
            ordinal: 1,
            name: "agent1".to_string(),
            kind: AgentKind::Hermes,
            product_role: AgentProductRole::Executor,
            role: AgentRole::Developer,
            sdlc_role: Some(domain::SdlcRole::Developer),
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

    async fn effective_config_fixture() -> (PathBuf, Agent, AppConfig, domain::AgentConfigRevision)
    {
        let root = temp_purge_root();
        let config = test_config(&root);
        let agent = test_agent(&root, Uuid::new_v4(), AgentStatus::Running);
        write_marker(&root.join("agent1"), agent.id, &agent.name).await;
        tokio::fs::create_dir_all(&agent.paths.workspace)
            .await
            .unwrap();
        let skill = |name: &str, state: SkillState| domain::AgentSkill {
            id: Uuid::new_v4(),
            agent_id: agent.id,
            name: name.into(),
            title: name.into(),
            state,
            source: "test".into(),
            content: Some("# Test skill\n".into()),
            updated_at: now().to_rfc3339(),
        };
        let revision = domain::AgentConfigRevision {
            agent_id: agent.id,
            revision: 1,
            state: "active".into(),
            snapshot: domain::AgentConfigurationSnapshot {
                config: UpdateAgentConfigRequest {
                    config_json: json!({"model":"test-model"}),
                    soul_md: "# Test SOUL\n".into(),
                    env_json: json!({"TEST_SECRET":"test-secret-never-return"}),
                },
                skills: vec![
                    skill("enabled", SkillState::Enabled),
                    skill("disabled", SkillState::Disabled),
                ],
            },
            validation_errors: vec![],
            last_error: None,
            is_desired: true,
            is_effective: true,
            draining: false,
            created_at: now().to_rfc3339(),
        };
        install_effective_fixture(&agent, &config, &revision).await;
        (root, agent, config, revision)
    }

    async fn install_effective_fixture(
        agent: &Agent,
        config: &AppConfig,
        revision: &domain::AgentConfigRevision,
    ) {
        for (path, body) in configuration_files(agent, config, revision).await.unwrap() {
            if body.is_empty() && path.file_name().is_some_and(|name| name == "SKILL.md") {
                continue;
            }
            tokio::fs::create_dir_all(path.parent().unwrap())
                .await
                .unwrap();
            tokio::fs::write(path, body).await.unwrap();
        }
    }

    #[tokio::test]
    async fn effective_configuration_readback_is_fresh_read_only_and_redacted() {
        let (root, agent, config, revision) = effective_config_fixture().await;
        let provisioner = FilesystemProvisioner;
        provisioner
            .verify_effective_configuration(&agent, &config, &revision)
            .await
            .unwrap();
        assert!(
            !Path::new(&agent.paths.config)
                .join("skills/disabled")
                .exists()
        );
        for relative in [
            "config.yaml",
            "SOUL.md",
            ".env",
            "skills/enabled/SKILL.md",
            ".fleet-config-revision.json",
        ] {
            let path = Path::new(&agent.paths.config).join(relative);
            let original = tokio::fs::read(&path).await.unwrap();
            // Same-size mutation catches implementations that trust only metadata/marker hashes.
            let mut changed = original.clone();
            changed[0] ^= 1;
            tokio::fs::write(&path, &changed).await.unwrap();
            let error = provisioner
                .verify_effective_configuration(&agent, &config, &revision)
                .await
                .unwrap_err();
            assert_eq!(
                error.to_string(),
                AppError::conflict("effective configuration readback failed").to_string()
            );
            assert!(!error.to_string().contains("test-secret-never-return"));
            assert_eq!(tokio::fs::read(&path).await.unwrap(), changed);
            tokio::fs::write(&path, original).await.unwrap();
            provisioner
                .verify_effective_configuration(&agent, &config, &revision)
                .await
                .unwrap();
        }
        let path = Path::new(&agent.paths.config).join("SOUL.md");
        tokio::fs::remove_file(&path).await.unwrap();
        assert!(
            provisioner
                .verify_effective_configuration(&agent, &config, &revision)
                .await
                .is_err()
        );
        assert!(!path.exists());
        install_effective_fixture(&agent, &config, &revision).await;
        let disabled = Path::new(&agent.paths.config).join("skills/disabled");
        tokio::fs::create_dir_all(&disabled).await.unwrap();
        tokio::fs::write(disabled.join("SKILL.md"), "")
            .await
            .unwrap();
        assert!(
            provisioner
                .verify_effective_configuration(&agent, &config, &revision)
                .await
                .is_err()
        );
        tokio::fs::remove_file(disabled.join("SKILL.md"))
            .await
            .unwrap();
        provisioner
            .verify_effective_configuration(&agent, &config, &revision)
            .await
            .unwrap();
        // Hermes-owned inventory is separate from Fleet-managed flat skill paths.
        let bundled = Path::new(&agent.paths.config).join("skills/category/bundled");
        tokio::fs::create_dir_all(&bundled).await.unwrap();
        tokio::fs::write(bundled.join("SKILL.md"), "bundled fixture preserved")
            .await
            .unwrap();
        let manifest = Path::new(&agent.paths.config).join("skills/.bundled_manifest");
        tokio::fs::write(&manifest, "category/bundled:test-digest\n")
            .await
            .unwrap();
        provisioner
            .verify_effective_configuration(&agent, &config, &revision)
            .await
            .unwrap();
        assert_eq!(
            tokio::fs::read_to_string(bundled.join("SKILL.md"))
                .await
                .unwrap(),
            "bundled fixture preserved"
        );
        assert_eq!(
            tokio::fs::read_to_string(manifest).await.unwrap(),
            "category/bundled:test-digest\n"
        );
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn base_package_effective_readback_uses_git_pin_and_closed_home_inventory() {
        let Ok(checkout) = std::env::var("FLEET_TEST_BASE_PACKAGE_CHECKOUT") else {
            return;
        };
        let (root, mut agent, mut config, mut revision) = effective_config_fixture().await;
        config.fleet.base_package_checkout = checkout;
        agent.sdlc_role = Some(domain::SdlcRole::Developer);
        agent.namespace_id = Some("123".into());
        agent.workflow_id = Some("456".into());
        let package = base_package::VerifiedRolePackage::read(
            Path::new(&config.fleet.base_package_checkout),
            domain::SdlcRole::Developer,
        )
        .await
        .unwrap();
        tokio::fs::remove_dir_all(Path::new(&agent.paths.config).join("skills"))
            .await
            .unwrap();
        revision.snapshot.skills.clear();
        revision.snapshot = package
            .prepare_snapshot(&agent, &base_package::binding_fixture(), revision.snapshot)
            .unwrap();
        install_effective_fixture(&agent, &config, &revision).await;
        FilesystemProvisioner
            .verify_effective_configuration(&agent, &config, &revision)
            .await
            .unwrap();

        let extra = Path::new(&agent.paths.config).join("skills/category/native/SKILL.md");
        tokio::fs::create_dir_all(extra.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&extra, "native skill not in Base allowlist")
            .await
            .unwrap();
        assert!(
            FilesystemProvisioner
                .verify_effective_configuration(&agent, &config, &revision)
                .await
                .is_err()
        );
        assert!(extra.exists());
        tokio::fs::remove_file(extra).await.unwrap();

        // A matching disk snapshot and client-editable proof still cannot replace Git provenance.
        let mut forged = revision.clone();
        forged.snapshot.config.config_json["fleet_sdlc_package"]["manifestSha256"] =
            json!("a".repeat(64));
        install_effective_fixture(&agent, &config, &forged).await;
        assert!(
            FilesystemProvisioner
                .verify_effective_configuration(&agent, &config, &forged)
                .await
                .is_err()
        );
        install_effective_fixture(&agent, &config, &revision).await;
        config.fleet.base_package_checkout.clear();
        assert!(
            FilesystemProvisioner
                .verify_effective_configuration(&agent, &config, &revision)
                .await
                .is_err()
        );
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn base_package_effective_readback_all_roles_rejects_unattested_support() {
        let Ok(checkout) = std::env::var("FLEET_TEST_BASE_PACKAGE_CHECKOUT") else {
            return;
        };
        for role in [
            domain::SdlcRole::ProjectManager,
            domain::SdlcRole::Analyst,
            domain::SdlcRole::Architect,
            domain::SdlcRole::Developer,
            domain::SdlcRole::Reviewer,
            domain::SdlcRole::Tester,
            domain::SdlcRole::DevOps,
        ] {
            let (root, mut agent, mut config, mut revision) = effective_config_fixture().await;
            config.fleet.base_package_checkout = checkout.clone();
            agent.sdlc_role = Some(role);
            agent.namespace_id = Some("123".into());
            agent.workflow_id = Some("456".into());
            let package = base_package::VerifiedRolePackage::read(Path::new(&checkout), role)
                .await
                .unwrap();
            let mut binding = base_package::binding_fixture();
            binding.role_key = package.proof().role.clone();
            binding.namespace_name = package.proof().namespace.clone();
            binding.profile = package.proof().profile.clone();
            binding.workflow_key = format!("hermes-sdlc:{}", binding.role_key);
            let skills = Path::new(&agent.paths.config).join("skills");
            tokio::fs::remove_dir_all(&skills).await.unwrap();
            revision.snapshot.skills.clear();
            revision.snapshot = package
                .prepare_snapshot(&agent, &binding, revision.snapshot)
                .unwrap();
            install_effective_fixture(&agent, &config, &revision).await;
            FilesystemProvisioner
                .verify_effective_configuration(&agent, &config, &revision)
                .await
                .unwrap();
            let name = package.proof().skill_sha256.keys().next().unwrap();
            for relative in [
                "references/guide.md",
                "scripts/helper.py",
                "assets/fixture.bin",
                "templates/config.yaml",
            ] {
                let extra = skills.join(name).join(relative);
                tokio::fs::create_dir_all(extra.parent().unwrap())
                    .await
                    .unwrap();
                tokio::fs::write(&extra, "unattested support fixture")
                    .await
                    .unwrap();
                assert!(
                    FilesystemProvisioner
                        .verify_effective_configuration(&agent, &config, &revision)
                        .await
                        .is_err()
                );
                assert_eq!(
                    tokio::fs::read_to_string(&extra).await.unwrap(),
                    "unattested support fixture"
                );
                tokio::fs::remove_file(extra).await.unwrap();
            }
            FilesystemProvisioner
                .verify_effective_configuration(&agent, &config, &revision)
                .await
                .unwrap();
            tokio::fs::remove_dir_all(root).await.unwrap();
        }
    }

    #[tokio::test]
    async fn effective_configuration_requires_correct_snapshot_marker_and_workspace() {
        let (root, mut agent, config, revision) = effective_config_fixture().await;
        for mode in 0..7 {
            let mut changed = revision.clone();
            match mode {
                0 => changed.agent_id = Uuid::new_v4(),
                1 => changed.revision += 1,
                2 => changed.state = "validated".into(),
                3 => changed.is_effective = false,
                4 => changed.draining = true,
                5 => changed.validation_errors.push("unvalidated".into()),
                _ => changed.snapshot.config.soul_md = "different snapshot".into(),
            }
            assert!(
                FilesystemProvisioner
                    .verify_effective_configuration(&agent, &config, &changed)
                    .await
                    .is_err()
            );
        }
        write_marker(&root.join("agent1"), Uuid::new_v4(), &agent.name).await;
        assert!(
            FilesystemProvisioner
                .verify_effective_configuration(&agent, &config, &revision)
                .await
                .is_err()
        );
        write_marker(&root.join("agent1"), agent.id, &agent.name).await;
        let marker = root.join("agent1/.fleet-agent.json");
        tokio::fs::write(&marker, "x".repeat(16_385)).await.unwrap();
        assert!(
            FilesystemProvisioner
                .verify_effective_configuration(&agent, &config, &revision)
                .await
                .is_err()
        );
        tokio::fs::remove_file(&marker).await.unwrap();
        tokio::fs::create_dir(&marker).await.unwrap();
        assert!(
            FilesystemProvisioner
                .verify_effective_configuration(&agent, &config, &revision)
                .await
                .is_err()
        );
        tokio::fs::remove_dir(&marker).await.unwrap();
        write_marker(&root.join("agent1"), agent.id, &agent.name).await;
        agent.paths.workspace = root.join("agent2/workspace").to_string_lossy().into_owned();
        assert!(
            FilesystemProvisioner
                .verify_effective_configuration(&agent, &config, &revision)
                .await
                .is_err()
        );
        agent.paths.workspace = root.join("agent1/workspace").to_string_lossy().into_owned();
        tokio::fs::remove_dir(&agent.paths.workspace).await.unwrap();
        assert!(
            FilesystemProvisioner
                .verify_effective_configuration(&agent, &config, &revision)
                .await
                .is_err()
        );
        assert!(!Path::new(&agent.paths.workspace).exists());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn effective_configuration_rejects_symlinked_files_and_directories() {
        let (root, agent, config, revision) = effective_config_fixture().await;
        let outside = root.join("outside");
        tokio::fs::create_dir_all(&outside).await.unwrap();
        let soul = Path::new(&agent.paths.config).join("SOUL.md");
        let target = outside.join("SOUL.md");
        tokio::fs::write(&target, &revision.snapshot.config.soul_md)
            .await
            .unwrap();
        tokio::fs::remove_file(&soul).await.unwrap();
        std::os::unix::fs::symlink(&target, &soul).unwrap();
        assert!(
            FilesystemProvisioner
                .verify_effective_configuration(&agent, &config, &revision)
                .await
                .is_err()
        );
        tokio::fs::remove_file(soul).await.unwrap();
        install_effective_fixture(&agent, &config, &revision).await;
        let workspace = Path::new(&agent.paths.workspace);
        tokio::fs::remove_dir(workspace).await.unwrap();
        std::os::unix::fs::symlink(&outside, workspace).unwrap();
        assert!(
            FilesystemProvisioner
                .verify_effective_configuration(&agent, &config, &revision)
                .await
                .is_err()
        );
        assert_eq!(
            tokio::fs::read_to_string(target).await.unwrap(),
            revision.snapshot.config.soul_md
        );
        tokio::fs::remove_dir_all(root).await.unwrap();
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
    fn pending_session_run_keeps_run_and_session_ids_distinct() {
        let session_id = Uuid::new_v4();
        let agent_id = Uuid::new_v4();
        let model = pending_session_run(session_id, agent_id, SessionRunRole::Primary, now());

        assert_ne!(model.id.unwrap(), session_id);
        assert_eq!(model.session_id.unwrap(), session_id);
        assert_eq!(model.agent_id.unwrap(), agent_id);
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

    #[test]
    fn streaming_redaction_withholds_split_credentials() {
        let secret = "private-provider-credential".to_string();
        assert_eq!(
            redact_stream_text("Output: private-prov", std::slice::from_ref(&secret)),
            "Output: "
        );
        assert_eq!(
            redact_stream_text("Output: private-provider-credential done", &[secret]),
            "Output: redacted done"
        );
        assert_eq!(
            redact_text("fc_docs and fc_abcdef123456abcdef123456abcdef123456"),
            "fc_docs and redacted"
        );
    }

    #[test]
    fn settings_snapshots_follow_effective_startup_config() {
        let mut config = AppConfig::default();
        config.server.port = 24001;
        config.fleet.agents_root = "/srv/fleet/agents".to_string();
        config.fleet.hermes_command = "hermes-cli".to_string();
        config.fleet.agent_port_base = 31000;
        config.fleet.project_workflow_url = Some("http://workflow:8000".to_string());
        config.auth.jwt_issuer = "central-auth".to_string();

        assert_eq!(
            runtime_settings_from_config(&config).agents_root,
            "/srv/fleet/agents"
        );
        assert_eq!(
            runtime_settings_from_config(&config).hermes_command,
            "hermes-cli"
        );
        assert_eq!(port_settings_from_config(&config).backend_port, 24001);
        assert_eq!(port_settings_from_config(&config).agent_port_base, 31000);
        assert_eq!(
            integration_settings_from_config(&config).project_workflow_status,
            "configured"
        );
        assert_eq!(
            integration_settings_from_config(&config).project_workflow_url,
            Some("http://workflow:8000".to_string())
        );
        assert_eq!(
            auth_settings_from_config(&config).jwt_issuer,
            "central-auth"
        );

        config.fleet.project_workflow_url = Some("   ".to_string());
        assert_eq!(
            integration_settings_from_config(&config),
            IntegrationSettings {
                project_workflow_url: None,
                project_workflow_status: "not_configured".to_string(),
                github_remote: None,
            }
        );
    }

    #[tokio::test]
    async fn storage_report_marks_stale_archived_agent() {
        let root = temp_purge_root();
        let agent_id = Uuid::new_v4();
        let agent_root = root.join("agent1");
        let mut config = test_config(&root);
        config.fleet.retention.stale_archived_days = 30;
        // Archived 40 days ago: stale.
        let mut agent = test_agent(&root, agent_id, AgentStatus::Archived);
        agent.updated_at = (chrono::Utc::now() - chrono::Duration::days(40))
            .to_rfc3339()
            .to_string();
        write_marker(&agent_root, agent_id, "agent1").await;

        let provisioner = FilesystemProvisioner;
        let report = provisioner
            .storage_report(&agent, &config)
            .await
            .expect("storage report");
        assert!(report.retention.archived);
        assert!(report.retention.purge_eligible);
        assert!(report.retention.stale);
        assert_eq!(report.retention.archived_days, Some(40));
        assert!(
            report
                .retention
                .retention_hint
                .contains("stale (over 30 days)")
        );

        // Archived 5 days ago: not stale yet.
        let mut fresh = test_agent(&root, agent_id, AgentStatus::Archived);
        fresh.updated_at = (chrono::Utc::now() - chrono::Duration::days(5))
            .to_rfc3339()
            .to_string();
        let report_fresh = provisioner
            .storage_report(&fresh, &config)
            .await
            .expect("storage report fresh");
        assert!(!report_fresh.retention.stale);
        assert_eq!(report_fresh.retention.archived_days, Some(5));
        assert!(
            report_fresh
                .retention
                .retention_hint
                .contains("purged explicitly")
        );

        let _ = tokio::fs::remove_dir_all(&root).await;
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
    async fn storage_report_summarizes_area_sizes_and_marker_state() {
        let root = temp_purge_root();
        let agent_id = Uuid::new_v4();
        let agent_root = root.join("agent1");
        let config = test_config(&root);
        let agent = test_agent(&root, agent_id, AgentStatus::Archived);
        write_marker(&agent_root, agent_id, "agent1").await;
        tokio::fs::create_dir_all(agent_root.join("runtime"))
            .await
            .expect("runtime dir");
        tokio::fs::create_dir_all(agent_root.join("workspace").join("nested"))
            .await
            .expect("workspace nested dir");
        tokio::fs::create_dir_all(agent_root.join("logs"))
            .await
            .expect("logs dir");
        tokio::fs::write(agent_root.join("runtime").join("bin.txt"), "abc")
            .await
            .expect("runtime file");
        tokio::fs::write(agent_root.join("workspace").join("task.md"), "hello")
            .await
            .expect("workspace file");
        tokio::fs::write(
            agent_root.join("workspace").join("nested").join("notes.md"),
            "notes",
        )
        .await
        .expect("nested file");

        let report = FilesystemProvisioner
            .storage_report(&agent, &config)
            .await
            .expect("storage report");

        assert!(report.root_exists);
        assert!(report.marker_present);
        assert!(report.marker_verified);
        assert_eq!(report.total_bytes, 13);
        assert_eq!(report.total_files, 3);
        assert!(report.retention.archived);
        assert!(report.retention.purge_eligible);
        let workspace = report
            .areas
            .iter()
            .find(|area| area.name == "workspace")
            .expect("workspace area");
        assert_eq!(workspace.bytes, 10);
        assert_eq!(workspace.files, 2);
        assert_eq!(workspace.directories, 1);
        assert!(workspace.last_modified_at.is_some());
        let _ = tokio::fs::remove_dir_all(&root).await;
    }

    #[tokio::test]
    async fn storage_report_marks_foreign_marker_not_purge_eligible() {
        let root = temp_purge_root();
        let agent_id = Uuid::new_v4();
        let agent_root = root.join("agent1");
        let config = test_config(&root);
        let agent = test_agent(&root, agent_id, AgentStatus::Archived);
        write_marker(&agent_root, Uuid::new_v4(), "agent1").await;

        let report = FilesystemProvisioner
            .storage_report(&agent, &config)
            .await
            .expect("storage report");

        assert!(report.marker_present);
        assert!(!report.marker_verified);
        assert!(!report.retention.purge_eligible);
        assert!(
            report
                .retention
                .retention_hint
                .contains("marker must match")
        );
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

    #[test]
    fn bulk_validation_rejects_empty_title_and_agents() {
        let req = BulkDeploymentRequest {
            job_kind: DeploymentJobKind::RuntimeUpdate,
            agent_ids: vec![],
            runtime_kind: None,
            title: "  ".to_string(),
            detail: None,
            rollback: false,
        };
        assert!(validate_bulk_deployment_request(&req).is_err());

        let req = BulkDeploymentRequest {
            agent_ids: vec![Uuid::new_v4()],
            title: "update".to_string(),
            ..req
        };
        assert!(validate_bulk_deployment_request(&req).is_ok());
    }

    #[test]
    fn workflow_config_rebind_preserves_unrelated_runtime_settings() {
        let config = serde_json::json!({
            "agent": "agent1",
            "terminal": { "cwd": "/work" },
            "namespace_id": "dev",
            "workflow_id": "workflow-dev"
        });

        assert_eq!(
            workflow_config_with_binding(config, "1", "1"),
            serde_json::json!({
                "agent": "agent1",
                "terminal": { "cwd": "/work" },
                "namespace_id": "1",
                "workflow_id": "1"
            })
        );
    }

    #[test]
    fn workflow_binding_status_distinguishes_connected_stale_and_unbound() {
        let namespaces = vec![("1".to_string(), "Основной".to_string())];
        let workflows = vec![("1".to_string(), "sdlc-business-tech-v1".to_string())];

        assert_eq!(
            workflow_binding_status(
                Some("1"),
                Some("Основной"),
                Some("1"),
                Some("sdlc-business-tech-v1"),
                &namespaces,
                &workflows,
            ),
            "connected"
        );
        assert_eq!(
            workflow_binding_status(
                Some("dev"),
                Some("Development"),
                Some("workflow-dev"),
                Some("Developer Workflow"),
                &namespaces,
                &workflows,
            ),
            "stale"
        );
        assert_eq!(
            workflow_binding_status(
                Some(""),
                Some(""),
                Some(""),
                Some(""),
                &namespaces,
                &workflows,
            ),
            "unbound"
        );
    }

    #[test]
    fn bulk_validation_rejects_oversized_batches() {
        let req = BulkDeploymentRequest {
            job_kind: DeploymentJobKind::RuntimeUpdate,
            agent_ids: (0..101).map(|_| Uuid::new_v4()).collect(),
            runtime_kind: None,
            title: "update".to_string(),
            detail: None,
            rollback: false,
        };
        assert!(validate_bulk_deployment_request(&req).is_err());
    }

    #[test]
    fn bulk_validation_rejects_rollback_for_provision() {
        let req = BulkDeploymentRequest {
            job_kind: DeploymentJobKind::Provision,
            agent_ids: vec![Uuid::new_v4()],
            runtime_kind: None,
            title: "provision".to_string(),
            detail: None,
            rollback: true,
        };
        assert!(validate_bulk_deployment_request(&req).is_err());

        let req = BulkDeploymentRequest {
            job_kind: DeploymentJobKind::RuntimeUpdate,
            agent_ids: vec![Uuid::new_v4()],
            runtime_kind: None,
            title: "update".to_string(),
            detail: None,
            rollback: true,
        };
        assert!(validate_bulk_deployment_request(&req).unwrap());
    }
}
