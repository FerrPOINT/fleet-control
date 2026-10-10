import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ApiError } from '@sdlc/ui/lib'
import * as fleet from '@/api/fleet'
import { getChatControls } from '@/api/task-chats'
import type { AgentSession, SessionAgentRun, RuntimeRunControlResponse } from '@/api/types'
import { useAuthStore } from '@/shared/auth/store'
import { readControlRecovery } from '@/shared/chat-control-recovery'
import { ChatRunControls } from './run-controls'
import { legacyControlHeld } from './core'

vi.mock('@/api/fleet', () => ({
  getSession: vi.fn(),
  listSessionAgentRuns: vi.fn(),
  steerSessionRun: vi.fn(),
  stopSessionRun: vi.fn(),
}))
vi.mock('@/api/task-chats', () => ({ getChatControls: vi.fn() }))
const sessionId = 'session-one'
const runId = 'run-one'
const controls = {
  active_run_id: runId,
  can_send: false,
  can_steer: true,
  can_stop: true,
  blocked_reason: null,
}
const receipt: RuntimeRunControlResponse = {
  session_id: sessionId,
  run_id: runId,
  runtime_run_id: 'runtime-one',
  accepted: true,
  state: 'running',
  message: 'Accepted',
}

function renderControls(taskBound = false) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })
  return render(
    <QueryClientProvider client={client}>
      <ChatRunControls sessionId={sessionId} taskBound={taskBound} />
    </QueryClientProvider>,
  )
}
async function steer() {
  fireEvent.change(await screen.findByLabelText('Уточнение активному запуску'), {
    target: { value: 'Original private input' },
  })
  fireEvent.click(screen.getByRole('button', { name: 'Передать уточнение запуску' }))
}
beforeEach(() => {
  vi.clearAllMocks()
  sessionStorage.clear()
  useAuthStore.setState({
    token: 'owned-fixture-token',
    userId: 'owner',
    signingOut: false,
    permissions: ['sessions:write_own'],
  })
  vi.mocked(getChatControls).mockResolvedValue(controls)
  vi.mocked(fleet.getSession).mockResolvedValue({
    user_id: 'owner',
    primary_agent_id: 'agent-one',
    task_bound: false,
  } as AgentSession)
  vi.mocked(fleet.listSessionAgentRuns).mockResolvedValue([
    {
      id: runId,
      session_id: sessionId,
      agent_id: 'agent-one',
      runtime_run_id: 'runtime-one',
      state: 'running',
    } as SessionAgentRun,
  ])
  vi.mocked(fleet.steerSessionRun).mockResolvedValue(receipt)
  vi.mocked(fleet.stopSessionRun).mockResolvedValue(receipt)
})

describe('scoped run commands', () => {
  it('keeps an owner without write permission read-only despite advertised controls', async () => {
    useAuthStore.setState({ permissions: ['sessions:read_own'] })
    renderControls()
    expect(await screen.findByRole('button', { name: 'Остановить запуск' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Передать уточнение запуску' })).toBeDisabled()
    expect(fleet.steerSessionRun).not.toHaveBeenCalled()
    expect(fleet.stopSessionRun).not.toHaveBeenCalled()
  })
  it('sends one original steer and clears only its verified metadata', async () => {
    renderControls()
    await steer()
    await screen.findByText('Команда принята. Состояние запуска проверяется отдельно.')
    expect(fleet.steerSessionRun).toHaveBeenCalledExactlyOnceWith(sessionId, runId, {
      input: 'Original private input',
    })
    expect(readControlRecovery(sessionId).state).toBe('none')
    expect(screen.getByLabelText('Уточнение активному запуску')).toHaveValue('')
  })

  it('holds lost steer acceptance through reload, retaining no private text or token', async () => {
    vi.mocked(fleet.steerSessionRun).mockRejectedValue(new Error('Lost response'))
    const view = renderControls()
    await steer()
    await screen.findByText('Lost response')
    expect(readControlRecovery(sessionId).state).toBe('pending')
    expect(legacyControlHeld(sessionId)).toBe(true)
    const raw = sessionStorage.getItem(`fleet-control.control-recovery.v1:${sessionId}`)!
    expect(raw).not.toContain('Original private input')
    expect(raw).not.toContain('owned-fixture-token')
    view.unmount()
    renderControls()
    expect(await screen.findByRole('button', { name: 'Остановить запуск' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Передать уточнение запуску' })).toBeDisabled()
    expect(fleet.steerSessionRun).toHaveBeenCalledTimes(1)
  })

  it('holds a foreign run receipt and preserves the original input', async () => {
    vi.mocked(fleet.steerSessionRun).mockResolvedValue({ ...receipt, run_id: 'foreign-run' })
    renderControls()
    await steer()
    await screen.findByText('Не удалось проверить подтверждение исходной команды.')
    expect(readControlRecovery(sessionId).state).toBe('pending')
    expect(screen.getByLabelText('Уточнение активному запуску')).toHaveValue(
      'Original private input',
    )
  })

  it.each([
    { user_id: 'another-owner', primary_agent_id: 'agent-one', task_bound: false },
    { user_id: 'owner', primary_agent_id: 'another-agent', task_bound: false },
    { user_id: 'owner', primary_agent_id: 'agent-one', task_bound: true },
  ])('does not dispatch after the authoritative session changed: %j', async (session) => {
    vi.mocked(fleet.getSession).mockResolvedValue(session as AgentSession)
    renderControls()
    await steer()
    await screen.findByText('Доступ или запуск изменился. Команда не отправлена.')
    expect(fleet.steerSessionRun).not.toHaveBeenCalled()
    expect(readControlRecovery(sessionId).state).toBe('none')
  })

  it('allows a verified task owner stop while exposing no ordinary task steer', async () => {
    vi.mocked(fleet.getSession).mockResolvedValue({
      user_id: 'owner',
      primary_agent_id: 'agent-one',
      task_bound: true,
    } as AgentSession)
    renderControls(true)
    fireEvent.click(await screen.findByRole('button', { name: 'Остановить запуск' }))
    await screen.findByText('Команда принята. Состояние запуска проверяется отдельно.')
    expect(fleet.stopSessionRun).toHaveBeenCalledExactlyOnceWith(sessionId, runId)
    expect(screen.queryByLabelText('Уточнение активному запуску')).not.toBeInTheDocument()
  })

  it('keeps a late ACK held after navigation', async () => {
    let resolve!: (value: RuntimeRunControlResponse) => void
    vi.mocked(fleet.steerSessionRun).mockReturnValue(
      new Promise((value) => {
        resolve = value
      }),
    )
    const view = renderControls()
    await steer()
    await waitFor(() => expect(fleet.steerSessionRun).toHaveBeenCalledTimes(1))
    view.unmount()
    await act(async () => resolve(receipt))
    expect(readControlRecovery(sessionId).state).toBe('pending')
  })

  it('releases a definite initial rejection without falsely acknowledging success', async () => {
    vi.mocked(fleet.steerSessionRun).mockRejectedValue(new ApiError(403, 'Forbidden'))
    renderControls()
    await steer()
    await screen.findByText('Forbidden')
    expect(readControlRecovery(sessionId).state).toBe('none')
    expect(screen.getByLabelText('Уточнение активному запуску')).toHaveValue(
      'Original private input',
    )
  })
})
