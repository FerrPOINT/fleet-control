export type AgentKind = 'hermes' | 'java_agent'
export type AgentRole = 'developer' | 'tester' | 'custom'
export type AgentStatus =
  'provisioning' | 'ready' | 'starting' | 'running' | 'stopped' | 'failed' | 'archived'
export type DesiredState = 'running' | 'stopped'
export type SkillState = 'enabled' | 'disabled' | 'missing' | 'dirty'
export type SessionState =
  'draft' | 'active' | 'handoff_requested' | 'blocked' | 'done' | 'archived'

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
  started_at: string | null
  stopped_at: string | null
  last_health_at: string | null
}

export interface Agent {
  id: string
  ordinal: number
  name: string
  kind: AgentKind
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
  agent_name: string
  title: string
  task_key: string | null
  state: SessionState
  namespace_id: string | null
  external_session_id: string | null
  last_message_preview: string | null
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

export interface FleetDashboard {
  total_agents: number
  running_agents: number
  failed_agents: number
  active_sessions: number
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
  is_system_admin: boolean
}

export interface UserResponse {
  id: string
  email: string
  username: string
  display_name: string
  is_system_admin: boolean
  is_active: boolean
}

export interface UserListResponse {
  users: UserResponse[]
}

export interface CreateAgentRequest {
  kind: AgentKind
  role: AgentRole
  display_name: string
  description?: string | null
  namespace_id?: string | null
  namespace_name?: string | null
  workflow_id?: string | null
  workflow_name?: string | null
}

export interface UpdateAgentRequest {
  role?: AgentRole
  display_name?: string
  description?: string
  namespace_id?: string
  workflow_id?: string
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
  agent_id: string
  title: string
  task_key?: string | null
  namespace_id?: string | null
}

export interface HandoffSessionRequest {
  target_agent_id: string
}

export interface RuntimeOperationResponse {
  agent_id: string
  status: AgentStatus
  message: string
}
