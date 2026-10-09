import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ChatDetailPage } from './index'
import * as fleet from '@/api/fleet'
import * as chats from '@/api/task-chats'
import { listRuntimeControls, lookupRuntimeControl } from '@/api/runtime-controls'
import { saveControlHandle } from './control-journal'
import type { AgentSession, SessionMessage } from '@/api/types'
import { useAuthStore } from '@/shared/auth/store'
import { ApiError, connectAuthenticatedEventStream } from '@sdlc/ui/lib'
import userEvent from '@testing-library/user-event'

vi.mock('@/api/fleet', () => ({
  getSession: vi.fn(),
  listAgentDirectory: vi.fn(),
  listSessionAgentRuns: vi.fn(),
  createSessionMessage: vi.fn(),
  steerSessionRun: vi.fn(),
  stopSessionRun: vi.fn(),
}))
vi.mock('@/api/runtime-controls', () => ({
  listRuntimeControls: vi.fn(async () => []),
  lookupRuntimeControl: vi.fn(),
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
function renderPage(tab = 'dialogue') {
  const router = createMemoryRouter(
    [
      { path: '/chats/:sessionId', element: <ChatDetailPage /> },
      { path: '/chats', element: <p>Список</p> },
    ],
    { initialEntries: [`/chats/session1?tab=${tab}`] },
  )
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })
  render(
    <QueryClientProvider client={client}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  )
  return { router, client }
}
beforeEach(() => {
  sessionStorage.clear()
  vi.clearAllMocks()
  vi.mocked(lookupRuntimeControl).mockRejectedValue(new Error('Lookup unavailable'))
  useAuthStore.setState({ userId: 'owner', token: null })
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
    expect(fleet.steerSessionRun).toHaveBeenCalledWith(
      'session1',
      'run-1',
      { input: 'A scoped steer' },
      expect.any(String),
    )
  })

  it('replays a lost stop response only against its original run and key', async () => {
    vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
    vi.mocked(chats.getChatControls).mockResolvedValue({
      can_send: false,
      can_steer: true,
      can_stop: true,
      active_run_id: 'original-run',
      blocked_reason: null,
    })
    vi.mocked(fleet.stopSessionRun).mockRejectedValue(new Error('Unknown stop outcome'))
    const { client } = renderPage()
    fireEvent.click(await screen.findByRole('button', { name: 'Остановить запуск' }))
    await screen.findByText('Unknown stop outcome')
    const original = vi.mocked(fleet.stopSessionRun).mock.calls[0]
    expect(original).toEqual(['session1', 'original-run', expect.any(String)])
    act(() =>
      client.setQueryData(['chat-controls', 'session1'], {
        can_send: false,
        can_steer: true,
        can_stop: true,
        active_run_id: 'new-run',
        blocked_reason: null,
      }),
    )
    fireEvent.click(screen.getByRole('button', { name: 'Остановить запуск' }))
    await waitFor(() => expect(fleet.stopSessionRun).toHaveBeenCalledTimes(2))
    expect(vi.mocked(fleet.stopSessionRun).mock.calls[1]).toEqual(original)
  })

  it('retains an unacknowledged steer returned with HTTP success without creating a new prompt', async () => {
    vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
    vi.mocked(chats.getChatControls).mockResolvedValue({
      can_send: false,
      can_steer: true,
      can_stop: true,
      active_run_id: 'original-run',
      blocked_reason: null,
    })
    vi.mocked(fleet.steerSessionRun).mockResolvedValue({
      session_id: 'session1',
      run_id: 'original-run',
      runtime_run_id: 'native-run',
      accepted: false,
      state: 'running',
      message: 'Acceptance unknown',
    })
    const { client } = renderPage()
    const input = await screen.findByLabelText('Уточнение активному запуску')
    fireEvent.change(input, { target: { value: 'Keep this guidance' } })
    fireEvent.click(screen.getByRole('button', { name: 'Передать уточнение запуску' }))
    await screen.findByText('Принятие команды не подтверждено. Текст и ключ команды сохранены.')
    expect(input).toHaveValue('Keep this guidance')
    expect(input).toBeDisabled()
    act(() =>
      client.setQueryData(['chat-controls', 'session1'], {
        can_send: true,
        can_steer: false,
        can_stop: false,
        active_run_id: null,
        blocked_reason: null,
      }),
    )
    expect(await screen.findByRole('button', { name: 'Отправить сообщение' })).toBeDisabled()
    expect(fleet.createSessionMessage).not.toHaveBeenCalled()
    expect(fleet.steerSessionRun).toHaveBeenCalledTimes(1)
  })

  it('replays an unacknowledged stop returned with HTTP success only against its original run', async () => {
    vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
    vi.mocked(chats.getChatControls).mockResolvedValue({
      can_send: false,
      can_steer: true,
      can_stop: true,
      active_run_id: 'original-run',
      blocked_reason: null,
    })
    vi.mocked(fleet.stopSessionRun).mockResolvedValue({
      session_id: 'session1',
      run_id: 'original-run',
      runtime_run_id: 'native-run',
      accepted: false,
      state: 'running',
      message: 'Acceptance unknown',
    })
    const { client } = renderPage()
    fireEvent.click(await screen.findByRole('button', { name: 'Остановить запуск' }))
    await screen.findByText(/Принятие остановки не подтверждено/)
    const original = vi.mocked(fleet.stopSessionRun).mock.calls[0]
    act(() =>
      client.setQueryData(['chat-controls', 'session1'], {
        can_send: false,
        can_steer: true,
        can_stop: true,
        active_run_id: 'new-run',
        blocked_reason: null,
      }),
    )
    fireEvent.click(screen.getByRole('button', { name: 'Остановить запуск' }))
    await waitFor(() => expect(fleet.stopSessionRun).toHaveBeenCalledTimes(2))
    expect(vi.mocked(fleet.stopSessionRun).mock.calls[1]).toEqual(original)
  })

  it('reads the new steer target after an earlier stop was acknowledged', async () => {
    vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
    vi.mocked(chats.getChatControls).mockResolvedValue({
      can_send: false,
      can_steer: true,
      can_stop: true,
      active_run_id: 'run-a',
      blocked_reason: null,
    })
    vi.mocked(fleet.stopSessionRun).mockResolvedValue({
      session_id: 'session1',
      run_id: 'run-a',
      runtime_run_id: 'native-a',
      accepted: true,
      state: 'stopping',
      message: 'Stopping acknowledged',
    })
    vi.mocked(fleet.steerSessionRun).mockResolvedValue({
      session_id: 'session1',
      run_id: 'run-b',
      runtime_run_id: 'native-b',
      accepted: false,
      state: 'running',
      message: 'Acceptance unknown',
    })
    const { client } = renderPage()
    fireEvent.click(await screen.findByRole('button', { name: 'Остановить запуск' }))
    await waitFor(() => expect(fleet.stopSessionRun).toHaveBeenCalledTimes(1))
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Остановить запуск' })).toBeEnabled(),
    )
    act(() =>
      client.setQueryData(['chat-controls', 'session1'], {
        can_send: false,
        can_steer: true,
        can_stop: true,
        active_run_id: 'run-b',
        blocked_reason: null,
      }),
    )
    fireEvent.change(screen.getByLabelText('Уточнение активному запуску'), {
      target: { value: 'Guidance for B only' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Передать уточнение запуску' }))
    await screen.findByText('Принятие команды не подтверждено. Текст и ключ команды сохранены.')
    await waitFor(() => expect(listRuntimeControls).toHaveBeenCalledWith('session1', 'run-b'))
    expect(fleet.steerSessionRun).toHaveBeenCalledWith(
      'session1',
      'run-b',
      { input: 'Guidance for B only' },
      expect.any(String),
    )
    expect(fleet.createSessionMessage).not.toHaveBeenCalled()
  })

  it('restores a pending steer without storing its text or dispatching after reload', async () => {
    saveControlHandle('owner', 'session1', {
      operation: 'steer',
      runId: 'original-run',
      key: 'original-key',
    })
    vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
    vi.mocked(chats.getChatControls).mockResolvedValue({
      can_send: true,
      can_steer: false,
      can_stop: false,
      active_run_id: null,
      blocked_reason: null,
    })
    renderPage()
    const input = await screen.findByLabelText('Сообщение агенту')
    expect(input).toHaveValue('')
    expect(input).toBeDisabled()
    await waitFor(() =>
      expect(lookupRuntimeControl).toHaveBeenCalledWith('session1', 'original-run', 'original-key'),
    )
    expect(screen.getByRole('button', { name: 'Отправить сообщение' })).toBeDisabled()
    expect(fleet.createSessionMessage).not.toHaveBeenCalled()
    expect(fleet.steerSessionRun).not.toHaveBeenCalled()
  })

  it('restores the original stop key instead of stopping the newly active run', async () => {
    saveControlHandle('owner', 'session1', {
      operation: 'stop',
      runId: 'original-run',
      key: 'original-key',
    })
    vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
    vi.mocked(chats.getChatControls).mockResolvedValue({
      can_send: false,
      can_steer: true,
      can_stop: true,
      active_run_id: 'new-run',
      blocked_reason: null,
    })
    vi.mocked(fleet.stopSessionRun).mockRejectedValue(new Error('Still unknown'))
    renderPage()
    fireEvent.click(await screen.findByRole('button', { name: 'Остановить запуск' }))
    await waitFor(() =>
      expect(fleet.stopSessionRun).toHaveBeenCalledWith('session1', 'original-run', 'original-key'),
    )
    expect(fleet.stopSessionRun).toHaveBeenCalledTimes(1)
  })

  it('does not settle a restored steer from an empty or missing lookup response', async () => {
    saveControlHandle('owner', 'session1', {
      operation: 'steer',
      runId: 'original-run',
      key: 'original-key',
    })
    vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
    vi.mocked(chats.getChatControls).mockResolvedValue({
      can_send: true,
      can_steer: false,
      can_stop: false,
      active_run_id: null,
      blocked_reason: null,
    })
    vi.mocked(lookupRuntimeControl).mockResolvedValue(JSON.parse('[]'))
    renderPage()
    await screen.findByText(
      'Доставка не подтверждена. Отсутствие квитанции не разрешает новую отправку.',
    )
    expect(
      screen.queryByRole('button', { name: 'Закрыть сверку уточнения' }),
    ).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Отправить сообщение' })).toBeDisabled()
    expect(fleet.createSessionMessage).not.toHaveBeenCalled()
  })

  it('releases an exact acknowledged steer only after explicit settlement, without redispatch', async () => {
    saveControlHandle('owner', 'session1', {
      operation: 'steer',
      runId: 'original-run',
      key: 'original-key',
    })
    vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
    vi.mocked(chats.getChatControls).mockResolvedValue({
      can_send: true,
      can_steer: false,
      can_stop: false,
      active_run_id: null,
      blocked_reason: null,
    })
    vi.mocked(lookupRuntimeControl).mockResolvedValue({
      id: 'command',
      actor_user_id: 'owner',
      agent_id: 'agent1',
      session_id: 'session1',
      session_run_id: 'original-run',
      operation: 'steer',
      state: 'acknowledged',
      acknowledgement: 'steered',
      observed_run_state: null,
      created_at: '2026-10-09T10:00:00Z',
      updated_at: '2026-10-09T10:00:01Z',
    })
    renderPage()
    const close = await screen.findByRole('button', { name: 'Закрыть сверку уточнения' })
    expect(screen.getByLabelText('Сообщение агенту')).toBeDisabled()
    fireEvent.click(close)
    expect(await screen.findByLabelText('Сообщение агенту')).toBeEnabled()
    expect(screen.getByLabelText('Сообщение агенту')).toHaveValue('')
    expect(fleet.steerSessionRun).not.toHaveBeenCalled()
    expect(fleet.createSessionMessage).not.toHaveBeenCalled()
    expect(
      screen.queryByRole('button', { name: 'Закрыть сверку уточнения' }),
    ).not.toBeInTheDocument()
  })

  it('does not send a control when metadata cannot be durably saved', async () => {
    vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
    vi.mocked(chats.getChatControls).mockResolvedValue({
      can_send: false,
      can_steer: true,
      can_stop: true,
      active_run_id: 'run',
      blocked_reason: null,
    })
    renderPage()
    fireEvent.change(await screen.findByLabelText('Уточнение активному запуску'), {
      target: { value: 'Do not send without an original handle' },
    })
    const write = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('Quota')
    })
    try {
      fireEvent.click(screen.getByRole('button', { name: 'Передать уточнение запуску' }))
      await screen.findByText(
        'Метаданные исходной команды недоступны. Новая отправка заблокирована.',
      )
      expect(fleet.steerSessionRun).not.toHaveBeenCalled()
      expect(fleet.createSessionMessage).not.toHaveBeenCalled()
      expect(screen.getByRole('button', { name: 'Передать уточнение запуску' })).toBeDisabled()
    } finally {
      write.mockRestore()
    }
  })

  it.each([
    { label: 'ASCII', input: 'a'.repeat(65536) },
    { label: 'UTF-8', input: 'я'.repeat(32768) },
  ])('allows exactly 64 KiB of $label steer', async ({ input }) => {
    vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
    vi.mocked(chats.getChatControls).mockResolvedValue({
      can_send: false,
      can_steer: true,
      can_stop: true,
      active_run_id: 'run-a',
      blocked_reason: null,
    })
    vi.mocked(fleet.steerSessionRun).mockResolvedValue({
      session_id: 'session1',
      run_id: 'run-a',
      runtime_run_id: 'native-a',
      accepted: false,
      state: 'running',
      message: 'Acceptance unknown',
    })
    renderPage()
    const editor = await screen.findByLabelText('Уточнение активному запуску')
    fireEvent.change(editor, { target: { value: input } })
    fireEvent.click(screen.getByRole('button', { name: 'Передать уточнение запуску' }))
    await waitFor(() =>
      expect(fleet.steerSessionRun).toHaveBeenCalledWith(
        'session1',
        'run-a',
        { input },
        expect.any(String),
      ),
    )
    expect(screen.queryByText('Уточнение не должно превышать 64 КиБ в UTF-8.')).toBeNull()
  })

  it.each([
    { label: 'ASCII', input: 'a'.repeat(65537) },
    { label: 'UTF-8', input: 'я'.repeat(32769) },
  ])('keeps oversized $label steer editable without reserving a handle', async ({ input }) => {
    vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
    vi.mocked(chats.getChatControls).mockResolvedValue({
      can_send: false,
      can_steer: true,
      can_stop: true,
      active_run_id: 'run-a',
      blocked_reason: null,
    })
    renderPage()
    const editor = await screen.findByLabelText('Уточнение активному запуску')
    fireEvent.change(editor, { target: { value: input } })
    await screen.findByText('Уточнение не должно превышать 64 КиБ в UTF-8.')
    expect(editor).toBeEnabled()
    expect(screen.getByRole('button', { name: 'Передать уточнение запуску' })).toBeDisabled()
    expect(fleet.steerSessionRun).not.toHaveBeenCalled()
    expect(sessionStorage.getItem('fleet-runtime-controls:v1:owner:session1')).toBeNull()
    fireEvent.change(editor, { target: { value: 'Corrected guidance' } })
    expect(screen.getByRole('button', { name: 'Передать уточнение запуску' })).toBeEnabled()
  })

  it.each([
    { operation: 'steer', accepted: true },
    { operation: 'steer', accepted: false },
    { operation: 'stop', accepted: true },
    { operation: 'stop', accepted: false },
  ] as const)(
    'ignores late $operation POST accepted=$accepted after exact settlement and successor',
    async ({ operation, accepted }) => {
      vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
      vi.mocked(chats.getChatControls).mockResolvedValue({
        can_send: false,
        can_steer: true,
        can_stop: true,
        active_run_id: 'run-a',
        blocked_reason: null,
      })
      let resolveOriginal!: (result: Awaited<ReturnType<typeof fleet.stopSessionRun>>) => void
      const control =
        operation === 'steer' ? vi.mocked(fleet.steerSessionRun) : vi.mocked(fleet.stopSessionRun)
      control
        .mockImplementationOnce(() => new Promise((resolve) => (resolveOriginal = resolve)))
        .mockRejectedValueOnce(new Error('Successor acceptance unknown'))
      vi.mocked(lookupRuntimeControl).mockImplementation(async (_session, _run, key) => {
        const originalKey =
          operation === 'steer'
            ? vi.mocked(fleet.steerSessionRun).mock.calls[0]?.[3]
            : vi.mocked(fleet.stopSessionRun).mock.calls[0]?.[2]
        if (key !== originalKey) throw new Error('Successor lookup unknown')
        return {
          id: 'command-a',
          actor_user_id: 'owner',
          agent_id: 'agent1',
          session_id: 'session1',
          session_run_id: 'run-a',
          operation,
          state: 'acknowledged',
          acknowledgement: operation === 'steer' ? 'steered' : 'stopping',
          observed_run_state: null,
          created_at: '2026-10-09T10:00:00Z',
          updated_at: '2026-10-09T10:00:01Z',
        }
      })
      renderPage()
      const editor = await screen.findByLabelText('Уточнение активному запуску')
      const dispatch = (input: string) => {
        if (operation === 'steer') fireEvent.change(editor, { target: { value: input } })
        fireEvent.click(
          screen.getByRole('button', {
            name: operation === 'steer' ? 'Передать уточнение запуску' : 'Остановить запуск',
          }),
        )
      }
      dispatch('Original guidance')
      fireEvent.click(
        await screen.findByRole('button', {
          name: operation === 'steer' ? 'Закрыть сверку уточнения' : 'Закрыть сверку остановки',
        }),
      )
      dispatch('Successor guidance')
      await screen.findByText('Successor acceptance unknown')
      const storageKey = 'fleet-runtime-controls:v1:owner:session1'
      const successor = JSON.parse(sessionStorage.getItem(storageKey)!)[operation]
      await act(async () => {
        resolveOriginal({
          session_id: 'session1',
          run_id: 'run-a',
          runtime_run_id: 'native-a',
          accepted,
          state: 'running',
          message: 'Late original response',
        })
      })
      expect(JSON.parse(sessionStorage.getItem(storageKey)!)[operation]).toEqual(successor)
      expect(
        screen.queryByText('Метаданные исходной команды недоступны. Новая отправка заблокирована.'),
      ).not.toBeInTheDocument()
      if (operation === 'steer') expect(editor).toHaveValue('Successor guidance')
      expect(control).toHaveBeenCalledTimes(2)
      expect(fleet.createSessionMessage).not.toHaveBeenCalled()
    },
  )
})
