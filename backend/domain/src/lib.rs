use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fmt, str::FromStr};
use utoipa::ToSchema;
use uuid::Uuid;

pub type Timestamp = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SystemRole {
    Admin,
    Operator,
    User,
}

impl SystemRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Operator => "operator",
            Self::User => "user",
        }
    }

    pub fn from_legacy(is_system_admin: bool) -> Self {
        if is_system_admin {
            Self::Admin
        } else {
            Self::User
        }
    }

    pub fn is_admin(self) -> bool {
        self == Self::Admin
    }

    pub fn can_operate_fleet(self) -> bool {
        matches!(self, Self::Admin | Self::Operator)
    }

    pub fn can_read_all_sessions(self) -> bool {
        matches!(self, Self::Admin | Self::Operator)
    }

    pub fn permissions(self) -> Vec<String> {
        let mut permissions = vec![
            "sessions:read_own".to_string(),
            "sessions:write_own".to_string(),
            "agents:read_directory".to_string(),
        ];
        if self.can_operate_fleet() {
            permissions.extend([
                "agents:manage".to_string(),
                "leaders:manage".to_string(),
                "executors:manage".to_string(),
                "runtime:manage".to_string(),
                "config:manage".to_string(),
                "skills:manage".to_string(),
                "deployments:manage".to_string(),
                "logs:read".to_string(),
                "audit_log:read".to_string(),
                "settings:manage".to_string(),
                "sessions:read_all".to_string(),
            ]);
        }
        if self.is_admin() {
            permissions.extend(["users:manage".to_string(), "rbac:manage".to_string()]);
        }
        permissions
    }
}

impl fmt::Display for SystemRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SystemRole {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "admin" => Ok(Self::Admin),
            "operator" => Ok(Self::Operator),
            "user" => Ok(Self::User),
            _ => Err(format!("unknown system role: {value}")),
        }
    }
}

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
pub enum AgentProductRole {
    Leader,
    Executor,
}

impl AgentProductRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Leader => "leader",
            Self::Executor => "executor",
        }
    }
}

impl fmt::Display for AgentProductRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for AgentProductRole {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "leader" => Ok(Self::Leader),
            "executor" => Ok(Self::Executor),
            _ => Err(format!("unknown agent product role: {value}")),
        }
    }
}

pub fn default_product_role() -> AgentProductRole {
    AgentProductRole::Executor
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    Developer,
    Tester,
    ItLead,
    Custom,
}

impl AgentRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Developer => "developer",
            Self::Tester => "tester",
            Self::ItLead => "it_lead",
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
            "it_lead" => Ok(Self::ItLead),
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
    Degraded,
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
            Self::Degraded => "degraded",
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
            "degraded" => Ok(Self::Degraded),
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
pub enum SessionVisibility {
    Private,
    LeaderScoped,
}

impl SessionVisibility {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::LeaderScoped => "leader_scoped",
        }
    }
}

impl fmt::Display for SessionVisibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SessionVisibility {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "private" => Ok(Self::Private),
            "leader_scoped" => Ok(Self::LeaderScoped),
            _ => Err(format!("unknown session visibility: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionParticipantType {
    User,
    Agent,
}

impl SessionParticipantType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Agent => "agent",
        }
    }
}

impl fmt::Display for SessionParticipantType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SessionParticipantType {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "user" => Ok(Self::User),
            "agent" => Ok(Self::Agent),
            _ => Err(format!("unknown participant type: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionRole {
    Owner,
    Primary,
    Leader,
    Executor,
    Observer,
}

impl SessionRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Primary => "primary",
            Self::Leader => "leader",
            Self::Executor => "executor",
            Self::Observer => "observer",
        }
    }
}

impl fmt::Display for SessionRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SessionRole {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "owner" => Ok(Self::Owner),
            "primary" => Ok(Self::Primary),
            "leader" => Ok(Self::Leader),
            "executor" => Ok(Self::Executor),
            "observer" => Ok(Self::Observer),
            _ => Err(format!("unknown session role: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MessageAuthorType {
    User,
    Agent,
    System,
}

impl MessageAuthorType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Agent => "agent",
            Self::System => "system",
        }
    }
}

impl fmt::Display for MessageAuthorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for MessageAuthorType {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "user" => Ok(Self::User),
            "agent" => Ok(Self::Agent),
            "system" => Ok(Self::System),
            _ => Err(format!("unknown message author type: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    UserPrompt,
    AssistantMessage,
    ToolEvent,
    SystemEvent,
    Control,
}

impl MessageKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UserPrompt => "user_prompt",
            Self::AssistantMessage => "assistant_message",
            Self::ToolEvent => "tool_event",
            Self::SystemEvent => "system_event",
            Self::Control => "control",
        }
    }
}

impl fmt::Display for MessageKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for MessageKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "user_prompt" => Ok(Self::UserPrompt),
            "assistant_message" => Ok(Self::AssistantMessage),
            "tool_event" => Ok(Self::ToolEvent),
            "system_event" => Ok(Self::SystemEvent),
            "control" => Ok(Self::Control),
            _ => Err(format!("unknown message kind: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionRunRole {
    Primary,
    Leader,
    Executor,
}

impl SessionRunRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Leader => "leader",
            Self::Executor => "executor",
        }
    }
}

impl fmt::Display for SessionRunRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SessionRunRole {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "primary" => Ok(Self::Primary),
            "leader" => Ok(Self::Leader),
            "executor" => Ok(Self::Executor),
            _ => Err(format!("unknown session run role: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionRunState {
    Pending,
    Running,
    Waiting,
    Completed,
    Failed,
    Cancelled,
    Stopping,
}

impl SessionRunState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Waiting => "waiting",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Stopping => "stopping",
        }
    }
}

impl fmt::Display for SessionRunState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SessionRunState {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "waiting" => Ok(Self::Waiting),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            "stopping" => Ok(Self::Stopping),
            _ => Err(format!("unknown session run state: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MessageDeliveryState {
    Pending,
    Dispatched,
    Completed,
    Failed,
    Mirrored,
}

impl MessageDeliveryState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Dispatched => "dispatched",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Mirrored => "mirrored",
        }
    }
}

impl fmt::Display for MessageDeliveryState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for MessageDeliveryState {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pending" => Ok(Self::Pending),
            "dispatched" => Ok(Self::Dispatched),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "mirrored" => Ok(Self::Mirrored),
            _ => Err(format!("unknown message delivery state: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeApprovalState {
    Pending,
    Approved,
    Denied,
    Cancelled,
}

impl RuntimeApprovalState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Denied => "denied",
            Self::Cancelled => "cancelled",
        }
    }
}

impl fmt::Display for RuntimeApprovalState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for RuntimeApprovalState {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pending" => Ok(Self::Pending),
            "approved" => Ok(Self::Approved),
            "denied" => Ok(Self::Denied),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(format!("unknown runtime approval state: {value}")),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentJobKind {
    Provision,
    RuntimeUpdate,
}

impl DeploymentJobKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Provision => "provision",
            Self::RuntimeUpdate => "runtime_update",
        }
    }
}

impl fmt::Display for DeploymentJobKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for DeploymentJobKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "provision" => Ok(Self::Provision),
            "runtime_update" => Ok(Self::RuntimeUpdate),
            _ => Err(format!("unknown deployment job kind: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentJobState {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl DeploymentJobState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl fmt::Display for DeploymentJobState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for DeploymentJobState {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(format!("unknown deployment job state: {value}")),
        }
    }
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
pub struct AgentStorageArea {
    pub name: String,
    pub path: String,
    pub exists: bool,
    pub is_directory: bool,
    pub bytes: u64,
    pub files: u64,
    pub directories: u64,
    pub symlinks: u64,
    pub last_modified_at: Option<Timestamp>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentRetentionReport {
    pub archived: bool,
    pub archived_since: Option<Timestamp>,
    pub purge_eligible: bool,
    pub retention_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentStorageReport {
    pub agent_id: Uuid,
    pub agent_name: String,
    pub root_path: String,
    pub root_exists: bool,
    pub marker_present: bool,
    pub marker_verified: bool,
    pub total_bytes: u64,
    pub total_files: u64,
    pub total_directories: u64,
    pub total_symlinks: u64,
    pub areas: Vec<AgentStorageArea>,
    pub retention: AgentRetentionReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentRuntime {
    pub desired_state: DesiredState,
    pub pid: Option<i32>,
    pub health_status: Option<String>,
    pub health_detail: Option<String>,
    pub command_preview: String,
    pub env_preview: Value,
    pub last_capabilities_json: Value,
    pub startup_command_redacted: Option<String>,
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
    pub product_role: AgentProductRole,
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
    pub primary_agent_id: Uuid,
    pub agent_name: String,
    pub primary_agent_name: String,
    pub user_id: Uuid,
    pub user_email: String,
    pub user_username: String,
    pub user_display_name: String,
    pub leader_agent_id: Option<Uuid>,
    pub leader_agent_name: Option<String>,
    pub parent_session_id: Option<Uuid>,
    pub created_by_leader_agent_id: Option<Uuid>,
    pub visibility: SessionVisibility,
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
pub struct LeaderExecutor {
    pub leader_agent_id: Uuid,
    pub executor_agent_id: Uuid,
    pub executor_name: String,
    pub executor_display_name: String,
    pub executor_profile: AgentRole,
    pub namespace_id: Option<String>,
    pub workflow_id: Option<String>,
    pub created_by_user_id: Option<Uuid>,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionParticipant {
    pub id: Uuid,
    pub session_id: Uuid,
    pub participant_type: SessionParticipantType,
    pub user_id: Option<Uuid>,
    pub agent_id: Option<Uuid>,
    pub session_role: SessionRole,
    pub display_name: String,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionMessage {
    pub id: Uuid,
    pub session_id: Uuid,
    pub author_type: MessageAuthorType,
    pub author_user_id: Option<Uuid>,
    pub author_agent_id: Option<Uuid>,
    pub author_display_name: String,
    pub body: String,
    pub message_kind: MessageKind,
    pub runtime_message_id: Option<String>,
    pub delivery_state: MessageDeliveryState,
    pub delivery_error: Option<String>,
    pub replayed: bool,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionAgentRun {
    pub id: Uuid,
    pub session_id: Uuid,
    pub agent_id: Uuid,
    pub agent_name: String,
    pub runtime_session_id: Option<String>,
    pub runtime_run_id: Option<String>,
    pub run_role: SessionRunRole,
    pub state: SessionRunState,
    pub last_error: Option<String>,
    pub last_event_at: Option<Timestamp>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub model_options: Value,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RuntimeApprovalRequest {
    pub id: Uuid,
    pub session_id: Uuid,
    pub session_run_id: Uuid,
    pub agent_id: Uuid,
    pub runtime_run_id: String,
    pub runtime_approval_id: Option<String>,
    pub prompt: String,
    pub detail: Value,
    pub state: RuntimeApprovalState,
    pub resolved_by_user_id: Option<Uuid>,
    pub resolved_at: Option<Timestamp>,
    pub created_at: Timestamp,
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
pub struct AgentDirectoryItem {
    pub id: Uuid,
    pub ordinal: i32,
    pub name: String,
    pub kind: AgentKind,
    pub product_role: AgentProductRole,
    pub role: AgentRole,
    pub status: AgentStatus,
    pub display_name: String,
    pub description: Option<String>,
    pub namespace_id: Option<String>,
    pub workflow_id: Option<String>,
    pub runtime_version: Option<String>,
    pub dashboard_port: Option<i32>,
    pub api_port: Option<i32>,
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
pub struct AuditLogEntry {
    pub id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub payload: Value,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeploymentJob {
    pub id: Uuid,
    pub job_kind: DeploymentJobKind,
    pub state: DeploymentJobState,
    pub agent_id: Option<Uuid>,
    pub runtime_kind: Option<AgentKind>,
    pub requested_by_user_id: Option<Uuid>,
    pub title: String,
    pub detail: Value,
    pub last_error: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FleetDashboard {
    pub total_agents: usize,
    pub leader_agents: usize,
    pub executor_agents: usize,
    pub running_agents: usize,
    pub failed_agents: usize,
    pub active_sessions: usize,
    pub private_sessions: usize,
    pub leader_scoped_sessions: usize,
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
    pub system_role: SystemRole,
    pub is_system_admin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub display_name: String,
    pub system_role: SystemRole,
    pub is_system_admin: bool,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserListResponse {
    pub users: Vec<UserResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserPermissionsResponse {
    pub user_id: Uuid,
    pub role: SystemRole,
    pub is_system_admin: bool,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateUserRoleRequest {
    pub role: SystemRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateAgentRequest {
    pub kind: AgentKind,
    #[serde(default = "default_product_role")]
    pub product_role: AgentProductRole,
    pub role: AgentRole,
    pub display_name: String,
    pub description: Option<String>,
    pub namespace_id: Option<String>,
    pub namespace_name: Option<String>,
    pub workflow_id: Option<String>,
    pub workflow_name: Option<String>,
    #[serde(default)]
    pub executor_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateAgentRequest {
    pub product_role: Option<AgentProductRole>,
    pub role: Option<AgentRole>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub namespace_id: Option<String>,
    pub workflow_id: Option<String>,
    pub executor_ids: Option<Vec<Uuid>>,
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
    pub primary_agent_id: Option<Uuid>,
    pub agent_id: Option<Uuid>,
    pub title: String,
    pub task_key: Option<String>,
    pub leader_agent_id: Option<Uuid>,
    pub parent_session_id: Option<Uuid>,
    pub namespace_id: Option<String>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateSessionDelegationRequest {
    pub executor_agent_id: Uuid,
    pub title: String,
    pub task_key: Option<String>,
    pub initial_message: Option<String>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HandoffSessionRequest {
    pub target_agent_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AssignSessionLeaderRequest {
    pub leader_agent_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateSessionMessageRequest {
    pub body: String,
    pub author_agent_id: Option<Uuid>,
    pub message_kind: Option<MessageKind>,
    pub runtime_message_id: Option<String>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SteerSessionRunRequest {
    pub input: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ResolveRuntimeApprovalRequest {
    pub choice: String,
    #[serde(default)]
    pub resolve_all: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RuntimeRunControlResponse {
    pub session_id: Uuid,
    pub run_id: Uuid,
    pub runtime_run_id: Option<String>,
    pub accepted: bool,
    pub state: SessionRunState,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateLeaderExecutorsRequest {
    pub executor_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PurgeAgentFilesRequest {
    pub confirmation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PurgeAgentFilesResponse {
    pub agent_id: Uuid,
    pub agent_name: String,
    pub purged_path: String,
    pub files_deleted: bool,
    pub marker_verified: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RuntimeOperationResponse {
    pub agent_id: Uuid,
    pub status: AgentStatus,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateDeploymentJobRequest {
    pub job_kind: DeploymentJobKind,
    pub agent_id: Option<Uuid>,
    pub runtime_kind: Option<AgentKind>,
    pub title: String,
    pub detail: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RuntimeSettings {
    pub agents_root: String,
    pub hermes_source: String,
    pub hermes_command: String,
    pub java_agent_source: String,
    pub java_agent_command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PortSettings {
    pub backend_port: u16,
    pub frontend_port: u16,
    pub agent_port_base: u16,
    pub agent_port_stride: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IntegrationSettings {
    pub project_workflow_url: Option<String>,
    pub project_workflow_status: String,
    pub github_remote: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthSettings {
    #[serde(default = "default_auth_mode")]
    pub mode: String,
    #[serde(default = "default_jwt_issuer")]
    pub jwt_issuer: String,
    #[serde(default = "default_jwt_audience")]
    pub jwt_audience: String,
    pub access_token_ttl_minutes: u64,
    pub refresh_token_ttl_days: u64,
    pub refresh_cookie_name: String,
    pub refresh_cookie_secure: bool,
    pub refresh_cookie_same_site: String,
    pub refresh_cookie_domain: Option<String>,
    pub refresh_cookie_path: String,
}

fn default_auth_mode() -> String {
    "hmac".to_string()
}

fn default_jwt_issuer() -> String {
    "fleet-control".to_string()
}

fn default_jwt_audience() -> String {
    "sdlc".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListResponse<T> {
    pub items: Vec<T>,
}
