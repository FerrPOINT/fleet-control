import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter, Route, Routes } from 'react-router'
import { toast } from 'sonner'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { SessionDetailPage } from './index'
import * as fleet from '@/api/fleet'
import type {
  Agent,
  AgentSession,
  LeaderExecutor,
  RuntimeRunControlResponse,
  SessionAgentRun,
  SessionMessage,
} from '@/api/types'
import { useAuthStore } from '@/shared/auth/store'

vi.mock('@/api/fleet', () => ({
  assignSessionLeader: vi.fn(),
  createSessionDelegation: vi.fn(),
  createSessionMessage: vi.fn(),
  getSession: vi.fn(),
  handoffSession: vi.fn(),
  listAgentDirectory: vi.fn(),
  listLeaderExecutors: vi.fn(),
  listSessionAgentRuns: vi.fn(),
  listSessionMessages: vi.fn(),
  listSessionParticipants: vi.fn(),
  resolveSessionRunApproval: vi.fn(),
  steerSessionRun: vi.fn(),
  stopSessionRun: vi.fn(),
}))
vi.mock('sonner', () => ({ toast: { success: vi.fn() } }))

const executor = {
  id: 'executor-1',
  name: 'agent-alpha',
  display_name: 'Agent Alpha',
  role: 'developer',
  product_role: 'executor',
  kind: 'hermes',
  status: 'running',
} as Agent

const secondExecutor = {
  ...executor,
  id: 'executor-2',
  name: 'agent-beta',
  display_name: 'Agent Beta',
} as Agent

const leader = {
  ...executor,
  id: 'leader-1',
  name: 'team-lead',
  display_name: 'Team Lead',
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
  leader_agent_id: leader.id,
  leader_agent_name: leader.name,
  parent_session_id: null,
  created_by_leader_agent_id: null,
  visibility: 'leader_scoped',
  title: 'Release verification',
  task_key: 'CARD-123',
  state: 'active',
  namespace_id: 'dev',
  external_session_id: null,
  last_message_preview: null,
  created_at: '2026-09-21T08:00:00Z',
  updated_at: '2026-09-21T08:30:00Z',
} as AgentSession

const teamExecutor = {
  leader_agent_id: leader.id,
  executor_agent_id: secondExecutor.id,
  executor_name: secondExecutor.name,
  executor_display_name: secondExecutor.display_name,
  executor_profile: secondExecutor.role,
  namespace_id: null,
  workflow_id: null,
  created_by_user_id: null,
  created_at: '2026-09-21T08:00:00Z',
} as LeaderExecutor

const message = {
  id: 'message-1',
  session_id: session.id,
  author_type: 'user',
  author_user_id: session.user_id,
  author_agent_id: null,
  author_display_name: session.user_display_name,
  body: 'Проверка',
  message_kind: 'user_prompt',
  runtime_message_id: null,
  delivery_state: 'dispatched',
  delivery_error: null,
  replayed: false,
  created_at: '2026-09-21T08:31:00Z',
} as SessionMessage

const runAlpha = {
  id: 'run-1',
  session_id: session.id,
  agent_id: executor.id,
  agent_name: 'Agent Alpha',
  runtime_session_id: 'runtime-session-1',
  runtime_run_id: 'runtime-run-1',
  run_role: 'primary',
  state: 'running',
  last_error: null,
  last_event_at: '2026-09-21T08:31:00Z',
  model: 'model-a',
  provider: 'provider-a',
  model_options: {},
  created_at: '2026-09-21T08:00:00Z',
  updated_at: '2026-09-21T08:31:00Z',
} as SessionAgentRun

const runBeta = {
  ...runAlpha,
  id: 'run-2',
  agent_id: secondExecutor.id,
  agent_name: 'Agent Beta',
  runtime_session_id: 'runtime-session-2',
  runtime_run_id: 'runtime-run-2',
} as SessionAgentRun

const controlResponse = {
  session_id: session.id,
  run_id: runAlpha.id,
  runtime_run_id: runAlpha.runtime_run_id,
  accepted: true,
  state: 'running',
  message: 'accepted',
} as RuntimeRunControlResponse

function renderPage() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[`/sessions/${session.id}`]}>
        <Routes>
          <Route path="/sessions/:sessionId" element={<SessionDetailPage />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  )
}

describe('SessionDetailPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    useAuthStore.setState({ permissions: [] })
    vi.mocked(fleet.getSession).mockResolvedValue(session)
    vi.mocked(fleet.listAgentDirectory).mockResolvedValue([executor, secondExecutor, leader])
    vi.mocked(fleet.listLeaderExecutors).mockResolvedValue([teamExecutor])
    vi.mocked(fleet.listSessionMessages).mockResolvedValue([])
    vi.mocked(fleet.listSessionAgentRuns).mockResolvedValue([])
    vi.mocked(fleet.listSessionParticipants).mockResolvedValue([])
    vi.mocked(fleet.assignSessionLeader).mockResolvedValue(session)
    vi.mocked(fleet.handoffSession).mockResolvedValue(session)
    vi.mocked(fleet.createSessionMessage).mockResolvedValue(message)
    vi.mocked(fleet.createSessionDelegation).mockResolvedValue({
      ...session,
      id: 'delegated-session',
      title: 'Проверка исполнителя',
    })
    vi.mocked(fleet.resolveSessionRunApproval).mockResolvedValue(controlResponse)
    vi.mocked(fleet.steerSessionRun).mockResolvedValue(controlResponse)
    vi.mocked(fleet.stopSessionRun).mockResolvedValue(controlResponse)
  })

  it('shows localized independent empty states', async () => {
    renderPage()

    expect(await screen.findByRole('heading', { name: session.title })).toBeVisible()
    expect(screen.getByText('Сообщений пока нет')).toBeVisible()
    expect(screen.getByText('Запусков среды пока нет')).toBeVisible()
    expect(screen.getByText('Участников пока нет')).toBeVisible()
    expect(screen.getByText('Управление сессией')).toBeVisible()
  })

  it('hides backend details on initial failure and retries the session query', async () => {
    vi.mocked(fleet.getSession)
      .mockRejectedValueOnce(new Error('database connection leaked'))
      .mockResolvedValueOnce(session)
    renderPage()

    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent('Не удалось загрузить сессию')
    expect(screen.queryByText(/database connection leaked/)).not.toBeInTheDocument()
    fireEvent.click(within(alert.parentElement!).getByRole('button', { name: 'Повторить' }))

    expect(await screen.findByRole('heading', { name: session.title })).toBeVisible()
  })

  it('normalizes a message and reuses its idempotency key on retry', async () => {
    vi.mocked(fleet.createSessionMessage)
      .mockRejectedValueOnce(new Error('offline'))
      .mockResolvedValueOnce(message)
    renderPage()

    const body = await screen.findByLabelText('Сообщение')
    fireEvent.change(body, { target: { value: '  Проверка  ' } })
    fireEvent.click(screen.getByRole('button', { name: 'Отправить' }))
    await screen.findByText(/Не удалось отправить сообщение/)
    const firstRequest = vi.mocked(fleet.createSessionMessage).mock.calls[0]?.[1]

    fireEvent.click(screen.getByRole('button', { name: 'Отправить' }))
    await waitFor(() => expect(fleet.createSessionMessage).toHaveBeenCalledTimes(2))
    const retryRequest = vi.mocked(fleet.createSessionMessage).mock.calls[1]?.[1]

    expect(firstRequest).toMatchObject({ body: 'Проверка', author_agent_id: null })
    expect(firstRequest?.idempotency_key).toBeTruthy()
    expect(retryRequest?.idempotency_key).toBe(firstRequest?.idempotency_key)
    expect(toast.success).toHaveBeenCalledWith('Сообщение отправлено')
  })

  it('normalizes delegation fields and reuses the request key on retry', async () => {
    vi.mocked(fleet.createSessionDelegation)
      .mockRejectedValueOnce(new Error('offline'))
      .mockResolvedValueOnce({
        ...session,
        id: 'delegated-session',
        title: 'Проверка исполнителя',
      })
    renderPage()

    const executorSelect = await screen.findByLabelText('Исполнитель')
    await waitFor(() => expect(executorSelect).toHaveValue(secondExecutor.id))
    fireEvent.change(screen.getByLabelText('Название задачи'), {
      target: { value: '  Проверка исполнителя  ' },
    })
    fireEvent.change(screen.getByLabelText('Первая инструкция'), {
      target: { value: '  Собери отчёт  ' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Делегировать' }))
    await screen.findByText(/Не удалось делегировать задачу/)
    const firstRequest = vi.mocked(fleet.createSessionDelegation).mock.calls[0]?.[1]

    fireEvent.click(screen.getByRole('button', { name: 'Делегировать' }))
    await waitFor(() => expect(fleet.createSessionDelegation).toHaveBeenCalledTimes(2))
    const retryRequest = vi.mocked(fleet.createSessionDelegation).mock.calls[1]?.[1]

    expect(firstRequest).toMatchObject({
      executor_agent_id: secondExecutor.id,
      title: 'Проверка исполнителя',
      initial_message: 'Собери отчёт',
    })
    expect(firstRequest?.idempotency_key).toBeTruthy()
    expect(retryRequest?.idempotency_key).toBe(firstRequest?.idempotency_key)
  })

  it('keeps other run controls available while one run action is pending', async () => {
    vi.mocked(fleet.listSessionAgentRuns).mockResolvedValue([runAlpha, runBeta])
    let resolveSteer: (response: RuntimeRunControlResponse) => void = () => undefined
    vi.mocked(fleet.steerSessionRun).mockImplementationOnce(
      () => new Promise((resolve) => (resolveSteer = resolve)),
    )
    renderPage()

    const firstInput = await screen.findByLabelText('Уточнение для Agent Alpha')
    const secondInput = screen.getByLabelText('Уточнение для Agent Beta')
    fireEvent.change(firstInput, { target: { value: '  Continue carefully  ' } })
    fireEvent.click(
      within(firstInput.closest('form')!).getByRole('button', {
        name: 'Направить уточнение для Agent Alpha',
      }),
    )
    await waitFor(() => expect(fleet.steerSessionRun).toHaveBeenCalledTimes(1))

    expect(firstInput).toBeDisabled()
    expect(secondInput).toBeEnabled()
    expect(screen.getByRole('button', { name: 'Остановить запуск Agent Beta' })).toBeEnabled()

    resolveSteer(controlResponse)
    await waitFor(() =>
      expect(toast.success).toHaveBeenCalledWith('Уточнение для Agent Alpha отправлено'),
    )
    expect(fleet.steerSessionRun).toHaveBeenCalledWith(session.id, runAlpha.id, {
      input: 'Continue carefully',
    })
  })

  it('keeps stop confirmation open while pending and allows retry after an error', async () => {
    vi.mocked(fleet.listSessionAgentRuns).mockResolvedValue([runAlpha, runBeta])
    let rejectStop: (error: Error) => void = () => undefined
    vi.mocked(fleet.stopSessionRun)
      .mockImplementationOnce(() => new Promise((_resolve, reject) => (rejectStop = reject)))
      .mockResolvedValueOnce(controlResponse)
    renderPage()

    const stopAlpha = await screen.findByRole('button', {
      name: 'Остановить запуск Agent Alpha',
    })
    fireEvent.click(stopAlpha)
    let dialog = await screen.findByRole('alertdialog')
    expect(dialog).toHaveTextContent('Агент Agent Alpha прекратит текущую работу')

    fireEvent.click(within(dialog).getByRole('button', { name: 'Отмена' }))
    expect(fleet.stopSessionRun).not.toHaveBeenCalled()

    fireEvent.click(stopAlpha)
    dialog = await screen.findByRole('alertdialog')
    fireEvent.click(within(dialog).getByRole('button', { name: 'Остановить запуск' }))
    await waitFor(() => expect(fleet.stopSessionRun).toHaveBeenCalledTimes(1))
    expect(within(dialog).getByRole('button', { name: 'Останавливаем...' })).toBeDisabled()
    expect(within(dialog).getByRole('button', { name: 'Отмена' })).toBeDisabled()

    rejectStop(new Error('offline'))
    expect(
      await within(dialog).findByText('Не удалось запросить остановку запуска. Повторите попытку.'),
    ).toBeVisible()
    fireEvent.click(within(dialog).getByRole('button', { name: 'Повторить остановку' }))

    await waitFor(() => expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument())
    expect(fleet.stopSessionRun).toHaveBeenCalledTimes(2)
    expect(fleet.stopSessionRun).toHaveBeenLastCalledWith(session.id, runAlpha.id)
    expect(toast.success).toHaveBeenCalledWith('Остановка запуска Agent Alpha запрошена')
  })
})
