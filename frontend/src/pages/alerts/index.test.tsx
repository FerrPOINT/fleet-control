import { describe, expect, it, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter } from 'react-router'
import { AlertsPage } from './index'
import * as fleet from '@/api/fleet'

vi.mock('@/api/fleet', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/fleet')>()),
  listFleetAlerts: vi.fn(),
  acknowledgeFleetAlert: vi.fn(),
}))

const alertsFixture = [
  {
    id: '00000000-0000-4000-8000-000000000a01',
    agent_id: '00000000-0000-4000-8000-000000000101',
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
    agent_id: '00000000-0000-4000-8000-000000000102',
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

function renderPage() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={['/alerts']}>
        <AlertsPage />
      </MemoryRouter>
    </QueryClientProvider>,
  )
}

describe('AlertsPage', () => {
  beforeEach(() => {
    vi.mocked(fleet.listFleetAlerts).mockResolvedValue(alertsFixture as never)
    vi.mocked(fleet.acknowledgeFleetAlert).mockResolvedValue(alertsFixture[0] as never)
  })

  it('lists fleet alerts with kind, severity and state badges', async () => {
    renderPage()

    await waitFor(() => expect(screen.getByText('agent_down')).toBeInTheDocument())
    expect(screen.getByText('agent_recovered')).toBeInTheDocument()
    expect(screen.getByText('critical')).toBeInTheDocument()
    expect(screen.getByText('info')).toBeInTheDocument()
    expect(screen.getAllByText('open').length).toBeGreaterThan(0)
    expect(screen.getAllByText('resolved').length).toBeGreaterThan(0)
  })

  it('shows acknowledge action only for open alerts', async () => {
    renderPage()

    await waitFor(() => expect(screen.getByText('agent_down')).toBeInTheDocument())
    const ackButtons = screen.queryAllByRole('button', { name: /^acknowledge$/i })
    expect(ackButtons).toHaveLength(1)
  })

  it('filters by state through the backend query', async () => {
    renderPage()

    await waitFor(() => expect(fleet.listFleetAlerts).toHaveBeenCalledWith(undefined))
  })
})
