import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter } from 'react-router'
import { toast } from 'sonner'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import * as fleet from '@/api/fleet'
import type { AgentDirectoryItem } from '@/api/types'
import { AlertsPage } from './index'

vi.mock('@/api/fleet', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/fleet')>()),
  listFleetAlerts: vi.fn(),
  listAgentDirectory: vi.fn(),
  acknowledgeFleetAlert: vi.fn(),
}))
vi.mock('sonner', () => ({ toast: { success: vi.fn() } }))

const agentId = '00000000-0000-4000-8000-000000000101'
const alertsFixture: fleet.FleetAlert[] = [
  {
    id: '00000000-0000-4000-8000-000000000a01',
    agent_id: agentId,
    kind: 'agent_down',
    severity: 'critical',
    detail: { previous: 'running', current: 'failed' },
    state: 'open',
    opened_at: '2026-09-14T10:00:00+00:00',
    resolved_at: null,
    acknowledged_at: null,
    acknowledged_by_user_id: null,
  },
  {
    id: '00000000-0000-4000-8000-000000000a02',
    agent_id: null,
    kind: 'agent_recovered',
    severity: 'info',
    detail: { recovered_to: 'running' },
    state: 'resolved',
    opened_at: '2026-09-14T09:00:00+00:00',
    resolved_at: '2026-09-14T09:05:00+00:00',
    acknowledged_at: null,
    acknowledged_by_user_id: null,
  },
]

const agentsFixture: AgentDirectoryItem[] = [
  {
    id: agentId,
    ordinal: 1,
    name: 'agent1',
    kind: 'hermes',
    product_role: 'executor',
    role: 'developer',
    status: 'failed',
    display_name: 'Dev One',
    description: null,
    namespace_id: null,
    workflow_id: null,
    runtime_version: null,
    dashboard_port: null,
    api_port: null,
  },
]

function renderPage(entry = '/alerts') {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[entry]}>
        <AlertsPage />
      </MemoryRouter>
    </QueryClientProvider>,
  )
}

describe('AlertsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(fleet.listFleetAlerts).mockResolvedValue(alertsFixture)
    vi.mocked(fleet.listAgentDirectory).mockResolvedValue(agentsFixture)
    vi.mocked(fleet.acknowledgeFleetAlert).mockResolvedValue({
      ...alertsFixture[0]!,
      state: 'acknowledged',
      acknowledged_at: '2026-09-14T10:03:00+00:00',
    })
  })

  it('renders localized responsive rows with agent context and technical detail', async () => {
    renderPage()

    expect(await screen.findByRole('heading', { name: 'Агент недоступен' })).toBeVisible()
    expect(screen.getByText('Критическое')).toBeVisible()
    expect(screen.getByText('Dev One · agent1')).toBeVisible()
    expect(screen.getByText(agentId)).toBeVisible()
    expect(screen.getByRole('heading', { name: 'Агент восстановлен' })).toBeVisible()
    expect(screen.getByText('Весь контур')).toBeVisible()

    fireEvent.click(screen.getAllByText('Технические данные')[0]!)
    expect(screen.getByText(/"previous": "running"/)).toBeVisible()
    expect(screen.getAllByRole('button', { name: 'Подтвердить' })).toHaveLength(1)
  })

  it('keeps the state filter in the URL-backed query', async () => {
    renderPage()
    await screen.findByRole('heading', { name: 'Агент недоступен' })

    fireEvent.click(screen.getByRole('button', { name: 'Открытые' }))

    await waitFor(() => expect(fleet.listFleetAlerts).toHaveBeenLastCalledWith('open'))
    expect(screen.getByRole('button', { name: 'Открытые' })).toHaveAttribute('aria-pressed', 'true')
  })

  it('distinguishes a load failure from an empty result and retries', async () => {
    vi.mocked(fleet.listFleetAlerts)
      .mockRejectedValueOnce(new Error('offline'))
      .mockResolvedValueOnce([])
    renderPage()

    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent('Не удалось загрузить оповещения')
    expect(screen.queryByText('Оповещений пока нет')).not.toBeInTheDocument()
    fireEvent.click(within(alert.parentElement!).getByRole('button', { name: 'Повторить' }))

    expect(await screen.findByText('Оповещений пока нет')).toBeVisible()
  })

  it('keeps an alert actionable after acknowledgement fails and confirms a retry', async () => {
    vi.mocked(fleet.acknowledgeFleetAlert)
      .mockRejectedValueOnce(new Error('conflict'))
      .mockResolvedValueOnce({
        ...alertsFixture[0]!,
        state: 'acknowledged',
        acknowledged_at: '2026-09-14T10:03:00+00:00',
      })
    renderPage()

    fireEvent.click(await screen.findByRole('button', { name: 'Подтвердить' }))
    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent('Не удалось подтвердить оповещение')
    expect(screen.getByRole('heading', { name: 'Агент недоступен' })).toBeVisible()

    fireEvent.click(screen.getByRole('button', { name: 'Повторить подтверждение' }))
    await waitFor(() => expect(fleet.acknowledgeFleetAlert).toHaveBeenCalledTimes(2))
    expect(fleet.acknowledgeFleetAlert).toHaveBeenLastCalledWith(alertsFixture[0]!.id)
    expect(toast.success).toHaveBeenCalledWith('Оповещение подтверждено')
  })

  it('falls back to an agent ID when the directory is unavailable', async () => {
    vi.mocked(fleet.listAgentDirectory)
      .mockRejectedValueOnce(new Error('directory offline'))
      .mockResolvedValueOnce(agentsFixture)
    renderPage()

    expect(await screen.findByText('Неизвестный агент')).toBeVisible()
    expect(screen.getByText(agentId)).toBeVisible()
    const directoryError = screen.getByText(/Не удалось загрузить имена агентов/)
    fireEvent.click(
      within(directoryError.parentElement!.parentElement!).getByRole('button', {
        name: 'Повторить',
      }),
    )

    expect(await screen.findByText('Dev One · agent1')).toBeVisible()
  })
})
