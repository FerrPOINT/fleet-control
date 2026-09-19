import { beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter } from 'react-router'
import { toast } from 'sonner'
import { WorkflowsPage } from './index'
import * as fleet from '@/api/fleet'
import type { Agent, WorkflowBinding, WorkflowCatalog } from '@/api/types'

vi.mock('@/api/fleet', () => ({
  getWorkflowCatalog: vi.fn(),
  listAgents: vi.fn(),
  listWorkflowBindings: vi.fn(),
  rebindWorkflowBinding: vi.fn(),
}))
vi.mock('sonner', () => ({ toast: { success: vi.fn() } }))

const agents = [
  { id: 'agent-1', name: 'first', display_name: 'First Agent' },
  { id: 'agent-2', name: 'second', display_name: 'Second Agent' },
] as Agent[]

const bindings = [
  {
    id: 'binding-1',
    agent_id: 'agent-1',
    workflow_name: 'Old workflow',
    namespace_name: 'Old namespace',
    binding_status: 'stale',
  },
  {
    id: 'binding-2',
    agent_id: 'agent-2',
    workflow_name: 'Other workflow',
    namespace_name: 'Other namespace',
    binding_status: 'stale',
  },
] as WorkflowBinding[]

const catalog = {
  namespaces: [
    { id: 'namespace-1', name: 'First namespace', workflow_id: 'workflow-1' },
    { id: 'namespace-2', name: 'Second namespace', workflow_id: 'workflow-2' },
  ],
  workflows: [
    { id: 'workflow-1', name: 'First workflow' },
    { id: 'workflow-2', name: 'Second workflow' },
  ],
} as WorkflowCatalog

function renderPage() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter>
        <WorkflowsPage />
      </MemoryRouter>
    </QueryClientProvider>,
  )
}

describe('WorkflowsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(fleet.listAgents).mockResolvedValue(agents)
    vi.mocked(fleet.listWorkflowBindings).mockResolvedValue(bindings)
    vi.mocked(fleet.getWorkflowCatalog).mockResolvedValue(catalog)
  })

  it('filters bindings by agent and status without losing the full list', async () => {
    renderPage()
    await screen.findByText('First Agent')
    expect(screen.getByText('Second Agent')).toBeInTheDocument()

    fireEvent.change(screen.getByLabelText('Поиск по агенту и процессу'), {
      target: { value: 'First Agent' },
    })
    expect(screen.getByText('First Agent')).toBeInTheDocument()
    expect(screen.queryByText('Second Agent')).not.toBeInTheDocument()

    fireEvent.change(screen.getByLabelText('Статус'), { target: { value: 'connected' } })
    expect(screen.getByText('По заданным условиям привязок нет')).toBeInTheDocument()
    fireEvent.change(screen.getByLabelText('Статус'), { target: { value: 'all' } })
    expect(screen.getByText('First Agent')).toBeInTheDocument()
  })

  it('keeps the chosen namespace after a failed rebind and permits a retry', async () => {
    vi.mocked(fleet.rebindWorkflowBinding)
      .mockRejectedValueOnce(new Error('upstream unavailable'))
      .mockResolvedValueOnce(bindings[0]!)
    renderPage()
    const selector = await screen.findByLabelText('Новое пространство для First Agent')
    fireEvent.change(selector, { target: { value: 'namespace-2' } })
    fireEvent.click(screen.getAllByRole('button', { name: 'Перепривязать' })[0]!)

    await screen.findByText(/upstream unavailable/)
    expect(selector).toHaveValue('namespace-2')
    expect(fleet.rebindWorkflowBinding).toHaveBeenCalledWith('agent-1', {
      namespace_id: 'namespace-2',
      workflow_id: 'workflow-2',
    })

    fireEvent.click(screen.getAllByRole('button', { name: 'Перепривязать' })[0]!)
    await waitFor(() => expect(fleet.rebindWorkflowBinding).toHaveBeenCalledTimes(2))
    await waitFor(() => expect(toast.success).toHaveBeenCalledWith('Привязка обновлена'))
  })

  it('does not block another binding while one rebind is pending', async () => {
    let finishFirst: (value: WorkflowBinding) => void = () => undefined
    vi.mocked(fleet.rebindWorkflowBinding).mockImplementation((id) =>
      id === 'agent-1'
        ? new Promise<WorkflowBinding>((resolve) => {
            finishFirst = resolve
          })
        : Promise.resolve(bindings[1]!),
    )
    renderPage()
    await screen.findByText('First Agent')
    fireEvent.click(screen.getAllByRole('button', { name: 'Перепривязать' })[0]!)
    await screen.findByRole('button', { name: 'Сохраняем...' })
    const otherButton = screen.getByRole('button', { name: 'Перепривязать' })
    expect(otherButton).toBeEnabled()
    fireEvent.click(otherButton)
    await waitFor(() =>
      expect(fleet.rebindWorkflowBinding).toHaveBeenCalledWith('agent-2', expect.anything()),
    )
    finishFirst(bindings[0]!)
  })

  it('shows a catalog error and restores rebinding after retry', async () => {
    vi.mocked(fleet.getWorkflowCatalog)
      .mockRejectedValueOnce(new Error('offline'))
      .mockResolvedValueOnce(catalog)
    renderPage()
    await screen.findByText(/Каталог Project Workflow недоступен/)
    expect(screen.getAllByRole('button', { name: 'Перепривязать' })[0]).toBeDisabled()
    const error = screen.getByRole('alert')
    fireEvent.click(within(error.parentElement!).getByRole('button', { name: 'Повторить' }))
    await waitFor(() =>
      expect(screen.getAllByRole('button', { name: 'Перепривязать' })[0]).toBeEnabled(),
    )
  })
})
