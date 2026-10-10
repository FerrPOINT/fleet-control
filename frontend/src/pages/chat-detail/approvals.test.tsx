import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { TaskApprovalsPanel } from './approvals'
import * as api from '@/api/task-approvals'

vi.mock('@/api/task-approvals', () => ({
  getTaskApprovals: vi.fn(),
  getTaskApprovalDecision: vi.fn(),
  decideTaskApproval: vi.fn(),
}))

const pending: api.RuntimeApprovalRequest = {
  id: 'approval-1',
  session_id: 'session-1',
  session_run_id: 'fleet-run-1',
  agent_id: 'agent-1',
  runtime_run_id: 'runtime-run-1',
  runtime_approval_id: 'runtime-approval-1',
  prompt: 'Разрешить чтение файла /workspace/requirements.md?',
  detail: { action: 'read_file', path: '/workspace/requirements.md', token: 'redacted' },
  state: 'pending',
  resolved_at: null,
  resolved_by_user_id: null,
  created_at: '2026-10-01T12:00:00Z',
}

function decision(
  state: api.ApprovalDecision['state'] = 'pending',
  overrides: Partial<api.ApprovalDecision> = {},
): api.ApprovalDecision {
  return {
    id: 'decision-1',
    session_id: pending.session_id,
    approval_id: pending.id,
    session_run_id: pending.session_run_id,
    actor_user_id: 'owner',
    choice: 'once',
    state,
    created_at: '2026-10-01T12:00:00Z',
    ...overrides,
  }
}

async function enabledChoice(name = 'Разрешить один раз') {
  const button = await screen.findByRole('button', { name })
  await waitFor(() => expect(button).toBeEnabled())
  return button
}

function newClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: 3, retryDelay: 1 },
    },
  })
}

function renderPanel(canResolve = true, client = newClient(), sessionId = 'session-1') {
  const view = render(
    <QueryClientProvider client={client}>
      <TaskApprovalsPanel sessionId={sessionId} canResolve={canResolve} />
    </QueryClientProvider>,
  )
  return { ...view, client }
}

beforeEach(() => {
  vi.clearAllMocks()
  vi.mocked(api.getTaskApprovals).mockResolvedValue([pending])
  vi.mocked(api.getTaskApprovalDecision).mockResolvedValue(null)
  vi.mocked(api.decideTaskApproval).mockResolvedValue(decision())
})

describe('TaskApprovalsPanel fixture states', () => {
  it('does not occupy the dialogue when no runtime requests exist', async () => {
    vi.mocked(api.getTaskApprovals).mockResolvedValue([])
    const client = newClient()
    render(
      <QueryClientProvider client={client}>
        <TaskApprovalsPanel sessionId="session-1" canResolve hideWhenEmpty />
      </QueryClientProvider>,
    )
    await waitFor(() => expect(client.getQueryData(['task-approvals', 'session-1'])).toEqual([]))
    expect(
      screen.queryByRole('heading', { name: 'Разрешения действий агента' }),
    ).not.toBeInTheDocument()
  })

  it('does not hide an unavailable approval list in the dialogue', async () => {
    vi.mocked(api.getTaskApprovals).mockRejectedValue(new Error('unavailable'))
    render(
      <QueryClientProvider client={newClient()}>
        <TaskApprovalsPanel sessionId="session-1" canResolve hideWhenEmpty />
      </QueryClientProvider>,
    )
    expect(await screen.findByText('Не удалось загрузить разрешения действий агента')).toBeVisible()
  })

  it('renders the exact request/run/action and server-redacted metadata without deciding', async () => {
    renderPanel()
    const request = await screen.findByRole('article', { name: 'read_file' })
    expect(request).toHaveTextContent('fleet-run-1')
    expect(request).toHaveTextContent('runtime-run-1')
    expect(request).toHaveTextContent('approval-1')
    expect(request).toHaveTextContent(pending.prompt)
    expect(within(request).getByText('Путь')).toBeVisible()
    expect(request).toHaveTextContent('redacted')
    await enabledChoice()
    expect(within(request).getByRole('button', { name: 'Разрешить один раз' })).toBeEnabled()
    expect(within(request).getByRole('button', { name: 'Запретить' })).toBeEnabled()
    expect(
      screen.queryByRole('button', { name: /always|всегда|сессию|все/i }),
    ).not.toBeInTheDocument()
    expect(api.decideTaskApproval).not.toHaveBeenCalled()
  })

  it('renders nested parameters as semantic fields instead of debug JSON', async () => {
    vi.mocked(api.getTaskApprovals).mockResolvedValue([
      {
        ...pending,
        detail: {
          action: 'read_file',
          arguments: { path: '/workspace/requirements.md', recursive: false },
          command: 'cat /workspace/requirements.md',
          args: ['--dry-run', '--verbose'],
          token: 'redacted',
        },
      },
    ])
    renderPanel()
    const request = await screen.findByRole('article', { name: 'read_file' })
    expect(within(request).getByText('Параметры')).toBeVisible()
    expect(within(request).getByText('Путь')).toBeVisible()
    expect(within(request).getByText('Команда')).toBeVisible()
    expect(within(request).getByText('cat /workspace/requirements.md')).toBeVisible()
    expect(within(request).getByText('Нет')).toBeVisible()
    expect(within(request).getByText('--dry-run')).toBeVisible()
    expect(request.querySelector('pre')).toBeNull()
    expect(request).not.toHaveTextContent('"arguments"')
  })

  it('shows loading without actions or a false empty state', () => {
    vi.mocked(api.getTaskApprovals).mockReturnValue(new Promise(() => {}))
    renderPanel()
    expect(screen.getByRole('status')).toHaveTextContent('Загрузка разрешений')
    expect(screen.queryByText('Запросов на разрешение нет')).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Разрешить один раз' })).not.toBeInTheDocument()
  })

  it('renders a true empty response', async () => {
    vi.mocked(api.getTaskApprovals).mockResolvedValue([])
    renderPanel()
    expect(await screen.findByText('Запросов на разрешение нет')).toBeVisible()
    expect(api.decideTaskApproval).not.toHaveBeenCalled()
  })

  it('keeps pending requests read-only when the passed permission is false', async () => {
    renderPanel(false)
    expect(await screen.findByText(pending.prompt)).toBeVisible()
    expect(screen.getByText('Только просмотр')).toBeVisible()
    const approve = screen.getByRole('button', { name: 'Разрешить один раз' })
    expect(approve).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Запретить' })).toBeDisabled()
    fireEvent.click(approve)
    expect(api.decideTaskApproval).not.toHaveBeenCalled()
  })

  it.each([null, '', '   '])(
    'does not attempt a decision without an exact runtime approval ID (%s)',
    async (runtimeApprovalId) => {
      vi.mocked(api.getTaskApprovals).mockResolvedValue([
        { ...pending, runtime_approval_id: runtimeApprovalId },
      ])
      renderPanel()
      await screen.findByText(
        'Идентификатор точного запроса агента отсутствует. Решение недоступно.',
      )
      await waitFor(() =>
        expect(screen.queryByText('Проверка сохранённого решения…')).not.toBeInTheDocument(),
      )
      const approve = screen.getByRole('button', { name: 'Разрешить один раз' })
      const deny = screen.getByRole('button', { name: 'Запретить' })
      expect(approve).toBeDisabled()
      expect(deny).toBeDisabled()
      fireEvent.click(approve)
      fireEvent.click(deny)
      fireEvent.click(screen.getByRole('button', { name: 'Обновить разрешения' }))
      await waitFor(() => expect(api.getTaskApprovalDecision).toHaveBeenCalledTimes(2))
      expect(approve).toBeDisabled()
      expect(deny).toBeDisabled()
      expect(api.decideTaskApproval).not.toHaveBeenCalled()
    },
  )

  it.each([
    ['approved', 'Разрешено'],
    ['denied', 'Отклонено'],
    ['cancelled', 'Отменено'],
  ] as const)('renders %s as a read-only server state', async (state, label) => {
    vi.mocked(api.getTaskApprovals).mockResolvedValue([{ ...pending, state }])
    renderPanel()
    expect(await screen.findByText(label)).toBeVisible()
    expect(screen.queryByRole('button', { name: 'Разрешить один раз' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Запретить' })).not.toBeInTheDocument()
    expect(api.decideTaskApproval).not.toHaveBeenCalled()
  })

  it('disables all choices while mutating and sends only one command on double click', async () => {
    let finish!: (receipt: api.ApprovalDecision) => void
    vi.mocked(api.decideTaskApproval).mockReturnValue(
      new Promise((resolve) => {
        finish = resolve
      }),
    )
    const { client } = renderPanel()
    const invalidate = vi.spyOn(client, 'invalidateQueries')
    const approve = await enabledChoice()
    fireEvent.click(approve)
    fireEvent.click(approve)
    fireEvent.click(screen.getByRole('button', { name: 'Запретить' }))
    await waitFor(() => expect(api.decideTaskApproval).toHaveBeenCalledTimes(1))
    expect(approve).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Запретить' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Обновить разрешения' })).toBeDisabled()
    expect(api.decideTaskApproval).toHaveBeenCalledWith('session-1', 'approval-1', {
      choice: 'once',
      idempotency_key: expect.stringMatching(/^[0-9a-f-]{36}$/),
    })
    finish(decision())
    await screen.findByText('Решение принято. Ожидается доставка агенту.')
    expect(invalidate.mock.calls.map(([filter]) => filter?.queryKey)).toEqual([
      ['task-approvals', 'session-1'],
      ['task-approval-decision', 'session-1'],
      ['chat-controls', 'session-1'],
      ['session-runs', 'session-1'],
      ['chat-history', 'session-1'],
      ['session', 'session-1'],
    ])
    expect(approve).toBeDisabled()
  })

  it('targets Deny to the selected approval even when another run also awaits approval', async () => {
    const second = {
      ...pending,
      id: 'approval-2',
      session_run_id: 'fleet-run-2',
      runtime_run_id: 'runtime-run-2',
      detail: { action: 'write_file' },
    }
    vi.mocked(api.getTaskApprovals).mockResolvedValue([pending, second])
    vi.mocked(api.decideTaskApproval).mockResolvedValue(
      decision('pending', {
        approval_id: second.id,
        session_run_id: second.session_run_id,
        choice: 'deny',
      }),
    )
    renderPanel()
    const request = await screen.findByRole('article', { name: 'write_file' })
    await waitFor(() =>
      expect(within(request).getByRole('button', { name: 'Запретить' })).toBeEnabled(),
    )
    fireEvent.click(within(request).getByRole('button', { name: 'Запретить' }))
    await waitFor(() =>
      expect(api.decideTaskApproval).toHaveBeenCalledWith('session-1', 'approval-2', {
        choice: 'deny',
        idempotency_key: expect.any(String),
      }),
    )
    await within(request).findByText('Решение принято. Ожидается доставка агенту.')
    expect(
      within(screen.getByRole('article', { name: 'read_file' })).getByRole('button', {
        name: 'Разрешить один раз',
      }),
    ).toBeEnabled()
    expect(api.decideTaskApproval).toHaveBeenCalledTimes(1)
  })

  it.each([
    ['pending', 'Решение принято. Ожидается доставка агенту.'],
    ['delivered', 'Решение доставлено. Ожидается обновление состояния.'],
    ['uncertain', 'Доставка решения не подтверждена. Требуется сверка состояния.'],
    ['failed', 'Не удалось доставить решение. Требуется сверка состояния.'],
  ] as const)(
    'keeps a %s receipt locked while allowing GET reconciliation',
    async (state, label) => {
      vi.mocked(api.decideTaskApproval).mockResolvedValue(decision(state))
      renderPanel()
      fireEvent.click(await enabledChoice())
      await screen.findByText(label)
      expect(screen.getByRole('button', { name: 'Запретить' })).toBeDisabled()
      fireEvent.click(screen.getByRole('button', { name: 'Обновить разрешения' }))
      await waitFor(() => expect(api.getTaskApprovals).toHaveBeenCalledTimes(3))
      expect(screen.getByRole('button', { name: 'Разрешить один раз' })).toBeDisabled()
      expect(api.decideTaskApproval).toHaveBeenCalledTimes(1)
      expect(screen.queryByText('Разрешено')).not.toBeInTheDocument()
    },
  )

  it('does not retry a lost POST response even when global mutation retries are enabled', async () => {
    vi.mocked(api.decideTaskApproval).mockRejectedValue(new TypeError('unknown acceptance'))
    renderPanel()
    fireEvent.click(await enabledChoice())
    await screen.findByText('Доставка решения не подтверждена. Требуется сверка состояния.')
    expect(api.decideTaskApproval).toHaveBeenCalledTimes(1)
    fireEvent.click(screen.getByRole('button', { name: 'Запретить' }))
    fireEvent.click(screen.getByRole('button', { name: 'Обновить разрешения' }))
    await waitFor(() => expect(api.getTaskApprovals).toHaveBeenCalledTimes(3))
    expect(api.decideTaskApproval).toHaveBeenCalledTimes(1)
  })

  it('checks for a historical command before enabling the first choice', async () => {
    let finish!: (value: api.ApprovalDecision | null) => void
    vi.mocked(api.getTaskApprovalDecision).mockReturnValue(
      new Promise((resolve) => {
        finish = resolve
      }),
    )
    renderPanel()
    const approve = await screen.findByRole('button', { name: 'Разрешить один раз' })
    expect(approve).toBeDisabled()
    expect(screen.getByText('Проверка сохранённого решения…')).toBeVisible()
    fireEvent.click(approve)
    expect(api.decideTaskApproval).not.toHaveBeenCalled()
    finish(null)
    await waitFor(() => expect(approve).toBeEnabled())
    expect(api.getTaskApprovalDecision).toHaveBeenCalledWith('session-1', 'approval-1')
  })

  it('reads an existing uncertain command on a fresh mount and never submits another choice', async () => {
    vi.mocked(api.getTaskApprovalDecision).mockResolvedValue(
      decision('uncertain', { choice: 'deny' }),
    )
    renderPanel()
    await screen.findByText('Доставка решения не подтверждена. Требуется сверка состояния.')
    expect(screen.getByRole('button', { name: 'Разрешить один раз' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Запретить' })).toBeDisabled()
    expect(screen.queryByText('Отклонено')).not.toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Обновить разрешения' }))
    await waitFor(() => expect(api.getTaskApprovalDecision).toHaveBeenCalledTimes(2))
    expect(api.decideTaskApproval).not.toHaveBeenCalled()
  })

  it('queries the saved command after a lost response and displays its actual delivery state', async () => {
    vi.mocked(api.decideTaskApproval).mockRejectedValue(new TypeError('lost response'))
    vi.mocked(api.getTaskApprovalDecision)
      .mockResolvedValueOnce(null)
      .mockResolvedValueOnce(decision('delivered'))
    renderPanel()
    fireEvent.click(await enabledChoice())
    await screen.findByText('Решение доставлено. Ожидается обновление состояния.')
    expect(api.getTaskApprovalDecision).toHaveBeenNthCalledWith(2, 'session-1', 'approval-1')
    expect(api.decideTaskApproval).toHaveBeenCalledTimes(1)
    expect(screen.queryByText('Разрешено')).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Запретить' })).toBeDisabled()
  })

  it('keeps choices disabled after a failed decision lookup and recovers through GET', async () => {
    vi.mocked(api.getTaskApprovalDecision).mockRejectedValueOnce(new Error('offline'))
    renderPanel()
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Не удалось сверить сохранённое решение',
    )
    expect(screen.getByRole('button', { name: 'Разрешить один раз' })).toBeDisabled()
    fireEvent.click(screen.getByRole('button', { name: 'Обновить разрешения' }))
    await enabledChoice()
    expect(api.decideTaskApproval).not.toHaveBeenCalled()
  })

  it('does not show a decision from a different approval or run as this request result', async () => {
    vi.mocked(api.getTaskApprovalDecision).mockResolvedValue(
      decision('delivered', {
        approval_id: 'foreign-approval',
        session_run_id: 'foreign-run',
      }),
    )
    renderPanel()
    await screen.findByRole('alert')
    expect(
      screen.queryByText('Решение доставлено. Ожидается обновление состояния.'),
    ).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Разрешить один раз' })).toBeDisabled()
    expect(api.decideTaskApproval).not.toHaveBeenCalled()
  })

  it('reconciles a 409 with the historical command instead of changing its actor, choice or key', async () => {
    vi.mocked(api.decideTaskApproval).mockRejectedValue(new Error('409 conflict'))
    vi.mocked(api.getTaskApprovalDecision)
      .mockResolvedValueOnce(null)
      .mockResolvedValueOnce(decision('uncertain', { choice: 'deny', actor_user_id: 'operator' }))
    renderPanel()
    fireEvent.click(await enabledChoice())
    await screen.findByText('Доставка решения не подтверждена. Требуется сверка состояния.')
    expect(screen.getByRole('button', { name: 'Разрешить один раз' })).toBeDisabled()
    expect(api.decideTaskApproval).toHaveBeenCalledTimes(1)
  })

  it('preserves the ambiguous attempt across remounts instead of issuing a new command', async () => {
    vi.mocked(api.decideTaskApproval).mockRejectedValue(new TypeError('unknown acceptance'))
    const first = renderPanel()
    fireEvent.click(await enabledChoice())
    await screen.findByText('Доставка решения не подтверждена. Требуется сверка состояния.')
    const original = vi.mocked(api.decideTaskApproval).mock.calls[0]![2]
    first.unmount()
    renderPanel(true, first.client)
    expect(
      await screen.findByText('Доставка решения не подтверждена. Требуется сверка состояния.'),
    ).toBeVisible()
    expect(screen.getByRole('button', { name: 'Разрешить один раз' })).toBeDisabled()
    fireEvent.click(screen.getByRole('button', { name: 'Запретить' }))
    expect(api.decideTaskApproval).toHaveBeenCalledTimes(1)
    expect(vi.mocked(api.decideTaskApproval).mock.calls[0]![2]).toEqual(original)
  })

  it('accepts the resolved server mirror after an ambiguous response', async () => {
    vi.mocked(api.decideTaskApproval).mockRejectedValue(new TypeError('unknown acceptance'))
    renderPanel()
    fireEvent.click(await enabledChoice('Запретить'))
    await screen.findByText('Доставка решения не подтверждена. Требуется сверка состояния.')
    vi.mocked(api.getTaskApprovals).mockResolvedValue([{ ...pending, state: 'denied' }])
    fireEvent.click(screen.getByRole('button', { name: 'Обновить разрешения' }))
    expect(await screen.findByText('Отклонено')).toBeVisible()
    expect(screen.queryByRole('button', { name: 'Разрешить один раз' })).not.toBeInTheDocument()
    expect(screen.queryByText(/Доставка решения не подтверждена/)).not.toBeInTheDocument()
    expect(api.decideTaskApproval).toHaveBeenCalledTimes(1)
  })

  it('recovers a GET error using GET only', async () => {
    vi.mocked(api.getTaskApprovals).mockRejectedValueOnce(new Error('offline'))
    renderPanel()
    expect(await screen.findByRole('alert')).toHaveTextContent('Не удалось загрузить разрешения')
    fireEvent.click(screen.getByRole('button', { name: 'Обновить разрешения' }))
    await screen.findByRole('article', { name: 'read_file' })
    expect(api.getTaskApprovals).toHaveBeenCalledTimes(2)
    expect(api.decideTaskApproval).not.toHaveBeenCalled()
  })

  it('hides cached actionable rows after a failed mirror refresh', async () => {
    const { client } = renderPanel()
    await screen.findByRole('article', { name: 'read_file' })
    vi.mocked(api.getTaskApprovals).mockRejectedValue(new Error('offline'))
    await client.invalidateQueries({ queryKey: ['task-approvals', 'session-1'] })
    await screen.findByRole('alert')
    expect(screen.queryByRole('button', { name: 'Разрешить один раз' })).not.toBeInTheDocument()
    expect(screen.queryByText('Запросов на разрешение нет')).not.toBeInTheDocument()
  })

  it('does not offer decisions for a request outside the current session', async () => {
    vi.mocked(api.getTaskApprovals).mockResolvedValue([
      { ...pending, session_id: 'foreign-session' },
    ])
    renderPanel()
    await screen.findByRole('alert')
    expect(screen.queryByRole('button', { name: 'Разрешить один раз' })).not.toBeInTheDocument()
    expect(api.decideTaskApproval).not.toHaveBeenCalled()
  })

  it('supports explicit keyboard activation and focuses the resulting status', async () => {
    const user = userEvent.setup()
    renderPanel()
    await enabledChoice()
    await user.tab()
    expect(screen.getByRole('button', { name: 'Обновить разрешения' })).toHaveFocus()
    await user.tab()
    expect(screen.getByRole('button', { name: 'Разрешить один раз' })).toHaveFocus()
    expect(api.decideTaskApproval).not.toHaveBeenCalled()
    await user.keyboard('{Enter}')
    await screen.findByText('Решение принято. Ожидается доставка агенту.')
    await waitFor(() =>
      expect(screen.getByRole('status', { name: 'Результат решения' })).toHaveFocus(),
    )
    expect(api.decideTaskApproval).toHaveBeenCalledTimes(1)
  })
})
