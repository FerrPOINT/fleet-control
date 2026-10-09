import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { listRuntimeControls, type RuntimeControlReceipt } from '@/api/runtime-controls'
import { RuntimeControlsPanel } from './runtime-controls'

vi.mock('@/api/runtime-controls', () => ({ listRuntimeControls: vi.fn() }))

const receipt: RuntimeControlReceipt = {
  id: 'command',
  session_id: 'session',
  session_run_id: 'run',
  agent_id: 'agent',
  actor_user_id: 'owner',
  operation: 'steer',
  state: 'uncertain',
  acknowledgement: null,
  observed_run_state: null,
  created_at: '2026-10-09T10:00:00Z',
  updated_at: '2026-10-09T10:00:01Z',
}

function show(runId: string | null = 'run') {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={client}>
      <RuntimeControlsPanel sessionId="session" runId={runId} />
    </QueryClientProvider>,
  )
}

beforeEach(() => vi.resetAllMocks())

describe('runtime control readback', () => {
  it.each([
    ['reserved', 'Команда зарезервирована. Отправка не подтверждена.'],
    ['submitted', 'Команда отправлена. Подтверждение ожидается.'],
    ['acknowledged', 'Команда принята runtime.'],
    ['uncertain', 'Исход команды неизвестен. Повторная отправка не разрешена.'],
    ['rejected', 'Команда не отправлена.'],
    ['terminal_observed', 'Запуск завершён. Доставка команды не подтверждена.'],
  ] as const)('shows authoritative %s without dispatching a command', async (state, text) => {
    vi.mocked(listRuntimeControls).mockResolvedValue([{ ...receipt, state }])
    show()
    expect(await screen.findByText(text)).toBeVisible()
    expect(listRuntimeControls).toHaveBeenCalledWith('session', 'run')
    expect(screen.getAllByRole('button')).toHaveLength(1)
    expect(screen.getByRole('button', { name: 'Проверить состояние команд' })).toBeVisible()
  })

  it.each(['stopping', 'already_terminal'])(
    'does not equate %s ACK with physical stop',
    async (acknowledgement) => {
      vi.mocked(listRuntimeControls).mockResolvedValue([
        { ...receipt, operation: 'stop', state: 'acknowledged', acknowledgement },
      ])
      show()
      expect(
        await screen.findByText(
          /Остановка процесса не подтверждена|остановка процесса не подтверждены/,
        ),
      ).toBeVisible()
      expect(screen.queryByText('Команда принята runtime.')).not.toBeInTheDocument()
    },
  )

  it('reads the original run again, never posts, when refreshed', async () => {
    vi.mocked(listRuntimeControls).mockResolvedValue([receipt])
    show()
    await screen.findByText('Исход команды неизвестен. Повторная отправка не разрешена.')
    fireEvent.click(screen.getByRole('button', { name: 'Проверить состояние команд' }))
    await waitFor(() => expect(listRuntimeControls).toHaveBeenCalledTimes(2))
    expect(vi.mocked(listRuntimeControls).mock.calls).toEqual([
      ['session', 'run'],
      ['session', 'run'],
    ])
  })

  it.each([{ session_id: 'foreign' }, { session_run_id: 'foreign' }])(
    'does not show a mismatched receipt %j',
    async (identity) => {
      vi.mocked(listRuntimeControls).mockResolvedValue([{ ...receipt, ...identity }])
      show()
      expect(await screen.findByRole('alert')).toHaveTextContent('Состояние команд недоступно')
      expect(screen.queryByText('Уточнение запуску')).not.toBeInTheDocument()
    },
  )

  it('reports failed readback without claiming delivery or completion', async () => {
    vi.mocked(listRuntimeControls).mockRejectedValue(new Error('private transport error'))
    show()
    expect(await screen.findByRole('alert')).toHaveTextContent('Это не подтверждает доставку')
    expect(screen.queryByText('private transport error')).not.toBeInTheDocument()
  })

  it('does not fetch without a concrete run', () => {
    const view = show(null)
    expect(view.container).toBeEmptyDOMElement()
    expect(listRuntimeControls).not.toHaveBeenCalled()
  })

  it('does not render an empty command history as success', async () => {
    vi.mocked(listRuntimeControls).mockResolvedValue([])
    const view = show()
    await waitFor(() => expect(view.container).toBeEmptyDOMElement())
    expect(listRuntimeControls).toHaveBeenCalledTimes(1)
  })
})
