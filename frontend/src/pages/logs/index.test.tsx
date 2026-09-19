import { beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter } from 'react-router'
import { LogsPage } from './index'
import * as fleet from '@/api/fleet'
import type { AgentLogEntry, AuditLogEntry } from '@/api/types'

vi.mock('@/api/fleet', () => ({
  listAgents: vi.fn(),
  listAuditLog: vi.fn(),
  listEvents: vi.fn(),
  listLogs: vi.fn(),
}))

const auditRows = Array.from({ length: 30 }, (_, index) => ({
  id: `audit-${index}`,
  actor_user_id: null,
  action: `session.create.${index}`,
  entity_type: 'session',
  entity_id: `session-${index}`,
  payload: { index },
  created_at: '2026-09-19T12:00:00Z',
})) as AuditLogEntry[]

function renderPage(path = '/logs') {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[path]}>
        <LogsPage />
      </MemoryRouter>
    </QueryClientProvider>,
  )
}

describe('LogsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(fleet.listAgents).mockResolvedValue([])
    vi.mocked(fleet.listLogs).mockResolvedValue([])
    vi.mocked(fleet.listEvents).mockResolvedValue([])
    vi.mocked(fleet.listAuditLog).mockResolvedValue(auditRows)
  })

  it('keeps the selected section in the URL and loads only that section', async () => {
    renderPage('/logs?tab=audit')
    expect(screen.getByRole('button', { name: 'Аудит' })).toHaveAttribute('aria-pressed', 'true')
    await waitFor(() => expect(fleet.listAuditLog).toHaveBeenCalledOnce())
    expect(fleet.listLogs).not.toHaveBeenCalled()

    fireEvent.click(screen.getByRole('button', { name: 'События' }))
    await waitFor(() => expect(fleet.listEvents).toHaveBeenCalledOnce())
    expect(screen.getByRole('button', { name: 'События' })).toHaveAttribute('aria-pressed', 'true')
  })

  it('applies audit filters explicitly, shows 25 rows, and expands more', async () => {
    renderPage('/logs?tab=audit')
    await screen.findByText('Показано 25 из 30')
    expect(screen.getAllByText(/session\.create\./)).toHaveLength(25)
    expect(screen.getAllByText(/19\.09\.2026/).length).toBeGreaterThan(0)

    fireEvent.change(screen.getByLabelText('Действие'), { target: { value: 'session.update' } })
    fireEvent.change(screen.getByLabelText('Тип объекта'), { target: { value: 'agent' } })
    expect(fleet.listAuditLog).toHaveBeenCalledOnce()
    fireEvent.click(screen.getByRole('button', { name: 'Применить' }))
    await waitFor(() =>
      expect(fleet.listAuditLog).toHaveBeenCalledWith({
        action: 'session.update',
        entity_type: 'agent',
        limit: 150,
      }),
    )

    fireEvent.click(await screen.findByRole('button', { name: 'Показать ещё' }))
    expect(screen.getByText('Показано 30 из 30')).toBeInTheDocument()
    expect(screen.getAllByText(/session\.create\./)).toHaveLength(30)
    fireEvent.click(screen.getByRole('button', { name: 'Сбросить' }))
    await waitFor(() =>
      expect(fleet.listAuditLog).toHaveBeenLastCalledWith({
        action: undefined,
        entity_type: undefined,
        limit: 150,
      }),
    )
  })

  it('retries process log errors without losing the selected agent', async () => {
    vi.mocked(fleet.listAgents).mockResolvedValue([
      { id: 'agent-1', name: 'agent1', display_name: 'First Agent' },
    ] as never)
    vi.mocked(fleet.listLogs)
      .mockRejectedValueOnce(new Error('unavailable'))
      .mockResolvedValue([
        {
          id: 'log-1',
          agent_id: 'agent-1',
          stream: 'stdout',
          message: 'process ready',
          created_at: '2026-09-19T12:00:00Z',
        },
      ] as AgentLogEntry[])
    renderPage()
    await screen.findByText('unavailable')
    fireEvent.click(screen.getByRole('button', { name: 'Повторить' }))
    await screen.findAllByText('process ready')
    fireEvent.change(screen.getByLabelText('Агент'), { target: { value: 'agent-1' } })
    await waitFor(() => expect(fleet.listLogs).toHaveBeenLastCalledWith('agent-1', 150))
  })

  it('distinguishes a filtered empty audit from a truly empty log', async () => {
    vi.mocked(fleet.listAuditLog).mockResolvedValue([])
    renderPage('/logs?tab=audit')
    await screen.findByText('Записей аудита пока нет')
    fireEvent.change(screen.getByLabelText('Действие'), { target: { value: 'missing' } })
    fireEvent.click(screen.getByRole('button', { name: 'Применить' }))
    await screen.findByText('По этим фильтрам записей нет')
  })
})
