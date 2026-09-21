import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter, Route, Routes } from 'react-router'
import { ThemeProvider } from '@sdlc/ui/lib'
import { endSso } from '@sdlc/ui/sso'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { AppShell } from './app-shell'
import { getCurrentUserPermissions } from '@/api/auth'
import { useAuthStore } from '@/shared/auth/store'

vi.mock('@/api/auth', () => ({ getCurrentUserPermissions: vi.fn() }))
vi.mock('@sdlc/ui/sso', () => ({ endSso: vi.fn() }))

const permissions = [
  'sessions:read_all',
  'sessions:write_own',
  'agents:read_directory',
  'agents:manage',
  'leaders:manage',
  'executors:manage',
]

function renderShell(path = '/sessions/session-1') {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={client}>
      <ThemeProvider>
        <MemoryRouter initialEntries={[path]}>
          <Routes>
            <Route element={<AppShell />}>
              <Route path="/" element={<h1>Dashboard content</h1>} />
              <Route path="/sessions/:sessionId" element={<h1>Session content</h1>} />
            </Route>
          </Routes>
        </MemoryRouter>
      </ThemeProvider>
    </QueryClientProvider>,
  )
}

describe('AppShell', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    useAuthStore.setState({
      token: 'test-token',
      userId: 'user-1',
      email: 'operator@example.test',
      username: 'operator',
      displayName: 'Fleet Operator',
      systemRole: 'operator',
      isSystemAdmin: false,
      permissions,
    })
    vi.mocked(getCurrentUserPermissions).mockResolvedValue({
      user_id: 'user-1',
      role: 'operator',
      is_system_admin: false,
      permissions,
    })
  })

  it('keeps direct-route navigation active and permission aware', async () => {
    renderShell()

    expect(await screen.findByRole('heading', { name: 'Session content' })).toBeVisible()
    const sessions = screen.getByRole('link', { name: 'Сессии' })
    expect(sessions).toHaveClass('bg-surface-raised')
    expect(screen.getByRole('link', { name: 'Агенты' })).toBeVisible()
    expect(screen.queryByRole('link', { name: 'Настройки' })).not.toBeInTheDocument()
    expect(screen.queryByRole('contentinfo')).not.toBeInTheDocument()
  })

  it('closes the mobile drawer with Escape and returns focus to its trigger', async () => {
    renderShell()
    const trigger = await screen.findByRole('button', { name: 'Открыть навигацию' })

    fireEvent.click(trigger)
    const dialog = await screen.findByRole('dialog')
    expect(within(dialog).getByRole('link', { name: 'Сессии' })).toHaveClass('bg-surface-raised')

    fireEvent.keyDown(document, { key: 'Escape' })
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument())
    expect(trigger).toHaveFocus()
  })

  it('clears local auth and starts central sign-out', async () => {
    renderShell()

    fireEvent.click(await screen.findByRole('button', { name: 'Выйти' }))

    expect(useAuthStore.getState().token).toBeNull()
    expect(endSso).toHaveBeenCalledWith(expect.objectContaining({ clientId: 'fleet-control' }))
  })
})
