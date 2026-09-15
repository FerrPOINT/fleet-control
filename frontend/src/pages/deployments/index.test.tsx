import { describe, expect, it, vi, beforeEach } from 'vitest'
import { render, screen, waitFor, fireEvent } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter } from 'react-router'
import { DeploymentsPage } from './index'
import * as fleet from '@/api/fleet'

vi.mock('@/api/fleet', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/fleet')>()),
  listDeploymentJobs: vi.fn(),
  listAgents: vi.fn(),
  listRuntimeTemplates: vi.fn(),
  getDeploymentJob: vi.fn(),
  cancelDeploymentJob: vi.fn(),
  createDeploymentJob: vi.fn(),
  bulkCreateDeploymentJobs: vi.fn(),
}))

const agentsFixture = [
  { id: 'a-101', name: 'agent1', display_name: 'Dev One', status: 'ready', kind: 'hermes', product_role: 'executor', role: 'developer', ordinal: 1 },
  { id: 'a-102', name: 'agent2', display_name: 'Dev Two', status: 'ready', kind: 'hermes', product_role: 'executor', role: 'developer', ordinal: 2 },
  { id: 'a-103', name: 'agent3', display_name: 'Gone', status: 'archived', kind: 'hermes', product_role: 'executor', role: 'tester', ordinal: 3 },
]

function renderPage(tab = 'jobs') {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[`/deployments?tab=${tab}`]}>
        <DeploymentsPage />
      </MemoryRouter>
    </QueryClientProvider>,
  )
}

describe('DeploymentsPage bulk actions', () => {
  beforeEach(() => {
    vi.mocked(fleet.listDeploymentJobs).mockResolvedValue([] as never)
    vi.mocked(fleet.listAgents).mockResolvedValue(agentsFixture as never)
    vi.mocked(fleet.listRuntimeTemplates).mockResolvedValue([] as never)
    vi.mocked(fleet.bulkCreateDeploymentJobs).mockResolvedValue({
      created: ['a-101', 'a-102'],
      skipped: [{ agent_id: 'a-103', reason: 'archived' }],
    } as never)
  })

  it('offers bulk runtime update over selected non-archived agents', async () => {
    renderPage('jobs')

    await waitFor(() => {
      expect(screen.getByText('Bulk actions')).toBeInTheDocument()
    })

    await screen.findAllByText(/Dev One/)
    const checks = screen.getAllByRole('checkbox')
    expect(checks.length).toBeGreaterThanOrEqual(2)
    const first = checks[0]!
    const second = checks[1]!
    fireEvent.click(first)
    fireEvent.click(second)

    fireEvent.click(screen.getByRole('button', { name: /bulk create/i }))

    await waitFor(() => {
      expect(fleet.bulkCreateDeploymentJobs).toHaveBeenCalledWith(
        expect.objectContaining({
          job_kind: 'runtime_update',
          agent_ids: ['a-101', 'a-102'],
        }),
      )
    })
  })
})
