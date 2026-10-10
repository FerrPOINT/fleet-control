import { render, screen } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter, Route, Routes } from 'react-router'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ChatDetailPage } from './index'
import { getSession } from '@/api/fleet'
import type { AgentSession } from '@/api/types'
import { useAuthStore } from '@/shared/auth/store'

vi.mock('@/api/fleet', () => ({ getSession: vi.fn() }))
vi.mock('./private-detail', () => ({ PrivateChatDetailPage: () => <p>Private controller</p> }))
vi.mock('./task-detail', () => ({ TaskChatDetailPage: () => <p>Task controller</p> }))

function renderDetail() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={['/chats/session-one']}>
        <Routes>
          <Route path="/chats/:sessionId" element={<ChatDetailPage />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  )
}

beforeEach(() => {
  vi.clearAllMocks()
  useAuthStore.setState({ token: 'owned-fixture', userId: 'owner', signingOut: false })
})

describe('authoritative chat binding boundary', () => {
  it('keeps a private session private despite a legacy task display name', async () => {
    vi.mocked(getSession).mockResolvedValue({
      task_bound: false,
      task_key: 'TASK-1',
    } as AgentSession)
    renderDetail()
    expect(await screen.findByText('Private controller')).toBeVisible()
    expect(screen.queryByText('Task controller')).not.toBeInTheDocument()
  })

  it('opens the task controller for a binding without a legacy display key', async () => {
    vi.mocked(getSession).mockResolvedValue({ task_bound: true, task_key: null } as AgentSession)
    renderDetail()
    expect(await screen.findByText('Task controller')).toBeVisible()
    expect(screen.queryByText('Private controller')).not.toBeInTheDocument()
  })

  it('holds unknown binding instead of exposing a writable private controller', async () => {
    vi.mocked(getSession).mockResolvedValue({ task_key: null } as AgentSession)
    renderDetail()
    expect(await screen.findByText('Не удалось проверить привязку чата')).toBeVisible()
    expect(screen.queryByText('Private controller')).not.toBeInTheDocument()
    expect(screen.queryByText('Task controller')).not.toBeInTheDocument()
  })

  it('keeps denied sessions closed', async () => {
    vi.mocked(getSession).mockRejectedValue(new Error('Forbidden'))
    renderDetail()
    expect(await screen.findByText('Чат недоступен')).toBeVisible()
    expect(screen.queryByText('Private controller')).not.toBeInTheDocument()
  })

  it('does not select a controller or read a session during logout', () => {
    useAuthStore.setState({ signingOut: true })
    renderDetail()
    expect(getSession).not.toHaveBeenCalled()
    expect(screen.queryByText('Private controller')).not.toBeInTheDocument()
    expect(screen.queryByText('Task controller')).not.toBeInTheDocument()
  })
})
