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

function renderPage(retry: boolean | number = false) {
  const client = new QueryClient({ defaultOptions: { queries: { retry, retryDelay: 1 } } })
  const result = render(
    <QueryClientProvider client={client}>
      <MemoryRouter>
        <WorkflowsPage />
      </MemoryRouter>
    </QueryClientProvider>,
  )
  return { ...result, client }
}

describe('WorkflowsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(fleet.listAgents).mockResolvedValue(agents)
    vi.mocked(fleet.listWorkflowBindings).mockResolvedValue(bindings)
    vi.mocked(fleet.getWorkflowCatalog).mockResolvedValue(catalog)
  })

  it.each(['pending read', 'failed retry'] as const)(
    'holds cached rebind choices during a catalog %s until a fresh success',
    async (phase) => {
      const { client } = renderPage(2)
      const selector = await screen.findByLabelText('Новое пространство для First Agent')
      const button = screen.getAllByRole('button', { name: 'Перепривязать' })[0]!
      await waitFor(() => expect(button).toBeEnabled())
      fireEvent.change(selector, { target: { value: 'namespace-2' } })
      let finish!: (value: WorkflowCatalog) => void
      const pending = new Promise<WorkflowCatalog>((resolve) => {
        finish = resolve
      })
      if (phase === 'failed retry')
        vi.mocked(fleet.getWorkflowCatalog).mockRejectedValueOnce(new Error('catalog GET 503'))
      vi.mocked(fleet.getWorkflowCatalog).mockReturnValueOnce(pending)
      const refresh = client.invalidateQueries({ queryKey: ['workflow-catalog'] })
      try {
        await waitFor(() =>
          expect(fleet.getWorkflowCatalog).toHaveBeenCalledTimes(phase === 'failed retry' ? 3 : 2),
        )
        expect(client.getQueryState(['workflow-catalog'])?.status).toBe('success')
        expect(client.getQueryState(['workflow-catalog'])?.fetchFailureCount).toBe(
          phase === 'failed retry' ? 1 : 0,
        )
        expect(selector).toHaveValue('namespace-2')
        expect(selector).toBeDisabled()
        expect(button).toBeDisabled()
        fireEvent.click(button)
        expect(fleet.rebindWorkflowBinding).not.toHaveBeenCalled()
      } finally {
        finish(catalog)
        await refresh
      }
      await waitFor(() => expect(button).toBeEnabled())
      expect(selector).toHaveValue('namespace-2')
      expect(fleet.rebindWorkflowBinding).not.toHaveBeenCalled()
      fireEvent.click(button)
      await waitFor(() =>
        expect(fleet.rebindWorkflowBinding).toHaveBeenCalledWith('agent-1', {
          namespace_id: 'namespace-2',
          workflow_id: 'workflow-2',
        }),
      )
    },
  )

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
  it('holds cached rebind choices after a failed catalog refresh and preserves selection', async () => {
    const { client } = renderPage()
    const selector = await screen.findByLabelText('Новое пространство для First Agent')
    fireEvent.change(selector, { target: { value: 'namespace-2' } })
    vi.mocked(fleet.getWorkflowCatalog).mockRejectedValue(new Error('catalog refresh offline'))
    await client.invalidateQueries({ queryKey: ['workflow-catalog'] })
    await screen.findByText(/Каталог Project Workflow недоступен/)
    expect(selector).toBeDisabled()
    expect(selector).toHaveValue('namespace-2')
    const button = screen.getAllByRole('button', { name: 'Перепривязать' })[0]!
    expect(button).toBeDisabled()
    fireEvent.click(button)
    expect(fleet.rebindWorkflowBinding).not.toHaveBeenCalled()
    vi.mocked(fleet.getWorkflowCatalog).mockResolvedValue(catalog)
    const error = screen.getByRole('alert')
    fireEvent.click(within(error.parentElement!).getByRole('button', { name: 'Повторить' }))
    await waitFor(() => expect(button).toBeEnabled())
    expect(selector).toHaveValue('namespace-2')
    fireEvent.click(button)
    await waitFor(() =>
      expect(fleet.rebindWorkflowBinding).toHaveBeenCalledWith('agent-1', {
        namespace_id: 'namespace-2',
        workflow_id: 'workflow-2',
      }),
    )
  })
})
