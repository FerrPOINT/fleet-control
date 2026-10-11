import { apiRequest } from './client'
import type { AgentDirectoryItem, AgentSession } from './types'

export type ChatsDirectoryAgent = {
  agent: AgentDirectoryItem
  matching_session_count: number
}
export type ChatsDirectoryPage = {
  agents: ChatsDirectoryAgent[]
  selected_agent_id: string | null
  items: AgentSession[]
  next_before: string | null
}
export type ChatsDirectoryQuery = {
  agentId?: string
  userIds?: string[]
  search?: string
  before?: string
  limit?: number
}

export function getChatsDirectory(query: ChatsDirectoryQuery = {}) {
  const params = new URLSearchParams()
  if (query.agentId) params.set('agent_id', query.agentId)
  if (query.userIds !== undefined) params.set('user_id', query.userIds.join(',') || 'all')
  if (query.search?.trim()) params.set('q', query.search.trim())
  if (query.before) params.set('before', query.before)
  params.set('limit', String(query.limit ?? 50))
  return apiRequest<ChatsDirectoryPage>(`/api/v1/chats/directory?${params}`)
}
