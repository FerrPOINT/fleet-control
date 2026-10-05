import { render, screen } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { listRuntimeControls } from '@/api/fleet'
import type { RuntimeControlReceipt } from '@/api/types'
import { RuntimeControlHistory, isUnresolvedControl } from './runtime-control-history'

vi.mock('@/api/fleet', () => ({ listRuntimeControls: vi.fn() }))

const receipt: RuntimeControlReceipt = {
  id: 'command-1',
  session_id: 'session-1',
  session_run_id: 'run-1',
  agent_id: 'agent-1',
  actor_user_id: 'owner-1',
  operation: 'stop',
  state: 'uncertain',
  acknowledgement: null,
  observed_run_state: null,
  created_at: '2026-10-05T00:00:00Z',
  updated_at: '2026-10-05T00:00:00Z',
}
function renderHistory() {
  render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <RuntimeControlHistory sessionId="session-1" runId="run-1" />
    </QueryClientProvider>,
  )
}
describe('RuntimeControlHistory', () => {
  beforeEach(() => vi.clearAllMocks())
  it('holds only unresolved effects and does not treat terminal observation as an ACK', () => {
    for (const state of ['reserved', 'submitted', 'uncertain'] as const)
      expect(isUnresolvedControl({ ...receipt, state })).toBe(true)
    for (const state of ['acknowledged', 'rejected', 'terminal_observed'] as const)
      expect(isUnresolvedControl({ ...receipt, state })).toBe(false)
  })
  it('shows actor and unknown outcome without exposing request input or credentials', async () => {
    vi.mocked(listRuntimeControls).mockResolvedValue([receipt])
    renderHistory()
    expect(await screen.findByText('Исход неизвестен, повторная отправка запрещена')).toBeVisible()
    expect(screen.getByRole('img', { name: receipt.actor_user_id })).toBeVisible()
    expect(listRuntimeControls).toHaveBeenCalledWith(receipt.session_id, receipt.session_run_id)
  })
  it('keeps command acceptance unknown after independently observed completion', async () => {
    vi.mocked(listRuntimeControls).mockResolvedValue([
      { ...receipt, state: 'terminal_observed', observed_run_state: 'completed' },
    ])
    renderHistory()
    expect(await screen.findByText('Запуск завершён; принятие команды неизвестно')).toBeVisible()
    expect(screen.queryByText('Runtime подтвердил команду')).not.toBeInTheDocument()
  })
  it('shows a readback failure instead of an empty or successful history', async () => {
    vi.mocked(listRuntimeControls).mockRejectedValue(new Error('offline'))
    renderHistory()
    expect(await screen.findByText('Не удалось проверить команды запуска')).toBeVisible()
    expect(screen.queryByText('Команд пока нет')).not.toBeInTheDocument()
  })
})
