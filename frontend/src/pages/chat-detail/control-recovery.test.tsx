import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { lookupRuntimeControl, type RuntimeControlReceipt } from '@/api/runtime-controls'
import { ControlRecovery } from './control-recovery'

vi.mock('@/api/runtime-controls', () => ({ lookupRuntimeControl: vi.fn() }))
const receipt: RuntimeControlReceipt = {
  id: 'command',
  actor_user_id: 'owner',
  agent_id: 'agent',
  session_id: 'session',
  session_run_id: 'original-run',
  operation: 'steer',
  state: 'uncertain',
  acknowledgement: null,
  observed_run_state: null,
  created_at: '2026-10-09T10:00:00Z',
  updated_at: '2026-10-09T10:00:01Z',
}
function show() {
  const settled = vi.fn()
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(
    <QueryClientProvider client={client}>
      <ControlRecovery
        actorId="owner"
        sessionId="session"
        handle={{ operation: 'steer', runId: 'original-run', key: 'original-key' }}
        onSettled={settled}
      />
    </QueryClientProvider>,
  )
  return settled
}
beforeEach(() => vi.resetAllMocks())

describe('original-key recovery', () => {
  it.each(['acknowledged', 'rejected', 'terminal_observed'] as const)(
    'settles %s only after explicit user action',
    async (state) => {
      vi.mocked(lookupRuntimeControl).mockResolvedValue({ ...receipt, state })
      const settled = show()
      const button = await screen.findByRole('button', { name: 'Закрыть сверку уточнения' })
      expect(settled).not.toHaveBeenCalled()
      expect(lookupRuntimeControl).toHaveBeenCalledWith('session', 'original-run', 'original-key')
      fireEvent.click(button)
      expect(settled).toHaveBeenCalledExactlyOnceWith({ ...receipt, state })
    },
  )
  it.each(['reserved', 'submitted', 'uncertain'] as const)('does not settle %s', async (state) => {
    vi.mocked(lookupRuntimeControl).mockResolvedValue({ ...receipt, state })
    const settled = show()
    await screen.findByText('Исход команды ещё не подтверждён. Новая отправка заблокирована.')
    expect(
      screen.queryByRole('button', { name: 'Закрыть сверку уточнения' }),
    ).not.toBeInTheDocument()
    expect(settled).not.toHaveBeenCalled()
  })
  it.each([
    { actor_user_id: 'other' },
    { session_id: 'other' },
    { session_run_id: 'other' },
    { operation: 'stop' } as const,
  ])('rejects a mismatched identity %j', async (change) => {
    vi.mocked(lookupRuntimeControl).mockResolvedValue({
      ...receipt,
      ...change,
      state: 'acknowledged',
    })
    const settled = show()
    await screen.findByText(
      'Доставка не подтверждена. Отсутствие квитанции не разрешает новую отправку.',
    )
    expect(
      screen.queryByRole('button', { name: 'Закрыть сверку уточнения' }),
    ).not.toBeInTheDocument()
    expect(settled).not.toHaveBeenCalled()
  })
  it('does not accept an old collection response as key lookup evidence', async () => {
    vi.mocked(lookupRuntimeControl).mockResolvedValue(JSON.parse(JSON.stringify([receipt])))
    const settled = show()
    await screen.findByText(
      'Доставка не подтверждена. Отсутствие квитанции не разрешает новую отправку.',
    )
    expect(settled).not.toHaveBeenCalled()
  })
  it('keeps absence or transport failure uncertain without a new POST', async () => {
    vi.mocked(lookupRuntimeControl).mockRejectedValue(new Error('Private error'))
    const settled = show()
    await screen.findByText(
      'Доставка не подтверждена. Отсутствие квитанции не разрешает новую отправку.',
    )
    expect(screen.queryByText('Private error')).not.toBeInTheDocument()
    expect(settled).not.toHaveBeenCalled()
  })

  it('does not settle stale acknowledged data after a failed fresh permission read', async () => {
    vi.mocked(lookupRuntimeControl)
      .mockResolvedValueOnce({ ...receipt, state: 'acknowledged' })
      .mockRejectedValue(new Error('Access revoked'))
    const settled = show()
    await screen.findByRole('button', { name: 'Закрыть сверку уточнения' })
    fireEvent.click(screen.getByRole('button', { name: 'Сверить исходную команду steer' }))
    await waitFor(() =>
      expect(
        screen.queryByRole('button', { name: 'Закрыть сверку уточнения' }),
      ).not.toBeInTheDocument(),
    )
    await screen.findByText(
      'Доставка не подтверждена. Отсутствие квитанции не разрешает новую отправку.',
    )
    expect(settled).not.toHaveBeenCalled()
  })
})
