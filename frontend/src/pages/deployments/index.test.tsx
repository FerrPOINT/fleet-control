import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter } from 'react-router'
import { toast } from 'sonner'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { DeploymentsPage } from './index'
import * as fleet from '@/api/fleet'
import type { Agent, DeploymentJob, RuntimeTemplate } from '@/api/types'

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
vi.mock('sonner', () => ({ toast: { success: vi.fn() } }))

const agentsFixture = [
  {
    id: 'a-101',
    name: 'agent1',
    display_name: 'Dev One',
    status: 'ready',
    kind: 'hermes',
    product_role: 'executor',
    role: 'developer',
    ordinal: 1,
    paths: { runtime: 'C:/agents/agent1/runtime' },
  },
  {
    id: 'a-102',
    name: 'agent2',
    display_name: 'Dev Two',
    status: 'ready',
    kind: 'hermes',
    product_role: 'executor',
    role: 'developer',
    ordinal: 2,
    paths: { runtime: 'C:/agents/agent2/runtime' },
  },
  {
    id: 'a-103',
    name: 'agent3',
    display_name: 'Gone',
    status: 'archived',
    kind: 'hermes',
    product_role: 'executor',
    role: 'tester',
    ordinal: 3,
    paths: { runtime: 'C:/agents/agent3/runtime' },
  },
] as Agent[]

const queuedJob = {
  id: 'job-1',
  job_kind: 'provision',
  state: 'queued',
  agent_id: 'a-101',
  runtime_kind: 'hermes',
  requested_by_user_id: 'user-1',
  title: 'Prepare Dev One',
  detail: { requested_from: 'test' },
  last_error: null,
  created_at: '2026-09-21T08:00:00Z',
  updated_at: '2026-09-21T08:00:00Z',
} as DeploymentJob

const cancelledJob = { ...queuedJob, state: 'cancelled' } as DeploymentJob

const templates = [
  {
    kind: 'hermes',
    display_name: 'Hermes',
    implemented: true,
    enabled: true,
    description: 'Managed Hermes runtime.',
    capabilities: { sessions: true },
  },
] as RuntimeTemplate[]

function renderPage(tab = 'jobs', jobId?: string) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const suffix = jobId ? `&job_id=${jobId}` : ''
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[`/deployments?tab=${tab}${suffix}`]}>
        <DeploymentsPage />
      </MemoryRouter>
    </QueryClientProvider>,
  )
}

describe('DeploymentsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(fleet.listDeploymentJobs).mockResolvedValue([queuedJob])
    vi.mocked(fleet.listAgents).mockResolvedValue(agentsFixture)
    vi.mocked(fleet.listRuntimeTemplates).mockResolvedValue(templates)
    vi.mocked(fleet.getDeploymentJob).mockResolvedValue(queuedJob)
    vi.mocked(fleet.createDeploymentJob).mockResolvedValue(queuedJob)
    vi.mocked(fleet.cancelDeploymentJob).mockResolvedValue(cancelledJob)
    vi.mocked(fleet.bulkCreateDeploymentJobs).mockResolvedValue({
      jobs: [queuedJob, { ...queuedJob, id: 'job-2' }],
      created: 2,
      skipped: 1,
    })
  })

  it('sends the required bulk title and reads numeric result counters', async () => {
    renderPage()

    await screen.findByRole('heading', { name: 'Массовое обновление' })
    fireEvent.click(await screen.findByRole('checkbox', { name: /Dev One/ }))
    fireEvent.click(screen.getByRole('checkbox', { name: /Dev Two/ }))
    fireEvent.click(screen.getByRole('checkbox', { name: /Вернуть предыдущую версию/ }))
    fireEvent.click(screen.getByRole('button', { name: 'Создать задания обновления' }))

    await waitFor(() =>
      expect(fleet.bulkCreateDeploymentJobs).toHaveBeenCalledWith({
        job_kind: 'runtime_update',
        agent_ids: ['a-101', 'a-102'],
        title: 'Обновление среды выбранных агентов',
        rollback: true,
        detail: { requested_from: 'deployments_page_bulk' },
      }),
    )
    expect(await screen.findByRole('status')).toHaveTextContent('Создано: 2. Пропущено: 1.')
    expect(toast.success).toHaveBeenCalledWith('Создано заданий: 2')
  })

  it('normalizes a single job title and updates the default title with the operation', async () => {
    renderPage()

    await screen.findByRole('heading', { name: 'Новое задание' })
    fireEvent.change(screen.getByLabelText('Операция'), { target: { value: 'runtime_update' } })
    expect(
      screen.getByLabelText('Название задания', { selector: '#deployment-title' }),
    ).toHaveValue('Обновление среды агента')
    fireEvent.change(screen.getByLabelText('Агент'), { target: { value: 'a-101' } })
    fireEvent.change(screen.getByLabelText('Название задания', { selector: '#deployment-title' }), {
      target: { value: '  Update Dev One  ' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Создать задание' }))

    await waitFor(() =>
      expect(fleet.createDeploymentJob).toHaveBeenCalledWith({
        title: 'Update Dev One',
        job_kind: 'runtime_update',
        runtime_kind: 'hermes',
        agent_id: 'a-101',
        detail: { requested_from: 'deployments_page' },
      }),
    )
    expect(toast.success).toHaveBeenCalledWith('Задание «Prepare Dev One» создано')
  })

  it('distinguishes a failed job request from an empty list and retries it', async () => {
    vi.mocked(fleet.listDeploymentJobs)
      .mockRejectedValueOnce(new Error('offline'))
      .mockResolvedValueOnce([])
    renderPage()

    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent('Не удалось загрузить задания')
    expect(screen.queryByText('Заданий развёртывания пока нет')).not.toBeInTheDocument()
    fireEvent.click(within(alert.parentElement!).getByRole('button', { name: 'Повторить' }))

    expect(await screen.findByText('Заданий развёртывания пока нет')).toBeVisible()
  })

  it('keeps a job visible after cancellation fails and confirms a successful retry', async () => {
    vi.mocked(fleet.cancelDeploymentJob)
      .mockRejectedValueOnce(new Error('conflict'))
      .mockResolvedValueOnce(cancelledJob)
    renderPage()

    const cancel = await screen.findByRole('button', { name: 'Отменить' })
    fireEvent.click(cancel)
    expect(await screen.findByRole('alert')).toHaveTextContent('Не удалось отменить задание')
    expect(screen.getByText('Prepare Dev One')).toBeVisible()

    fireEvent.click(cancel)
    await waitFor(() =>
      expect(toast.success).toHaveBeenCalledWith('Задание «Prepare Dev One» отменено'),
    )
    expect(fleet.cancelDeploymentJob).toHaveBeenCalledTimes(2)
  })

  it('loads a requested job detail and exposes technical data on a direct URL', async () => {
    renderPage('detail', queuedJob.id)

    expect(await screen.findByRole('heading', { name: queuedJob.title })).toBeVisible()
    expect(screen.getByRole('heading', { name: 'Детали задания' })).toBeVisible()
    expect(screen.getByRole('heading', { name: 'Технические данные' })).toBeVisible()
    expect(fleet.getDeploymentJob).toHaveBeenCalledWith(queuedJob.id)
    expect(fleet.listDeploymentJobs).not.toHaveBeenCalled()
  })

  it('shows overview query errors independently', async () => {
    vi.mocked(fleet.listRuntimeTemplates).mockRejectedValue(new Error('templates offline'))
    renderPage('overview')

    expect(await screen.findByText('Не удалось загрузить шаблоны сред.')).toBeVisible()
    expect(screen.getByText('Dev One')).toBeVisible()
    expect(screen.queryByText('Шаблонов сред пока нет')).not.toBeInTheDocument()
  })
})
