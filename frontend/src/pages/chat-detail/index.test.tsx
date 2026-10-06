import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ChatDetailPage } from './index'
import * as fleet from '@/api/fleet'
import * as chats from '@/api/task-chats'
import type { AgentSession, SessionMessage } from '@/api/types'
import { useAuthStore } from '@/shared/auth/store'
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
function renderPage(tab = 'dialogue', queryRetries: false | number = false) {
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
  render(
    <QueryClientProvider client={client}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  )
  return { router, client }
}
beforeEach(() => {
  vi.clearAllMocks()
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
  vi.mocked(fleet.listRuntimeControls).mockResolvedValue([])
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
