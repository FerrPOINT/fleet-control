import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter } from 'react-router'
import { DashboardPage } from './index'
import * as fleet from '@/api/fleet'

vi.mock('@/api/fleet', () => ({ getDashboard: vi.fn() }))

describe('DashboardPage localization', () => {
  it('shows Russian labels and empty states', async () => {
    vi.mocked(fleet.getDashboard).mockResolvedValue({
      total_agents: 0,
      leader_agents: 0,
      executor_agents: 0,
      running_agents: 0,
      failed_agents: 0,
      active_sessions: 0,
      private_sessions: 0,
      leader_scoped_sessions: 0,
      agents: [],
      recent_events: [],
    } as never)
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })

    render(
      <QueryClientProvider client={client}>
        <MemoryRouter>
          <DashboardPage />
        </MemoryRouter>
      </QueryClientProvider>,
    )

    expect(await screen.findByRole('heading', { name: 'Обзор агентов' })).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Создать агента' })).toBeInTheDocument()
    expect(await screen.findByText('Агентов пока нет')).toBeInTheDocument()
    expect(screen.getByText('Событий пока нет')).toBeInTheDocument()
    expect(screen.queryByText('Recent events')).not.toBeInTheDocument()
  })
})
