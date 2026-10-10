import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ChatDetailPage } from './index'
import * as fleet from '@/api/fleet'
import * as chats from '@/api/task-chats'
import { useAuthStore } from '@/shared/auth/store'
import type { AgentDirectoryItem, AgentSession, SessionMessage } from '@/api/types'
import { ApiError } from '@sdlc/ui/lib'
import { chatMessageRequest, payloadDigest } from './core'

vi.mock('@/api/fleet', () => ({
  getSession: vi.fn(),
  listAgentDirectory: vi.fn(),
  listSessionAgentRuns: vi.fn(),
  listSessionMessages: vi.fn(),
  createSessionMessage: vi.fn(),
  steerSessionRun: vi.fn(),
  stopSessionRun: vi.fn(),
}))
vi.mock('@/api/task-chats', async (original) => ({
  ...(await original<typeof import('@/api/task-chats')>()),
  getTaskContext: vi.fn(),
  getChatControls: vi.fn(),
  getChatHistory: vi.fn(),
  getClarifications: vi.fn(),
  getRequirements: vi.fn(),
  listPendingAnswerCommands: vi.fn(),
}))
vi.mock('@sdlc/ui/lib', async (original) => ({
  ...(await original<typeof import('@sdlc/ui/lib')>()),
  connectAuthenticatedEventStream: vi.fn(() => () => {}),
}))

function renderPage() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })
  const router = createMemoryRouter([{ path: '/chats/:sessionId', element: <ChatDetailPage /> }], {
    initialEntries: ['/chats/session1?tab=dialogue'],
  })
  render(
    <QueryClientProvider client={client}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  )
  return client
}

beforeEach(() => {
  vi.resetAllMocks()
  sessionStorage.clear()
  useAuthStore.setState({
    userId: 'owner',
    token: 'fixture-token',
    signingOut: false,
    permissions: ['sessions:write_own'],
  })
  vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
  vi.mocked(fleet.getSession).mockResolvedValue({
    id: 'session1',
    user_id: 'owner',
    user_display_name: 'Owner',
    primary_agent_id: 'agent1',
    primary_agent_name: 'Executor',
    title: 'Free chat',
    state: 'active',
    visibility: 'private',
    pending_delivery: false,
    task_bound: false,
  } as AgentSession)
  vi.mocked(fleet.listAgentDirectory).mockResolvedValue([
    { id: 'agent1', product_role: 'executor', status: 'running' } as AgentDirectoryItem,
  ])
  vi.mocked(fleet.listSessionAgentRuns).mockResolvedValue([])
  vi.mocked(fleet.listSessionMessages).mockResolvedValue([])
  vi.mocked(chats.getChatControls).mockResolvedValue({
    can_send: true,
    can_steer: false,
    can_stop: false,
    active_run_id: null,
    blocked_reason: null,
  })
  vi.mocked(chats.getChatHistory).mockResolvedValue({ items: [], next_before: null })
  vi.mocked(chats.getClarifications).mockResolvedValue({ questions: [] })
  vi.mocked(chats.getRequirements).mockResolvedValue({ revisions: [] })
  vi.mocked(chats.listPendingAnswerCommands).mockResolvedValue([])
})

describe('verified free-chat boundary', () => {
  it('keeps private cursor order and overlap while appending a command after an older page', async () => {
    const message = (id: string, body: string): SessionMessage => ({
      id,
      body,
      session_id: 'session1',
      author_type: 'user',
      author_user_id: 'owner',
      author_agent_id: null,
      author_display_name: 'Owner',
      message_kind: 'user_prompt',
      runtime_message_id: null,
      delivery_state: 'completed',
      delivery_error: null,
      created_at: '2026-10-01T12:00:00Z',
      replayed: false,
    })
    let saved: SessionMessage | undefined
    vi.mocked(chats.getChatHistory).mockImplementation(async (_id, before) =>
      before
        ? {
            items: [message('oldest', 'Oldest'), message('latest', 'Outdated overlap')],
            next_before: null,
          }
        : {
            items: [message('latest', 'Latest'), ...(saved ? [saved] : [])],
            next_before: 'latest',
          },
    )
    vi.mocked(fleet.createSessionMessage).mockImplementation(async (_id, input) => {
      saved = {
        ...message('saved', input.body),
        request_payload_hash: await payloadDigest(input),
      }
      return saved
    })
    renderPage()
    await screen.findByText('Latest')
    fireEvent.click(screen.getByRole('button', { name: 'Предыдущие сообщения' }))
    await screen.findByText('Oldest')
    expect(chats.getChatHistory).toHaveBeenCalledWith('session1', 'latest')
    expect(screen.queryByText('Outdated overlap')).not.toBeInTheDocument()
    fireEvent.change(screen.getByLabelText('Сообщение'), { target: { value: 'Newest command' } })
    await waitFor(() => expect(screen.getByRole('button', { name: 'Отправить' })).toBeEnabled())
    fireEvent.click(screen.getByRole('button', { name: 'Отправить' }))
    await screen.findByText('Newest command')
    expect(
      [...document.querySelectorAll('.fc-chat-message > p')].map((node) => node.textContent),
    ).toEqual(['Oldest', 'Latest', 'Newest command'])
    expect(fleet.createSessionMessage).toHaveBeenCalledTimes(1)
    const input = vi.mocked(fleet.createSessionMessage).mock.calls[0]![1]
    expect(input).toEqual(chatMessageRequest('Newest command', input.idempotency_key!))
    expect(sessionStorage.getItem('fleet-control.chat-dispatch.v1:session1')).toBeNull()
  })

  it('hides the composer when the session itself is inaccessible', async () => {
    vi.mocked(chats.getTaskContext).mockRejectedValue(new ApiError(403, 'Forbidden'))
    vi.mocked(fleet.getSession).mockRejectedValue(new ApiError(403, 'Forbidden'))
    renderPage()
    await screen.findByText(/Нет доступа к чату/)
    expect(screen.queryByRole('textbox')).not.toBeInTheDocument()
    expect(fleet.createSessionMessage).not.toHaveBeenCalled()
  })

  it('uses standalone dispatch only after a verified unbound task context', async () => {
    renderPage()
    await screen.findByLabelText('Сообщение')
    expect(chats.getTaskContext).toHaveBeenCalledWith('session1')
    expect(chats.getChatHistory).toHaveBeenCalledWith('session1', undefined)
    expect(fleet.listSessionMessages).not.toHaveBeenCalled()
    expect(chats.getClarifications).not.toHaveBeenCalled()
    expect(chats.listPendingAnswerCommands).not.toHaveBeenCalled()
  })

  it('does not treat a pending binding read as permission to enter standalone chat', () => {
    vi.mocked(chats.getTaskContext).mockReturnValue(new Promise(() => {}))
    renderPage()
    expect(screen.getByText('Загрузка чата')).toBeVisible()
    expect(screen.queryByLabelText('Сообщение')).not.toBeInTheDocument()
    expect(fleet.listSessionMessages).not.toHaveBeenCalled()
    expect(fleet.createSessionMessage).not.toHaveBeenCalled()
  })

  it('does not fall through to a free send after binding access fails', async () => {
    vi.mocked(chats.getTaskContext).mockRejectedValue(new ApiError(403, 'Binding access revoked'))
    renderPage()
    await screen.findByText('Binding access revoked')
    expect(screen.queryByRole('textbox')).not.toBeInTheDocument()
    expect(fleet.listSessionMessages).not.toHaveBeenCalled()
    expect(fleet.createSessionMessage).not.toHaveBeenCalled()
  })

  it('retains a free draft but blocks dispatch after cached binding access is revoked', async () => {
    const client = renderPage()
    const editor = await screen.findByLabelText('Сообщение')
    fireEvent.change(editor, { target: { value: 'Original free draft' } })
    await waitFor(() => expect(screen.getByRole('button', { name: 'Отправить' })).toBeEnabled())
    vi.mocked(chats.getTaskContext).mockRejectedValue(new ApiError(403, 'Binding access revoked'))
    await act(() => client.invalidateQueries({ queryKey: ['task-context', 'session1'] }))
    await screen.findByText('Binding access revoked')
    expect(editor).toHaveValue('Original free draft')
    expect(screen.getByRole('button', { name: 'Отправить' })).toBeDisabled()
    fireEvent.click(screen.getByRole('button', { name: 'Отправить' }))
    expect(fleet.createSessionMessage).not.toHaveBeenCalled()
  })
})
