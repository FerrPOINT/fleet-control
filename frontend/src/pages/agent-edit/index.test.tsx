import { beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter, Route, Routes } from 'react-router'
import { AgentEditPage } from './index'
import * as fleet from '@/api/fleet'
import type { Agent } from '@/api/types'

vi.mock('@/api/fleet', () => ({
  getAgent: vi.fn(),
  listExecutors: vi.fn(),
  listLeaderExecutors: vi.fn(),
  updateAgent: vi.fn(),
}))

const executor = {
  id: 'executor-1',
  name: 'agent1',
  display_name: 'Executor One',
  kind: 'hermes',
  product_role: 'executor',
  role: 'developer',
  status: 'ready',
  description: '',
  namespace_id: null,
  workflow_id: null,
} as Agent
const leader = {
  ...executor,
  id: 'leader-1',
  name: 'agent2',
  display_name: 'Leader One',
  product_role: 'leader',
  role: 'it_lead',
} as Agent

function renderEdit(role: 'executor' | 'leader') {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const path = role === 'leader' ? '/leaders/leader-1/edit' : '/executors/executor-1/edit'
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[path]}>
        <Routes>
          <Route
            path="/leaders/:leaderId/edit"
            element={<AgentEditPage defaultProductRole="leader" />}
          />
          <Route
            path="/executors/:agentId/edit"
            element={<AgentEditPage defaultProductRole="executor" />}
          />
          <Route path="/leaders/:leaderId" element={<span>Leader saved</span>} />
          <Route path="/executors/:agentId" element={<span>Executor saved</span>} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  )
}

describe('AgentEditPage team loading', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(fleet.listExecutors).mockResolvedValue([executor])
    vi.mocked(fleet.listLeaderExecutors).mockResolvedValue([])
    vi.mocked(fleet.updateAgent).mockImplementation(
      async (_id, request) =>
        ({
          ...executor,
          ...request,
        }) as Agent,
    )
  })

  it('does not request a leader team when editing an executor', async () => {
    vi.mocked(fleet.getAgent).mockResolvedValue(executor)
    renderEdit('executor')

    await screen.findByRole('heading', { name: 'Edit Executor One' })
    expect(fleet.listLeaderExecutors).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: 'Save agent' }))

    await waitFor(() => expect(fleet.updateAgent).toHaveBeenCalledOnce())
    expect(fleet.updateAgent).toHaveBeenCalledWith(
      executor.id,
      expect.not.objectContaining({ executor_ids: expect.anything() }),
    )
  })

  it('blocks a leader save until the team loads, then preserves assignments', async () => {
    vi.mocked(fleet.getAgent).mockResolvedValue(leader)
    vi.mocked(fleet.listLeaderExecutors)
      .mockRejectedValueOnce(new Error('service unavailable'))
      .mockResolvedValueOnce([{ executor_agent_id: executor.id }] as never)
    renderEdit('leader')

    await screen.findByRole('heading', { name: 'Edit Leader One' })
    await screen.findByText(/Could not load managed executors: service unavailable/)
    expect(screen.getByRole('button', { name: 'Save agent' })).toBeDisabled()
    expect(screen.queryByRole('checkbox')).not.toBeInTheDocument()
    expect(fleet.updateAgent).not.toHaveBeenCalled()

    fireEvent.click(screen.getByRole('button', { name: 'Retry' }))
    await waitFor(() => expect(screen.getByRole('button', { name: 'Save agent' })).toBeEnabled())
    expect(screen.getByRole('checkbox')).toBeChecked()
    fireEvent.click(screen.getByRole('button', { name: 'Save agent' }))

    await waitFor(() => expect(fleet.updateAgent).toHaveBeenCalledOnce())
    expect(fleet.updateAgent).toHaveBeenCalledWith(
      leader.id,
      expect.objectContaining({ executor_ids: [executor.id] }),
    )
  })

  it('can promote an executor without querying a non-existent leader team', async () => {
    vi.mocked(fleet.getAgent).mockResolvedValue(executor)
    renderEdit('executor')

    await screen.findByRole('heading', { name: 'Edit Executor One' })
    fireEvent.change(screen.getByLabelText('Product role'), { target: { value: 'leader' } })
    await screen.findByText('Managed executors')
    expect(fleet.listLeaderExecutors).not.toHaveBeenCalled()
    expect(screen.getByRole('button', { name: 'Save agent' })).toBeEnabled()
    fireEvent.click(screen.getByRole('button', { name: 'Save agent' }))

    await waitFor(() => expect(fleet.updateAgent).toHaveBeenCalledOnce())
    expect(fleet.updateAgent).toHaveBeenCalledWith(
      executor.id,
      expect.objectContaining({ product_role: 'leader', executor_ids: [] }),
    )
  })
})
