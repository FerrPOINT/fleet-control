import { beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, render, screen } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter } from 'react-router'
import { AgentsPage } from './index'
import * as fleet from '@/api/fleet'
import type { Agent, AgentStorageReview, RuntimeTemplate } from '@/api/types'

vi.mock('@/api/fleet', () => ({
  createAgent: vi.fn(),
  getAgentStorageReview: vi.fn(),
  listAgents: vi.fn(),
  listExecutors: vi.fn(),
  listRuntimeTemplates: vi.fn(),
  listSessions: vi.fn(),
}))

vi.mock('@/shared/session-user-filter', () => ({
  SessionUserFilter: () => <div data-testid="session-user-filter" />,
  useSessionUserFilter: () => ({ selectedUserIds: [] }),
}))

const agent = {
  id: 'agent-1',
  name: 'agent1',
  kind: 'hermes',
  product_role: 'executor',
  role: 'developer',
  status: 'ready',
  display_name: 'Developer Hermes',
  namespace_id: 'dev',
  workflow_id: 'workflow-dev',
  api_port: 29001,
  dashboard_port: 29002,
} as Agent

const storageReview = {
  reviewed_at: '2026-09-22T09:00:00Z',
  total_agents: 1,
  total_bytes: 2048,
  archived_agents: 0,
  archived_bytes: 0,
  purge_eligible_agents: 0,
  missing_root_agents: 0,
  marker_issue_agents: 0,
  items: [],
} satisfies AgentStorageReview

const templates = [
  {
    kind: 'hermes',
    display_name: 'Hermes',
    implemented: true,
    enabled: true,
    description: 'Hermes runtime',
    capabilities: {},
  },
] satisfies RuntimeTemplate[]

function renderPage(createMode = false) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter>
        <AgentsPage createMode={createMode} />
      </MemoryRouter>
    </QueryClientProvider>,
  )
}

describe('AgentsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(fleet.listAgents).mockResolvedValue([agent])
    vi.mocked(fleet.listRuntimeTemplates).mockResolvedValue(templates)
    vi.mocked(fleet.listSessions).mockResolvedValue([])
    vi.mocked(fleet.getAgentStorageReview).mockResolvedValue(storageReview)
    vi.mocked(fleet.listExecutors).mockResolvedValue([])
  })

  it('renders a localized compact directory with explicit empty session and storage states', async () => {
    renderPage()

    expect(await screen.findByRole('heading', { name: 'Агенты' })).toBeVisible()
    expect(screen.getByRole('link', { name: 'Создать агента' })).toBeVisible()
    expect(await screen.findByRole('link', { name: 'Открыть' })).toHaveClass('h-10')
    expect(screen.getByText('Developer Hermes').closest('article')).toBeVisible()
    expect(screen.getByText('Панель агента')).toBeVisible()
    expect(screen.getByText('Сессий пока нет')).toBeVisible()
    expect(screen.getByText('Хранилище агентов')).toBeVisible()
    expect(screen.getByText('Нет архивных агентов, готовых к физической очистке')).toBeVisible()
    expect(screen.queryByText('Storage review')).not.toBeInTheDocument()
    expect(screen.queryByText('No sessions yet')).not.toBeInTheDocument()
  })

  it('retries a failed agent request without presenting an empty directory', async () => {
    vi.mocked(fleet.listAgents)
      .mockRejectedValueOnce(new Error('offline'))
      .mockResolvedValueOnce([agent])
    renderPage()

    expect(await screen.findByRole('alert')).toHaveTextContent('Не удалось загрузить агентов')
    expect(screen.queryByText('Агентов пока нет')).not.toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Повторить' }))
    expect(await screen.findByText('Developer Hermes')).toBeVisible()
  })

  it('localizes the create form and keeps its primary selectors touch-sized', async () => {
    renderPage(true)

    expect(await screen.findByRole('heading', { name: 'Создание агента' })).toBeVisible()
    expect(await screen.findByText('Параметры нового агента')).toBeVisible()
    expect(screen.getByRole('combobox', { name: 'Тип агента' })).toHaveClass('h-10')
    expect(screen.getByRole('combobox', { name: 'Специализация' })).toHaveClass('h-10')
    expect(screen.getByRole('button', { name: 'Создать агента' })).toBeEnabled()
    expect(screen.queryByText('Provision wizard')).not.toBeInTheDocument()
  })

  it('locks the create draft while saving and preserves it after an error', async () => {
    let rejectCreate: (reason?: unknown) => void = () => undefined
    vi.mocked(fleet.createAgent).mockImplementation(
      () =>
        new Promise<Agent>((_resolve, reject) => {
          rejectCreate = reject
        }),
    )
    renderPage(true)

    const displayName = await screen.findByLabelText('Отображаемое имя')
    fireEvent.change(displayName, { target: { value: 'QA Agent' } })
    fireEvent.click(screen.getByRole('button', { name: 'Создать агента' }))

    const pending = await screen.findByRole('button', { name: 'Создаём...' })
    expect(pending).toBeDisabled()
    expect(displayName).toBeDisabled()
    expect(screen.getByRole('combobox', { name: 'Тип агента' })).toBeDisabled()
    expect(pending.closest('form')).toHaveAttribute('aria-busy', 'true')

    await act(async () => rejectCreate(new Error('offline')))
    expect(await screen.findByRole('alert')).toHaveTextContent('Данные формы сохранены')
    expect(displayName).toBeEnabled()
    expect(displayName).toHaveValue('QA Agent')
    expect(screen.getByRole('button', { name: 'Создать агента' })).toBeEnabled()
  })
})
