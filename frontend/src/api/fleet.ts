import { apiRequest } from './client'
import type {
  Agent,
  AgentConfig,
  AgentLogEntry,
  AgentSession,
  CreateAgentRequest,
  CreateSessionRequest,
  FleetDashboard,
  HandoffSessionRequest,
  RuntimeOperationResponse,
  RuntimeTemplate,
  UpdateAgentConfigRequest,
  UpdateAgentRequest,
  UpdateSkillRequest,
  WorkflowBinding,
} from './types'

export function getDashboard() {
  return apiRequest<FleetDashboard>('/api/v1/dashboard')
}

export function listAgents() {
  return apiRequest<Agent[]>('/api/v1/agents')
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

export function archiveAgent(id: string) {
  return apiRequest<Agent>(`/api/v1/agents/${id}`, { method: 'DELETE' })
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

export function listSessions(agentId?: string) {
  const query = agentId ? `?agent_id=${agentId}` : ''
  return apiRequest<AgentSession[]>(`/api/v1/sessions${query}`)
}

export function createSession(req: CreateSessionRequest) {
  return apiRequest<AgentSession>('/api/v1/sessions', { method: 'POST', body: JSON.stringify(req) })
}

export function getSession(id: string) {
  return apiRequest<AgentSession>(`/api/v1/sessions/${id}`)
}

export function handoffSession(id: string, req: HandoffSessionRequest) {
  return apiRequest<AgentSession>(`/api/v1/sessions/${id}/handoff`, {
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

export function listLogs(agentId?: string, limit = 100) {
  const params = new URLSearchParams({ limit: String(limit) })
  if (agentId) params.set('agent_id', agentId)
  return apiRequest<AgentLogEntry[]>(`/api/v1/logs?${params}`)
}
