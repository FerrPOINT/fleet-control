import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ChatDetailPage } from './index'
import * as fleet from '@/api/fleet'
import * as chats from '@/api/task-chats'
import { listRuntimeControls, lookupRuntimeControl } from '@/api/runtime-controls'
import { saveControlHandle } from './control-journal'
import type { AgentDirectoryItem, AgentSession, SessionMessage } from '@/api/types'
import type { ClarificationCommand } from '@/api/clarification-custody'
import { useAuthStore } from '@/shared/auth/store'
import { ApiError, connectAuthenticatedEventStream } from '@sdlc/ui/lib'
import userEvent from '@testing-library/user-event'

vi.mock('@/api/fleet', () => ({
  getSession: vi.fn(),
  listAgentDirectory: vi.fn(),
  listSessionAgentRuns: vi.fn(),
  listSessionMessages: vi.fn(),
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
  listPendingAnswerCommands: vi.fn(),
  deliverAnswerCommand: vi.fn(),
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
const sessionFixture = {
  id: 'session1',
  user_id: 'owner',
  user_display_name: 'Owner',
  primary_agent_id: 'agent1',
  primary_agent_name: 'PM',
  title: 'Task',
  state: 'active',
  pending_delivery: false,
  visibility: 'private',
  task_key: 'TASK-1',
  task_bound: true,
} as AgentSession

function renderPage(tab = 'dialogue', search = '') {
  const router = createMemoryRouter(
    [
      { path: '/chats/:sessionId', element: <ChatDetailPage /> },
      { path: '/chats', element: <p>Список</p> },
    ],
    { initialEntries: [`/chats/session1?tab=${tab}${search ? `&${search}` : ''}`] },
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
  vi.mocked(chats.listPendingAnswerCommands).mockResolvedValue([])
  sessionStorage.clear()
  vi.clearAllMocks()
  vi.mocked(lookupRuntimeControl).mockRejectedValue(new Error('Lookup unavailable'))
  useAuthStore.setState({
    userId: 'owner',
    token: 'fixture-token',
    signingOut: false,
    permissions: ['sessions:write_own'],
  })
  vi.mocked(fleet.getSession).mockResolvedValue(sessionFixture)
  vi.mocked(fleet.listAgentDirectory).mockResolvedValue([
    { id: 'agent1', product_role: 'executor', status: 'running' } as AgentDirectoryItem,
  ])
  vi.mocked(fleet.listSessionAgentRuns).mockResolvedValue([])
  vi.mocked(fleet.listSessionMessages).mockResolvedValue([])
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

describe('PM delivered answer continuation receipt', () => {
  const command: ClarificationCommand = {
    id: 'delivered-pm-original',
    session_id: 'session1',
    question_id: 'q1',
    request: {
      expected_question_version: 1,
      requirement_revision: 3,
      selected_option_ids: ['project'],
      text: 'Original delivered answer',
      comment: null,
      idempotency_key: 'original-delivered-key',
    },
    payload_sha256: 'a'.repeat(64),
    state: 'delivered',
    continuation_state: 'pending',
    answer: {
      id: 'saved-answer',
      question_id: 'q1',
      question_version: 1,
      requirement_revision: 3,
      selected_option_ids: ['project'],
      text: 'Original delivered answer',
      comment: null,
      author_subject: 'subject-owner',
      created_at: '2026-10-10T00:00:00Z',
    },
    rejection_status: null,
    created_at: '2026-10-10T00:00:00Z',
    updated_at: '2026-10-10T00:00:00Z',
  }

  it('reloads delivered pending custody and recovers only the original command ID', async () => {
    vi.mocked(chats.listPendingAnswerCommands).mockResolvedValue([command])
    vi.mocked(chats.deliverAnswerCommand).mockResolvedValue(command)
    renderPage('clarification')
    await screen.findByText(/доставлен, продолжение PM требует сверки/)
    expect(screen.getByRole('button', { name: 'Сохранить ответ' })).toBeDisabled()
    expect(chats.deliverAnswerCommand).not.toHaveBeenCalled()
    await userEvent.click(screen.getByRole('button', { name: 'Продолжить исходную команду' }))
    await screen.findByText('Ответ доставлен. Продолжение PM ещё не подтверждено.')
    expect(chats.deliverAnswerCommand).toHaveBeenCalledExactlyOnceWith('session1', command.id)
    expect(chats.answerClarification).not.toHaveBeenCalled()
    expect(chats.confirmRequirements).not.toHaveBeenCalled()
    expect(screen.getByText(command.request.text!)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Сохранить ответ' })).toBeDisabled()
    expect(command.request.idempotency_key).toBe('original-delivered-key')
    expect(sessionStorage.length).toBe(0)
  })

  it('keeps delivered pending custody after recovery authorization fails', async () => {
    vi.mocked(chats.listPendingAnswerCommands).mockResolvedValue([command])
    vi.mocked(chats.deliverAnswerCommand).mockRejectedValue(new ApiError(403, 'Доступ отозван'))
    renderPage('clarification')
    await screen.findByText(/доставлен, продолжение PM требует сверки/)
    await userEvent.click(screen.getByRole('button', { name: 'Продолжить исходную команду' }))
    await screen.findByText('Доступ отозван')
    expect(screen.getByText(command.request.text!)).toBeInTheDocument()
    expect(chats.deliverAnswerCommand).toHaveBeenCalledExactlyOnceWith('session1', command.id)
    expect(chats.answerClarification).not.toHaveBeenCalled()
    expect(chats.confirmRequirements).not.toHaveBeenCalled()
    await act(async () => {
      useAuthStore.setState({ userId: 'read-only-operator' })
    })
    expect(screen.getByRole('button', { name: 'Продолжить исходную команду' })).toBeDisabled()
  })

  it('settles continuation only on a confirmed receipt and never claims requirements published', async () => {
    vi.mocked(chats.listPendingAnswerCommands).mockResolvedValue([command])
    vi.mocked(chats.deliverAnswerCommand).mockImplementation(async () => {
      vi.mocked(chats.listPendingAnswerCommands).mockResolvedValue([])
      return { ...command, continuation_state: 'confirmed' }
    })
    renderPage('clarification')
    await screen.findByText(/доставлен, продолжение PM требует сверки/)
    await userEvent.click(screen.getByRole('button', { name: 'Продолжить исходную команду' }))
    await screen.findByText(
      'Ответ доставлен. Продолжение PM подтверждено. Требования ещё не опубликованы.',
    )
    await waitFor(() =>
      expect(
        screen.queryByRole('button', { name: 'Продолжить исходную команду' }),
      ).not.toBeInTheDocument(),
    )
    expect(chats.answerClarification).not.toHaveBeenCalled()
    expect(chats.confirmRequirements).not.toHaveBeenCalled()
  })

  it('keeps idle PM free-form disabled with an explicit unsupported contract reason', async () => {
    vi.mocked(chats.getChatControls).mockResolvedValue({
      can_send: false,
      can_steer: false,
      can_stop: false,
      active_run_id: null,
      blocked_reason: 'pm_idle_prompt_contract_unavailable',
    })
    renderPage()
    await screen.findByText('Новый запуск PM без сохранённого ответа пока недоступен')
    expect(screen.getByRole('textbox', { name: 'Сообщение агенту' })).toBeDisabled()
    expect(fleet.createSessionMessage).not.toHaveBeenCalled()
    expect(fleet.steerSessionRun).not.toHaveBeenCalled()
  })

  it('routes an authorized task-bound composer to steer, never ordinary message or answer POST', async () => {
    vi.mocked(chats.getChatControls).mockResolvedValue({
      can_send: false,
      can_steer: true,
      can_stop: true,
      active_run_id: 'pm-run',
      blocked_reason: null,
    })
    vi.mocked(fleet.steerSessionRun).mockResolvedValue({
      session_id: 'session1',
      run_id: 'pm-run',
      runtime_run_id: 'native-pm',
      accepted: true,
      state: 'running',
      message: 'Guidance acknowledged',
    })
    renderPage()
    const editor = await screen.findByLabelText('Уточнение активному запуску')
    fireEvent.change(editor, { target: { value: 'Original owner guidance' } })
    await userEvent.click(screen.getByRole('button', { name: 'Передать уточнение запуску' }))
    await waitFor(() =>
      expect(fleet.steerSessionRun).toHaveBeenCalledExactlyOnceWith(
        'session1',
        'pm-run',
        { input: 'Original owner guidance' },
        expect.any(String),
      ),
    )
    expect(fleet.createSessionMessage).not.toHaveBeenCalled()
    expect(chats.answerClarification).not.toHaveBeenCalled()
    expect(chats.confirmRequirements).not.toHaveBeenCalled()
  })
  it.each(['session', 'task-context', 'chat-controls', 'changed-owner'])(
    'disables bound Stop after losing %s authority despite cached controls',
    async (authority) => {
      vi.mocked(chats.getChatControls).mockResolvedValue({
        can_send: false,
        can_steer: true,
        can_stop: true,
        active_run_id: 'pm-run',
        blocked_reason: null,
      })
      const { client } = renderPage()
      const stop = await screen.findByRole('button', { name: 'Остановить запуск' })
      await waitFor(() => expect(stop).toBeEnabled())
      if (authority === 'changed-owner') {
        act(() => {
          client.setQueryData(['session', 'session1'], (value: AgentSession) => ({
            ...value,
            user_id: 'another-owner',
          }))
        })
      } else {
        const denied = new ApiError(403, 'Access revoked')
        if (authority === 'session') vi.mocked(fleet.getSession).mockRejectedValue(denied)
        if (authority === 'task-context') vi.mocked(chats.getTaskContext).mockRejectedValue(denied)
        if (authority === 'chat-controls')
          vi.mocked(chats.getChatControls).mockRejectedValue(denied)
        await act(async () => {
          await client.refetchQueries({ queryKey: [authority, 'session1'], exact: true })
        })
      }
      if (authority === 'session') {
        await screen.findByText('Нет доступа к чату или чат недоступен.')
        expect(screen.queryByRole('button', { name: 'Остановить запуск' })).not.toBeInTheDocument()
      } else {
        await waitFor(() =>
          expect(screen.getByRole('button', { name: 'Остановить запуск' })).toBeDisabled(),
        )
        fireEvent.click(screen.getByRole('button', { name: 'Остановить запуск' }))
      }
      expect(fleet.stopSessionRun).not.toHaveBeenCalled()
      expect(sessionStorage.getItem('fleet-runtime-controls:v1:owner:session1')).toBeNull()
    },
  )
})

describe('production chat', () => {
  it('retains the original confirmation key while switching tabs after an unknown receipt', async () => {
    vi.mocked(chats.confirmRequirements).mockRejectedValueOnce(new Error('Unknown confirmation'))
    renderPage('requirements')
    fireEvent.click(await screen.findByRole('checkbox', { name: /Подтверждаю цель/ }))
    fireEvent.click(screen.getByRole('button', { name: 'Подтвердить редакцию 3' }))
    await screen.findByText('Unknown confirmation')
    const original = vi.mocked(chats.confirmRequirements).mock.calls.at(0)
    await userEvent.click(screen.getByRole('tab', { name: /Диалог/ }))
    await waitFor(() =>
      expect(screen.getByRole('tab', { name: /Диалог/ })).toHaveAttribute('aria-selected', 'true'),
    )
    await userEvent.click(screen.getByRole('tab', { name: /Требования/ }))
    expect(screen.getByRole('button', { name: 'Подтвердить редакцию 3' })).toBeEnabled()
    fireEvent.click(screen.getByRole('button', { name: 'Подтвердить редакцию 3' }))
    await waitFor(() => expect(chats.confirmRequirements).toHaveBeenCalledTimes(2))
    expect(vi.mocked(chats.confirmRequirements).mock.calls.at(1)).toEqual(original)
    await screen.findByText('Подтверждение сохранено. Следующее назначение проверяется отдельно.')
  })

  it('accepts the matching late confirmation while its tab is hidden', async () => {
    let resolve!: (value: Awaited<ReturnType<typeof chats.confirmRequirements>>) => void
    vi.mocked(chats.confirmRequirements).mockReturnValue(
      new Promise((value) => {
        resolve = value
      }),
    )
    renderPage('requirements')
    fireEvent.click(await screen.findByRole('checkbox', { name: /Подтверждаю цель/ }))
    fireEvent.click(screen.getByRole('button', { name: 'Подтвердить редакцию 3' }))
    await waitFor(() => expect(chats.confirmRequirements).toHaveBeenCalledOnce())
    await userEvent.click(screen.getByRole('tab', { name: /Диалог/ }))
    await waitFor(() =>
      expect(screen.getByRole('tab', { name: /Диалог/ })).toHaveAttribute('aria-selected', 'true'),
    )
    await act(async () =>
      resolve({
        id: 'confirmation',
        task_id: 'task',
        revision: 3,
        content_hash: 'hash3',
        owner_subject: 'subject-owner',
        created_at: '2026-10-01T12:00:00Z',
        stage: 'Backlog',
      }),
    )
    expect(
      sessionStorage.getItem('fleet-control.chat-dispatch.v1:session1:confirmation'),
    ).toBeNull()
    await userEvent.click(screen.getByRole('tab', { name: /Требования/ }))
    expect(screen.getByRole('button', { name: 'Подтвердить редакцию 3' })).toBeDisabled()
  })
  it('holds a foreign answer receipt without discarding its draft', async () => {
    vi.mocked(chats.answerClarification).mockResolvedValue({
      id: 'foreign-answer',
      question_id: 'foreign-question',
      question_version: 1,
      requirement_revision: 3,
      selected_option_ids: ['project'],
      text: null,
      comment: null,
      author_subject: 'subject-owner',
      created_at: '2026-10-01T12:00:00Z',
    })
    renderPage('clarification')
    fireEvent.click(await screen.findByRole('radio', { name: /Участники проекта/ }))
    fireEvent.change(screen.getByLabelText('Комментарий'), { target: { value: 'Retained draft' } })
    fireEvent.click(screen.getByRole('button', { name: 'Сохранить ответ' }))
    await screen.findByText('Не удалось проверить подтверждение сохранения ответа.')
    expect(screen.getByLabelText('Комментарий')).toHaveValue('Retained draft')
    expect(sessionStorage.getItem('fleet-control.chat-dispatch.v1:session1:answer')).not.toBeNull()
    expect(
      screen.queryByText('Ответ сохранён. Требования ещё не опубликованы.'),
    ).not.toBeInTheDocument()
  })

  it('does not release an unknown answer after a later definitive rejection', async () => {
    vi.mocked(chats.answerClarification)
      .mockRejectedValueOnce(new Error('Lost first answer'))
      .mockRejectedValueOnce(new ApiError(409, 'Later version conflict'))
    renderPage('clarification')
    fireEvent.click(await screen.findByRole('radio', { name: /Участники проекта/ }))
    fireEvent.click(screen.getByRole('button', { name: 'Сохранить ответ' }))
    await screen.findByText('Lost first answer')
    const original = vi.mocked(chats.answerClarification).mock.calls.at(0)?.[2]
    expect(original).toBeDefined()
    fireEvent.click(screen.getByRole('button', { name: 'Повторить исходный ответ' }))
    await screen.findByText('Later version conflict')
    expect(vi.mocked(chats.answerClarification).mock.calls.at(1)?.[2]).toEqual(original)
    expect(sessionStorage.getItem('fleet-control.chat-dispatch.v1:session1:answer')).not.toBeNull()
    expect(screen.getByRole('radio', { name: /Участники проекта/ })).toBeDisabled()
  })

  it('holds unknown requirements confirmation through reload', async () => {
    vi.mocked(chats.confirmRequirements).mockRejectedValue(new Error('Lost confirmation receipt'))
    renderPage('requirements')
    fireEvent.click(await screen.findByRole('checkbox', { name: /Подтверждаю цель/ }))
    fireEvent.click(screen.getByRole('button', { name: 'Подтвердить редакцию 3' }))
    await screen.findByText('Lost confirmation receipt')
    expect(
      sessionStorage.getItem('fleet-control.chat-dispatch.v1:session1:confirmation'),
    ).not.toBeNull()
    cleanup()
    renderPage('requirements')
    await screen.findByText(
      'Исход подтверждения требует сверки после перезагрузки. Новая команда заблокирована.',
    )
    expect(screen.getByRole('button', { name: 'Подтвердить редакцию 3' })).toBeDisabled()
    expect(chats.confirmRequirements).toHaveBeenCalledTimes(1)
  })

  it('holds an unknown answer through reload without saving its private payload', async () => {
    vi.mocked(chats.answerClarification).mockRejectedValue(new Error('Lost answer receipt'))
    const view = renderPage('clarification')
    fireEvent.click(await screen.findByRole('radio', { name: /Участники проекта/ }))
    fireEvent.change(screen.getByLabelText('Комментарий'), {
      target: { value: 'Private answer comment' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Сохранить ответ' }))
    await screen.findByText('Lost answer receipt')
    const raw = sessionStorage.getItem('fleet-control.chat-dispatch.v1:session1:answer')!
    expect(raw).not.toContain('Private answer comment')
    expect(raw).not.toContain('owned-fixture-token')
    view.router.dispose()
    // A new mounted form models reload; storage survives, private form memory does not.
    cleanup()
    renderPage('clarification')
    await screen.findByText(
      'Исход сохранения ответа требует сверки после перезагрузки. Новый ответ заблокирован.',
    )
    expect(screen.getByRole('button', { name: 'Сохранить ответ' })).toBeDisabled()
    expect(chats.answerClarification).toHaveBeenCalledTimes(1)
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

  it('recovers server custody after reload even when new-answer permission closed', async () => {
    const command: ClarificationCommand = {
      id: 'durable-original',
      session_id: 'session1',
      question_id: 'q1',
      request: {
        expected_question_version: 1,
        requirement_revision: 3,
        selected_option_ids: ['project'],
        text: 'Retained only on server',
        comment: null,
        idempotency_key: 'original-key',
      },
      payload_sha256: 'a'.repeat(64),
      state: 'uncertain',
      answer: null,
      rejection_status: null,
      created_at: '2026-10-10T00:00:00Z',
      updated_at: '2026-10-10T00:00:00Z',
    }
    vi.mocked(chats.listPendingAnswerCommands).mockResolvedValue([command])
    vi.mocked(chats.deliverAnswerCommand).mockResolvedValue(command)
    vi.mocked(chats.getTaskContext).mockResolvedValue({
      ...context,
      tracker: { ...context.tracker!, permissions: { can_answer: false, can_confirm: false } },
    })
    renderPage('clarification')
    await screen.findByText('Retained only on server')
    expect(screen.getByRole('button', { name: 'Сохранить ответ' })).toBeDisabled()
    await userEvent.click(screen.getByRole('button', { name: 'Продолжить исходную команду' }))
    await waitFor(() =>
      expect(chats.deliverAnswerCommand).toHaveBeenCalledWith('session1', 'durable-original'),
    )
    expect(chats.answerClarification).not.toHaveBeenCalled()
    expect(chats.confirmRequirements).not.toHaveBeenCalled()
    expect(sessionStorage.length).toBe(0)
    expect(screen.getByRole('button', { name: 'Сохранить ответ' })).toBeDisabled()
  })
  it('holds new answer creation until server journal readback completes', async () => {
    let resolve!: (values: ClarificationCommand[]) => void
    vi.mocked(chats.listPendingAnswerCommands).mockImplementation(
      () =>
        new Promise((done) => {
          resolve = done
        }),
    )
    renderPage('clarification')
    await screen.findByRole('radio', { name: /Участники проекта/ })
    expect(screen.getByRole('button', { name: 'Сохранить ответ' })).toBeDisabled()
    expect(chats.answerClarification).not.toHaveBeenCalled()
    await act(async () => {
      resolve([])
    })
    await userEvent.click(screen.getByRole('radio', { name: /Участники проекта/ }))
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Сохранить ответ' })).toBeEnabled(),
    )
  })
  it('journal readback failure cannot authorize a new key', async () => {
    vi.mocked(chats.listPendingAnswerCommands).mockRejectedValue(new Error('Journal unavailable'))
    renderPage('clarification')
    await screen.findByText('Journal unavailable')
    expect(screen.getByRole('button', { name: 'Сохранить ответ' })).toBeDisabled()
    expect(chats.answerClarification).not.toHaveBeenCalled()
  })
  describe('answer custody permissions and session isolation', () => {
    const command: ClarificationCommand = {
      id: 'persisted-original',
      session_id: 'session1',
      question_id: 'q1',
      request: {
        expected_question_version: 1,
        requirement_revision: 3,
        selected_option_ids: ['project'],
        text: 'Original retained answer',
        comment: 'Original retained comment',
        idempotency_key: 'persisted-original-key',
      },
      payload_sha256: 'b'.repeat(64),
      state: 'uncertain',
      answer: null,
      rejection_status: null,
      created_at: '2026-10-10T00:00:00Z',
      updated_at: '2026-10-10T00:00:00Z',
    }

    it.each(['session owner', 'Tracker access', 'journal access'])(
      'blocks cached server-command delivery after %s is revoked',
      async (authority) => {
        vi.mocked(chats.listPendingAnswerCommands).mockResolvedValue([command])
        const { client } = renderPage('clarification')
        const recover = await screen.findByRole('button', { name: 'Продолжить исходную команду' })
        await waitFor(() => expect(recover).toBeEnabled())
        if (authority === 'session owner') {
          const session = await fleet.getSession('session1')
          vi.mocked(fleet.getSession).mockResolvedValue({ ...session, user_id: 'other-owner' })
          await act(() => client.invalidateQueries({ queryKey: ['session', 'session1'] }))
        } else if (authority === 'Tracker access') {
          vi.mocked(chats.getTaskContext).mockRejectedValue(new ApiError(403, 'Access revoked'))
          await act(() => client.invalidateQueries({ queryKey: ['task-context', 'session1'] }))
        } else {
          vi.mocked(chats.listPendingAnswerCommands).mockRejectedValue(
            new ApiError(403, 'Journal access revoked'),
          )
          await act(() =>
            client.invalidateQueries({ queryKey: ['clarification-commands', 'session1'] }),
          )
          expect(client.getQueryState(['clarification-commands', 'session1'])?.status).toBe('error')
        }
        await waitFor(() => expect(recover).toBeDisabled())
        await userEvent.click(recover)
        expect(screen.getByText(command.request.text!)).toBeVisible()
        expect(
          client.getQueryData<ClarificationCommand[]>(['clarification-commands', 'session1']),
        ).toEqual([command])
        expect(chats.deliverAnswerCommand).not.toHaveBeenCalled()
        expect(chats.answerClarification).not.toHaveBeenCalled()
        expect(screen.getByRole('button', { name: 'Сохранить ответ' })).toBeDisabled()
      },
    )

    it('blocks an in-memory original retry after journal access is revoked', async () => {
      vi.mocked(chats.listPendingAnswerCommands)
        .mockResolvedValueOnce([])
        .mockRejectedValue(new ApiError(403, 'Journal access revoked'))
      vi.mocked(chats.answerClarification).mockRejectedValue(new Error('Answer outcome unknown'))
      renderPage('clarification')
      await userEvent.click(await screen.findByRole('radio', { name: /Участники проекта/ }))
      await userEvent.click(screen.getByRole('button', { name: 'Сохранить ответ' }))
      await screen.findByText('Journal access revoked')
      const retry = screen.getByRole('button', { name: 'Повторить исходный ответ' })
      expect(retry).toBeDisabled()
      await userEvent.click(retry)
      expect(chats.answerClarification).toHaveBeenCalledTimes(1)
      expect(chats.deliverAnswerCommand).not.toHaveBeenCalled()
      expect(screen.getByRole('radio', { name: /Участники проекта/ })).toBeChecked()
    })

    it('does not carry a pending delivery or its late result into another session', async () => {
      let finish!: (result: ClarificationCommand) => void
      vi.mocked(chats.deliverAnswerCommand).mockImplementation(
        () =>
          new Promise((resolve) => {
            finish = resolve
          }),
      )
      vi.mocked(chats.listPendingAnswerCommands).mockImplementation(async (id) =>
        id === 'session1' ? [command] : [],
      )
      const session = await fleet.getSession('session1')
      vi.mocked(fleet.getSession).mockImplementation(async (id) => ({ ...session, id }))
      const { router } = renderPage('clarification')
      await userEvent.click(
        await screen.findByRole('button', { name: 'Продолжить исходную команду' }),
      )
      await waitFor(() => expect(chats.deliverAnswerCommand).toHaveBeenCalledTimes(1))
      await act(() => router.navigate('/chats/session2?tab=clarification'))
      await waitFor(() => expect(chats.listPendingAnswerCommands).toHaveBeenCalledWith('session2'))
      expect(screen.queryByText(command.request.text!)).not.toBeInTheDocument()
      expect(
        screen.queryByRole('button', { name: 'Продолжить исходную команду' }),
      ).not.toBeInTheDocument()
      await act(async () => {
        finish(command)
      })
      expect(
        screen.queryByText('Доставка исходного ответа ещё не подтверждена.'),
      ).not.toBeInTheDocument()
      expect(chats.deliverAnswerCommand).toHaveBeenCalledExactlyOnceWith('session1', command.id)
      expect(chats.answerClarification).not.toHaveBeenCalled()
      await act(() => router.navigate('/chats/session1?tab=clarification'))
      await screen.findByText(command.request.text!)
      await waitFor(() =>
        expect(screen.getByRole('button', { name: 'Продолжить исходную команду' })).toBeEnabled(),
      )
      expect(chats.deliverAnswerCommand).toHaveBeenCalledTimes(1)
    })

    it('keeps stored custody distinct from delivery after the question changes', async () => {
      let stored: ClarificationCommand[] = []
      vi.mocked(chats.listPendingAnswerCommands).mockImplementation(async () => stored)
      vi.mocked(chats.answerClarification).mockImplementation(async (_id, _question, payload) => {
        stored = [{ ...command, state: 'stored', request: payload }]
        throw new Error('Stored command delivery unknown')
      })
      vi.mocked(chats.deliverAnswerCommand).mockImplementation(async () => {
        stored = [{ ...stored[0]!, state: 'uncertain' }]
        return stored[0]!
      })
      const { client } = renderPage('clarification')
      await userEvent.click(await screen.findByRole('radio', { name: /Участники проекта/ }))
      fireEvent.change(screen.getByLabelText('Свой вариант или детали'), {
        target: { value: command.request.text },
      })
      fireEvent.change(screen.getByLabelText('Комментарий'), {
        target: { value: command.request.comment },
      })
      await userEvent.click(screen.getByRole('button', { name: 'Сохранить ответ' }))
      await screen.findByRole('heading', { name: 'Сохранённый ответ: ожидает доставки' })
      const original = structuredClone(vi.mocked(chats.answerClarification).mock.calls[0])
      const persisted = structuredClone(stored[0]!)
      expect(screen.getByLabelText('Свой вариант или детали')).toHaveValue(command.request.text)
      expect(screen.getByLabelText('Комментарий')).toHaveValue(command.request.comment)
      expect(screen.getByLabelText('Комментарий')).toBeDisabled()
      const freshQuestions = [
        { ...question, version: 2, requirement_revision: 4 },
        { ...question, id: 'q2', text: 'Другой вопрос' },
      ]
      vi.mocked(chats.getClarifications).mockResolvedValue({ questions: freshQuestions })
      await act(async () => {
        client.setQueryData(['clarifications', 'session1'], {
          questions: freshQuestions,
        })
      })
      await screen.findByText('Вопрос изменился. Несохранённый ответ сохранён отдельно.')
      expect(
        screen.getByRole('button', { name: 'Перенести черновик и проверить новый вопрос' }),
      ).toBeDisabled()
      await userEvent.click(screen.getByRole('button', { name: /2\. Другой вопрос/ }))
      expect(screen.getByRole('button', { name: 'Сохранить ответ' })).toBeDisabled()
      await userEvent.click(screen.getByRole('button', { name: 'Продолжить исходную команду' }))
      await screen.findByText('Доставка исходного ответа ещё не подтверждена.')
      await screen.findByRole('heading', { name: 'Сохранённый ответ: требует сверки' })
      expect(stored).toEqual([{ ...persisted, state: 'uncertain' }])
      expect(stored[0]!.request).toEqual(original![2])
      expect(chats.deliverAnswerCommand).toHaveBeenCalledExactlyOnceWith('session1', command.id)
      expect(chats.answerClarification).toHaveBeenCalledTimes(1)
      expect(screen.getByRole('button', { name: 'Сохранить ответ' })).toBeDisabled()
      expect(
        screen.queryByText('Исходный ответ подтверждён. Требования ещё не опубликованы.'),
      ).not.toBeInTheDocument()
      expect(
        screen.queryByText('Ответ сохранён. Требования ещё не опубликованы.'),
      ).not.toBeInTheDocument()
      expect(chats.confirmRequirements).not.toHaveBeenCalled()
      expect(sessionStorage.length).toBe(0)
    })

    it('does not let a read-only operator confirm the owner revision despite cached permissions', async () => {
      useAuthStore.setState({
        userId: 'operator',
        permissions: ['sessions:read_all', 'agents:manage'],
      })
      renderPage('requirements')
      const consent = await screen.findByRole('checkbox', { name: /Подтверждаю цель/ })
      const confirm = screen.getByRole('button', { name: 'Подтвердить редакцию 3' })
      expect(consent).toBeDisabled()
      expect(confirm).toBeDisabled()
      await userEvent.click(consent)
      await userEvent.click(confirm)
      expect(consent).not.toBeChecked()
      expect(chats.confirmRequirements).not.toHaveBeenCalled()
      expect(chats.listPendingAnswerCommands).not.toHaveBeenCalled()
      expect(chats.deliverAnswerCommand).not.toHaveBeenCalled()
    })
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
  describe('clarification UX regression', () => {
    it('preserves independent choices, custom text and comments across questions and tabs', async () => {
      vi.mocked(chats.getClarifications).mockResolvedValue({
        questions: [question, { ...question, id: 'q2', text: 'Второй вопрос', mode: 'text' }],
      })
      const { router } = renderPage('clarification', 'returnTo=%2Fchats%3Fsearch%3Dtask')
      await userEvent.click(await screen.findByRole('radio', { name: /Участники проекта/ }))
      fireEvent.change(screen.getByLabelText('Свой вариант или детали'), {
        target: { value: 'Первый вариант' },
      })
      fireEvent.change(screen.getByLabelText('Комментарий'), {
        target: { value: 'Первый комментарий' },
      })
      await userEvent.click(screen.getByRole('button', { name: /2\. Второй вопрос/ }))
      expect(router.state.location.search).toContain('question=q2')
      fireEvent.change(screen.getByLabelText('Ваш ответ'), { target: { value: 'Второй вариант' } })
      fireEvent.change(screen.getByLabelText('Комментарий'), {
        target: { value: 'Второй комментарий' },
      })
      await userEvent.click(screen.getByRole('tab', { name: /Требования/ }))
      await userEvent.click(screen.getByRole('tab', { name: /Уточнения/ }))
      expect(screen.getByLabelText('Ваш ответ')).toHaveValue('Второй вариант')
      expect(screen.getByLabelText('Комментарий')).toHaveValue('Второй комментарий')
      await userEvent.click(screen.getByRole('button', { name: /1\. Кто видит задачи/ }))
      expect(screen.getByRole('radio', { name: /Участники проекта/ })).toBeChecked()
      expect(screen.getByLabelText('Свой вариант или детали')).toHaveValue('Первый вариант')
      expect(screen.getByLabelText('Комментарий')).toHaveValue('Первый комментарий')
      expect(new URLSearchParams(router.state.location.search).get('returnTo')).toBe(
        '/chats?search=task',
      )
      expect(chats.answerClarification).not.toHaveBeenCalled()
      expect(sessionStorage.length).toBe(0)
      const stored = Array.from({ length: localStorage.length }, (_, index) =>
        localStorage.getItem(localStorage.key(index)!),
      ).join('')
      expect(stored).not.toContain('Первый вариант')
      expect(stored).not.toContain('Второй комментарий')
    })

    it.each([409, 412])(
      'requires explicit recheck after conflict %s even when automatic readback returns the same version',
      async (status) => {
        vi.mocked(chats.answerClarification).mockRejectedValue(
          new ApiError(status, 'Редакция изменилась'),
        )
        renderPage('clarification')
        await userEvent.click(await screen.findByRole('radio', { name: /Участники проекта/ }))
        fireEvent.change(screen.getByLabelText('Комментарий'), {
          target: { value: 'Не терять ответ' },
        })
        await userEvent.click(screen.getByRole('button', { name: 'Сохранить ответ' }))
        await screen.findByText('Редакция изменилась')
        await waitFor(() => expect(chats.getClarifications).toHaveBeenCalledTimes(2))
        expect(screen.getByLabelText('Комментарий')).toHaveValue('Не терять ответ')
        expect(screen.getByRole('radio', { name: /Участники проекта/ })).toBeChecked()
        const save = screen.getByRole('button', { name: 'Сохранить ответ' })
        expect(save).toBeDisabled()
        await userEvent.click(save)
        expect(chats.answerClarification).toHaveBeenCalledTimes(1)
        await userEvent.click(screen.getByRole('button', { name: 'Проверить актуальный вопрос' }))
        await waitFor(() => expect(save).toBeEnabled())
        expect(screen.getByLabelText('Комментарий')).toHaveValue('Не терять ответ')
        expect(chats.getClarifications).toHaveBeenCalledTimes(3)
        expect(chats.answerClarification).toHaveBeenCalledTimes(1)
        const originalKey = vi.mocked(chats.answerClarification).mock.calls[0]![2].idempotency_key
        await userEvent.click(save)
        await waitFor(() => expect(chats.answerClarification).toHaveBeenCalledTimes(2))
        expect(vi.mocked(chats.answerClarification).mock.calls[1]![2]).toEqual(
          expect.objectContaining({
            selected_option_ids: ['project'],
            comment: 'Не терять ответ',
          }),
        )
        expect(vi.mocked(chats.answerClarification).mock.calls[1]![2].idempotency_key).not.toBe(
          originalKey,
        )
      },
    )

    it('keeps a conflicting draft blocked when explicit recheck fails', async () => {
      vi.mocked(chats.answerClarification).mockRejectedValue(
        new ApiError(409, 'Редакция изменилась'),
      )
      renderPage('clarification')
      await userEvent.click(await screen.findByRole('radio', { name: /Участники проекта/ }))
      fireEvent.change(screen.getByLabelText('Комментарий'), {
        target: { value: 'Сохранить при ошибке' },
      })
      await userEvent.click(screen.getByRole('button', { name: 'Сохранить ответ' }))
      await screen.findByText('Редакция изменилась')
      await waitFor(() => expect(chats.getClarifications).toHaveBeenCalledTimes(2))
      vi.mocked(chats.getClarifications).mockRejectedValue(new Error('Recheck unavailable'))
      await userEvent.click(screen.getByRole('button', { name: 'Проверить актуальный вопрос' }))
      await waitFor(() =>
        expect(screen.getAllByText('Recheck unavailable').length).toBeGreaterThan(0),
      )
      expect(screen.getByRole('button', { name: 'Сохранить ответ' })).toBeDisabled()
      expect(chats.answerClarification).toHaveBeenCalledTimes(1)
      vi.mocked(chats.getClarifications).mockResolvedValue({ questions: [question] })
      await userEvent.click(screen.getByRole('button', { name: 'Проверить актуальный вопрос' }))
      await waitFor(() =>
        expect(screen.getByRole('button', { name: 'Сохранить ответ' })).toBeEnabled(),
      )
      expect(screen.getByLabelText('Комментарий')).toHaveValue('Сохранить при ошибке')
      expect(screen.getByRole('radio', { name: /Участники проекта/ })).toBeChecked()
    })

    it('waits for all recheck reads and retains the server-custody hold', async () => {
      vi.mocked(chats.answerClarification).mockRejectedValue(new ApiError(409, 'Conflict'))
      renderPage('clarification')
      await userEvent.click(await screen.findByRole('radio', { name: /Участники проекта/ }))
      await userEvent.click(screen.getByRole('button', { name: 'Сохранить ответ' }))
      await screen.findByText('Conflict')
      await waitFor(() => expect(chats.listPendingAnswerCommands).toHaveBeenCalledTimes(2))
      let finish!: (commands: ClarificationCommand[]) => void
      vi.mocked(chats.listPendingAnswerCommands).mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            finish = resolve
          }),
      )
      const recheck = screen.getByRole('button', { name: 'Проверить актуальный вопрос' })
      await userEvent.click(recheck)
      expect(recheck).toBeDisabled()
      expect(screen.getByRole('button', { name: 'Сохранить ответ' })).toBeDisabled()
      await userEvent.click(recheck)
      expect(chats.listPendingAnswerCommands).toHaveBeenCalledTimes(3)
      await act(async () => {
        finish([
          {
            id: 'held',
            session_id: 'session1',
            question_id: 'q1',
            state: 'uncertain',
            payload_sha256: 'a'.repeat(64),
            answer: null,
            rejection_status: null,
            created_at: '2026-10-10T00:00:00Z',
            updated_at: '2026-10-10T00:00:00Z',
            request: {
              expected_question_version: 1,
              requirement_revision: 3,
              selected_option_ids: ['project'],
              text: null,
              comment: null,
              idempotency_key: 'original-held',
            },
          },
        ])
      })
      await waitFor(() => expect(recheck).not.toBeInTheDocument())
      expect(screen.getByRole('button', { name: 'Сохранить ответ' })).toBeDisabled()
      expect(screen.getByRole('radio', { name: /Участники проекта/ })).toBeChecked()
      expect(chats.answerClarification).toHaveBeenCalledTimes(1)
    })

    it('warns on browser unload and restores focus and the draft when Escape cancels leaving', async () => {
      const { router } = renderPage('clarification')
      fireEvent.change(await screen.findByLabelText('Комментарий'), {
        target: { value: 'Остаться с черновиком' },
      })
      const unload = new Event('beforeunload', { cancelable: true })
      window.dispatchEvent(unload)
      expect(unload.defaultPrevented).toBe(true)
      const back = screen.getByRole('link', { name: 'Вернуться к чатам' })
      await userEvent.click(back)
      await screen.findByRole('dialog', { name: 'Остались несохранённые изменения' })
      await userEvent.keyboard('{Escape}')
      await waitFor(() => expect(back).toHaveFocus())
      expect(router.state.location.pathname).toBe('/chats/session1')
      expect(screen.getByLabelText('Комментарий')).toHaveValue('Остаться с черновиком')
      await userEvent.click(back)
      await userEvent.click(await screen.findByRole('button', { name: 'Уйти без сохранения' }))
      await screen.findByText('Список')
      expect(router.state.location.pathname).toBe('/chats')
      const cleanUnload = new Event('beforeunload', { cancelable: true })
      window.dispatchEvent(cleanUnload)
      expect(cleanUnload.defaultPrevented).toBe(false)
    })

    it('supports keyboard tab navigation and URL history without losing the selected question', async () => {
      vi.mocked(chats.getClarifications).mockResolvedValue({
        questions: [question, { ...question, id: 'q2', text: 'Второй вопрос' }],
      })
      const { router } = renderPage('clarification', 'question=q2')
      const clarification = await screen.findByRole('tab', { name: /Уточнения/ })
      await screen.findByRole('button', { name: /2\. Второй вопрос/ })
      act(() => clarification.focus())
      await userEvent.keyboard('{ArrowRight}')
      await waitFor(() => expect(router.state.location.search).toContain('tab=requirements'))
      const requirements = screen.getByRole('tab', { name: /Требования/ })
      await waitFor(() => expect(requirements).toHaveAttribute('aria-selected', 'true'))
      expect(requirements).toHaveFocus()
      const revisionSelect = await screen.findByRole('combobox', { name: 'Редакция требований' })
      act(() => revisionSelect.focus())
      await act(() => router.navigate(-1))
      await waitFor(() => expect(clarification).toHaveAttribute('aria-selected', 'true'))
      await waitFor(() => expect(clarification).toHaveFocus())
      expect(screen.getByRole('button', { name: /2\. Второй вопрос/ })).toHaveAttribute(
        'aria-current',
        'true',
      )
      expect(screen.getByRole('radio', { name: /Участники проекта/ })).not.toBeChecked()
    })

    it('focuses the selected question after keyboard activation without preselecting a recommendation', async () => {
      vi.mocked(chats.getClarifications).mockResolvedValue({
        questions: [question, { ...question, id: 'q2', text: 'Второй вопрос' }],
      })
      renderPage('clarification')
      const second = await screen.findByRole('button', { name: /2\. Второй вопрос/ })
      act(() => second.focus())
      await userEvent.keyboard('{Enter}')
      expect(screen.getByRole('group', { name: 'Второй вопрос' })).toHaveFocus()
      expect(screen.getByRole('radio', { name: /Участники проекта/ })).not.toBeChecked()
      await userEvent.tab()
      expect(screen.getByRole('radio', { name: /Участники проекта/ })).toHaveFocus()
      await userEvent.keyboard(' ')
      expect(screen.getByRole('radio', { name: /Участники проекта/ })).toBeChecked()
    })

    it('keeps native radio keyboard selection and does not steal editor focus on refresh', async () => {
      vi.mocked(chats.getClarifications).mockResolvedValue({
        questions: [
          {
            ...question,
            options: [
              ...question.options,
              { id: 'team', label: 'Команда', consequences: 'Только команда' },
            ],
          },
        ],
      })
      const { client } = renderPage('clarification')
      const recommended = await screen.findByRole('radio', { name: /Участники проекта/ })
      await waitFor(() => expect(recommended).toBeEnabled())
      act(() => recommended.focus())
      await userEvent.keyboard('{ArrowDown}')
      expect(screen.getByRole('radio', { name: /Команда/ })).toBeChecked()
      expect(recommended).not.toBeChecked()
      const comment = screen.getByLabelText('Комментарий')
      act(() => comment.focus())
      await act(async () => {
        await client.invalidateQueries({ queryKey: ['clarifications', 'session1'] })
      })
      expect(comment).toHaveFocus()
    })

    it('announces command delivery in an existing polite live region', async () => {
      renderPage('clarification')
      await screen.findByRole('radio', { name: /Участники проекта/ })
      const status = screen.getByRole('status', { name: 'Статус команды' })
      expect(status).toHaveAttribute('aria-live', 'polite')
      expect(status).toHaveAttribute('aria-atomic', 'true')
      expect(status).toBeEmptyDOMElement()
      await userEvent.click(screen.getByRole('radio', { name: /Участники проекта/ }))
      await userEvent.click(screen.getByRole('button', { name: 'Сохранить ответ' }))
      await waitFor(() =>
        expect(status).toHaveTextContent('Ответ сохранён. Требования ещё не опубликованы.'),
      )
      expect(screen.getByRole('status', { name: 'Статус команды' })).toBe(status)
    })

    it('announces refreshed user-message delivery without replacing its status node', async () => {
      const pending: SessionMessage = {
        ...message('user-message', 'Ожидающее сообщение'),
        author_type: 'user' as const,
        delivery_state: 'pending',
      }
      vi.mocked(chats.getChatHistory).mockResolvedValue({ items: [pending], next_before: null })
      const { client } = renderPage()
      await screen.findByText('Ожидающее сообщение')
      const status = screen.getByRole('status', { name: 'Доставка сообщения' })
      expect(status).toHaveAttribute('aria-live', 'polite')
      const before = status.textContent
      await act(async () => {
        client.setQueryData(['chat-history', 'session1'], {
          pages: [{ items: [{ ...pending, delivery_state: 'completed' }], next_before: null }],
          pageParams: [undefined],
        })
      })
      expect(screen.getByRole('status', { name: 'Доставка сообщения' })).toBe(status)
      expect(status.textContent).not.toBe(before)
    })
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
    expect(submit).toBeDisabled()
    fireEvent.click(screen.getByRole('button', { name: 'Повторить исходный ответ' }))
    await waitFor(() => expect(chats.answerClarification).toHaveBeenCalledTimes(2))
    const calls = vi.mocked(chats.answerClarification).mock.calls
    expect(calls[1]).toEqual(calls[0])
  })
  it('can reconcile the original answer after the last question closes', async () => {
    vi.mocked(chats.answerClarification).mockRejectedValue(new Error('Connection interrupted'))
    const { client } = renderPage('clarification')
    fireEvent.click(await screen.findByRole('radio', { name: /Участники проекта/ }))
    vi.mocked(chats.getTaskContext).mockResolvedValue({
      ...context,
      tracker: {
        ...context.tracker!,
        waiting_reason: 'Ответы сохранены',
        permissions: { can_answer: false, can_confirm: true },
      },
    })
    vi.mocked(chats.getClarifications).mockResolvedValue({
      questions: [{ ...question, state: 'answered' }],
    })
    fireEvent.click(screen.getByRole('button', { name: 'Сохранить ответ' }))
    await screen.findByText('Connection interrupted')
    await screen.findByText('Ответы сохранены')
    await waitFor(() => {
      expect(
        client.getQueryData<chats.TaskContextResponse>(['task-context', 'session1'])?.tracker
          ?.permissions.can_answer,
      ).toBe(false)
      expect(
        client.getQueryData<{ questions: chats.Question[] }>(['clarifications', 'session1'])
          ?.questions[0]?.state,
      ).toBe('answered')
    })
    expect(screen.getByRole('radio', { name: /Участники проекта/ })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Сохранить ответ' })).toBeDisabled()
    const retry = screen.getByRole('button', { name: 'Повторить исходный ответ' })
    expect(retry).toBeEnabled()
    fireEvent.click(retry)
    await waitFor(() => expect(chats.answerClarification).toHaveBeenCalledTimes(2))
    const calls = vi.mocked(chats.answerClarification).mock.calls
    expect(calls[1]).toEqual(calls[0])
    expect(chats.confirmRequirements).not.toHaveBeenCalled()
  })
  it('cannot replay an uncertain answer after session ownership changes', async () => {
    vi.mocked(chats.answerClarification).mockRejectedValue(new Error('Connection interrupted'))
    const { client } = renderPage('clarification')
    fireEvent.click(await screen.findByRole('radio', { name: /Участники проекта/ }))
    fireEvent.click(screen.getByRole('button', { name: 'Сохранить ответ' }))
    await screen.findByText('Connection interrupted')
    vi.mocked(fleet.getSession).mockResolvedValue({
      id: 'session1',
      user_id: 'other',
      primary_agent_id: 'agent1',
      visibility: 'private',
    } as AgentSession)
    await act(() => client.invalidateQueries({ queryKey: ['session', 'session1'] }))
    const retry = screen.getByRole('button', { name: 'Повторить исходный ответ' })
    await waitFor(() => expect(retry).toBeDisabled())
    fireEvent.click(retry)
    expect(chats.answerClarification).toHaveBeenCalledTimes(1)
  })
  it('cannot replay an uncertain answer when fresh Tracker access fails', async () => {
    vi.mocked(chats.answerClarification).mockRejectedValue(new Error('Connection interrupted'))
    const { client } = renderPage('clarification')
    fireEvent.click(await screen.findByRole('radio', { name: /Участники проекта/ }))
    fireEvent.click(screen.getByRole('button', { name: 'Сохранить ответ' }))
    await screen.findByText('Connection interrupted')
    vi.mocked(chats.getTaskContext).mockRejectedValue(new ApiError(403, 'Project access revoked'))
    await act(() => client.invalidateQueries({ queryKey: ['task-context', 'session1'] }))
    const retry = screen.getByRole('button', { name: 'Повторить исходный ответ' })
    await waitFor(() => expect(retry).toBeDisabled())
    fireEvent.click(retry)
    expect(chats.answerClarification).toHaveBeenCalledTimes(1)
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
  it('does not replace an uncertain answer command by answering a different question', async () => {
    vi.mocked(chats.getClarifications).mockResolvedValue({
      questions: [question, { ...question, id: 'q2', text: 'Второй вопрос' }],
    })
    vi.mocked(chats.answerClarification).mockRejectedValue(new Error('Answer outcome unknown'))
    renderPage('clarification')
    fireEvent.click(await screen.findByRole('radio', { name: /Участники проекта/ }))
    fireEvent.click(screen.getByRole('button', { name: 'Сохранить ответ' }))
    await screen.findByText('Answer outcome unknown')
    fireEvent.click(screen.getByRole('button', { name: /2\. Второй вопрос/ }))
    expect(screen.getByText('Непроверенный ответ: Кто видит задачи?')).toBeVisible()
    fireEvent.click(screen.getByRole('radio', { name: /Участники проекта/ }))
    expect(screen.getByRole('button', { name: 'Сохранить ответ' })).toBeDisabled()
    expect(chats.answerClarification).toHaveBeenCalledTimes(1)
    fireEvent.click(screen.getByRole('button', { name: 'Повторить исходный ответ' }))
    await waitFor(() => expect(chats.answerClarification).toHaveBeenCalledTimes(2))
    const calls = vi.mocked(chats.answerClarification).mock.calls
    expect(calls[1]).toEqual(calls[0])
  })
  it('does not rekey an uncertain answer when a newer question version arrives', async () => {
    vi.mocked(chats.answerClarification).mockRejectedValue(new Error('Answer outcome unknown'))
    const { client } = renderPage('clarification')
    fireEvent.click(await screen.findByRole('radio', { name: /Участники проекта/ }))
    fireEvent.change(screen.getByLabelText('Комментарий'), { target: { value: 'Original input' } })
    fireEvent.click(screen.getByRole('button', { name: 'Сохранить ответ' }))
    await screen.findByText('Answer outcome unknown')
    client.setQueryData(['clarifications', 'session1'], {
      questions: [{ ...question, version: 2 }],
    })
    await screen.findByText('Вопрос изменился. Несохранённый ответ сохранён отдельно.')
    expect(
      screen.getByRole('button', { name: 'Перенести черновик и проверить новый вопрос' }),
    ).toBeDisabled()
    fireEvent.click(screen.getByRole('button', { name: 'Повторить исходный ответ' }))
    await waitFor(() => expect(chats.answerClarification).toHaveBeenCalledTimes(2))
    const calls = vi.mocked(chats.answerClarification).mock.calls
    expect(calls[1]).toEqual(calls[0])
  })
  it('unlocks the next question only after the original answer is acknowledged', async () => {
    vi.mocked(chats.getClarifications).mockResolvedValue({
      questions: [question, { ...question, id: 'q2', text: 'Второй вопрос' }],
    })
    vi.mocked(chats.answerClarification).mockRejectedValueOnce(new Error('Answer outcome unknown'))
    renderPage('clarification')
    fireEvent.click(await screen.findByRole('radio', { name: /Участники проекта/ }))
    fireEvent.click(screen.getByRole('button', { name: 'Сохранить ответ' }))
    await screen.findByText('Answer outcome unknown')
    fireEvent.click(screen.getByRole('button', { name: /2\. Второй вопрос/ }))
    expect(screen.getByRole('radio', { name: /Участники проекта/ })).toBeDisabled()
    fireEvent.click(screen.getByRole('button', { name: 'Повторить исходный ответ' }))
    await screen.findByText('Ответ сохранён. Требования ещё не опубликованы.')
    expect(
      screen.queryByRole('button', { name: 'Повторить исходный ответ' }),
    ).not.toBeInTheDocument()
    expect(screen.getByRole('radio', { name: /Участники проекта/ })).toBeEnabled()
    const calls = vi.mocked(chats.answerClarification).mock.calls
    expect(calls).toHaveLength(2)
    expect(calls[1]).toEqual(calls[0])
    expect(chats.confirmRequirements).not.toHaveBeenCalled()
  })
  it('confirms exact hash/revision separately', async () => {
    renderPage('requirements')
    const confirm = await screen.findByRole('button', { name: 'Подтвердить редакцию 3' })
    const revisionSelect = screen.getByRole('combobox', { name: 'Редакция требований' })
    expect(revisionSelect).toHaveClass('bg-surface', 'text-text-primary', 'border-border')
    expect(revisionSelect).toHaveClass('scheme-dark', '[[data-theme=light]_&]:scheme-light')
    expect(revisionSelect).toHaveValue('3')
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
  it('keeps ordinary task commands disabled even if a control response advertises steer', async () => {
    vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
    vi.mocked(chats.getChatControls).mockResolvedValue({
      can_send: false,
      can_steer: true,
      can_stop: true,
      active_run_id: 'run-1',
      blocked_reason: null,
    })
    renderPage()
    expect(await screen.findByLabelText('Сообщение агенту')).toBeDisabled()
    expect(screen.queryByLabelText('Уточнение активному запуску')).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Отправить сообщение' })).toBeDisabled()
    expect(fleet.createSessionMessage).not.toHaveBeenCalled()
    expect(fleet.steerSessionRun).not.toHaveBeenCalled()
    expect(fleet.stopSessionRun).not.toHaveBeenCalled()
  })

  it('never converts an uncertain steer into a new prompt after controls change', async () => {
    vi.mocked(fleet.getSession).mockResolvedValue({ ...sessionFixture, task_bound: false })
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
    vi.mocked(fleet.getSession).mockResolvedValue({ ...sessionFixture, task_bound: false })
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
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Остановить запуск' })).toBeEnabled(),
    )
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
    vi.mocked(fleet.getSession).mockResolvedValue({ ...sessionFixture, task_bound: false })
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
    vi.mocked(fleet.getSession).mockResolvedValue({ ...sessionFixture, task_bound: false })
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
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Остановить запуск' })).toBeEnabled(),
    )
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
    vi.mocked(fleet.getSession).mockResolvedValue({ ...sessionFixture, task_bound: false })
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
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Остановить запуск' })).toBeEnabled(),
    )
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
    vi.mocked(fleet.getSession).mockResolvedValue({ ...sessionFixture, task_bound: false })
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
    vi.mocked(fleet.getSession).mockResolvedValue({ ...sessionFixture, task_bound: false })
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
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Остановить запуск' })).toBeEnabled(),
    )
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
    vi.mocked(fleet.getSession).mockResolvedValue({ ...sessionFixture, task_bound: false })
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
    vi.mocked(fleet.getSession).mockResolvedValue({ ...sessionFixture, task_bound: false })
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
    vi.mocked(fleet.getSession).mockResolvedValue({ ...sessionFixture, task_bound: false })
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
    vi.mocked(fleet.getSession).mockResolvedValue({ ...sessionFixture, task_bound: false })
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
    vi.mocked(fleet.getSession).mockResolvedValue({ ...sessionFixture, task_bound: false })
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
      vi.mocked(fleet.getSession).mockResolvedValue({ ...sessionFixture, task_bound: false })
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
      const dispatch = async (input: string) => {
        if (operation === 'steer') fireEvent.change(editor, { target: { value: input } })
        await waitFor(() =>
          expect(
            screen.getByRole('button', {
              name: operation === 'steer' ? 'Передать уточнение запуску' : 'Остановить запуск',
            }),
          ).toBeEnabled(),
        )
        fireEvent.click(
          screen.getByRole('button', {
            name: operation === 'steer' ? 'Передать уточнение запуску' : 'Остановить запуск',
          }),
        )
      }
      await dispatch('Original guidance')
      fireEvent.click(
        await screen.findByRole('button', {
          name: operation === 'steer' ? 'Закрыть сверку уточнения' : 'Закрыть сверку остановки',
        }),
      )
      await dispatch('Successor guidance')
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

describe('production runtime control authority', () => {
  const storageKey = 'fleet-runtime-controls:v1:owner:session1'
  const privateSession = { ...sessionFixture, task_bound: false }
  const acknowledgement: Awaited<ReturnType<typeof fleet.steerSessionRun>> = {
    session_id: 'session1',
    run_id: 'original-run',
    runtime_run_id: 'native-original',
    accepted: true,
    state: 'running',
    message: 'Original command acknowledged',
  }

  beforeEach(() => {
    vi.mocked(fleet.getSession).mockResolvedValue(privateSession)
    vi.mocked(chats.getTaskContext).mockResolvedValue({ binding: null, tracker: null })
    vi.mocked(chats.getChatControls).mockResolvedValue({
      can_send: false,
      can_steer: true,
      can_stop: true,
      active_run_id: 'original-run',
      blocked_reason: null,
    })
  })

  it('keeps a private owner without write permission read-only despite advertised controls', async () => {
    useAuthStore.setState({ permissions: [] })
    renderPage()
    const steer = await screen.findByRole('button', { name: 'Передать уточнение запуску' })
    const stop = screen.getByRole('button', { name: 'Остановить запуск' })
    expect(steer).toBeDisabled()
    expect(stop).toBeDisabled()
    fireEvent.click(steer)
    fireEvent.click(stop)
    expect(fleet.steerSessionRun).not.toHaveBeenCalled()
    expect(fleet.stopSessionRun).not.toHaveBeenCalled()
    expect(fleet.createSessionMessage).not.toHaveBeenCalled()
    expect(sessionStorage.getItem(storageKey)).toBeNull()
  })

  it('clears the original private steer handle only after its exact accepted receipt', async () => {
    vi.mocked(fleet.steerSessionRun).mockResolvedValue(acknowledgement)
    renderPage()
    const editor = await screen.findByLabelText('Уточнение активному запуску')
    fireEvent.change(editor, { target: { value: 'Original private guidance' } })
    const submit = screen.getByRole('button', { name: 'Передать уточнение запуску' })
    await waitFor(() => expect(submit).toBeEnabled())
    await userEvent.click(submit)
    await screen.findByText('Команда принята. Выполнение проверяется по статусу.')
    expect(fleet.steerSessionRun).toHaveBeenCalledExactlyOnceWith(
      'session1',
      'original-run',
      { input: 'Original private guidance' },
      expect.any(String),
    )
    expect(JSON.parse(sessionStorage.getItem(storageKey)!)).toEqual({})
    expect(editor).toHaveValue('')
    expect(fleet.createSessionMessage).not.toHaveBeenCalled()
  })

  it.each([
    { label: 'run', receipt: { ...acknowledgement, run_id: 'foreign-run' } },
    { label: 'session', receipt: { ...acknowledgement, session_id: 'foreign-session' } },
  ])('holds an accepted private steer receipt for a foreign $label', async ({ receipt }) => {
    vi.mocked(fleet.steerSessionRun).mockResolvedValue(receipt)
    renderPage()
    const editor = await screen.findByLabelText('Уточнение активному запуску')
    fireEvent.change(editor, { target: { value: 'Keep the original guidance' } })
    const submit = screen.getByRole('button', { name: 'Передать уточнение запуску' })
    await waitFor(() => expect(submit).toBeEnabled())
    await userEvent.click(submit)
    await screen.findByText('Ответ runtime не соответствует исходной команде')
    const originalKey = vi.mocked(fleet.steerSessionRun).mock.calls[0]![3]
    expect(JSON.parse(sessionStorage.getItem(storageKey)!)).toEqual({
      steer: { operation: 'steer', runId: 'original-run', key: originalKey },
    })
    expect(editor).toHaveValue('Keep the original guidance')
    expect(screen.getByRole('button', { name: 'Передать уточнение запуску' })).toBeDisabled()
    expect(screen.queryByText('Команда принята. Выполнение проверяется по статусу.')).toBeNull()
    expect(fleet.steerSessionRun).toHaveBeenCalledTimes(1)
    expect(fleet.createSessionMessage).not.toHaveBeenCalled()
  })

  it.each([
    { label: 'owner', session: { ...privateSession, user_id: 'another-owner' } },
    { label: 'agent', session: { ...privateSession, primary_agent_id: 'another-agent' } },
    { label: 'task binding', session: { ...privateSession, task_bound: true } },
  ])('reserves no private steer after fresh $label authority changed', async ({ session }) => {
    renderPage()
    const editor = await screen.findByLabelText('Уточнение активному запуску')
    fireEvent.change(editor, { target: { value: 'Retain the unreserved guidance' } })
    const submit = screen.getByRole('button', { name: 'Передать уточнение запуску' })
    await waitFor(() => expect(submit).toBeEnabled())
    vi.mocked(fleet.getSession).mockResolvedValue(session)
    await userEvent.click(submit)
    await screen.findByText(
      'Только чтение: отправка требует актуальных прав владельца и доступного исполнителя.',
    )
    expect(editor).toHaveValue('Retain the unreserved guidance')
    expect(fleet.steerSessionRun).not.toHaveBeenCalled()
    expect(fleet.stopSessionRun).not.toHaveBeenCalled()
    expect(fleet.createSessionMessage).not.toHaveBeenCalled()
    expect(sessionStorage.getItem(storageKey)).toBeNull()
  })

  it('acknowledges an authorized PM stop without dispatching an ordinary prompt or answer', async () => {
    vi.mocked(fleet.getSession).mockResolvedValue(sessionFixture)
    vi.mocked(chats.getTaskContext).mockResolvedValue(context)
    vi.mocked(fleet.stopSessionRun).mockResolvedValue(acknowledgement)
    renderPage()
    const stop = await screen.findByRole('button', { name: 'Остановить запуск' })
    await waitFor(() => expect(stop).toBeEnabled())
    await userEvent.click(stop)
    await waitFor(() => expect(JSON.parse(sessionStorage.getItem(storageKey)!)).toEqual({}))
    expect(fleet.stopSessionRun).toHaveBeenCalledExactlyOnceWith(
      'session1',
      'original-run',
      expect.any(String),
    )
    expect(fleet.steerSessionRun).not.toHaveBeenCalled()
    expect(fleet.createSessionMessage).not.toHaveBeenCalled()
    expect(chats.answerClarification).not.toHaveBeenCalled()
    expect(chats.confirmRequirements).not.toHaveBeenCalled()
  })

  it('retains private steer custody when its accepted ACK arrives after navigation', async () => {
    let resolveOriginal!: (receipt: Awaited<ReturnType<typeof fleet.steerSessionRun>>) => void
    vi.mocked(fleet.steerSessionRun).mockImplementationOnce(
      () => new Promise((resolve) => (resolveOriginal = resolve)),
    )
    const { router } = renderPage()
    fireEvent.change(await screen.findByLabelText('Уточнение активному запуску'), {
      target: { value: 'Private guidance not stored with custody' },
    })
    const submit = screen.getByRole('button', { name: 'Передать уточнение запуску' })
    await waitFor(() => expect(submit).toBeEnabled())
    await userEvent.click(submit)
    await waitFor(() => expect(fleet.steerSessionRun).toHaveBeenCalledTimes(1))
    const originalKey = vi.mocked(fleet.steerSessionRun).mock.calls[0]![3]
    const original = sessionStorage.getItem(storageKey)!
    expect(JSON.parse(original)).toEqual({
      steer: { operation: 'steer', runId: 'original-run', key: originalKey },
    })
    expect(original).not.toContain('Private guidance')
    expect(original).not.toContain('fixture-token')
    await act(async () => {
      await router.navigate('/chats')
    })
    await userEvent.click(await screen.findByRole('button', { name: /^Выйти$/ }))
    await screen.findByText('Список')
    await act(async () => resolveOriginal(acknowledgement))
    expect(sessionStorage.getItem(storageKey)).toBe(original)
    renderPage()
    await screen.findByText(
      'Исход уточнения запуску неизвестен. Нельзя повторить его как новый prompt; требуется сверка runtime.',
    )
    expect(screen.getByRole('button', { name: 'Передать уточнение запуску' })).toBeDisabled()
    expect(fleet.steerSessionRun).toHaveBeenCalledTimes(1)
    expect(fleet.createSessionMessage).not.toHaveBeenCalled()
  })

  it('keeps a definite preflight rejection unreserved without falsely acknowledging success', async () => {
    renderPage()
    const editor = await screen.findByLabelText('Уточнение активному запуску')
    fireEvent.change(editor, { target: { value: 'Never dispatched guidance' } })
    const submit = screen.getByRole('button', { name: 'Передать уточнение запуску' })
    await waitFor(() => expect(submit).toBeEnabled())
    vi.mocked(fleet.getSession).mockRejectedValue(new ApiError(403, 'Preflight access revoked'))
    await userEvent.click(submit)
    await screen.findByText('Preflight access revoked')
    expect(editor).toHaveValue('Never dispatched guidance')
    expect(sessionStorage.getItem(storageKey)).toBeNull()
    expect(fleet.steerSessionRun).not.toHaveBeenCalled()
    expect(fleet.stopSessionRun).not.toHaveBeenCalled()
    expect(fleet.createSessionMessage).not.toHaveBeenCalled()
    expect(screen.queryByText('Команда принята. Выполнение проверяется по статусу.')).toBeNull()
  })

  it('keeps the original private prompt retry visible after lost ACK and active runtime controls', async () => {
    vi.mocked(chats.getChatControls).mockResolvedValue({
      can_send: true,
      can_steer: false,
      can_stop: false,
      active_run_id: null,
      blocked_reason: null,
    })
    vi.mocked(fleet.createSessionMessage).mockRejectedValue(new Error('Prompt ACK lost'))
    const { client } = renderPage()
    fireEvent.change(await screen.findByLabelText('Сообщение'), {
      target: { value: 'Original prompt with uncertain acceptance' },
    })
    const submit = screen.getByRole('button', { name: /^Отправить$/ })
    await waitFor(() => expect(submit).toBeEnabled())
    await userEvent.click(submit)
    await screen.findByText(
      'Ответ неизвестен. Черновик и исходный ключ сохранены; повторить можно только тот же запрос.',
    )
    expect(fleet.createSessionMessage).toHaveBeenCalledTimes(1)
    const original = vi.mocked(fleet.createSessionMessage).mock.calls[0]!
    expect(original[1]).toEqual({
      author_agent_id: null,
      body: 'Original prompt with uncertain acceptance',
      idempotency_key: expect.any(String),
      message_kind: null,
      runtime_message_id: null,
    })
    vi.mocked(chats.getChatControls).mockResolvedValue({
      can_send: false,
      can_steer: true,
      can_stop: true,
      active_run_id: 'original-run',
      blocked_reason: null,
    })
    await act(async () => {
      await client.refetchQueries({ queryKey: ['chat-controls', 'session1'], exact: true })
    })
    const retry = await screen.findByRole('button', { name: 'Повторить исходный запрос' })
    expect(retry).toBeEnabled()
    expect(screen.getByLabelText('Сообщение')).toHaveValue(original[1].body)
    expect(screen.getByLabelText('Сообщение')).toBeDisabled()
    await userEvent.click(retry)
    await waitFor(() => expect(fleet.createSessionMessage).toHaveBeenCalledTimes(2))
    expect(vi.mocked(fleet.createSessionMessage).mock.calls[1]).toEqual(original)
    expect(fleet.steerSessionRun).not.toHaveBeenCalled()
    expect(fleet.stopSessionRun).not.toHaveBeenCalled()
    expect(sessionStorage.getItem(storageKey)).toBeNull()
  })
})
