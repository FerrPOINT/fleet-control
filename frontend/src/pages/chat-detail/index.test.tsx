import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ChatDetailPage } from './index'
import { AuthBoundary } from '@/app/auth-boundary'
import * as fleet from '@/api/fleet'
import * as chats from '@/api/task-chats'
import * as controlLookup from '@/api/runtime-control-lookup'
import type { AgentSession, SessionMessage } from '@/api/types'
import { ssoConfig, useAuthStore } from '@/shared/auth/store'
import { apiBaseUrl } from '@/api/client'
import { controlRecoveryService, readControlRecovery } from '@/shared/chat-control-recovery'
import { ApiError, connectAuthenticatedEventStream } from '@sdlc/ui/lib'
import userEvent from '@testing-library/user-event'

vi.mock('@/api/fleet', () => ({
  getSession: vi.fn(),
  listAgentDirectory: vi.fn(),
  listSessionAgentRuns: vi.fn(),
  listRuntimeControls: vi.fn(),
  createSessionMessage: vi.fn(),
  steerSessionRun: vi.fn(),
  stopSessionRun: vi.fn(),
}))
vi.mock('@/api/runtime-control-lookup', async (original) => ({
  ...(await original<typeof import('@/api/runtime-control-lookup')>()),
  lookupRuntimeControlByDigest: vi.fn(),
  runtimeControlPayloadSha256: vi.fn(async () => 'a'.repeat(64)),
}))
vi.mock('@/api/task-chats', async (original) => ({
  ...(await original<typeof import('@/api/task-chats')>()),
  getTaskContext: vi.fn(),
  getChatControls: vi.fn(),
  getChatHistory: vi.fn(),
  getClarifications: vi.fn(),
  getRequirements: vi.fn(),
  answerClarification: vi.fn(),
  confirmRequirements: vi.fn(),
}))
vi.mock('@sdlc/ui/lib', async (original) => ({
  ...(await original<typeof import('@sdlc/ui/lib')>()),
  connectAuthenticatedEventStream: vi.fn(() => () => {}),
}))
const question: chats.Question = {
  id: 'q1',
  request_id: 'request',
  task_id: 'task',
  root_task_id: 'task',
  assignment_id: 'assignment',
  execution_id: 'execution',
  agent_id: 'agent1',
  assignment_version: 1,
  checkpoint_id: 'checkpoint',
  author_subject: 'pm',
  created_at: '2026-10-01T12:00:00Z',
  version: 1,
  requirement_revision: 3,
  text: 'Кто видит задачи?',
  rationale: 'Определяет доступ',
  required: true,
  mode: 'single',
  options: [
    {
      id: 'project',
      label: 'Участники проекта',
      consequences: 'Только проект',
      is_custom: false,
    },
  ],
  recommended_option_id: 'project',
  requirement_reference: 'REQ-04',
  state: 'open',
  answer: null,
}
const context: chats.TaskContextResponse = {
  binding: {
    tracker_instance_id: 'tracker',
    project_id: 'project',
    task_id: 'task',
    root_task_id: 'task',
    agent_id: 'agent1',
    owner_subject: 'subject-owner',
  },
  tracker: {
    contract_version: 1,
    tracker_instance_id: 'tracker',
    project_id: 'project',
    task_id: 'task',
    root_task_id: 'task',
    owner_subject: 'subject-owner',
    stage: 'Draft',
    requirement_revision: 3,
    waiting_reason: 'Требуется ответ',
    permissions: { can_answer: true, can_confirm: true },
    assignment: null,
  },
}
const revision: chats.RequirementsRevision = {
  revision: 3,
  author_subject: 'pm',
  content_hash: 'hash3',
  created_at: '2026-10-01T12:00:00Z',
  ...{
    goal: 'Настоящие требования',
    scope: ['Портал'],
    exclusions: ['Email'],
    scenarios: ['Создать заявку'],
    acceptance_criteria: ['Автор проверяет'],
    constraints: [],
    dependencies: [],
    assumptions: [],
    checklist: [],
    prerequisites: [],
  },
}
const originalIssuer = ssoConfig.issuer
function renderPage(tab = 'dialogue', queryRetries: false | number = false, isolatedAuth = false) {
  const router = createMemoryRouter(
    [
      { path: '/chats/:sessionId', element: <ChatDetailPage /> },
      { path: '/chats', element: <p>Список</p> },
    ],
    { initialEntries: [`/chats/session1?tab=${tab}`] },
  )
  const client = new QueryClient({
    defaultOptions: { queries: { retry: queryRetries }, mutations: { retry: false } },
  })
  const view = render(
    isolatedAuth ? (
      <AuthBoundary>
        <RouterProvider router={router} />
      </AuthBoundary>
    ) : (
      <QueryClientProvider client={client}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    ),
  )
  return { router, client, ...view }
}
beforeEach(() => {
  vi.clearAllMocks()
  sessionStorage.clear()
  ssoConfig.issuer = originalIssuer
  useAuthStore.setState({ userId: 'owner', token: 'fixture-token', signingOut: false })
  vi.mocked(fleet.getSession).mockResolvedValue({
    id: 'session1',
    user_id: 'owner',
    user_display_name: 'Owner',
    primary_agent_id: 'agent1',
    primary_agent_name: 'PM',
    title: 'Task',
    visibility: 'private',
    task_key: 'TASK-1',
  } as AgentSession)
  vi.mocked(fleet.listAgentDirectory).mockResolvedValue([])
  vi.mocked(fleet.listSessionAgentRuns).mockResolvedValue([])
  vi.mocked(fleet.listRuntimeControls).mockResolvedValue([])
  vi.mocked(controlLookup.lookupRuntimeControlByDigest).mockRejectedValue(
    new ApiError(404, 'Original control not found'),
  )
  vi.mocked(chats.getTaskContext).mockResolvedValue(context)
  vi.mocked(chats.getChatControls).mockResolvedValue({
    can_send: false,
    can_steer: false,
    can_stop: false,
    active_run_id: null,
    blocked_reason: 'workflow_assignment_required',
  })
  vi.mocked(chats.getChatHistory).mockResolvedValue({ items: [], next_before: null })
  vi.mocked(chats.getClarifications).mockResolvedValue({ questions: [question] })
  vi.mocked(chats.getRequirements).mockResolvedValue({ revisions: [revision] })
  vi.mocked(chats.answerClarification).mockResolvedValue({
    id: 'answer',
    question_id: 'q1',
    question_version: 1,
    requirement_revision: 3,
    selected_option_ids: ['project'],
    text: null,
    comment: null,
    author_subject: 'subject-owner',
    created_at: '2026-10-01T12:00:00Z',
  })
  vi.mocked(chats.confirmRequirements).mockResolvedValue({
    id: 'confirmation',
    task_id: 'task',
    revision: 3,
    content_hash: 'hash3',
    owner_subject: 'subject-owner',
    created_at: '2026-10-01T12:00:00Z',
    stage: 'Backlog',
  })
})
describe('production chat', () => {
  it.each(['pending', 'failed'] as const)(
    'does not present absent task facts as authoritative while context is %s',
    async (state) => {
      if (state === 'pending')
        vi.mocked(chats.getTaskContext).mockReturnValue(new Promise(() => {}))
      else
        vi.mocked(chats.getTaskContext).mockRejectedValue(
          new ApiError(503, 'Task context unavailable'),
        )
      renderPage()
      await screen.findByRole('heading', { name: 'Task' })
      if (state === 'failed')
        await waitFor(() =>
          expect(screen.getAllByText('Контекст не обновлён').length).toBeGreaterThan(0),
        )
      expect(screen.queryByText('Пока нет редакции')).not.toBeInTheDocument()
      expect(screen.queryByText('Не назначен')).not.toBeInTheDocument()
      expect(screen.queryByText('Нет привязки к SDLC')).not.toBeInTheDocument()
    },
  )
  it.each(['clarification', 'requirements'])(
    'shows initial context failure rather than an unbound %s workspace',
    async (tab) => {
      vi.mocked(chats.getTaskContext).mockRejectedValue(
        new ApiError(503, 'Task context unavailable'),
      )
      renderPage(tab)
      expect(
        await within(await screen.findByRole('tabpanel')).findByText('Task context unavailable'),
      ).toBeVisible()
      expect(screen.queryByText('Нет привязки к SDLC')).not.toBeInTheDocument()
      expect(screen.queryByText('Свободный чат: уточнения не привязаны')).not.toBeInTheDocument()
      expect(chats.getClarifications).not.toHaveBeenCalled()
      expect(chats.getRequirements).not.toHaveBeenCalled()
    },
  )
  it('marks failed context reads without discarding the answer draft or declaring no assignment', async () => {
    const { client } = renderPage('clarification')
    fireEvent.change(await screen.findByLabelText('Комментарий'), {
      target: { value: 'Keep owner draft' },
    })
    vi.mocked(chats.getTaskContext).mockRejectedValue(new ApiError(503, 'Task context unavailable'))
    await act(async () => {
      await client.invalidateQueries({ queryKey: ['task-context', 'session1'] })
    })
    expect(screen.getByLabelText('Комментарий')).toHaveValue('Keep owner draft')
    await waitFor(() => expect(screen.getByLabelText('Комментарий')).toBeDisabled())
    expect(screen.queryByText('Не назначен')).not.toBeInTheDocument()
    expect(screen.getAllByText('Контекст не обновлён').length).toBeGreaterThan(1)
    vi.mocked(chats.getTaskContext).mockResolvedValue(context)
    await act(async () => {
      await client.invalidateQueries({ queryKey: ['task-context', 'session1'] })
    })
    expect(screen.getByLabelText('Комментарий')).toHaveValue('Keep owner draft')
    await waitFor(() => expect(screen.getByLabelText('Комментарий')).toBeEnabled())
    expect(chats.answerClarification).not.toHaveBeenCalled()
  })
  it.each(['clarification', 'requirements'])(
    'does not reuse a cached unbound %s result after context access is denied',
    async (tab) => {
      vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
      const { client } = renderPage(tab)
      await within(await screen.findByRole('tabpanel')).findByText(
        tab === 'clarification' ? 'Свободный чат: уточнения не привязаны' : 'Нет привязки к SDLC',
      )
      vi.mocked(chats.getTaskContext).mockRejectedValue(new ApiError(403, 'Task context denied'))
      await act(async () => {
        await client.invalidateQueries({ queryKey: ['task-context', 'session1'] })
      })
      expect(
        await within(await screen.findByRole('tabpanel')).findByText('Task context denied'),
      ).toBeVisible()
      expect(screen.queryByText('Свободный чат: уточнения не привязаны')).not.toBeInTheDocument()
      expect(screen.queryByText('Нет привязки к SDLC')).not.toBeInTheDocument()
      expect(screen.getAllByText('Task context denied')).toHaveLength(1)
    },
  )
  it('removes the actual clarification form and cached task after another user signs in', async () => {
    renderPage('clarification', false, true)
    await screen.findByRole('radio', { name: /Участники проекта/ })
    fireEvent.change(screen.getByLabelText('Комментарий'), {
      target: { value: 'Private owner answer' },
    })
    vi.mocked(fleet.getSession).mockRejectedValue(new ApiError(403, 'Forbidden'))
    act(() =>
      useAuthStore
        .getState()
        .setAuth({ token: 'other-token', userId: 'other', email: 'other@example.test' }),
    )
    expect(screen.queryByDisplayValue('Private owner answer')).not.toBeInTheDocument()
    expect(screen.queryByText('TASK-1')).not.toBeInTheDocument()
    expect(await screen.findByRole('alert')).toHaveTextContent('Forbidden')
    expect(chats.answerClarification).not.toHaveBeenCalled()
  })

  it('retains original unknown control metadata when the authorization boundary removes a chat', async () => {
    vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
    vi.mocked(chats.getChatControls).mockResolvedValue({
      can_send: false,
      can_steer: true,
      can_stop: true,
      active_run_id: 'run-original',
    })
    vi.mocked(fleet.steerSessionRun).mockRejectedValueOnce(new Error('Unknown acceptance'))
    renderPage('dialogue', false, true)
    fireEvent.change(await screen.findByLabelText('Уточнение активному запуску'), {
      target: { value: 'Original private guidance' },
    })
    await userEvent.click(screen.getByRole('button', { name: 'Передать уточнение запуску' }))
    await waitFor(() => expect(fleet.steerSessionRun).toHaveBeenCalledTimes(1))
    const original = readControlRecovery('session1')
    expect(original.state).toBe('pending')
    vi.mocked(fleet.getSession).mockRejectedValue(new ApiError(403, 'Forbidden'))
    act(() =>
      useAuthStore
        .getState()
        .setAuth({ token: 'other-token', userId: 'other', email: 'other@example.test' }),
    )
    expect(screen.queryByDisplayValue('Original private guidance')).not.toBeInTheDocument()
    expect(await screen.findByRole('alert')).toHaveTextContent('Forbidden')
    expect(readControlRecovery('session1')).toEqual(original)
    expect(JSON.stringify(original)).not.toContain('Original private guidance')
    expect(JSON.stringify(original)).not.toContain('fixture-token')
    expect(fleet.steerSessionRun).toHaveBeenCalledTimes(1)
  })

  it.each(
    (['answer', 'confirmation'] as const).flatMap((command) =>
      (['logout', 'signing-out', 'another-actor', 'another-service'] as const).map((access) => ({
        command,
        access,
      })),
    ),
  )(
    'holds a late $command ACK after $access without losing the original command',
    async ({ command, access }) => {
      const isAnswer = command === 'answer'
      let acknowledge!: () => void
      if (isAnswer) {
        vi.mocked(chats.answerClarification).mockImplementationOnce(
          () =>
            new Promise((resolve) => {
              acknowledge = () =>
                resolve({
                  id: 'answer',
                  question_id: 'q1',
                  question_version: 1,
                  requirement_revision: 3,
                  selected_option_ids: ['project'],
                  text: null,
                  comment: 'Original private answer',
                  author_subject: 'subject-owner',
                  created_at: '2026-10-01T12:00:00Z',
                })
            }),
        )
      } else {
        vi.mocked(chats.confirmRequirements).mockImplementationOnce(
          () =>
            new Promise((resolve) => {
              acknowledge = () =>
                resolve({
                  id: 'confirmation',
                  task_id: 'task',
                  revision: 3,
                  content_hash: 'hash3',
                  owner_subject: 'subject-owner',
                  created_at: '2026-10-01T12:00:00Z',
                  stage: 'Backlog',
                })
            }),
        )
      }
      const { client } = renderPage(isAnswer ? 'clarification' : 'requirements')
      await userEvent.click(
        isAnswer
          ? await screen.findByRole('radio', { name: /Участники проекта/ })
          : await screen.findByRole('checkbox', { name: /Подтверждаю цель/ }),
      )
      if (isAnswer)
        fireEvent.change(screen.getByLabelText('Комментарий'), {
          target: { value: 'Original private answer' },
        })
      await userEvent.click(
        screen.getByRole('button', {
          name: isAnswer ? 'Сохранить ответ' : 'Подтвердить редакцию 3',
        }),
      )
      const mutation = isAnswer ? chats.answerClarification : chats.confirmRequirements
      await waitFor(() => expect(mutation).toHaveBeenCalledTimes(1))
      await act(async () => {
        if (access === 'logout') useAuthStore.getState().logout()
        else if (access === 'signing-out') useAuthStore.getState().startSignOut()
        else if (access === 'another-service') {
          ssoConfig.issuer = 'https://another-auth.example.test'
          useAuthStore.setState({ token: 'another-service-token' })
        } else
          useAuthStore.setState({
            userId: 'operator',
            token: 'other-token',
            permissions: ['sessions:read_all'],
          })
        acknowledge()
      })
      const retryName = isAnswer ? 'Повторить исходный ответ' : 'Повторить исходное подтверждение'
      await waitFor(() => expect(screen.getByRole('button', { name: retryName })).toBeDisabled())
      expect(
        screen.queryByText('Подтверждение сохранено. Следующее назначение проверяется отдельно.'),
      ).not.toBeInTheDocument()
      if (isAnswer)
        expect(screen.getByLabelText('Комментарий')).toHaveValue('Original private answer')
      expect(mutation).toHaveBeenCalledTimes(1)
      await act(async () => {
        ssoConfig.issuer = originalIssuer
        useAuthStore.setState({ userId: 'owner', token: 'returned-token', signingOut: false })
        await client.invalidateQueries({ queryKey: ['session', 'session1'] })
      })
      await waitFor(() => expect(screen.getByRole('button', { name: retryName })).toBeEnabled())
      await userEvent.click(screen.getByRole('button', { name: retryName }))
      await waitFor(() => expect(mutation).toHaveBeenCalledTimes(2))
      expect(vi.mocked(mutation).mock.calls[1]).toEqual(vi.mocked(mutation).mock.calls[0])
    },
  )
  it.each(
    (['steer', 'stop'] as const).flatMap((operation) =>
      (
        [
          'ack',
          '404',
          '409',
          'uncertain',
          'terminal-no-ack',
          'wrong-run',
          'wrong-agent',
          'wrong-actor',
          'wrong-operation',
          'other-user',
          'other-service',
          'signing-out',
          'logout',
          'session401',
          'session403',
        ] as const
      ).map((outcome) => ({ operation, outcome })),
    ),
  )('restores held $operation metadata after remount: $outcome', async ({ operation, outcome }) => {
    vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
    vi.mocked(chats.getChatControls).mockResolvedValue({
      can_send: false,
      can_steer: true,
      can_stop: true,
      active_run_id: 'run-original',
    })
    const post = operation === 'steer' ? fleet.steerSessionRun : fleet.stopSessionRun
    vi.mocked(post).mockRejectedValueOnce(new Error('Lost original acknowledgement'))
    const first = renderPage()
    const input = await screen.findByLabelText('Уточнение активному запуску')
    fireEvent.change(input, { target: { value: 'Private guidance stays in memory' } })
    const actionName = operation === 'steer' ? 'Передать уточнение запуску' : 'Остановить запуск'
    await waitFor(() => expect(screen.getByRole('button', { name: actionName })).toBeEnabled())
    fireEvent.click(screen.getByRole('button', { name: actionName }))
    await waitFor(() => expect(post).toHaveBeenCalledTimes(1))
    await waitFor(() => expect(first.client.isMutating()).toBe(0))
    const pending = readControlRecovery('session1')
    expect(pending.state).toBe('pending')
    if (pending.state !== 'pending') throw new Error('Expected original metadata')
    const original = pending.command
    expect(JSON.stringify(original)).not.toContain('Private guidance')
    first.unmount()
    first.client.clear()
    vi.mocked(controlLookup.lookupRuntimeControlByDigest).mockClear()
    vi.mocked(chats.getChatControls).mockResolvedValue({
      can_send: false,
      can_steer: true,
      can_stop: true,
      active_run_id: 'run-successor',
    })
    const ack = {
      id: 'original-command',
      session_id: 'session1',
      session_run_id: 'run-original',
      agent_id: 'agent1',
      actor_user_id: 'owner',
      operation,
      state: 'acknowledged' as const,
      acknowledgement: operation === 'steer' ? 'steered' : 'stopping',
      observed_run_state: null,
      created_at: '2026-10-01T12:00:00Z',
      updated_at: '2026-10-01T12:00:00Z',
    }
    if (outcome === '404' || outcome === '409')
      vi.mocked(controlLookup.lookupRuntimeControlByDigest).mockRejectedValue(
        new ApiError(Number(outcome), 'Readback held'),
      )
    else
      vi.mocked(controlLookup.lookupRuntimeControlByDigest).mockResolvedValue({
        ...ack,
        session_run_id: outcome === 'wrong-run' ? 'run-successor' : ack.session_run_id,
        actor_user_id: outcome === 'wrong-actor' ? 'other' : ack.actor_user_id,
        agent_id: outcome === 'wrong-agent' ? 'another-agent' : ack.agent_id,
        operation:
          outcome === 'wrong-operation' ? (operation === 'steer' ? 'stop' : 'steer') : operation,
        state:
          outcome === 'uncertain'
            ? 'uncertain'
            : outcome === 'terminal-no-ack'
              ? 'terminal_observed'
              : 'acknowledged',
        acknowledgement: outcome === 'terminal-no-ack' ? null : ack.acknowledgement,
      })
    if (outcome === 'other-user') useAuthStore.setState({ userId: 'other' })
    if (outcome === 'logout') useAuthStore.getState().logout()
    if (outcome === 'signing-out') useAuthStore.getState().startSignOut()
    if (outcome === 'other-service')
      sessionStorage.setItem(
        'fleet-control.control-recovery.v1:session1',
        JSON.stringify({ ...original, service: 'another-service' }),
      )
    if (outcome === 'session401' || outcome === 'session403')
      vi.mocked(fleet.getSession).mockRejectedValue(
        new ApiError(Number(outcome.slice(7)), 'Session denied after reload'),
      )
    const restored = renderPage()
    if (outcome.startsWith('session')) {
      await screen.findByText('Session denied after reload')
      expect(screen.queryByRole('button', { name: actionName })).not.toBeInTheDocument()
    } else {
      const nextInput = await screen.findByLabelText('Уточнение активному запуску')
      expect(nextInput).toHaveValue('')
      if (outcome === 'ack') {
        await screen.findByText(
          operation === 'steer'
            ? 'Runtime подтвердил исходное уточнение. Состояние запуска проверяется отдельно.'
            : 'Runtime подтвердил исходную остановку. Завершение запуска проверяется отдельно.',
        )
        await waitFor(() => expect(nextInput).toBeEnabled())
        expect(readControlRecovery('session1')).toEqual({ state: 'none' })
      } else {
        await waitFor(() => expect(restored.client.isFetching()).toBe(0))
        expect(nextInput).toBeDisabled()
        expect(screen.getByRole('button', { name: actionName })).toBeDisabled()
        expect(readControlRecovery('session1').state).toBe('pending')
      }
    }
    if (
      ['other-user', 'other-service', 'signing-out', 'logout', 'session401', 'session403'].includes(
        outcome,
      )
    )
      expect(controlLookup.lookupRuntimeControlByDigest).not.toHaveBeenCalled()
    else
      expect(controlLookup.lookupRuntimeControlByDigest).toHaveBeenCalledWith(
        'session1',
        'run-original',
        original.key,
        { operation, payload_sha256: original.payloadSha256 },
      )
    expect(post).toHaveBeenCalledTimes(1)
    expect(fleet.createSessionMessage).not.toHaveBeenCalled()
  })
  it('does not POST if the tab cannot retain reconciliation metadata', async () => {
    vi.mocked(chats.getChatControls).mockResolvedValue({
      can_send: false,
      can_steer: true,
      can_stop: true,
      active_run_id: 'run-original',
    })
    renderPage()
    const input = await screen.findByLabelText('Уточнение активному запуску')
    fireEvent.change(input, { target: { value: 'Must not be sent' } })
    const action = screen.getByRole('button', { name: 'Передать уточнение запуску' })
    await waitFor(() => expect(action).toBeEnabled())
    const broken = vi
      .spyOn(Object.getPrototypeOf(sessionStorage), 'setItem')
      .mockImplementation(() => {})
    fireEvent.click(action)
    await screen.findByText('Не удалось сохранить ключ сверки. Команда не отправлена.')
    expect(fleet.steerSessionRun).not.toHaveBeenCalled()
    expect(fleet.stopSessionRun).not.toHaveBeenCalled()
    broken.mockRestore()
  })

  it.each(
    (['steer', 'stop'] as const).flatMap((operation) =>
      (['acknowledged', 'terminal-ack', 'run-changed'] as const).map((outcome) => ({
        operation,
        outcome,
      })),
    ),
  )(
    'recovers lost initial $operation ID only from fresh original-key $outcome',
    async ({ operation, outcome }) => {
      vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
      vi.mocked(chats.getChatControls).mockResolvedValue({
        can_send: false,
        can_steer: true,
        can_stop: true,
        active_run_id: 'run-original',
      })
      const post = operation === 'steer' ? fleet.steerSessionRun : fleet.stopSessionRun
      vi.mocked(post).mockRejectedValueOnce(new TypeError('Lost initial command ID'))
      const { client } = renderPage()
      const action = await screen.findByRole('button', {
        name: operation === 'steer' ? 'Передать уточнение запуску' : 'Остановить запуск',
      })
      const input = screen.getByLabelText('Уточнение активному запуску')
      if (operation === 'steer')
        fireEvent.change(input, { target: { value: ' Original scoped guidance ' } })
      await waitFor(() => expect(action).toBeEnabled())
      fireEvent.click(action)
      await waitFor(() =>
        expect(controlLookup.lookupRuntimeControlByDigest).toHaveBeenCalledTimes(1),
      )
      await waitFor(() => expect(client.isMutating()).toBe(0))
      expect(action).toBeDisabled()
      if (operation === 'steer') expect(input).toHaveValue(' Original scoped guidance ')
      expect(post).toHaveBeenCalledTimes(1)
      const originalLookup = vi.mocked(controlLookup.lookupRuntimeControlByDigest).mock.calls[0]!
      expect(originalLookup).toEqual([
        'session1',
        'run-original',
        expect.any(String),
        { operation, payload_sha256: 'a'.repeat(64) },
      ])
      vi.mocked(controlLookup.lookupRuntimeControlByDigest).mockResolvedValue({
        id: 'historical-command',
        session_id: 'session1',
        session_run_id: 'run-original',
        agent_id: 'agent1',
        actor_user_id: 'owner',
        operation,
        state: outcome === 'terminal-ack' ? 'terminal_observed' : 'acknowledged',
        acknowledgement: operation === 'steer' ? 'steered' : 'stopping',
        observed_run_state: null,
        created_at: '2026-10-01T12:00:00Z',
        updated_at: '2026-10-01T12:00:00Z',
      })
      if (outcome === 'run-changed')
        await act(async () => {
          client.setQueryData(['chat-controls', 'session1'], {
            can_send: false,
            can_steer: true,
            can_stop: true,
            active_run_id: 'run-new',
          })
        })
      fireEvent.click(
        screen.getByRole('button', {
          name:
            operation === 'steer' ? 'Проверить исходное уточнение' : 'Проверить исходную остановку',
        }),
      )
      await waitFor(() =>
        expect(controlLookup.lookupRuntimeControlByDigest).toHaveBeenCalledTimes(2),
      )
      await screen.findByText(
        operation === 'steer'
          ? 'Runtime подтвердил исходное уточнение. Состояние запуска проверяется отдельно.'
          : 'Runtime подтвердил исходную остановку. Завершение запуска проверяется отдельно.',
      )
      if (operation === 'steer') expect(input).toHaveValue('')
      expect(screen.queryByText('Lost initial command ID')).not.toBeInTheDocument()
      expect(vi.mocked(controlLookup.lookupRuntimeControlByDigest).mock.calls[1]).toEqual(
        originalLookup,
      )
      expect(post).toHaveBeenCalledTimes(1)
      expect(fleet.createSessionMessage).not.toHaveBeenCalled()
    },
  )
  it.each(
    (['steer', 'stop'] as const).flatMap((operation) =>
      (
        [
          '404',
          '409',
          'reserved',
          'submitted',
          'uncertain',
          'terminal-unconfirmed',
          'wrong-session',
          'wrong-run',
          'wrong-agent',
          'wrong-actor',
          'wrong-operation',
        ] as const
      ).map((outcome) => ({ operation, outcome })),
    ),
  )(
    'holds lost $operation after original-key $outcome without another POST',
    async ({ operation, outcome }) => {
      vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
      vi.mocked(chats.getChatControls).mockResolvedValue({
        can_send: false,
        can_steer: true,
        can_stop: true,
        active_run_id: 'run-original',
      })
      const post = operation === 'steer' ? fleet.steerSessionRun : fleet.stopSessionRun
      vi.mocked(post).mockRejectedValueOnce(new Error('Lost initial control ACK'))
      const { client } = renderPage()
      const input = await screen.findByLabelText('Уточнение активному запуску')
      const action = screen.getByRole('button', {
        name: operation === 'steer' ? 'Передать уточнение запуску' : 'Остановить запуск',
      })
      if (operation === 'steer')
        fireEvent.change(input, { target: { value: 'Original retained input' } })
      await waitFor(() => expect(action).toBeEnabled())
      fireEvent.click(action)
      await waitFor(() =>
        expect(controlLookup.lookupRuntimeControlByDigest).toHaveBeenCalledTimes(1),
      )
      await waitFor(() => expect(client.isMutating()).toBe(0))
      const original = vi.mocked(controlLookup.lookupRuntimeControlByDigest).mock.calls[0]!
      if (outcome === '404' || outcome === '409')
        vi.mocked(controlLookup.lookupRuntimeControlByDigest).mockRejectedValue(
          new ApiError(Number(outcome), `Lookup ${outcome}`),
        )
      else
        vi.mocked(controlLookup.lookupRuntimeControlByDigest).mockResolvedValue({
          id: 'historical-command',
          session_id: outcome === 'wrong-session' ? 'another-session' : 'session1',
          session_run_id: outcome === 'wrong-run' ? 'another-run' : 'run-original',
          agent_id: outcome === 'wrong-agent' ? 'another-agent' : 'agent1',
          actor_user_id: outcome === 'wrong-actor' ? 'another-actor' : 'owner',
          operation:
            outcome === 'wrong-operation' ? (operation === 'steer' ? 'stop' : 'steer') : operation,
          state:
            outcome === 'reserved' || outcome === 'submitted' || outcome === 'uncertain'
              ? outcome
              : outcome === 'terminal-unconfirmed'
                ? 'terminal_observed'
                : 'acknowledged',
          acknowledgement:
            outcome === 'terminal-unconfirmed'
              ? null
              : operation === 'steer'
                ? 'steered'
                : 'stopping',
          observed_run_state: null,
          created_at: '2026-10-01T12:00:00Z',
          updated_at: '2026-10-01T12:00:00Z',
        })
      const check = screen.getByRole('button', {
        name:
          operation === 'steer' ? 'Проверить исходное уточнение' : 'Проверить исходную остановку',
      })
      await waitFor(() => expect(check).toBeEnabled())
      fireEvent.click(check)
      await waitFor(() =>
        expect(controlLookup.lookupRuntimeControlByDigest).toHaveBeenCalledTimes(2),
      )
      await waitFor(() =>
        expect(client.isFetching({ queryKey: ['runtime-control-lookup'] })).toBe(0),
      )
      expect(vi.mocked(controlLookup.lookupRuntimeControlByDigest).mock.calls[1]).toEqual(original)
      expect(action).toBeDisabled()
      expect(input).toBeDisabled()
      if (operation === 'steer') expect(input).toHaveValue('Original retained input')
      fireEvent.click(action)
      expect(post).toHaveBeenCalledTimes(1)
      expect(fleet.createSessionMessage).not.toHaveBeenCalled()
    },
  )
  it.each(['steer', 'stop'] as const)(
    'holds cached $operation ACK during failed GET and restores only a fresh receipt',
    async (operation) => {
      vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
      vi.mocked(chats.getChatControls).mockResolvedValue({
        can_send: false,
        can_steer: true,
        can_stop: true,
        active_run_id: 'run-original',
      })
      const post = operation === 'steer' ? fleet.steerSessionRun : fleet.stopSessionRun
      vi.mocked(post).mockRejectedValueOnce(new Error('Lost control ACK'))
      const { client } = renderPage()
      const input = await screen.findByLabelText('Уточнение активному запуску')
      const action = screen.getByRole('button', {
        name: operation === 'steer' ? 'Передать уточнение запуску' : 'Остановить запуск',
      })
      if (operation === 'steer')
        fireEvent.change(input, { target: { value: 'Cached ACK must stay held' } })
      await waitFor(() => expect(action).toBeEnabled())
      fireEvent.click(action)
      await waitFor(() =>
        expect(controlLookup.lookupRuntimeControlByDigest).toHaveBeenCalledTimes(1),
      )
      await waitFor(() => expect(client.isMutating()).toBe(0))
      const original = vi.mocked(controlLookup.lookupRuntimeControlByDigest).mock.calls[0]!
      const queryKey = [
        'runtime-control-lookup',
        'session1',
        controlRecoveryService(apiBaseUrl, ssoConfig.issuer),
        'owner',
        'run-original',
        original[2],
      ]
      const ack = {
        id: 'original-command',
        session_id: 'session1',
        session_run_id: 'run-original',
        agent_id: 'agent1',
        actor_user_id: 'owner',
        operation,
        state: 'acknowledged' as const,
        acknowledgement: operation === 'steer' ? 'steered' : 'stopping',
        observed_run_state: null,
        created_at: '2026-10-01T12:00:00Z',
        updated_at: '2026-10-01T12:00:00Z',
      }
      let rejectRead!: (reason: unknown) => void
      vi.mocked(controlLookup.lookupRuntimeControlByDigest).mockReturnValueOnce(
        new Promise((_resolve, reject) => {
          rejectRead = reject
        }),
      )
      let refresh!: Promise<void>
      await act(async () => {
        client.setQueryData(queryKey, ack)
        refresh = client.invalidateQueries({ queryKey })
      })
      expect(client.getQueryState(queryKey)?.status).toBe('success')
      expect(client.getQueryState(queryKey)?.fetchStatus).toBe('fetching')
      expect(action).toBeDisabled()
      if (operation === 'steer') expect(input).toHaveValue('Cached ACK must stay held')
      await act(async () => {
        rejectRead(new ApiError(503, 'Lookup read failed'))
        await refresh
      })
      expect(client.getQueryState(queryKey)?.data).toEqual(ack)
      expect(action).toBeDisabled()
      vi.mocked(controlLookup.lookupRuntimeControlByDigest).mockResolvedValue(ack)
      fireEvent.click(
        screen.getByRole('button', {
          name:
            operation === 'steer' ? 'Проверить исходное уточнение' : 'Проверить исходную остановку',
        }),
      )
      await screen.findByText(
        operation === 'steer'
          ? 'Runtime подтвердил исходное уточнение. Состояние запуска проверяется отдельно.'
          : 'Runtime подтвердил исходную остановку. Завершение запуска проверяется отдельно.',
      )
      expect(post).toHaveBeenCalledTimes(1)
      expect(fleet.createSessionMessage).not.toHaveBeenCalled()
    },
  )
  it.each(
    (['steer', 'stop'] as const).flatMap((operation) =>
      (['malformed-json', 'incomplete-body'] as const).map((response) => ({ operation, response })),
    ),
  )('holds $operation when its 2xx reply is $response', async ({ operation, response }) => {
    vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
    vi.mocked(chats.getChatControls).mockResolvedValue({
      can_send: false,
      can_steer: true,
      can_stop: true,
      active_run_id: 'run-original',
    })
    const post = operation === 'steer' ? fleet.steerSessionRun : fleet.stopSessionRun
    if (response === 'malformed-json')
      vi.mocked(post).mockRejectedValueOnce(new ApiError(200, 'invalid JSON response'))
    else vi.mocked(post).mockResolvedValueOnce({} as Awaited<ReturnType<typeof post>>)
    renderPage()
    const input = await screen.findByLabelText('Уточнение активному запуску')
    const action = screen.getByRole('button', {
      name: operation === 'steer' ? 'Передать уточнение запуску' : 'Остановить запуск',
    })
    if (operation === 'steer')
      fireEvent.change(input, { target: { value: 'Original input survives invalid reply' } })
    await waitFor(() => expect(action).toBeEnabled())
    fireEvent.click(action)
    await waitFor(() => expect(controlLookup.lookupRuntimeControlByDigest).toHaveBeenCalledTimes(1))
    expect(action).toBeDisabled()
    if (operation === 'steer') expect(input).toHaveValue('Original input survives invalid reply')
    expect(post).toHaveBeenCalledTimes(1)
    expect(fleet.createSessionMessage).not.toHaveBeenCalled()
  })
  it.each(['reconnect', 'inactive delta', 'active delta'] as const)(
    'reports unread chat content only when displayed content changes: %s',
    async (event) => {
      useAuthStore.setState({ token: 'fixture-token' })
      vi.mocked(chats.getChatHistory).mockResolvedValue({
        items: [message('reading', 'Retained message')],
        next_before: null,
      })
      vi.mocked(chats.getChatControls).mockResolvedValue({
        can_send: false,
        can_steer: true,
        can_stop: true,
        active_run_id: 'run-active',
      })
      vi.mocked(fleet.listSessionAgentRuns).mockResolvedValue([
        {
          id: 'run-active',
          session_id: 'session1',
          agent_id: 'agent1',
          agent_name: 'PM',
          runtime_session_id: 'native-session',
          runtime_run_id: 'native-run',
          run_role: 'primary',
          state: 'running',
          last_error: null,
          last_event_at: null,
          model: null,
          provider: null,
          model_options: {},
          created_at: '2026-10-01T12:00:00Z',
          updated_at: '2026-10-01T12:00:00Z',
        },
      ])
      renderPage()
      await screen.findByText('Retained message')
      const pane = document.querySelector<HTMLDivElement>('.fc-chat-panel .fc-chat-scroll')!
      Object.defineProperties(pane, { scrollHeight: { value: 1200 }, clientHeight: { value: 400 } })
      pane.scrollTop = 240
      fireEvent.scroll(pane)
      await act(async () => {
        const stream = vi.mocked(connectAuthenticatedEventStream).mock.calls[0]![0]
        if (event === 'reconnect') stream.onOpen?.()
        else
          stream.onEvent('session', {
            type: 'session_run_delta',
            run_id: event === 'active delta' ? 'run-active' : 'run-old',
            text: 'New streamed content',
          })
      })
      if (event === 'active delta') {
        expect(await screen.findByRole('button', { name: 'Новые сообщения' })).toBeVisible()
        expect(screen.getByText('New streamed content')).toBeVisible()
      } else {
        expect(screen.queryByRole('button', { name: 'Новые сообщения' })).not.toBeInTheDocument()
        expect(screen.queryByText('New streamed content')).not.toBeInTheDocument()
      }
      expect(pane.scrollTop).toBe(240)
    },
  )
  it.each([
    'acknowledged',
    'terminal-ack',
    'run-changed',
    'wrong-id',
    'wrong-run',
    'wrong-actor',
    'terminal-unconfirmed',
  ] as const)('reconciles only the known original steer receipt: %s', async (outcome) => {
    vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
    vi.mocked(chats.getChatControls).mockResolvedValue({
      can_send: false,
      can_steer: true,
      can_stop: true,
      active_run_id: 'run-original',
    })
    const receipt = {
      id: 'original-command',
      actor_user_id: 'owner',
      agent_id: 'agent1',
      session_id: 'session1',
      session_run_id: 'run-original',
      operation: 'steer' as const,
      state: 'uncertain' as const,
      acknowledgement: null,
      observed_run_state: null,
      created_at: '2026-10-01T12:00:00Z',
      updated_at: '2026-10-01T12:00:00Z',
    }
    let submitted = false
    vi.mocked(fleet.listRuntimeControls).mockImplementation(async () =>
      submitted ? [receipt] : [],
    )
    vi.mocked(fleet.steerSessionRun).mockImplementationOnce(async () => {
      submitted = true
      return {
        session_id: 'session1',
        run_id: 'run-original',
        runtime_run_id: 'native-original',
        state: 'waiting',
        accepted: false,
        message: 'Unknown command outcome',
        command: receipt,
      }
    })
    const { client } = renderPage()
    const input = await screen.findByLabelText('Уточнение активному запуску')
    fireEvent.change(input, { target: { value: 'Original scoped guidance' } })
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Передать уточнение запуску' })).toBeEnabled(),
    )
    fireEvent.click(screen.getByRole('button', { name: 'Передать уточнение запуску' }))
    await screen.findByText(/Исход уточнения запуску неизвестен/)
    await waitFor(() => expect(client.isMutating()).toBe(0))
    expect(input).toBeDisabled()
    const fresh = {
      ...receipt,
      state: outcome.startsWith('terminal')
        ? ('terminal_observed' as const)
        : ('acknowledged' as const),
      acknowledgement: outcome === 'terminal-unconfirmed' ? null : 'steered',
      id: outcome === 'wrong-id' ? 'another-command' : receipt.id,
      session_run_id: outcome === 'wrong-run' ? 'another-run' : receipt.session_run_id,
      actor_user_id: outcome === 'wrong-actor' ? 'another-owner' : receipt.actor_user_id,
    }
    vi.mocked(fleet.listRuntimeControls).mockImplementation(async (_sessionId, runId) =>
      runId === 'run-original' ? [fresh] : [],
    )
    if (outcome === 'run-changed') {
      await act(async () => {
        client.setQueryData(['chat-controls', 'session1'], {
          can_send: false,
          can_steer: true,
          can_stop: true,
          active_run_id: 'run-new',
        })
      })
    }
    await act(async () => {
      await client.invalidateQueries({ queryKey: ['runtime-controls', 'session1', 'run-original'] })
    })
    if (outcome === 'acknowledged' || outcome === 'terminal-ack' || outcome === 'run-changed') {
      await waitFor(() => expect(input).toHaveValue(''))
      expect(input).toBeEnabled()
      fireEvent.change(input, { target: { value: 'New explicit guidance' } })
      expect(screen.getByRole('button', { name: 'Передать уточнение запуску' })).toBeEnabled()
    } else {
      expect(input).toHaveValue('Original scoped guidance')
      expect(input).toBeDisabled()
      expect(screen.getByRole('button', { name: 'Передать уточнение запуску' })).toBeDisabled()
    }
    expect(fleet.steerSessionRun).toHaveBeenCalledTimes(1)
    expect(fleet.createSessionMessage).not.toHaveBeenCalled()
  })
  it.each(['reconnect', 'session event'] as const)(
    'refreshes session access on %s and hides the workspace after denial',
    async (event) => {
      const dispose = vi.fn()
      vi.mocked(connectAuthenticatedEventStream).mockReturnValueOnce(dispose)
      useAuthStore.setState({ token: 'fixture-token' })
      renderPage('clarification')
      await screen.findByRole('radio', { name: /Участники проекта/ })
      vi.mocked(fleet.getSession).mockRejectedValue(new ApiError(403, 'Session access revoked'))
      await act(async () => {
        const stream = vi.mocked(connectAuthenticatedEventStream).mock.calls[0]![0]
        if (event === 'reconnect') stream.onOpen?.()
        else stream.onEvent('session', { type: 'session_changed' })
      })
      await screen.findByText('Session access revoked')
      await waitFor(() => expect(dispose).toHaveBeenCalledTimes(1))
      expect(screen.queryByRole('tab', { name: /Уточнения/ })).not.toBeInTheDocument()
      expect(screen.queryByRole('heading', { name: 'Task' })).not.toBeInTheDocument()
      expect(chats.answerClarification).not.toHaveBeenCalled()
      expect(chats.confirmRequirements).not.toHaveBeenCalled()
    },
  )
  it('recovers session read access through GET without losing the answer draft', async () => {
    const { client } = renderPage('clarification')
    fireEvent.click(await screen.findByRole('radio', { name: /Участники проекта/ }))
    fireEvent.change(screen.getByLabelText('Комментарий'), {
      target: { value: 'Preserve this draft' },
    })
    const original = client.getQueryData<AgentSession>(['session', 'session1'])!
    vi.mocked(fleet.getSession).mockRejectedValue(new ApiError(403, 'Session access revoked'))
    await act(async () => {
      await client.invalidateQueries({ queryKey: ['session', 'session1'] })
    })
    await screen.findByText('Session access revoked')
    vi.mocked(fleet.getSession).mockResolvedValue(original)
    fireEvent.click(screen.getByRole('button', { name: 'Проверить доступ к чату' }))
    expect(await screen.findByRole('radio', { name: /Участники проекта/ })).toBeChecked()
    expect(screen.getByLabelText('Комментарий')).toHaveValue('Preserve this draft')
    expect(chats.answerClarification).not.toHaveBeenCalled()
    expect(chats.confirmRequirements).not.toHaveBeenCalled()
    expect(fleet.getSession).toHaveBeenCalledTimes(3)
  })
  it('refreshes session metadata on a session event', async () => {
    useAuthStore.setState({ token: 'fixture-token' })
    renderPage('clarification')
    await screen.findByRole('heading', { name: 'Task' })
    const original = await vi.mocked(fleet.getSession).mock.results[0]!.value
    vi.mocked(fleet.getSession).mockResolvedValue({ ...original, title: 'Renamed task' })
    await act(async () => {
      vi.mocked(connectAuthenticatedEventStream).mock.calls[0]![0].onEvent('session', {
        type: 'session_changed',
      })
    })
    await screen.findByRole('heading', { name: 'Renamed task' })
    expect(fleet.getSession).toHaveBeenCalledTimes(2)
  })
  it.each(['prompt', 'answer', 'confirmation'] as const)(
    'holds owner-only %s during a failed session read retry',
    async (command) => {
      if (command === 'prompt') {
        vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
        vi.mocked(chats.getChatControls).mockResolvedValue({
          can_send: true,
          can_steer: false,
          can_stop: false,
          active_run_id: null,
        })
      }
      const { client } = renderPage(
        command === 'prompt' ? 'dialogue' : command === 'answer' ? 'clarification' : 'requirements',
        1,
      )
      if (command === 'prompt')
        fireEvent.change(await screen.findByLabelText('Сообщение агенту'), {
          target: { value: 'Retained prompt' },
        })
      else
        await userEvent.click(
          command === 'answer'
            ? await screen.findByRole('radio', { name: /Участники проекта/ })
            : await screen.findByRole('checkbox', { name: /Подтверждаю цель/ }),
        )
      const button = screen.getByRole('button', {
        name:
          command === 'prompt'
            ? 'Отправить сообщение'
            : command === 'answer'
              ? 'Сохранить ответ'
              : 'Подтвердить редакцию 3',
      })
      await waitFor(() => expect(button).toBeEnabled())
      const original = client.getQueryData<AgentSession>(['session', 'session1'])!
      let resolveRetry!: () => void
      vi.mocked(fleet.getSession)
        .mockRejectedValueOnce(new ApiError(503, 'Session retry pending'))
        .mockImplementationOnce(async () => {
          await new Promise<void>((resolve) => {
            resolveRetry = resolve
          })
          return original
        })
      let refresh!: Promise<unknown>
      await act(async () => {
        refresh = client.invalidateQueries({ queryKey: ['session', 'session1'] })
      })
      await waitFor(() => expect(resolveRetry).toBeTypeOf('function'), { timeout: 3000 })
      expect(client.getQueryState(['session', 'session1'])?.status).toBe('success')
      expect(button).toBeDisabled()
      fireEvent.click(button)
      expect(fleet.createSessionMessage).not.toHaveBeenCalled()
      expect(chats.answerClarification).not.toHaveBeenCalled()
      expect(chats.confirmRequirements).not.toHaveBeenCalled()
      await act(async () => {
        resolveRetry()
        await refresh
      })
      await waitFor(() => expect(button).toBeEnabled())
      if (command === 'prompt')
        expect(screen.getByLabelText('Сообщение агенту')).toHaveValue('Retained prompt')
      else
        expect(
          screen.getByRole(command === 'answer' ? 'radio' : 'checkbox', {
            name: command === 'answer' ? /Участники проекта/ : /Подтверждаю цель/,
          }),
        ).toBeChecked()
    },
  )
  it.each([
    ['prompt', 'controls'],
    ['steer', 'controls'],
    ['steer', 'commands'],
    ['stop', 'commands'],
  ] as const)('holds %s during failed %s read retry', async (command, source) => {
    vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
    const available = {
      can_send: command === 'prompt',
      can_steer: command === 'steer',
      can_stop: command === 'stop',
      active_run_id: command === 'prompt' ? null : 'run-original',
      blocked_reason: null,
    }
    vi.mocked(chats.getChatControls).mockResolvedValue(available)
    const { client } = renderPage('dialogue', 1)
    if (command !== 'stop')
      await userEvent.type(
        await screen.findByLabelText(
          command === 'steer' ? 'Уточнение активному запуску' : 'Сообщение агенту',
        ),
        'Keep original draft',
      )
    const button = await screen.findByRole('button', {
      name:
        command === 'stop'
          ? 'Остановить запуск'
          : command === 'steer'
            ? 'Передать уточнение запуску'
            : 'Отправить сообщение',
    })
    await waitFor(() => expect(button).toBeEnabled())
    let resolveRetry!: () => void
    function holdRead<T>(read: (...args: string[]) => Promise<T>, fresh: T) {
      vi.mocked(read)
        .mockRejectedValueOnce(new ApiError(503, 'Read retry pending'))
        .mockImplementationOnce(async () => {
          await new Promise<void>((resolve) => {
            resolveRetry = resolve
          })
          return fresh
        })
    }
    if (source === 'controls') holdRead(chats.getChatControls, available)
    else holdRead(fleet.listRuntimeControls, [])
    const key =
      source === 'controls'
        ? ['chat-controls', 'session1']
        : ['runtime-controls', 'session1', 'run-original']
    let refresh!: Promise<unknown>
    await act(async () => {
      refresh = client.invalidateQueries({ queryKey: key })
    })
    await waitFor(() => expect(resolveRetry).toBeTypeOf('function'), { timeout: 3000 })
    expect(client.getQueryState(key)?.status).toBe('success')
    expect(button).toBeDisabled()
    fireEvent.click(button)
    expect(fleet.createSessionMessage).not.toHaveBeenCalled()
    expect(fleet.steerSessionRun).not.toHaveBeenCalled()
    expect(fleet.stopSessionRun).not.toHaveBeenCalled()
    await act(async () => {
      resolveRetry()
      await refresh
    })
    await waitFor(() => expect(button).toBeEnabled())
    if (command !== 'stop')
      expect(
        screen.getByLabelText(
          command === 'steer' ? 'Уточнение активному запуску' : 'Сообщение агенту',
        ),
      ).toHaveValue('Keep original draft')
  })
  it.each([
    ['answer', 'task-context'],
    ['answer', 'clarifications'],
    ['confirmation', 'task-context'],
    ['confirmation', 'requirements'],
  ] as const)('holds %s while the failed %s read is retrying', async (command, source) => {
    const isAnswer = command === 'answer'
    const { client } = renderPage(isAnswer ? 'clarification' : 'requirements', 1)
    await userEvent.click(
      isAnswer
        ? await screen.findByRole('radio', { name: /Участники проекта/ })
        : await screen.findByRole('checkbox', { name: /Подтверждаю цель/ }),
    )
    const submit = screen.getByRole('button', {
      name: isAnswer ? 'Сохранить ответ' : 'Подтвердить редакцию 3',
    })
    await waitFor(() => expect(submit).toBeEnabled())
    let resolveRetry!: () => void
    function holdRead<T>(read: (id: string) => Promise<T>, fresh: T) {
      vi.mocked(read)
        .mockRejectedValueOnce(new ApiError(503, 'Authority refresh unavailable'))
        .mockImplementationOnce(async () => {
          await new Promise<void>((resolve) => {
            resolveRetry = resolve
          })
          return fresh
        })
    }
    if (source === 'task-context') holdRead(chats.getTaskContext, context)
    else if (source === 'clarifications')
      holdRead(chats.getClarifications, { questions: [question] })
    else holdRead(chats.getRequirements, { revisions: [revision] })
    let refresh!: Promise<unknown>
    await act(async () => {
      refresh = client.invalidateQueries({ queryKey: [source, 'session1'] })
    })
    await waitFor(() => expect(resolveRetry).toBeTypeOf('function'), { timeout: 3000 })
    expect(client.getQueryState([source, 'session1'])?.status).toBe('success')
    expect(submit).toBeDisabled()
    fireEvent.click(submit)
    expect(chats.answerClarification).not.toHaveBeenCalled()
    expect(chats.confirmRequirements).not.toHaveBeenCalled()
    await act(async () => {
      resolveRetry()
      await refresh
    })
    await waitFor(() => expect(submit).toBeEnabled())
    expect(
      isAnswer
        ? screen.getByRole('radio', { name: /Участники проекта/ })
        : screen.getByRole('checkbox', { name: /Подтверждаю цель/ }),
    ).toBeChecked()
  })
  it('restores the reading position after visiting clarification without losing the answer draft', async () => {
    vi.mocked(chats.getChatHistory).mockResolvedValue({
      items: [message('reading', 'A retained history message')],
      next_before: null,
    })
    const { router } = renderPage()
    await screen.findByText('A retained history message')
    const original = document.querySelector<HTMLDivElement>('.fc-chat-panel .fc-chat-scroll')!
    Object.defineProperties(original, {
      scrollHeight: { value: 1200 },
      clientHeight: { value: 400 },
    })
    original.scrollTop = 240
    fireEvent.scroll(original)
    await userEvent.click(screen.getByRole('tab', { name: /Уточнения/ }))
    await waitFor(() => expect(router.state.location.search).toContain('tab=clarification'))
    fireEvent.change(await screen.findByLabelText('Комментарий'), {
      target: { value: 'Original answer draft' },
    })
    await userEvent.click(screen.getByRole('tab', { name: /Диалог/ }))
    await waitFor(() => expect(router.state.location.search).toContain('tab=dialogue'))
    const returned = document.querySelector<HTMLDivElement>('.fc-chat-panel .fc-chat-scroll')!
    expect(returned).not.toBe(original)
    expect(returned.scrollTop).toBe(240)
    await userEvent.click(screen.getByRole('tab', { name: /Уточнения/ }))
    expect(await screen.findByLabelText('Комментарий')).toHaveValue('Original answer draft')
    expect(chats.answerClarification).not.toHaveBeenCalled()
  })
  it('holds stop after the first failed controls read while its retry is still pending', async () => {
    vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
    const available = {
      can_send: false,
      can_steer: false,
      can_stop: true,
      active_run_id: 'run-original',
      blocked_reason: null,
    }
    vi.mocked(chats.getChatControls).mockResolvedValue(available)
    const { client } = renderPage('dialogue', 1)
    const stop = await screen.findByRole('button', { name: 'Остановить запуск' })
    await waitFor(() => expect(stop).toBeEnabled())
    let resolveRetry!: (value: typeof available) => void
    vi.mocked(chats.getChatControls)
      .mockRejectedValueOnce(new ApiError(503, 'Retrying controls'))
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveRetry = resolve
          }),
      )
    let refresh!: Promise<unknown>
    await act(async () => {
      refresh = client.invalidateQueries({ queryKey: ['chat-controls', 'session1'] })
    })
    await waitFor(() => expect(resolveRetry).toBeTypeOf('function'), { timeout: 3000 })
    expect(client.getQueryState(['chat-controls', 'session1'])?.status).toBe('success')
    expect(stop).toBeDisabled()
    fireEvent.click(stop)
    expect(fleet.stopSessionRun).not.toHaveBeenCalled()
    await act(async () => {
      resolveRetry(available)
      await refresh
    })
    await waitFor(() => expect(stop).toBeEnabled())
  })
  it('holds a cached stop permission after a failed refresh and recovers only from fresh controls', async () => {
    vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
    const available = {
      can_send: false,
      can_steer: false,
      can_stop: true,
      active_run_id: 'run-original',
      blocked_reason: null,
    }
    vi.mocked(chats.getChatControls).mockResolvedValue(available)
    const { client } = renderPage()
    const stop = await screen.findByRole('button', { name: 'Остановить запуск' })
    await waitFor(() => expect(stop).toBeEnabled())
    vi.mocked(chats.getChatControls).mockRejectedValue(new Error('Stop authority unavailable'))
    await act(async () => {
      await client.invalidateQueries({ queryKey: ['chat-controls', 'session1'] })
    })
    await screen.findByText('Stop authority unavailable')
    expect(stop).toBeDisabled()
    fireEvent.click(stop)
    expect(fleet.stopSessionRun).not.toHaveBeenCalled()
    vi.mocked(chats.getChatControls).mockResolvedValue({
      ...available,
      active_run_id: 'run-current',
    })
    await userEvent.click(screen.getByRole('button', { name: 'Проверить доступность отправки' }))
    await waitFor(() => expect(stop).toBeEnabled())
    await userEvent.click(stop)
    await waitFor(() =>
      expect(fleet.stopSessionRun).toHaveBeenCalledWith(
        'session1',
        'run-current',
        expect.any(String),
      ),
    )
  })
  it.each(['different-owner', 'missing-run'] as const)(
    'does not issue a stop with cached controls for %s',
    async (condition) => {
      vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
      vi.mocked(chats.getChatControls).mockResolvedValue({
        can_send: false,
        can_steer: false,
        can_stop: true,
        active_run_id: condition === 'missing-run' ? null : 'run-original',
        blocked_reason: null,
      })
      if (condition === 'different-owner') useAuthStore.setState({ userId: 'operator' })
      const { client } = renderPage()
      const stop = await screen.findByRole('button', { name: 'Остановить запуск' })
      if (condition === 'different-owner')
        await waitFor(() =>
          expect(client.getQueryData(['runtime-controls', 'session1', 'run-original'])).toEqual([]),
        )
      expect(stop).toBeDisabled()
      fireEvent.click(stop)
      expect(fleet.stopSessionRun).not.toHaveBeenCalled()
    },
  )
  it.each(['answer', 'confirmation', 'prompt'] as const)(
    'holds a timed-out %s and retries its original command',
    async (command) => {
      const isAnswer = command === 'answer'
      const isPrompt = command === 'prompt'
      const mutation = isAnswer
        ? chats.answerClarification
        : isPrompt
          ? fleet.createSessionMessage
          : chats.confirmRequirements
      vi.mocked(mutation).mockRejectedValue(new ApiError(408, 'Command timed out'))
      if (isPrompt) {
        vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
        vi.mocked(chats.getChatControls).mockResolvedValue({
          can_send: true,
          can_steer: false,
          can_stop: false,
          active_run_id: null,
          blocked_reason: null,
        })
      }
      renderPage(isAnswer ? 'clarification' : isPrompt ? 'dialogue' : 'requirements')
      if (isPrompt)
        await userEvent.type(await screen.findByLabelText('Сообщение агенту'), 'Original prompt')
      else
        await userEvent.click(
          isAnswer
            ? await screen.findByRole('radio', { name: /Участники проекта/ })
            : await screen.findByRole('checkbox', { name: /Подтверждаю цель/ }),
        )
      const submitName = isAnswer
        ? 'Сохранить ответ'
        : isPrompt
          ? 'Отправить сообщение'
          : 'Подтвердить редакцию 3'
      await userEvent.click(screen.getByRole('button', { name: submitName }))
      await screen.findByText('Command timed out')
      expect(
        isAnswer
          ? screen.getByLabelText('Комментарий')
          : isPrompt
            ? screen.getByLabelText('Сообщение агенту')
            : screen.getByRole('checkbox', { name: /Подтверждаю цель/ }),
      ).toBeDisabled()
      const retryName = isAnswer
        ? 'Повторить исходный ответ'
        : isPrompt
          ? submitName
          : 'Повторить исходное подтверждение'
      await waitFor(() => expect(screen.getByRole('button', { name: retryName })).toBeEnabled())
      await userEvent.click(screen.getByRole('button', { name: retryName }))
      await waitFor(() => expect(mutation).toHaveBeenCalledTimes(2))
      expect(vi.mocked(mutation).mock.calls[1]).toEqual(vi.mocked(mutation).mock.calls[0])
    },
  )
  it('preserves the original prompt after a denied retry until its matching receipt arrives', async () => {
    vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
    vi.mocked(chats.getChatControls).mockResolvedValue({
      can_send: true,
      can_steer: false,
      can_stop: false,
      active_run_id: null,
      blocked_reason: null,
    })
    vi.mocked(fleet.createSessionMessage)
      .mockRejectedValueOnce(new Error('Prompt ACK lost'))
      .mockRejectedValueOnce(new ApiError(403, 'Prompt retry denied'))
      .mockResolvedValueOnce(message('original-prompt', 'Original prompt'))
    renderPage()
    const input = await screen.findByLabelText('Сообщение агенту')
    await userEvent.type(input, 'Original prompt')
    const submit = screen.getByRole('button', { name: 'Отправить сообщение' })
    await userEvent.click(submit)
    await screen.findByText('Prompt ACK lost')
    expect(input).toBeDisabled()
    await userEvent.click(submit)
    await screen.findByText('Prompt retry denied')
    expect(input).toBeDisabled()
    expect(input).toHaveValue('Original prompt')
    await userEvent.click(submit)
    await waitFor(() => expect(input).toBeEnabled())
    expect(input).toHaveValue('')
    const calls = vi.mocked(fleet.createSessionMessage).mock.calls
    expect(calls).toHaveLength(3)
    expect(calls[1]).toEqual(calls[0])
    expect(calls[2]).toEqual(calls[0])
  })
  it.each(['answer', 'confirmation'] as const)(
    'does not treat a rejected %s retry as proof that the original command failed',
    async (command) => {
      const isAnswer = command === 'answer'
      const mutation = isAnswer ? chats.answerClarification : chats.confirmRequirements
      vi.mocked(mutation)
        .mockRejectedValueOnce(new Error('Original ACK lost'))
        .mockRejectedValue(new ApiError(403, 'Retry denied'))
      const { client } = renderPage(isAnswer ? 'clarification' : 'requirements')
      await userEvent.click(
        isAnswer
          ? await screen.findByRole('radio', { name: /Участники проекта/ })
          : await screen.findByRole('checkbox', { name: /Подтверждаю цель/ }),
      )
      await userEvent.click(
        screen.getByRole('button', {
          name: isAnswer ? 'Сохранить ответ' : 'Подтвердить редакцию 3',
        }),
      )
      await screen.findByText('Original ACK lost')
      const retryName = isAnswer ? 'Повторить исходный ответ' : 'Повторить исходное подтверждение'
      await waitFor(() => expect(screen.getByRole('button', { name: retryName })).toBeEnabled())
      await userEvent.click(screen.getByRole('button', { name: retryName }))
      await screen.findByText('Retry denied')
      await waitFor(() => expect(screen.getByRole('button', { name: retryName })).toBeEnabled())
      expect(mutation).toHaveBeenCalledTimes(2)
      expect(vi.mocked(mutation).mock.calls[1]).toEqual(vi.mocked(mutation).mock.calls[0])
      expect(screen.getByRole('button', { name: retryName })).toBeVisible()
      await act(async () => {
        if (isAnswer)
          client.setQueryData(['clarifications', 'session1'], {
            questions: [{ ...question, version: 2 }],
          })
        else {
          client.setQueryData(['requirements', 'session1'], {
            revisions: [revision, { ...revision, revision: 4, content_hash: 'hash4' }],
          })
          client.setQueryData(['task-context', 'session1'], {
            ...context,
            tracker: { ...context.tracker!, requirement_revision: 4 },
          })
        }
      })
      await waitFor(() =>
        expect(
          screen.getByRole('button', {
            name: isAnswer ? 'Сохранить ответ' : 'Подтвердить редакцию 4',
          }),
        ).toBeDisabled(),
      )
      expect(
        isAnswer
          ? screen.getByLabelText('Комментарий')
          : screen.getByRole('checkbox', { name: /Подтверждаю цель/ }),
      ).toBeDisabled()
      if (isAnswer) {
        vi.mocked(chats.getClarifications).mockResolvedValue({
          questions: [{ ...question, version: 2 }],
        })
        vi.mocked(chats.answerClarification).mockResolvedValueOnce({
          id: 'answer',
          question_id: 'q1',
          question_version: 1,
          requirement_revision: 3,
          selected_option_ids: ['project'],
          text: null,
          comment: null,
          author_subject: 'subject-owner',
          created_at: '2026-10-01T12:00:00Z',
        })
      } else {
        vi.mocked(chats.getRequirements).mockResolvedValue({
          revisions: [revision, { ...revision, revision: 4, content_hash: 'hash4' }],
        })
        vi.mocked(chats.getTaskContext).mockResolvedValue({
          ...context,
          tracker: { ...context.tracker!, requirement_revision: 4 },
        })
        vi.mocked(chats.confirmRequirements).mockResolvedValueOnce({
          id: 'confirmation',
          task_id: 'task',
          revision: 3,
          content_hash: 'hash3',
          owner_subject: 'subject-owner',
          created_at: '2026-10-01T12:00:00Z',
          stage: 'Backlog',
        })
      }
      await userEvent.click(screen.getByRole('button', { name: retryName }))
      await waitFor(() =>
        expect(screen.queryByRole('button', { name: retryName })).not.toBeInTheDocument(),
      )
      await waitFor(() =>
        expect(
          isAnswer
            ? screen.getByLabelText('Комментарий')
            : screen.getByRole('checkbox', { name: /Подтверждаю цель/ }),
        ).toBeEnabled(),
      )
      expect(mutation).toHaveBeenCalledTimes(3)
      expect(vi.mocked(mutation).mock.calls[2]).toEqual(vi.mocked(mutation).mock.calls[0])
    },
  )
  it('does not present an empty cached question list as fresh after a failed refresh', async () => {
    vi.mocked(chats.getClarifications).mockResolvedValue({ questions: [] })
    const { client } = renderPage('clarification')
    await screen.findByText('Уточнений пока нет')
    vi.mocked(chats.getClarifications).mockRejectedValue(new Error('Question source unavailable'))
    await client.invalidateQueries({ queryKey: ['clarifications', 'session1'] })
    expect(await screen.findByText('Question source unavailable')).toBeVisible()
    expect(screen.queryByText('Уточнений пока нет')).not.toBeInTheDocument()
  })
  it.each(['source unavailable', 'read only', 'answered'])(
    'preserves a stale draft without transferring it when the question is %s',
    async (state) => {
      const { client } = renderPage('clarification')
      fireEvent.change(await screen.findByLabelText('Комментарий'), {
        target: { value: 'Original draft' },
      })
      await act(async () => {
        client.setQueryData(['clarifications', 'session1'], {
          questions: [
            { ...question, version: 2, state: state === 'answered' ? 'answered' : 'open' },
          ],
        })
        if (state === 'read only')
          client.setQueryData(['task-context', 'session1'], {
            ...context,
            tracker: {
              ...context.tracker!,
              permissions: { can_answer: false, can_confirm: false },
            },
          })
      })
      if (state === 'source unavailable') {
        vi.mocked(chats.getClarifications).mockRejectedValue(new Error('Question refresh failed'))
        await client.invalidateQueries({ queryKey: ['clarifications', 'session1'] })
        await screen.findByText('Question refresh failed')
      }
      expect(await screen.findByText('Original draft')).toBeVisible()
      expect(
        screen.getByRole('button', {
          name: 'Перенести черновик и проверить новый вопрос',
        }),
      ).toBeDisabled()
      expect(chats.answerClarification).not.toHaveBeenCalled()
    },
  )
  it.each(['answer', 'confirmation'] as const)(
    'holds both %s retry buttons until command readback finishes',
    async (command) => {
      const isAnswer = command === 'answer'
      const mutation = isAnswer ? chats.answerClarification : chats.confirmRequirements
      vi.mocked(mutation).mockRejectedValue(new Error('Lost command ACK'))
      renderPage(isAnswer ? 'clarification' : 'requirements')
      await userEvent.click(
        isAnswer
          ? await screen.findByRole('radio', { name: /Участники проекта/ })
          : await screen.findByRole('checkbox', { name: /Подтверждаю цель/ }),
      )
      let resolveContext!: (value: chats.TaskContextResponse) => void
      vi.mocked(chats.getTaskContext).mockImplementation(
        () =>
          new Promise((resolve) => {
            resolveContext = resolve
          }),
      )
      const submit = screen.getByRole('button', {
        name: isAnswer ? 'Сохранить ответ' : 'Подтвердить редакцию 3',
      })
      await userEvent.click(submit)
      await screen.findByText('Lost command ACK')
      const retry = screen.getByRole('button', {
        name: isAnswer ? 'Повторить исходный ответ' : 'Повторить исходное подтверждение',
      })
      expect(submit).toBeDisabled()
      expect(retry).toBeDisabled()
      expect(mutation).toHaveBeenCalledTimes(1)
      await act(async () => resolveContext(context))
      await waitFor(() => expect(submit).toBeEnabled())
      expect(retry).toBeEnabled()
    },
  )
  it('keeps stale requirements readable and blocks consent after a failed document refresh', async () => {
    const { client } = renderPage('requirements')
    await screen.findByText('Настоящие требования')
    vi.mocked(chats.getRequirements).mockRejectedValue(new Error('Requirements refresh failed'))
    await client.invalidateQueries({ queryKey: ['requirements', 'session1'] })
    expect(await screen.findByText('Requirements refresh failed')).toBeVisible()
    expect(screen.getByText('Настоящие требования')).toBeVisible()
    expect(screen.getByRole('checkbox', { name: /Подтверждаю цель/ })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Подтвердить редакцию 3' })).toBeDisabled()
  })
  it('keeps a question draft readable and blocked after a failed question refresh', async () => {
    const { client } = renderPage('clarification')
    await userEvent.type(await screen.findByLabelText('Комментарий'), 'Retained draft')
    vi.mocked(chats.getClarifications).mockRejectedValue(new Error('Questions refresh failed'))
    await client.invalidateQueries({ queryKey: ['clarifications', 'session1'] })
    expect(await screen.findByText('Questions refresh failed')).toBeVisible()
    expect(screen.getByLabelText('Комментарий')).toHaveValue('Retained draft')
    expect(screen.getByLabelText('Комментарий')).toBeDisabled()
  })
  it('does not report empty history on a failed first load and offers a controls retry', async () => {
    vi.mocked(chats.getChatHistory).mockRejectedValue(new Error('History unavailable'))
    vi.mocked(chats.getChatControls).mockRejectedValueOnce(new Error('Controls unavailable'))
    renderPage()
    expect(await screen.findByText('History unavailable')).toBeVisible()
    expect(screen.queryByText('Сообщений пока нет')).not.toBeInTheDocument()
    await userEvent.click(
      await screen.findByRole('button', { name: 'Проверить доступность отправки' }),
    )
    await waitFor(() => expect(chats.getChatControls).toHaveBeenCalledTimes(2))
    expect(fleet.createSessionMessage).not.toHaveBeenCalled()
  })
  it('holds every question and stale-draft transfer after an unknown answer outcome', async () => {
    vi.mocked(chats.answerClarification).mockRejectedValue(new Error('Lost answer ACK'))
    vi.mocked(chats.getClarifications).mockResolvedValue({
      questions: [question, { ...question, id: 'q2', text: 'Второй вопрос' }],
    })
    const { client } = renderPage('clarification')
    await userEvent.click(await screen.findByRole('radio', { name: /Участники проекта/ }))
    await userEvent.click(screen.getByRole('button', { name: 'Сохранить ответ' }))
    await screen.findByText('Lost answer ACK')
    await userEvent.click(screen.getByRole('button', { name: /Второй вопрос/ }))
    expect(screen.getByLabelText('Комментарий')).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Сохранить ответ' })).toBeDisabled()
    client.setQueryData(['clarifications', 'session1'], {
      questions: [{ ...question, version: 2 }],
    })
    expect(
      await screen.findByRole('button', { name: 'Перенести черновик и проверить новый вопрос' }),
    ).toBeDisabled()
    expect(chats.answerClarification).toHaveBeenCalledTimes(1)
    await userEvent.click(screen.getByRole('button', { name: 'Повторить исходный ответ' }))
    await waitFor(() => expect(chats.answerClarification).toHaveBeenCalledTimes(2))
    expect(vi.mocked(chats.answerClarification).mock.calls[1]).toEqual(
      vi.mocked(chats.answerClarification).mock.calls[0],
    )
  })
  it('keeps confirmation identity and unknown outcome across tabs and a new revision', async () => {
    vi.mocked(chats.confirmRequirements).mockRejectedValue(new Error('Lost confirmation ACK'))
    const { client } = renderPage('requirements')
    await userEvent.click(await screen.findByRole('checkbox', { name: /Подтверждаю цель/ }))
    await userEvent.click(screen.getByRole('button', { name: 'Подтвердить редакцию 3' }))
    await screen.findByText('Lost confirmation ACK')
    await userEvent.click(screen.getByRole('tab', { name: /Диалог/ }))
    await userEvent.click(screen.getByRole('tab', { name: /Требования/ }))
    expect(screen.getByRole('checkbox', { name: /Подтверждаю цель/ })).toBeDisabled()
    await userEvent.click(screen.getByRole('button', { name: 'Подтвердить редакцию 3' }))
    await waitFor(() => expect(chats.confirmRequirements).toHaveBeenCalledTimes(2))
    expect(vi.mocked(chats.confirmRequirements).mock.calls[1]).toEqual(
      vi.mocked(chats.confirmRequirements).mock.calls[0],
    )
    client.setQueryData(['requirements', 'session1'], {
      revisions: [revision, { ...revision, revision: 4, content_hash: 'hash4' }],
    })
    client.setQueryData(['task-context', 'session1'], {
      ...context,
      tracker: { ...context.tracker, requirement_revision: 4 },
    })
    expect(await screen.findByRole('button', { name: 'Подтвердить редакцию 4' })).toBeDisabled()
    expect(screen.getByRole('status')).toHaveTextContent(
      'Исход подтверждения редакции 3 неизвестен',
    )
    expect(chats.confirmRequirements).toHaveBeenCalledTimes(2)
  })
  it('does not submit another confirmation when a pending request loses its view', async () => {
    let resolveConfirmation!: (value: Awaited<ReturnType<typeof chats.confirmRequirements>>) => void
    vi.mocked(chats.confirmRequirements).mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveConfirmation = resolve
        }),
    )
    renderPage('requirements')
    await userEvent.click(await screen.findByRole('checkbox', { name: /Подтверждаю цель/ }))
    await userEvent.click(screen.getByRole('button', { name: 'Подтвердить редакцию 3' }))
    await userEvent.click(screen.getByRole('tab', { name: /Диалог/ }))
    await userEvent.click(screen.getByRole('tab', { name: /Требования/ }))
    expect(screen.getByRole('button', { name: 'Подтвердить редакцию 3' })).toBeDisabled()
    await act(async () =>
      resolveConfirmation({
        id: 'confirmation',
        task_id: 'task',
        revision: 3,
        content_hash: 'hash3',
        owner_subject: 'subject-owner',
        created_at: '2026-10-01T12:00:00Z',
        stage: 'Backlog',
      }),
    )
    await screen.findByText('Подтверждение сохранено. Следующее назначение проверяется отдельно.')
    await userEvent.click(screen.getByRole('tab', { name: /Диалог/ }))
    await userEvent.click(screen.getByRole('tab', { name: /Требования/ }))
    expect(screen.getByRole('button', { name: 'Подтвердить редакцию 3' })).toBeDisabled()
    expect(chats.confirmRequirements).toHaveBeenCalledTimes(1)
  })
  it('refreshes permissions and context as well as transcript on stream reconnect', async () => {
    useAuthStore.setState({ token: 'fixture-token' })
    renderPage('clarification')
    await screen.findByRole('radio', { name: /Участники проекта/ })
    vi.mocked(chats.getTaskContext).mockResolvedValue({
      ...context,
      tracker: { ...context.tracker!, permissions: { can_answer: false, can_confirm: false } },
    })
    await act(async () => vi.mocked(connectAuthenticatedEventStream).mock.calls[0]?.[0].onOpen?.())
    await waitFor(() =>
      expect(screen.getByRole('radio', { name: /Участники проекта/ })).toBeDisabled(),
    )
    expect(chats.getChatControls).toHaveBeenCalledTimes(2)
    expect(chats.getRequirements).toHaveBeenCalledTimes(2)
  })
  it('shows Tracker Analysis without granting prompt or confirmation actions', async () => {
    vi.mocked(chats.getTaskContext).mockResolvedValue({
      ...context,
      tracker: {
        ...context.tracker!,
        stage: 'Analysis',
        permissions: { can_answer: false, can_confirm: false },
      },
    })
    renderPage()
    expect((await screen.findAllByText('Analysis'))[0]).toBeVisible()
    expect(screen.getByRole('button', { name: 'Отправить сообщение' })).toBeDisabled()
    await userEvent.click(screen.getByRole('tab', { name: /Требования/ }))
    expect(await screen.findByRole('button', { name: 'Подтвердить редакцию 3' })).toBeDisabled()
    expect(fleet.createSessionMessage).not.toHaveBeenCalled()
    expect(chats.confirmRequirements).not.toHaveBeenCalled()
  })
  const message = (
    id: string,
    body: string,
    created_at = '2026-10-02T12:00:00Z',
  ): SessionMessage => ({
    id,
    body,
    created_at,
    session_id: 'session1',
    author_type: 'agent',
    author_user_id: null,
    author_agent_id: 'agent1',
    author_display_name: 'PM',
    message_kind: 'assistant_message',
    runtime_message_id: null,
    delivery_state: 'completed',
    delivery_error: null,
    replayed: false,
  })
  it('shows unresolved delivery without permitting a second prompt after a fresh load', async () => {
    vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
    vi.mocked(chats.getChatControls).mockResolvedValue({
      can_send: false,
      can_steer: false,
      can_stop: false,
      active_run_id: null,
      blocked_reason: 'dispatch_pending_or_uncertain',
    })
    vi.mocked(chats.getChatHistory).mockResolvedValue({
      items: [
        {
          ...message('original', 'Original saved prompt'),
          author_type: 'user',
          author_user_id: 'owner',
          author_agent_id: null,
          author_display_name: 'Owner',
          message_kind: 'user_prompt',
          delivery_state: 'pending',
          delivery_error: 'Hermes run acceptance is unknown',
        },
      ],
      next_before: null,
    })
    renderPage()
    expect(await screen.findByText('Original saved prompt')).toBeVisible()
    expect(screen.getByRole('alert')).toHaveTextContent('Hermes run acceptance is unknown')
    expect(screen.getByText('Доставка ожидается или требует сверки')).toBeVisible()
    expect(screen.getByLabelText('Сообщение агенту')).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Отправить сообщение' })).toBeDisabled()
    expect(fleet.createSessionMessage).not.toHaveBeenCalled()
    expect(fleet.steerSessionRun).not.toHaveBeenCalled()
  })
  it.each(['reconnect', 'message event'])(
    'keeps older loading and catches new messages after %s',
    async (event) => {
      useAuthStore.setState({ token: 'fixture-token' })
      let resolveOlder!: (page: chats.HistoryPage) => void
      vi.mocked(chats.getChatHistory)
        .mockResolvedValueOnce({
          items: [message('latest', 'Existing latest')],
          next_before: 'older-cursor',
        })
        .mockImplementationOnce(
          () =>
            new Promise((resolve) => {
              resolveOlder = resolve
            }),
        )
        .mockResolvedValueOnce({
          items: [message('latest', 'Existing latest'), message('new', 'Arrived while loading')],
          next_before: 'older-cursor',
        })
        .mockResolvedValueOnce({ items: [message('older', 'Older history')], next_before: null })
      renderPage()
      await userEvent.click(await screen.findByRole('button', { name: 'Предыдущие сообщения' }))
      await waitFor(() => expect(chats.getChatHistory).toHaveBeenCalledTimes(2))
      await act(async () => {
        const stream = vi.mocked(connectAuthenticatedEventStream).mock.calls[0]?.[0]
        expect(stream).toBeDefined()
        if (event === 'reconnect') stream?.onOpen?.()
        else stream?.onEvent('session', { type: 'session_message_changed' })
        resolveOlder({ items: [message('older', 'Older history')], next_before: null })
      })
      await waitFor(() =>
        expect(
          screen.queryByRole('button', { name: 'Предыдущие сообщения' }),
        ).not.toBeInTheDocument(),
      )
      await screen.findByText('Arrived while loading')
      expect(
        [...document.querySelectorAll('.fc-chat-message > p')].map((node) => node.textContent),
      ).toEqual(['Older history', 'Existing latest', 'Arrived while loading'])
      expect(chats.getChatHistory).toHaveBeenCalledTimes(4)
    },
  )

  it('keeps server append order across clock rollback, older pages and overlap', async () => {
    vi.mocked(chats.getChatHistory)
      .mockResolvedValueOnce({
        items: [
          message('second', 'Second message', '2026-10-02T12:00:00Z'),
          message('third', 'Third message', '2026-10-01T12:00:00Z'),
        ],
        next_before: 'second',
      })
      .mockResolvedValueOnce({
        items: [
          message('first', 'First message', '2026-10-03T12:00:00Z'),
          message('second', 'Outdated overlap', '2026-10-02T12:00:00Z'),
        ],
        next_before: null,
      })
    renderPage()
    await screen.findByText('Third message')
    const texts = () =>
      [...document.querySelectorAll('.fc-chat-message > p')].map((node) => node.textContent)
    expect(texts()).toEqual(['Second message', 'Third message'])
    await userEvent.click(screen.getByRole('button', { name: 'Предыдущие сообщения' }))
    await waitFor(() =>
      expect(texts()).toEqual(['First message', 'Second message', 'Third message']),
    )
    expect(screen.queryByText('Outdated overlap')).not.toBeInTheDocument()
  })

  it('requires explicit answer and does not publish after saving it', async () => {
    renderPage('clarification')
    const choice = await screen.findByRole('radio', { name: /Участники проекта/ })
    expect(choice).not.toBeChecked()
    expect(screen.getByRole('button', { name: 'Сохранить ответ' })).toBeDisabled()
    fireEvent.click(choice)
    fireEvent.click(screen.getByRole('button', { name: 'Сохранить ответ' }))
    await waitFor(() =>
      expect(chats.answerClarification).toHaveBeenCalledWith(
        'session1',
        'q1',
        expect.objectContaining({
          expected_question_version: 1,
          requirement_revision: 3,
          selected_option_ids: ['project'],
          idempotency_key: expect.any(String),
        }),
      ),
    )
    expect(chats.confirmRequirements).not.toHaveBeenCalled()
  })
  it('retains the draft across tabs and after a conflict', async () => {
    vi.mocked(chats.answerClarification).mockRejectedValue(new ApiError(409, 'Редакция изменилась'))
    const { router } = renderPage('clarification')
    fireEvent.click(await screen.findByRole('radio', { name: /Участники проекта/ }))
    fireEvent.change(screen.getByLabelText('Комментарий'), { target: { value: 'Мой черновик' } })
    fireEvent.click(screen.getByRole('button', { name: 'Сохранить ответ' }))
    await screen.findByText('Редакция изменилась')
    await userEvent.click(screen.getByRole('tab', { name: /Диалог/ }))
    await waitFor(() => expect(router.state.location.search).toContain('tab=dialogue'))
    await userEvent.click(screen.getByRole('tab', { name: /Уточнения/ }))
    await waitFor(() => expect(router.state.location.search).toContain('tab=clarification'))
    expect(screen.getByLabelText('Комментарий')).toHaveValue('Мой черновик')
  })
  it('never allows an operator reading another owner chat to consent', async () => {
    useAuthStore.setState({ userId: 'operator', permissions: ['sessions:read_all'] })
    renderPage('clarification')
    expect(await screen.findByRole('radio', { name: /Участники проекта/ })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Сохранить ответ' })).toBeDisabled()
  })
  it('keeps task documents and drafts readable after the bound PM is replaced', async () => {
    const { client } = renderPage('clarification')
    const comment = await screen.findByLabelText('Комментарий')
    fireEvent.change(comment, { target: { value: 'Original unsaved note' } })
    vi.mocked(chats.getTaskContext).mockResolvedValue({
      ...context,
      tracker: {
        ...context.tracker!,
        assignment: {
          assignment_id: 'replacement-assignment',
          execution_id: 'replacement-execution',
          agent_id: 'replacement-agent',
          version: 2,
          machine_subject: 'replacement-pm',
        },
        permissions: { can_answer: false, can_confirm: false },
      },
    })
    await client.invalidateQueries({ queryKey: ['task-context', 'session1'] })
    await waitFor(() => expect(comment).toBeDisabled())
    expect(comment).toHaveValue('Original unsaved note')
    expect(screen.getByRole('radio', { name: /Участники проекта/ })).toBeDisabled()
    fireEvent.click(screen.getByRole('button', { name: 'Сохранить ответ' }))
    expect(chats.answerClarification).not.toHaveBeenCalled()
    await userEvent.click(screen.getByRole('tab', { name: /Требования/ }))
    await screen.findByText('Настоящие требования')
    expect(screen.getByRole('checkbox', { name: /Подтверждаю цель/ })).toBeDisabled()
    expect(chats.confirmRequirements).not.toHaveBeenCalled()
  })
  it('replays the exact answer key after unknown acceptance without editable payload', async () => {
    vi.mocked(chats.answerClarification).mockRejectedValue(new Error('Connection interrupted'))
    renderPage('clarification')
    fireEvent.click(await screen.findByRole('radio', { name: /Участники проекта/ }))
    const submit = screen.getByRole('button', { name: 'Сохранить ответ' })
    fireEvent.click(submit)
    await screen.findByText('Connection interrupted')
    expect(screen.getByLabelText('Комментарий')).toBeDisabled()
    fireEvent.click(submit)
    await waitFor(() => expect(chats.answerClarification).toHaveBeenCalledTimes(2))
    const calls = vi.mocked(chats.answerClarification).mock.calls
    expect(calls[1]).toEqual(calls[0])
  })
  it('warns when leaving the chat but not when selecting another tab', async () => {
    const { router } = renderPage('clarification')
    fireEvent.click(await screen.findByRole('radio', { name: /Участники проекта/ }))
    await userEvent.click(screen.getByRole('tab', { name: /Диалог/ }))
    await waitFor(() => expect(router.state.location.search).toContain('tab=dialogue'))
    fireEvent.click(screen.getByRole('link', { name: 'Вернуться к чатам' }))
    await screen.findByRole('dialog')
    expect(router.state.location.pathname).toBe('/chats/session1')
  })
  it('confirms exact hash/revision separately', async () => {
    renderPage('requirements')
    const confirm = await screen.findByRole('button', { name: 'Подтвердить редакцию 3' })
    expect(confirm).toBeDisabled()
    fireEvent.click(screen.getByRole('checkbox', { name: /Подтверждаю цель/ }))
    fireEvent.click(confirm)
    await waitFor(() =>
      expect(chats.confirmRequirements).toHaveBeenCalledWith(
        'session1',
        3,
        'hash3',
        expect.any(String),
      ),
    )
  })
  it('resets consent when the displayed requirements revision changes', async () => {
    const { client } = renderPage('requirements')
    fireEvent.click(await screen.findByRole('checkbox', { name: /Подтверждаю цель/ }))
    expect(screen.getByRole('button', { name: 'Подтвердить редакцию 3' })).toBeEnabled()
    client.setQueryData(['requirements', 'session1'], {
      revisions: [revision, { ...revision, revision: 4, content_hash: 'hash4' }],
    })
    client.setQueryData(['task-context', 'session1'], {
      ...context,
      tracker: { ...context.tracker, requirement_revision: 4 },
    })
    const confirm = await screen.findByRole('button', { name: 'Подтвердить редакцию 4' })
    expect(confirm).toBeDisabled()
    expect(screen.getByRole('checkbox', { name: /Подтверждаю цель/ })).not.toBeChecked()
    expect(chats.confirmRequirements).not.toHaveBeenCalled()
  })
  it('returns focus to the context trigger after Escape', async () => {
    renderPage()
    const trigger = await screen.findByRole('button', { name: /^Контекст задачи$/ })
    await userEvent.click(trigger)
    await screen.findByRole('dialog', { name: 'Контекст задачи' })
    await userEvent.keyboard('{Escape}')
    await waitFor(() => expect(trigger).toHaveFocus())
  })
  it('keeps dialogue available when Tracker fails and disables structured commands', async () => {
    vi.mocked(chats.getTaskContext).mockRejectedValue(new Error('Tracker недоступен'))
    renderPage()
    await screen.findByText('Tracker недоступен')
    expect(screen.getByRole('tab', { name: /Диалог/ })).toBeVisible()
    expect(screen.getByRole('button', { name: 'Отправить сообщение' })).toBeDisabled()
  })
  it('retains and exposes a stale-version draft until explicit review', async () => {
    const { client } = renderPage('clarification')
    fireEvent.click(await screen.findByRole('radio', { name: /Участники проекта/ }))
    fireEvent.change(screen.getByLabelText('Комментарий'), {
      target: { value: 'Ответ старой редакции' },
    })
    client.setQueryData(['clarifications', 'session1'], {
      questions: [{ ...question, version: 2 }],
    })
    await screen.findByText('Вопрос изменился. Несохранённый ответ сохранён отдельно.')
    expect(screen.getByText('Ответ старой редакции')).toBeVisible()
    expect(screen.getByRole('button', { name: 'Сохранить ответ' })).toBeDisabled()
    fireEvent.click(
      screen.getByRole('button', { name: 'Перенести черновик и проверить новый вопрос' }),
    )
    expect(screen.getByLabelText('Комментарий')).toHaveValue('Ответ старой редакции')
    fireEvent.click(screen.getByRole('button', { name: 'Сохранить ответ' }))
    await waitFor(() =>
      expect(chats.answerClarification).toHaveBeenCalledWith(
        'session1',
        'q1',
        expect.objectContaining({ expected_question_version: 2, comment: 'Ответ старой редакции' }),
      ),
    )
  })
  it.each(['renamed', 'replaced'] as const)(
    'preserves the original selected label when the new question option is %s',
    async (change) => {
      const { client } = renderPage('clarification')
      fireEvent.click(await screen.findByRole('radio', { name: /Участники проекта/ }))
      const nextOption = {
        ...question.options[0]!,
        id: change === 'renamed' ? 'project' : 'everyone',
        label: 'Все пользователи',
      }
      client.setQueryData(['clarifications', 'session1'], {
        questions: [{ ...question, version: 2, options: [nextOption] }],
      })
      const retained = await screen.findByRole('alert')
      expect(within(retained).getByText('Участники проекта')).toBeVisible()
      expect(within(retained).queryByText('Все пользователи')).not.toBeInTheDocument()
      expect(screen.getByRole('radio', { name: /Все пользователи/ })).not.toBeChecked()
      expect(screen.getByRole('button', { name: 'Сохранить ответ' })).toBeDisabled()
      expect(chats.answerClarification).not.toHaveBeenCalled()
      fireEvent.click(
        screen.getByRole('button', { name: 'Перенести черновик и проверить новый вопрос' }),
      )
      expect(screen.queryByRole('alert')).not.toBeInTheDocument()
      if (change === 'renamed') {
        expect(screen.getByRole('radio', { name: /Все пользователи/ })).toBeChecked()
        fireEvent.click(screen.getByRole('button', { name: 'Сохранить ответ' }))
        await waitFor(() =>
          expect(chats.answerClarification).toHaveBeenCalledWith(
            'session1',
            'q1',
            expect.objectContaining({
              expected_question_version: 2,
              selected_option_ids: ['project'],
            }),
          ),
        )
      } else {
        expect(screen.getByRole('radio', { name: /Все пользователи/ })).not.toBeChecked()
        expect(screen.getByRole('button', { name: 'Сохранить ответ' })).toBeDisabled()
      }
    },
  )
  it('retains the reviewed version labels after transferring a draft and receiving another version', async () => {
    const { client } = renderPage('clarification')
    fireEvent.click(await screen.findByRole('radio', { name: /Участники проекта/ }))
    const update = (version: number, label: string) =>
      client.setQueryData(['clarifications', 'session1'], {
        questions: [{ ...question, version, options: [{ ...question.options[0]!, label }] }],
      })
    update(2, 'Все пользователи')
    await screen.findByRole('alert')
    fireEvent.click(
      screen.getByRole('button', { name: 'Перенести черновик и проверить новый вопрос' }),
    )
    expect(screen.getByRole('radio', { name: /Все пользователи/ })).toBeChecked()
    update(3, 'Только администраторы')
    const retained = await screen.findByRole('alert')
    expect(within(retained).getByText('Все пользователи')).toBeVisible()
    expect(within(retained).queryByText('Участники проекта')).not.toBeInTheDocument()
    expect(within(retained).queryByText('Только администраторы')).not.toBeInTheDocument()
    expect(screen.getByRole('radio', { name: /Только администраторы/ })).not.toBeChecked()
    expect(screen.getByRole('button', { name: 'Сохранить ответ' })).toBeDisabled()
    expect(chats.answerClarification).not.toHaveBeenCalled()
  })
  it('disables cached permissions after a failed context refresh', async () => {
    const { client } = renderPage('clarification')
    fireEvent.click(await screen.findByRole('radio', { name: /Участники проекта/ }))
    expect(screen.getByRole('button', { name: 'Сохранить ответ' })).toBeEnabled()
    vi.mocked(chats.getTaskContext).mockRejectedValue(new Error('Tracker failed during refresh'))
    await client.invalidateQueries({ queryKey: ['task-context', 'session1'] })
    await screen.findByText('Tracker failed during refresh')
    expect(screen.getByRole('button', { name: 'Сохранить ответ' })).toBeDisabled()
    await userEvent.click(screen.getByRole('tab', { name: /Требования/ }))
    expect(await screen.findByRole('button', { name: 'Подтвердить редакцию 3' })).toBeDisabled()
  })
  it('does not overwrite a new-version answer with an older retained draft', async () => {
    const { client } = renderPage('clarification')
    fireEvent.change(await screen.findByLabelText('Комментарий'), {
      target: { value: 'Old draft' },
    })
    client.setQueryData(['clarifications', 'session1'], {
      questions: [{ ...question, version: 2 }],
    })
    await screen.findByText('Вопрос изменился. Несохранённый ответ сохранён отдельно.')
    fireEvent.change(screen.getByLabelText('Комментарий'), { target: { value: 'New draft' } })
    expect(
      screen.getByRole('button', { name: 'Перенести черновик и проверить новый вопрос' }),
    ).toBeDisabled()
    expect(screen.getByLabelText('Комментарий')).toHaveValue('New draft')
    expect(screen.getByText('Old draft')).toBeVisible()
  })
  it('never converts an uncertain steer into a new prompt after controls change', async () => {
    vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
    vi.mocked(chats.getChatControls).mockResolvedValue({
      can_send: false,
      can_steer: true,
      can_stop: true,
      active_run_id: 'run-1',
      blocked_reason: null,
    })
    vi.mocked(fleet.steerSessionRun).mockRejectedValue(new Error('Unknown steer outcome'))
    const { client } = renderPage()
    const input = await screen.findByLabelText('Уточнение активному запуску')
    fireEvent.change(input, { target: { value: 'A scoped steer' } })
    fireEvent.click(screen.getByRole('button', { name: 'Передать уточнение запуску' }))
    await screen.findByText('Unknown steer outcome')
    client.setQueryData(['chat-controls', 'session1'], {
      can_send: true,
      can_steer: false,
      can_stop: false,
      active_run_id: null,
      blocked_reason: null,
    })
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Отправить сообщение' })).toBeDisabled(),
    )
    expect(fleet.createSessionMessage).not.toHaveBeenCalled()
    expect(fleet.steerSessionRun).toHaveBeenCalledTimes(1)
  })
})
