pub mod entities;
pub mod runtime;

use app::{AgentProvisioner, FleetRepository, RuntimeStatePatch, SessionListFilter};
use async_trait::async_trait;
use domain::{
    Agent, AgentConfig, AgentEvent, AgentKind, AgentLogEntry, AgentPaths, AgentRole, AgentRuntime,
    AgentSession, AgentStatus, CreateAgentRequest, CreateSessionRequest, DesiredState,
    HandoffSessionRequest, RuntimeTemplate, SessionState, SkillState, UpdateAgentConfigRequest,
    UpdateAgentRequest, UpdateSkillRequest, UserResponse, WorkflowBinding,
};
use entities::{
    agent, agent_config, agent_event, agent_log, agent_runtime, agent_session, agent_skill,
    runtime_template, user, workflow_binding,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectOptions, Database, DatabaseConnection,
    EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
    TransactionTrait,
};
use sea_orm_migration::MigratorTrait;
use serde_json::{Value, json};
use shared::{AppConfig, AppError, DatabaseConfig};
use std::{
    path::{Component, Path, PathBuf},
    time::Duration,
};
use uuid::Uuid;

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

fn parse_role(value: &str) -> AgentRole {
    value.parse().unwrap_or(AgentRole::Custom)
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

fn redacted_env(kind: AgentKind, agent: &Agent) -> Value {
    match kind {
        AgentKind::Hermes => json!({
            "HERMES_HOME": &agent.paths.config,
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

fn command_preview(kind: AgentKind, config: &AppConfig, agent: &Agent) -> String {
    match kind {
        AgentKind::Hermes => format!(
            "{} dashboard --host 127.0.0.1 --port {}",
            config.fleet.hermes_command,
            agent.dashboard_port.unwrap_or_default()
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

fn user_response(model: user::Model) -> UserResponse {
    UserResponse {
        id: model.id,
        email: model.email,
        username: model.username,
        display_name: model.display_name,
        is_system_admin: model.is_system_admin,
        is_active: model.is_active,
    }
}

fn user_record(model: user::Model) -> app::auth::UserRecord {
    app::auth::UserRecord {
        id: model.id,
        email: model.email,
        username: model.username,
        display_name: model.display_name,
        password_hash: model.password_hash,
        refresh_token_hash: model.refresh_token_hash,
        is_system_admin: model.is_system_admin,
        is_active: model.is_active,
    }
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
                    "sessions": true,
                    "health": "process"
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

    async fn get_agent(&self, id: Uuid) -> Result<Agent, AppError> {
        load_agent(&self.db, id).await
    }

    async fn create_agent(
        &self,
        req: CreateAgentRequest,
        config: &AppConfig,
    ) -> Result<Agent, AppError> {
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
        let count = agent::Entity::find()
            .count(&txn)
            .await
            .map_err(AppError::database)?;
        let ordinal = i32::try_from(count + 1).map_err(AppError::internal)?;
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

        txn.commit().await.map_err(AppError::database)?;
        self.get_agent(id).await
    }

    async fn update_agent(&self, id: Uuid, req: UpdateAgentRequest) -> Result<Agent, AppError> {
        let mut model = agent::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("agent", id))?
            .into_active_model();
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
        if !filter.user_ids.is_empty() {
            query = query.filter(agent_session::Column::UserId.is_in(filter.user_ids));
        }
        let sessions = query.all(&self.db).await.map_err(AppError::database)?;
        let mut result = Vec::with_capacity(sessions.len());
        for row in sessions {
            let agent = agent::Entity::find_by_id(row.agent_id)
                .one(&self.db)
                .await
                .map_err(AppError::database)?
                .ok_or_else(|| AppError::not_found("agent", row.agent_id))?;
            let user = user::Entity::find_by_id(row.user_id)
                .one(&self.db)
                .await
                .map_err(AppError::database)?
                .ok_or_else(|| AppError::not_found("user", row.user_id))?;
            result.push(session_from_model(row, agent.name, user));
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
        let user = user::Entity::find_by_id(row.user_id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("user", row.user_id))?;
        Ok(session_from_model(row, agent.name, user))
    }

    async fn create_session(
        &self,
        req: CreateSessionRequest,
        user_id: Uuid,
    ) -> Result<AgentSession, AppError> {
        let agent = agent::Entity::find_by_id(req.agent_id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("agent", req.agent_id))?;
        user::Entity::find_by_id(user_id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("user", user_id))?;
        let id = Uuid::new_v4();
        let ts = now();
        agent_session::Entity::insert(agent_session::ActiveModel {
            id: Set(id),
            agent_id: Set(req.agent_id),
            user_id: Set(user_id),
            title: Set(req.title),
            task_key: Set(req.task_key),
            state: Set(SessionState::Draft.as_str().to_string()),
            namespace_id: Set(req.namespace_id.or(agent.namespace_id.clone())),
            external_session_id: Set(None),
            last_message_preview: Set(Some("Session created in Fleet Control".to_string())),
            created_at: Set(ts),
            updated_at: Set(ts),
        })
        .exec(&self.db)
        .await
        .map_err(AppError::database)?;
        self.get_session(id).await
    }

    async fn handoff_session(
        &self,
        id: Uuid,
        req: HandoffSessionRequest,
    ) -> Result<AgentSession, AppError> {
        let target = agent::Entity::find_by_id(req.target_agent_id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("target_agent", req.target_agent_id))?;
        let mut model = agent_session::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("agent_session", id))?
            .into_active_model();
        model.agent_id = Set(req.target_agent_id);
        model.state = Set(SessionState::HandoffRequested.as_str().to_string());
        model.namespace_id = Set(target.namespace_id);
        model.last_message_preview = Set(Some(format!("Handoff requested to {}", target.name)));
        model.updated_at = Set(now());
        model.update(&self.db).await.map_err(AppError::database)?;
        self.get_session(id).await
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
}

fn session_from_model(
    row: agent_session::Model,
    agent_name: String,
    user: user::Model,
) -> AgentSession {
    AgentSession {
        id: row.id,
        agent_id: row.agent_id,
        agent_name,
        user_id: row.user_id,
        user_email: user.email,
        user_username: user.username,
        user_display_name: user.display_name,
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
        let root = PathBuf::from(&config.fleet.agents_root);
        let agent_root = safe_agent_root(&root, &agent.name)?;
        tokio::fs::create_dir_all(&agent_root)
            .await
            .map_err(AppError::internal)?;
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
            agent_root.join(".fleet-agent.json"),
            serde_json::to_string_pretty(&json!({
                "id": agent.id,
                "ordinal": agent.ordinal,
                "name": agent.name,
                "kind": agent.kind.as_str()
            }))
            .map_err(AppError::internal)?,
        )
        .await?;

        match agent.kind {
            AgentKind::Hermes => provision_hermes(agent, config).await,
            AgentKind::JavaAgent => provision_java_agent_contract(agent, config).await,
        }
    }
}

async fn provision_hermes(agent: &Agent, config: &AppConfig) -> Result<(), AppError> {
    let config_path = PathBuf::from(&agent.paths.config);
    let runtime_path = PathBuf::from(&agent.paths.runtime);
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
    write_if_missing(
        config_path.join(".env"),
        "# Managed by Fleet Control. Secrets are redacted in API responses.\n".to_string(),
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

async fn provision_java_agent_contract(agent: &Agent, config: &AppConfig) -> Result<(), AppError> {
    let config_path = PathBuf::from(&agent.paths.config);
    let runtime_path = PathBuf::from(&agent.paths.runtime);
    write_if_missing(
        config_path.join("application-fleet.yml"),
        format!(
            "agent:\n  name: {}\n  workspace: {}\nserver:\n  port: {}\n",
            agent.name,
            agent.paths.workspace.replace('\\', "/"),
            agent.api_port.unwrap_or_default()
        ),
    )
    .await?;
    write_if_missing(
        runtime_path.join("source.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "java_agent",
            "source": config.fleet.java_agent_source,
            "materialization": "phase_2_contract"
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

fn safe_agent_root(root: &Path, name: &str) -> Result<PathBuf, AppError> {
    if !name.starts_with("agent") || !name.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return Err(AppError::validation("invalid agent directory name"));
    }
    let path = root.join(name);
    ensure_inside(root, &path)?;
    Ok(path)
}

fn ensure_inside(root: &Path, path: &Path) -> Result<(), AppError> {
    for component in path.components() {
        if matches!(component, Component::ParentDir) {
            return Err(AppError::validation("path traversal is not allowed"));
        }
    }
    if path.is_absolute() {
        return Ok(());
    }
    if path.starts_with(root) || root.is_relative() {
        return Ok(());
    }
    Err(AppError::validation("path must stay inside agents root"))
}
