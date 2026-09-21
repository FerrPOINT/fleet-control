import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter } from 'react-router'
import { toast } from 'sonner'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { SessionsPage } from './index'
import * as fleet from '@/api/fleet'
import * as auth from '@/api/auth'
import type { Agent, AgentSession } from '@/api/types'
import { useAuthStore } from '@/shared/auth/store'

vi.mock('@/api/fleet', () => ({
  createSession: vi.fn(),
  listAgentDirectory: vi.fn(),
  listLeaderExecutors: vi.fn(),
  listSessions: vi.fn(),
}))
vi.mock('@/api/auth', () => ({ listUsers: vi.fn() }))
vi.mock('sonner', () => ({ toast: { success: vi.fn() } }))

const executor = {
  id: 'executor-1',
  name: 'agent1',
  display_name: 'Developer Hermes',
  role: 'developer',
  product_role: 'executor',
  kind: 'hermes',
  status: 'ready',
} as Agent

const leader = {
  ...executor,
  id: 'leader-1',
  name: 'agent2',
  display_name: 'IT Lead Hermes',
  role: 'it_lead',
  product_role: 'leader',
} as Agent

const session = {
  id: 'session-1',
  agent_id: executor.id,
  primary_agent_id: executor.id,
  agent_name: executor.name,
  primary_agent_name: executor.name,
  user_id: 'user-1',
  user_email: 'operator@example.test',
  user_username: 'operator',
  user_display_name: 'Fleet Operator',
  leader_agent_id: null,
  leader_agent_name: null,
  parent_session_id: null,
  created_by_leader_agent_id: null,
  visibility: 'private',
  title: 'Release verification',
  task_key: 'CARD-123',
  state: 'draft',
  namespace_id: 'dev',
  external_session_id: null,
  last_message_preview: null,
  created_at: '2026-09-21T08:00:00Z',
  updated_at: '2026-09-21T08:00:00Z',
} as AgentSession

function renderPage() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter>
        <SessionsPage />
      </MemoryRouter>
    </QueryClientProvider>,
  )
}

describe('SessionsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    useAuthStore.setState({
      userId: null,
      email: null,
      username: null,
      displayName: null,
      systemRole: 'user',
      isSystemAdmin: false,
      permissions: [],
    })
    vi.mocked(fleet.listAgentDirectory).mockResolvedValue([executor])
    vi.mocked(fleet.listLeaderExecutors).mockResolvedValue([])
    vi.mocked(fleet.listSessions).mockResolvedValue([])
    vi.mocked(fleet.createSession).mockResolvedValue(session)
    vi.mocked(auth.listUsers).mockResolvedValue([])
  })

  it('creates a session with normalized values and confirms the result', async () => {
    renderPage()

    const agent = await screen.findByLabelText('Агент')
    await waitFor(() => expect(agent).toHaveValue(executor.id))
    const title = screen.getByLabelText('Название')
    expect(title).toHaveValue('Новая рабочая сессия')
    fireEvent.change(title, { target: { value: '  Release verification  ' } })
    fireEvent.change(screen.getByLabelText('Ключ задачи'), {
      target: { value: '  CARD-123  ' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Создать сессию' }))

    await waitFor(() =>
      expect(fleet.createSession).toHaveBeenCalledWith(
        expect.objectContaining({
          primary_agent_id: executor.id,
          title: 'Release verification',
          task_key: 'CARD-123',
          leader_agent_id: null,
        }),
      ),
    )
    expect(toast.success).toHaveBeenCalledWith('Сессия «Release verification» создана')
    expect(title).toHaveValue('Новая рабочая сессия')
  })

  it('rejects a whitespace title before sending a request', async () => {
    renderPage()
    const title = await screen.findByLabelText('Название')

    fireEvent.change(title, { target: { value: '   ' } })
    fireEvent.blur(title)

    expect(screen.getByRole('alert')).toHaveTextContent('Введите название сессии')
    expect(screen.getByRole('button', { name: 'Создать сессию' })).toBeDisabled()
    expect(fleet.createSession).not.toHaveBeenCalled()
  })

  it('keeps creation unavailable until an agent request is retried successfully', async () => {
    vi.mocked(fleet.listAgentDirectory)
      .mockRejectedValueOnce(new Error('offline'))
      .mockResolvedValueOnce([executor])
    renderPage()

    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent('Не удалось загрузить агентов')
    expect(screen.getByRole('button', { name: 'Создать сессию' })).toBeDisabled()
    fireEvent.click(within(alert.parentElement!).getByRole('button', { name: 'Повторить' }))

    expect(await screen.findByLabelText('Агент')).toHaveValue(executor.id)
    expect(screen.getByRole('button', { name: 'Создать сессию' })).toBeEnabled()
  })

  it('distinguishes a session request failure from an empty result', async () => {
    vi.mocked(fleet.listSessions)
      .mockRejectedValueOnce(new Error('offline'))
      .mockResolvedValueOnce([session])
    renderPage()

    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent('Не удалось загрузить сессии')
    expect(
      screen.queryByText('Сессий для выбранных пользователей пока нет'),
    ).not.toBeInTheDocument()
    fireEvent.click(within(alert.parentElement!).getByRole('button', { name: 'Повторить' }))

    expect(await screen.findByRole('link', { name: /Release verification/ })).toBeVisible()
    expect(screen.getByText('Владелец: Fleet Operator')).toBeVisible()
    expect(screen.getByText('Пространство: dev')).toBeVisible()
  })

  it('still allows a private session when leader teams cannot be loaded', async () => {
    vi.mocked(fleet.listAgentDirectory).mockResolvedValue([executor, leader])
    vi.mocked(fleet.listLeaderExecutors).mockRejectedValue(new Error('offline'))
    renderPage()

    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent('Пока можно создать только личную сессию')
    expect(screen.getByLabelText('Лидер')).toBeDisabled()
    const create = screen.getByRole('button', { name: 'Создать сессию' })
    expect(create).toBeEnabled()
    fireEvent.click(create)

    await waitFor(() =>
      expect(fleet.createSession).toHaveBeenCalledWith(
        expect.objectContaining({ leader_agent_id: null }),
      ),
    )
  })
})
