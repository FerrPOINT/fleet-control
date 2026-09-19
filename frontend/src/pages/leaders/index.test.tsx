import { beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter, Route, Routes } from 'react-router'
import { toast } from 'sonner'
import { LeaderDetailPage } from './index'
import * as fleet from '@/api/fleet'
import * as auth from '@/api/auth'
import type { Agent, LeaderExecutor } from '@/api/types'

vi.mock('@/api/fleet', () => ({
  listAgents: vi.fn(),
  listExecutors: vi.fn(),
  listLeaderExecutors: vi.fn(),
  listSessions: vi.fn(),
  updateLeaderExecutors: vi.fn(),
}))
vi.mock('@/api/auth', () => ({ listUsers: vi.fn() }))
vi.mock('sonner', () => ({ toast: { success: vi.fn() } }))

const leader = {
  id: 'leader-1',
  name: 'lead',
  display_name: 'Lead Hermes',
  role: 'it_lead',
  product_role: 'leader',
  kind: 'hermes',
  status: 'ready',
} as Agent

const executors = [
  { id: 'executor-1', name: 'alpha', display_name: 'Alpha Executor', role: 'developer' },
  { id: 'executor-2', name: 'beta', display_name: 'Beta Executor', role: 'tester' },
] as Agent[]

const originalTeam = [
  { leader_agent_id: leader.id, executor_agent_id: 'executor-1' },
] as LeaderExecutor[]
const expandedTeam = [
  ...originalTeam,
  { leader_agent_id: leader.id, executor_agent_id: 'executor-2' },
] as LeaderExecutor[]

function renderPage() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={['/leaders/leader-1']}>
        <Routes>
          <Route path="/leaders/:leaderId" element={<LeaderDetailPage />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  )
}

describe('LeaderDetailPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(fleet.listAgents).mockResolvedValue([leader, ...executors])
    vi.mocked(fleet.listExecutors).mockResolvedValue(executors)
    vi.mocked(fleet.listLeaderExecutors).mockResolvedValue(originalTeam)
    vi.mocked(fleet.listSessions).mockResolvedValue([])
    vi.mocked(auth.listUsers).mockResolvedValue([])
    vi.mocked(fleet.updateLeaderExecutors).mockResolvedValue(expandedTeam)
  })

  it('blocks saving while the team failed to load and retries safely', async () => {
    vi.mocked(fleet.listLeaderExecutors)
      .mockRejectedValueOnce(new Error('offline'))
      .mockResolvedValueOnce(originalTeam)
    renderPage()
    const error = await screen.findByRole('alert')
    expect(error).toHaveTextContent('Не удалось загрузить состав команды')
    expect(screen.getByRole('button', { name: 'Сохранить команду' })).toBeDisabled()
    expect(screen.queryByRole('checkbox', { name: /Alpha Executor/ })).not.toBeInTheDocument()

    fireEvent.click(within(error.parentElement!).getByRole('button', { name: 'Повторить' }))
    const checkbox = await screen.findByRole('checkbox', { name: /Alpha Executor/ })
    expect(checkbox).toBeChecked()
    expect(screen.getByRole('button', { name: 'Сохранить команду' })).toBeDisabled()
    expect(fleet.updateLeaderExecutors).not.toHaveBeenCalled()
  })

  it('blocks saving when the available executors failed to load', async () => {
    vi.mocked(fleet.listExecutors)
      .mockRejectedValueOnce(new Error('offline'))
      .mockResolvedValueOnce(executors)
    renderPage()
    const error = await screen.findByRole('alert')
    expect(error).toHaveTextContent('Не удалось загрузить исполнителей')
    expect(screen.getByRole('button', { name: 'Сохранить команду' })).toBeDisabled()
    fireEvent.click(within(error.parentElement!).getByRole('button', { name: 'Повторить' }))
    await screen.findByRole('checkbox', { name: /Alpha Executor/ })
    expect(screen.getByRole('button', { name: 'Сохранить команду' })).toBeDisabled()
  })

  it('keeps the draft after a failed save and confirms the successful retry', async () => {
    vi.mocked(fleet.updateLeaderExecutors)
      .mockRejectedValueOnce(new Error('write failed'))
      .mockResolvedValueOnce(expandedTeam)
    vi.mocked(fleet.listLeaderExecutors)
      .mockResolvedValueOnce(originalTeam)
      .mockResolvedValueOnce(expandedTeam)
    renderPage()
    const beta = await screen.findByRole('checkbox', { name: /Beta Executor/ })
    fireEvent.click(beta)
    const save = screen.getByRole('button', { name: 'Сохранить команду' })
    fireEvent.click(save)
    await screen.findByText('write failed')
    expect(beta).toBeChecked()
    expect(save).toBeEnabled()
    expect(fleet.updateLeaderExecutors).toHaveBeenCalledWith('leader-1', {
      executor_ids: ['executor-1', 'executor-2'],
    })

    fireEvent.click(save)
    await waitFor(() => expect(toast.success).toHaveBeenCalledWith('Состав команды сохранён'))
    expect(screen.getByRole('button', { name: 'Сохранить команду' })).toBeDisabled()
  })

  it('locks the team while a save is pending', async () => {
    let finishSave: (value: LeaderExecutor[]) => void = () => undefined
    vi.mocked(fleet.updateLeaderExecutors).mockImplementation(
      () =>
        new Promise<LeaderExecutor[]>((resolve) => {
          finishSave = resolve
        }),
    )
    vi.mocked(fleet.listLeaderExecutors)
      .mockResolvedValueOnce(originalTeam)
      .mockResolvedValueOnce(expandedTeam)
    renderPage()
    const beta = await screen.findByRole('checkbox', { name: /Beta Executor/ })
    fireEvent.click(beta)
    fireEvent.click(screen.getByRole('button', { name: 'Сохранить команду' }))
    expect(await screen.findByRole('button', { name: 'Сохраняем...' })).toBeDisabled()
    expect(beta).toBeDisabled()
    finishSave(expandedTeam)
    await waitFor(() => expect(toast.success).toHaveBeenCalledWith('Состав команды сохранён'))
  })

  it('distinguishes a session request failure from an empty session list', async () => {
    vi.mocked(fleet.listSessions)
      .mockRejectedValueOnce(new Error('offline'))
      .mockResolvedValueOnce([])
    renderPage()
    const error = await screen.findByRole('alert')
    expect(error).toHaveTextContent('Не удалось загрузить сессии лидера')
    expect(screen.queryByText('Сессий этого лидера пока нет')).not.toBeInTheDocument()
    fireEvent.click(within(error.parentElement!).getByRole('button', { name: 'Повторить' }))
    await screen.findByText('Сессий этого лидера пока нет')
  })
})
