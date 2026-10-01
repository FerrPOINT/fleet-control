import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter } from 'react-router'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ChatsPage, groupChats } from './index'
import * as fleet from '@/api/fleet'
import * as auth from '@/api/auth'
import type { AgentDirectoryItem, AgentSession } from '@/api/types'
import { useAuthStore } from '@/shared/auth/store'

vi.mock('@/api/fleet', () => ({
  createSession: vi.fn(),
  listAgentDirectory: vi.fn(),
  listSessions: vi.fn(),
}))
vi.mock('@/api/auth', () => ({ listUsers: vi.fn() }))

const developer = {
  id: 'developer',
  name: 'agent1',
  display_name: 'Developer',
  kind: 'hermes',
  product_role: 'executor',
  role: 'developer',
  sdlc_role: 'developer',
  status: 'running',
} as AgentDirectoryItem
const tester = {
  ...developer,
  id: 'tester',
  name: 'agent2',
  display_name: 'Tester',
  sdlc_role: 'tester',
} as AgentDirectoryItem
const session = {
  id: 'chat-1',
  primary_agent_id: developer.id,
  user_id: 'owner',
  user_display_name: 'Owner',
  title: 'Implement login',
  task_key: 'TASK-1',
  state: 'active',
  visibility: 'private',
  updated_at: '2026-10-01T12:00:00Z',
  last_message_preview: 'Login requirements',
} as AgentSession

function renderPage() {
  return render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <MemoryRouter>
        <ChatsPage />
      </MemoryRouter>
    </QueryClientProvider>,
  )
}

describe('ChatsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    useAuthStore.setState({
      userId: 'owner',
      displayName: 'Owner',
      email: 'owner@example.test',
      systemRole: 'user',
      isSystemAdmin: false,
      permissions: ['sessions:write_own'],
    })
    vi.mocked(fleet.listAgentDirectory).mockResolvedValue([developer, tester])
    vi.mocked(fleet.listSessions).mockResolvedValue([session])
    vi.mocked(auth.listUsers).mockResolvedValue([])
    vi.mocked(fleet.createSession).mockResolvedValue(session)
  })

  it('keeps each concrete agent chat separate even when the task is the same', () => {
    const groups = groupChats(
      [developer, tester],
      [session, { ...session, id: 'chat-2', primary_agent_id: tester.id }],
    )
    expect(groups[0]?.sessions.map((item) => item.id)).toEqual(['chat-1'])
    expect(groups[1]?.sessions.map((item) => item.id)).toEqual(['chat-2'])
  })

  it('defaults to the owner filter and switches agent groups', async () => {
    renderPage()
    expect(await screen.findByRole('link', { name: /Implement login/ })).toHaveAttribute(
      'href',
      '/chats/chat-1?returnTo=%2Fchats%3Fagent%3Ddeveloper%26users%3Downer',
    )
    expect(fleet.listSessions).toHaveBeenCalledWith(undefined, ['owner'])
    expect(auth.listUsers).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: /Tester/ }))
    expect(screen.queryByRole('link', { name: /Implement login/ })).not.toBeInTheDocument()
    expect(screen.getByText('Сессий по выбранному фильтру нет')).toBeVisible()
  })

  it('reuses a private-chat idempotency key after a failed create', async () => {
    vi.mocked(fleet.createSession).mockRejectedValueOnce(new Error('offline'))
    renderPage()
    fireEvent.click(await screen.findByRole('button', { name: 'Новый чат' }))
    fireEvent.change(screen.getByLabelText('Название'), { target: { value: '  Private work  ' } })
    fireEvent.click(screen.getByRole('button', { name: 'Создать сессию' }))
    await screen.findByRole('alert')
    const first = vi.mocked(fleet.createSession).mock.calls[0]?.[0]
    expect(first).toMatchObject({
      primary_agent_id: developer.id,
      title: 'Private work',
      leader_agent_id: null,
    })
    fireEvent.click(screen.getByRole('button', { name: 'Создать сессию' }))
    await waitFor(() => expect(fleet.createSession).toHaveBeenCalledTimes(2))
    expect(vi.mocked(fleet.createSession).mock.calls[1]?.[0].idempotency_key).toBe(
      first?.idempotency_key,
    )
  })

  it('renders loading failures instead of an empty directory', async () => {
    vi.mocked(fleet.listAgentDirectory).mockRejectedValue(new Error('offline'))
    renderPage()
    expect(await screen.findByRole('alert')).toBeVisible()
    expect(screen.queryByRole('button', { name: 'Новый чат' })).not.toBeInTheDocument()
  })
})
