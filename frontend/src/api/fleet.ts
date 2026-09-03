import { apiRequest } from './client'
import type {
  Agent,
  AgentConfig,
  AgentDirectoryItem,
  AgentLogEntry,
  AgentSession,
  AgentEvent,
  AgentStorageReport,
  AuditLogEntry,
  AssignSessionLeaderRequest,
  CreateAgentRequest,
  CreateDeploymentJobRequest,
  CreateSessionDelegationRequest,
  CreateSessionMessageRequest,
  CreateSessionRequest,
  DeploymentJob,
  FleetDashboard,
  HandoffSessionRequest,
  AuthSettings,
  IntegrationSettings,
  LeaderExecutor,
  PortSettings,
  PurgeAgentFilesRequest,
  PurgeAgentFilesResponse,
  RuntimeOperationResponse,
  RuntimeRunControlResponse,
  RuntimeSettings,
  RuntimeTemplate,
  ResolveRuntimeApprovalRequest,
  SessionAgentRun,
  SessionMessage,
  SessionParticipant,
  SteerSessionRunRequest,
  UpdateAgentConfigRequest,
  UpdateAgentRequest,
  UpdateLeaderExecutorsRequest,
  UpdateSkillRequest,
  WorkflowBinding,
} from './types'

export function getDashboard() {
  return apiRequest<FleetDashboard>('/api/v1/dashboard')
}

export function listAgents() {
  return apiRequest<Agent[]>('/api/v1/agents')
}

export function listAgentDirectory() {
  return apiRequest<AgentDirectoryItem[]>('/api/v1/agent-directory')
}

export function listLeaders() {
  return apiRequest<Agent[]>('/api/v1/leaders')
}

export function listExecutors() {
  return apiRequest<Agent[]>('/api/v1/executors')
}

export function createAgent(req: CreateAgentRequest) {
  return apiRequest<Agent>('/api/v1/agents', { method: 'POST', body: JSON.stringify(req) })
}

export function getAgent(id: string) {
  return apiRequest<Agent>(`/api/v1/agents/${id}`)
}

export function updateAgent(id: string, req: UpdateAgentRequest) {
  return apiRequest<Agent>(`/api/v1/agents/${id}`, {
    method: 'PATCH',
    body: JSON.stringify(req),
  })
}

export function listLeaderExecutors(id: string) {
  return apiRequest<LeaderExecutor[]>(`/api/v1/leaders/${id}/executors`)
}

export function updateLeaderExecutors(id: string, req: UpdateLeaderExecutorsRequest) {
  return apiRequest<LeaderExecutor[]>(`/api/v1/leaders/${id}/executors`, {
    method: 'PUT',
    body: JSON.stringify(req),
  })
}

export function archiveAgent(id: string) {
  return apiRequest<Agent>(`/api/v1/agents/${id}`, { method: 'DELETE' })
}

export function getAgentStorage(id: string) {
  return apiRequest<AgentStorageReport>(`/api/v1/agents/${id}/storage`)
}

export function purgeAgentFiles(id: string, req: PurgeAgentFilesRequest) {
  return apiRequest<PurgeAgentFilesResponse>(`/api/v1/agents/${id}/purge-files`, {
    method: 'POST',
    body: JSON.stringify(req),
  })
}

export function runAgentOperation(
  id: string,
  operation: 'provision' | 'start' | 'stop' | 'restart' | 'health',
) {
  return apiRequest<RuntimeOperationResponse | Agent>(`/api/v1/agents/${id}/${operation}`, {
    method: 'POST',
    body: '{}',
  })
}

export function getAgentConfig(id: string) {
  return apiRequest<AgentConfig>(`/api/v1/agents/${id}/config`)
}

export function updateAgentConfig(id: string, req: UpdateAgentConfigRequest) {
  return apiRequest<AgentConfig>(`/api/v1/agents/${id}/config`, {
    method: 'PUT',
    body: JSON.stringify(req),
  })
}

export function listAgentSkills(id: string) {
  return apiRequest<import('./types').AgentSkill[]>(`/api/v1/agents/${id}/skills`)
}

export function updateAgentSkill(id: string, name: string, req: UpdateSkillRequest) {
  return apiRequest<import('./types').AgentSkill>(`/api/v1/agents/${id}/skills/${name}`, {
    method: 'PUT',
    body: JSON.stringify(req),
  })
}

export function listSessions(agentId?: string, userIds?: string[], leaderAgentId?: string) {
  const params = new URLSearchParams()
  if (agentId) params.set('agent_id', agentId)
  if (leaderAgentId) params.set('leader_agent_id', leaderAgentId)
  if (userIds) params.set('user_id', userIds.length ? userIds.join(',') : 'all')
  const query = params.toString()
  return apiRequest<AgentSession[]>(`/api/v1/sessions${query ? `?${query}` : ''}`)
}

export function createSession(req: CreateSessionRequest) {
  return apiRequest<AgentSession>('/api/v1/sessions', { method: 'POST', body: JSON.stringify(req) })
}

export function getSession(id: string) {
  return apiRequest<AgentSession>(`/api/v1/sessions/${id}`)
}

export function listSessionMessages(id: string) {
  return apiRequest<SessionMessage[]>(`/api/v1/sessions/${id}/messages`)
}

export function listSessionParticipants(id: string) {
  return apiRequest<SessionParticipant[]>(`/api/v1/sessions/${id}/participants`)
}

export function createSessionMessage(id: string, req: CreateSessionMessageRequest) {
  return apiRequest<SessionMessage>(`/api/v1/sessions/${id}/messages`, {
    method: 'POST',
    body: JSON.stringify(req),
  })
}

export function createSessionDelegation(id: string, req: CreateSessionDelegationRequest) {
  return apiRequest<AgentSession>(`/api/v1/sessions/${id}/delegations`, {
    method: 'POST',
    body: JSON.stringify(req),
  })
}

export function assignSessionLeader(id: string, req: AssignSessionLeaderRequest) {
  return apiRequest<AgentSession>(`/api/v1/sessions/${id}/leader`, {
    method: 'PUT',
    body: JSON.stringify(req),
  })
}

export function handoffSession(id: string, req: HandoffSessionRequest) {
  return apiRequest<AgentSession>(`/api/v1/sessions/${id}/handoff`, {
    method: 'POST',
    body: JSON.stringify(req),
  })
}

export function listSessionAgentRuns(id: string) {
  return apiRequest<SessionAgentRun[]>(`/api/v1/sessions/${id}/runs`)
}

export function steerSessionRun(id: string, runId: string, req: SteerSessionRunRequest) {
  return apiRequest<RuntimeRunControlResponse>(`/api/v1/sessions/${id}/runs/${runId}/steer`, {
    method: 'POST',
    body: JSON.stringify(req),
  })
}

export function stopSessionRun(id: string, runId: string) {
  return apiRequest<RuntimeRunControlResponse>(`/api/v1/sessions/${id}/runs/${runId}/stop`, {
    method: 'POST',
    body: '{}',
  })
}

export function resolveSessionRunApproval(
  id: string,
  runId: string,
  req: ResolveRuntimeApprovalRequest,
) {
  return apiRequest<RuntimeRunControlResponse>(`/api/v1/sessions/${id}/runs/${runId}/approval`, {
    method: 'POST',
    body: JSON.stringify(req),
  })
}

export function listWorkflowBindings() {
  return apiRequest<WorkflowBinding[]>('/api/v1/workflow-bindings')
}

export function listRuntimeTemplates() {
  return apiRequest<RuntimeTemplate[]>('/api/v1/runtime-templates')
}

export function listDeploymentJobs(limit = 100) {
  return apiRequest<DeploymentJob[]>(`/api/v1/deployments/jobs?limit=${limit}`)
}

export function getDeploymentJob(id: string) {
  return apiRequest<DeploymentJob>(`/api/v1/deployments/jobs/${id}`)
}

export function createDeploymentJob(req: CreateDeploymentJobRequest) {
  return apiRequest<DeploymentJob>('/api/v1/deployments/jobs', {
    method: 'POST',
    body: JSON.stringify(req),
  })
}

export function cancelDeploymentJob(id: string) {
  return apiRequest<DeploymentJob>(`/api/v1/deployments/jobs/${id}/cancel`, {
    method: 'POST',
    body: '{}',
  })
}

export function listLogs(agentId?: string, limit = 100) {
  const params = new URLSearchParams({ limit: String(limit) })
  if (agentId) params.set('agent_id', agentId)
  return apiRequest<AgentLogEntry[]>(`/api/v1/logs?${params}`)
}

export function listEvents(limit = 100) {
  return apiRequest<AgentEvent[]>(`/api/v1/events/recent?limit=${limit}`)
}

export function listAuditLog(
  filters: {
    actor_user_id?: string
    action?: string
    entity_type?: string
    entity_id?: string
    date_from?: string
    date_to?: string
    limit?: number
  } = {},
) {
  const params = new URLSearchParams()
  for (const [key, value] of Object.entries(filters)) {
    if (value !== undefined && value !== '') params.set(key, String(value))
  }
  const query = params.toString()
  return apiRequest<AuditLogEntry[]>(`/api/v1/audit-log${query ? `?${query}` : ''}`)
}

export function getRuntimeSettings() {
  return apiRequest<RuntimeSettings>('/api/v1/settings/runtime')
}

export function updateRuntimeSettings(req: RuntimeSettings) {
  return apiRequest<RuntimeSettings>('/api/v1/settings/runtime', {
    method: 'PUT',
    body: JSON.stringify(req),
  })
}

export function getPortSettings() {
  return apiRequest<PortSettings>('/api/v1/settings/ports')
}

export function updatePortSettings(req: PortSettings) {
  return apiRequest<PortSettings>('/api/v1/settings/ports', {
    method: 'PUT',
    body: JSON.stringify(req),
  })
}

export function getIntegrationSettings() {
  return apiRequest<IntegrationSettings>('/api/v1/settings/integrations')
}

export function updateIntegrationSettings(req: IntegrationSettings) {
  return apiRequest<IntegrationSettings>('/api/v1/settings/integrations', {
    method: 'PUT',
    body: JSON.stringify(req),
  })
}

export function getAuthSettings() {
  return apiRequest<AuthSettings>('/api/v1/settings/auth')
}

export function updateAuthSettings(req: AuthSettings) {
  return apiRequest<AuthSettings>('/api/v1/settings/auth', {
    method: 'PUT',
    body: JSON.stringify(req),
  })
}
