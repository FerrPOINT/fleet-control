import { beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter } from 'react-router'
import { ExecutorsPage } from './index'
import * as fleet from '@/api/fleet'
import * as auth from '@/api/auth'
import type { Agent, AgentSession } from '@/api/types'

vi.mock('@/api/fleet', () => ({
  listExecutors: vi.fn(),
  listSessions: vi.fn(),
}))
vi.mock('@/api/auth', () => ({ listUsers: vi.fn() }))

const executors = [
  {
    id: 'agent-1',
    name: 'alpha',
    display_name: 'Alpha Executor',
    role: 'developer',
    product_role: 'executor',
    kind: 'hermes',
    status: 'running',
  },
  {
    id: 'agent-2',
    name: 'beta',
    display_name: 'Beta Executor',
    role: 'tester',
    product_role: 'executor',
    kind: 'hermes',
    status: 'stopped',
  },
] as Agent[]

const sessions = [
  {
    id: 'session-1',
    primary_agent_id: 'agent-1',
    title: 'Release check',
    user_display_name: 'QA User',
    user_id: 'user-1',
    visibility: 'private',
  },
] as AgentSession[]

function renderPage() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter>
        <ExecutorsPage />
      </MemoryRouter>
    </QueryClientProvider>,
  )
}

describe('ExecutorsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(fleet.listExecutors).mockResolvedValue(executors)
    vi.mocked(fleet.listSessions).mockResolvedValue(sessions)
    vi.mocked(auth.listUsers).mockResolvedValue([])
  })

  it('searches executors and reveals their sessions on demand', async () => {
    renderPage()
    await screen.findByText('Alpha Executor')
    expect(screen.getByText('Beta Executor')).toBeInTheDocument()
    expect(screen.queryByText('Release check')).not.toBeVisible()

    fireEvent.click(screen.getByText('1 сессия'))
    expect(screen.getByText('Release check')).toBeVisible()
    expect(fleet.listExecutors).toHaveBeenCalledTimes(1)

    fireEvent.change(screen.getByLabelText('Найти исполнителя'), {
      target: { value: 'beta' },
    })
    expect(screen.queryByText('Alpha Executor')).not.toBeInTheDocument()
    expect(screen.getByText('Beta Executor')).toBeInTheDocument()
    expect(screen.getByText('Показано 1 из 1')).toBeInTheDocument()
  })

  it('keeps session errors distinct from empty sessions and can retry', async () => {
    vi.mocked(fleet.listSessions)
      .mockRejectedValueOnce(new Error('offline'))
      .mockResolvedValueOnce(sessions)
    renderPage()
    await screen.findByText('Alpha Executor')
    const error = await screen.findByRole('alert')
    expect(error).toHaveTextContent('Не удалось загрузить сессии исполнителей.')
    expect(screen.queryByText('Сессий для выбранных пользователей нет')).not.toBeInTheDocument()

    fireEvent.click(within(error.parentElement!).getByRole('button', { name: 'Повторить' }))
    await waitFor(() => expect(screen.getByText('1 сессия')).toBeInTheDocument())
  })

  it('retries executor loading without showing a false empty state', async () => {
    vi.mocked(fleet.listExecutors)
      .mockRejectedValueOnce(new Error('offline'))
      .mockResolvedValueOnce(executors)
    renderPage()
    const error = await screen.findByRole('alert')
    expect(error).toHaveTextContent('Не удалось загрузить исполнителей.')
    expect(screen.queryByText('Исполнителей пока нет')).not.toBeInTheDocument()

    fireEvent.click(within(error.parentElement!).getByRole('button', { name: 'Повторить' }))
    await screen.findByText('Alpha Executor')
  })

  it('reports a user directory failure and recovers without clearing the executor list', async () => {
    vi.mocked(auth.listUsers).mockRejectedValueOnce(new Error('offline')).mockResolvedValueOnce([])
    renderPage()
    await screen.findByText('Alpha Executor')
    const error = await screen.findByRole('alert')
    expect(error).toHaveTextContent('Не удалось загрузить пользователей')
    fireEvent.click(within(error).getByRole('button', { name: 'Повторить' }))
    await waitFor(() => expect(screen.queryByRole('alert')).not.toBeInTheDocument())
    expect(screen.getByText('Beta Executor')).toBeInTheDocument()
  })

  it('loads more executors without querying the backend again', async () => {
    vi.mocked(fleet.listExecutors).mockResolvedValue(
      Array.from({ length: 26 }, (_, index) => ({
        ...executors[0],
        id: `agent-${index}`,
        name: `agent-${index}`,
        display_name: `Executor ${String(index).padStart(2, '0')}`,
      })) as Agent[],
    )
    renderPage()
    await screen.findByText('Показано 25 из 26')
    expect(screen.getAllByRole('link', { name: 'Открыть' })).toHaveLength(25)
    fireEvent.click(screen.getByRole('button', { name: 'Показать ещё' }))
    expect(screen.getByText('Показано 26 из 26')).toBeInTheDocument()
    expect(screen.getAllByRole('link', { name: 'Открыть' })).toHaveLength(26)
    expect(fleet.listExecutors).toHaveBeenCalledTimes(1)
  })
})
