use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fmt, str::FromStr};
use utoipa::ToSchema;
use uuid::Uuid;

pub type Timestamp = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    Hermes,
    JavaAgent,
}

impl AgentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hermes => "hermes",
            Self::JavaAgent => "java_agent",
        }
    }
}

impl fmt::Display for AgentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for AgentKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "hermes" => Ok(Self::Hermes),
            "java_agent" => Ok(Self::JavaAgent),
            _ => Err(format!("unknown agent kind: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    Developer,
    Tester,
    Custom,
}

impl AgentRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Developer => "developer",
            Self::Tester => "tester",
            Self::Custom => "custom",
        }
    }
}

impl fmt::Display for AgentRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for AgentRole {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "developer" => Ok(Self::Developer),
            "tester" => Ok(Self::Tester),
            "custom" => Ok(Self::Custom),
            _ => Err(format!("unknown agent role: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Provisioning,
    Ready,
    Starting,
    Running,
    Stopped,
    Failed,
    Archived,
}

impl AgentStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Provisioning => "provisioning",
            Self::Ready => "ready",
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
            Self::Archived => "archived",
        }
    }
}

impl fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for AgentStatus {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "provisioning" => Ok(Self::Provisioning),
            "ready" => Ok(Self::Ready),
            "starting" => Ok(Self::Starting),
            "running" => Ok(Self::Running),
            "stopped" => Ok(Self::Stopped),
            "failed" => Ok(Self::Failed),
            "archived" => Ok(Self::Archived),
            _ => Err(format!("unknown agent status: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DesiredState {
    Running,
    Stopped,
}

impl DesiredState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Stopped => "stopped",
        }
    }
}

impl fmt::Display for DesiredState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for DesiredState {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "running" => Ok(Self::Running),
            "stopped" => Ok(Self::Stopped),
            _ => Err(format!("unknown desired state: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SkillState {
    Enabled,
    Disabled,
    Missing,
    Dirty,
}

impl SkillState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
            Self::Missing => "missing",
            Self::Dirty => "dirty",
        }
    }
}

impl fmt::Display for SkillState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SkillState {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "enabled" => Ok(Self::Enabled),
            "disabled" => Ok(Self::Disabled),
            "missing" => Ok(Self::Missing),
            "dirty" => Ok(Self::Dirty),
            _ => Err(format!("unknown skill state: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Draft,
    Active,
    HandoffRequested,
    Blocked,
    Done,
    Archived,
}

impl SessionState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::HandoffRequested => "handoff_requested",
            Self::Blocked => "blocked",
            Self::Done => "done",
            Self::Archived => "archived",
        }
    }
}

impl fmt::Display for SessionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SessionState {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "draft" => Ok(Self::Draft),
            "active" => Ok(Self::Active),
            "handoff_requested" => Ok(Self::HandoffRequested),
            "blocked" => Ok(Self::Blocked),
            "done" => Ok(Self::Done),
            "archived" => Ok(Self::Archived),
            _ => Err(format!("unknown session state: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentPaths {
    pub runtime: String,
    pub config: String,
    pub workspace: String,
    pub logs: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentRuntime {
    pub desired_state: DesiredState,
    pub pid: Option<i32>,
    pub health_status: Option<String>,
    pub health_detail: Option<String>,
    pub command_preview: String,
    pub env_preview: Value,
    pub started_at: Option<Timestamp>,
    pub stopped_at: Option<Timestamp>,
    pub last_health_at: Option<Timestamp>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkflowBinding {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub namespace_id: Option<String>,
    pub namespace_name: Option<String>,
    pub workflow_id: Option<String>,
    pub workflow_name: Option<String>,
    pub binding_status: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Agent {
    pub id: Uuid,
    pub ordinal: i32,
    pub name: String,
    pub kind: AgentKind,
    pub role: AgentRole,
    pub status: AgentStatus,
    pub display_name: String,
    pub description: Option<String>,
    pub namespace_id: Option<String>,
    pub workflow_id: Option<String>,
    pub runtime_version: Option<String>,
    pub dashboard_port: Option<i32>,
    pub api_port: Option<i32>,
    pub paths: AgentPaths,
    pub runtime: AgentRuntime,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentConfig {
    pub agent_id: Uuid,
    pub config_json: Value,
    pub soul_md: String,
    pub env_json: Value,
    pub updated_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentSkill {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub name: String,
    pub title: String,
    pub state: SkillState,
    pub source: String,
    pub content: Option<String>,
    pub updated_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentSession {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub agent_name: String,
    pub title: String,
    pub task_key: Option<String>,
    pub state: SessionState,
    pub namespace_id: Option<String>,
    pub external_session_id: Option<String>,
    pub last_message_preview: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RuntimeTemplate {
    pub kind: AgentKind,
    pub display_name: String,
    pub implemented: bool,
    pub enabled: bool,
    pub description: String,
    pub capabilities: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentEvent {
    pub id: Uuid,
    pub agent_id: Option<Uuid>,
    pub event_type: String,
    pub message: String,
    pub payload: Value,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentLogEntry {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub stream: String,
    pub message: String,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FleetDashboard {
    pub total_agents: usize,
    pub running_agents: usize,
    pub failed_agents: usize,
    pub active_sessions: usize,
    pub agents: Vec<Agent>,
    pub recent_events: Vec<AgentEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub email: String,
    pub username: String,
    pub display_name: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthResponse {
    pub access_token: String,
    pub user_id: Uuid,
    pub email: String,
    pub username: String,
    pub display_name: String,
    pub is_system_admin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub display_name: String,
    pub is_system_admin: bool,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserListResponse {
    pub users: Vec<UserResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateAgentRequest {
    pub kind: AgentKind,
    pub role: AgentRole,
    pub display_name: String,
    pub description: Option<String>,
    pub namespace_id: Option<String>,
    pub namespace_name: Option<String>,
    pub workflow_id: Option<String>,
    pub workflow_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateAgentRequest {
    pub role: Option<AgentRole>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub namespace_id: Option<String>,
    pub workflow_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateAgentConfigRequest {
    pub config_json: Value,
    pub soul_md: String,
    pub env_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateSkillRequest {
    pub state: SkillState,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateSessionRequest {
    pub agent_id: Uuid,
    pub title: String,
    pub task_key: Option<String>,
    pub namespace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HandoffSessionRequest {
    pub target_agent_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RuntimeOperationResponse {
    pub agent_id: Uuid,
    pub status: AgentStatus,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListResponse<T> {
    pub items: Vec<T>,
}
