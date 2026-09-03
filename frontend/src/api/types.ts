export type AgentKind = 'hermes' | 'java_agent'
export type SystemRole = 'admin' | 'operator' | 'user'
export type AgentProductRole = 'leader' | 'executor'
export type AgentRole = 'developer' | 'tester' | 'it_lead' | 'custom'
export type AgentStatus =
  'provisioning' | 'ready' | 'starting' | 'running' | 'degraded' | 'stopped' | 'failed' | 'archived'
export type DesiredState = 'running' | 'stopped'
export type SkillState = 'enabled' | 'disabled' | 'missing' | 'dirty'
export type SessionState =
  'draft' | 'active' | 'handoff_requested' | 'blocked' | 'done' | 'archived'
export type SessionVisibility = 'private' | 'leader_scoped'
export type SessionParticipantType = 'user' | 'agent'
export type SessionParticipantRole = 'owner' | 'primary' | 'leader' | 'executor' | 'observer'
export type MessageAuthorType = 'user' | 'agent' | 'system'
export type MessageKind =
  'user_prompt' | 'assistant_message' | 'tool_event' | 'system_event' | 'control'
export type SessionRunRole = 'primary' | 'leader' | 'executor'
export type SessionRunState =
  'pending' | 'running' | 'waiting' | 'completed' | 'failed' | 'cancelled' | 'stopping'
export type MessageDeliveryState = 'pending' | 'dispatched' | 'completed' | 'failed' | 'mirrored'

export interface AgentPaths {
  runtime: string
  config: string
  workspace: string
  logs: string
}

export interface AgentRuntime {
  desired_state: DesiredState
  pid: number | null
  health_status: string | null
  health_detail: string | null
  command_preview: string
  env_preview: Record<string, unknown>
  last_capabilities_json: Record<string, unknown>
  startup_command_redacted: string | null
  started_at: string | null
  stopped_at: string | null
  last_health_at: string | null
}

export interface Agent {
  id: string
  ordinal: number
  name: string
  kind: AgentKind
  product_role: AgentProductRole
  role: AgentRole
  status: AgentStatus
  display_name: string
  description: string | null
  namespace_id: string | null
  workflow_id: string | null
  runtime_version: string | null
  dashboard_port: number | null
  api_port: number | null
  paths: AgentPaths
  runtime: AgentRuntime
  created_at: string
  updated_at: string
}

export interface AgentDirectoryItem {
  id: string
  ordinal: number
  name: string
  kind: AgentKind
  product_role: AgentProductRole
  role: AgentRole
  status: AgentStatus
  display_name: string
  description: string | null
  namespace_id: string | null
  workflow_id: string | null
  runtime_version: string | null
  dashboard_port: number | null
  api_port: number | null
}

export interface RuntimeTemplate {
  kind: AgentKind
  display_name: string
  implemented: boolean
  enabled: boolean
  description: string
  capabilities: Record<string, unknown>
}

export interface AgentConfig {
  agent_id: string
  config_json: Record<string, unknown>
  soul_md: string
  env_json: Record<string, unknown>
  updated_at: string
}

export interface AgentSkill {
  id: string
  agent_id: string
  name: string
  title: string
  state: SkillState
  source: string
  content: string | null
  updated_at: string
}

export interface AgentSession {
  id: string
  agent_id: string
  primary_agent_id: string
  agent_name: string
  primary_agent_name: string
  user_id: string
  user_email: string
  user_username: string
  user_display_name: string
  leader_agent_id: string | null
  leader_agent_name: string | null
  parent_session_id: string | null
  created_by_leader_agent_id: string | null
  visibility: SessionVisibility
  title: string
  task_key: string | null
  state: SessionState
  namespace_id: string | null
  external_session_id: string | null
  last_message_preview: string | null
  created_at: string
  updated_at: string
}

export interface LeaderExecutor {
  leader_agent_id: string
  executor_agent_id: string
  executor_name: string
  executor_display_name: string
  executor_profile: AgentRole
  namespace_id: string | null
  workflow_id: string | null
  created_by_user_id: string | null
  created_at: string
}

export interface SessionParticipant {
  id: string
  session_id: string
  participant_type: SessionParticipantType
  user_id: string | null
  agent_id: string | null
  session_role: SessionParticipantRole
  display_name: string
  created_at: string
}

export interface SessionMessage {
  id: string
  session_id: string
  author_type: MessageAuthorType
  author_user_id: string | null
  author_agent_id: string | null
  author_display_name: string
  body: string
  message_kind: MessageKind
  runtime_message_id: string | null
  delivery_state: MessageDeliveryState
  delivery_error: string | null
  replayed: boolean
  created_at: string
}

export interface SessionAgentRun {
  id: string
  session_id: string
  agent_id: string
  agent_name: string
  runtime_session_id: string | null
  runtime_run_id: string | null
  run_role: SessionRunRole
  state: SessionRunState
  last_error: string | null
  last_event_at: string | null
  model: string | null
  provider: string | null
  model_options: Record<string, unknown>
  created_at: string
  updated_at: string
}

export interface WorkflowBinding {
  id: string
  agent_id: string
  namespace_id: string | null
  namespace_name: string | null
  workflow_id: string | null
  workflow_name: string | null
  binding_status: string
  created_at: string
  updated_at: string
}

export interface AgentEvent {
  id: string
  agent_id: string | null
  event_type: string
  message: string
  payload: Record<string, unknown>
  created_at: string
}

export interface AgentLogEntry {
  id: string
  agent_id: string
  stream: string
  message: string
  created_at: string
}

export interface AuditLogEntry {
  id: string
  actor_user_id: string | null
  action: string
  entity_type: string
  entity_id: string | null
  payload: Record<string, unknown>
  created_at: string
}

export type DeploymentJobKind = 'provision' | 'runtime_update'
export type DeploymentJobState = 'queued' | 'running' | 'completed' | 'failed' | 'cancelled'

export interface DeploymentJob {
  id: string
  job_kind: DeploymentJobKind
  state: DeploymentJobState
  agent_id: string | null
  runtime_kind: AgentKind | null
  requested_by_user_id: string | null
  title: string
  detail: Record<string, unknown>
  last_error: string | null
  created_at: string
  updated_at: string
}

export interface FleetDashboard {
  total_agents: number
  leader_agents: number
  executor_agents: number
  running_agents: number
  failed_agents: number
  active_sessions: number
  private_sessions: number
  leader_scoped_sessions: number
  agents: Agent[]
  recent_events: AgentEvent[]
}

export interface RegisterRequest {
  email: string
  username: string
  display_name: string
  password: string
}

export interface LoginRequest {
  email: string
  password: string
}

export interface AuthResponse {
  access_token: string
  user_id: string
  email: string
  username: string
  display_name: string
  system_role: SystemRole
  is_system_admin: boolean
}

export interface UserResponse {
  id: string
  email: string
  username: string
  display_name: string
  system_role: SystemRole
  is_system_admin: boolean
  is_active: boolean
}

export interface UserListResponse {
  users: UserResponse[]
}

export interface UserPermissionsResponse {
  user_id: string
  role: SystemRole
  is_system_admin: boolean
  permissions: string[]
}

export interface UpdateUserRoleRequest {
  role: SystemRole
}

export interface CreateAgentRequest {
  kind: AgentKind
  product_role: AgentProductRole
  role: AgentRole
  display_name: string
  description?: string | null
  namespace_id?: string | null
  namespace_name?: string | null
  workflow_id?: string | null
  workflow_name?: string | null
  executor_ids?: string[]
}

export interface UpdateAgentRequest {
  product_role?: AgentProductRole
  role?: AgentRole
  display_name?: string
  description?: string
  namespace_id?: string
  workflow_id?: string
  executor_ids?: string[]
}

export interface UpdateAgentConfigRequest {
  config_json: Record<string, unknown>
  soul_md: string
  env_json: Record<string, unknown>
}

export interface UpdateSkillRequest {
  state: SkillState
  content?: string | null
}

export interface CreateSessionRequest {
  primary_agent_id?: string
  agent_id?: string
  title: string
  task_key?: string | null
  leader_agent_id?: string | null
  parent_session_id?: string | null
  namespace_id?: string | null
  idempotency_key?: string | null
}

export interface HandoffSessionRequest {
  target_agent_id: string
}

export interface CreateSessionDelegationRequest {
  executor_agent_id: string
  title: string
  task_key?: string | null
  initial_message?: string | null
  idempotency_key?: string | null
}

export interface AssignSessionLeaderRequest {
  leader_agent_id: string | null
}

export interface CreateSessionMessageRequest {
  body: string
  author_agent_id?: string | null
  message_kind?: MessageKind | null
  runtime_message_id?: string | null
  idempotency_key?: string | null
}

export interface SteerSessionRunRequest {
  input: string
}

export interface ResolveRuntimeApprovalRequest {
  choice: string
  resolve_all?: boolean
}

export interface RuntimeRunControlResponse {
  session_id: string
  run_id: string
  runtime_run_id: string | null
  accepted: boolean
  state: SessionRunState
  message: string
}

export interface UpdateLeaderExecutorsRequest {
  executor_ids: string[]
}

export interface PurgeAgentFilesRequest {
  confirmation: string
}

export interface PurgeAgentFilesResponse {
  agent_id: string
  agent_name: string
  purged_path: string
  files_deleted: boolean
  marker_verified: boolean
  message: string
}

export interface RuntimeOperationResponse {
  agent_id: string
  status: AgentStatus
  message: string
}

export interface CreateDeploymentJobRequest {
  job_kind: DeploymentJobKind
  agent_id?: string | null
  runtime_kind?: AgentKind | null
  title: string
  detail?: Record<string, unknown> | null
}

export interface RuntimeSettings {
  agents_root: string
  hermes_source: string
  hermes_command: string
  java_agent_source: string
  java_agent_command: string
}

export interface PortSettings {
  backend_port: number
  frontend_port: number
  agent_port_base: number
  agent_port_stride: number
}

export interface IntegrationSettings {
  project_workflow_url: string | null
  project_workflow_status: string
  github_remote: string | null
}

export interface AuthSettings {
  mode: string
  jwt_issuer: string
  jwt_audience: string
  access_token_ttl_minutes: number
  refresh_token_ttl_days: number
  refresh_cookie_name: string
  refresh_cookie_secure: boolean
  refresh_cookie_same_site: string
  refresh_cookie_domain: string | null
  refresh_cookie_path: string
}
