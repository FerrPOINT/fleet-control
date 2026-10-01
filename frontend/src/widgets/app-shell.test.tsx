import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter, Route, Routes } from 'react-router'
import { ThemeProvider } from '@sdlc/ui/lib'
import { endSso } from '@sdlc/ui/sso'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
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
const desktop = {
  matches: false,
  media: '(min-width: 768px)',
  addEventListener: vi.fn(),
  removeEventListener: vi.fn(),
}

function renderShell(path = '/sessions/session-1') {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={client}>
      <ThemeProvider>
        <MemoryRouter initialEntries={[path]}>
          <Routes>
            <Route element={<AppShell />}>
              <Route path="/" element={<h1>Dashboard content</h1>} />
              <Route path="/agents/new" element={<h1>Create agent content</h1>} />
              <Route path="/agents/:agentId/edit" element={<h1>Edit agent content</h1>} />
              <Route path="/settings" element={<h1>Settings content</h1>} />
              <Route path="/agents/:agentId/runtime" element={<h1>Runtime content</h1>} />
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
    desktop.matches = false
    vi.stubGlobal(
      'matchMedia',
      vi.fn((query: string) =>
        query === desktop.media
          ? desktop
          : { ...desktop, media: query, addEventListener: vi.fn(), removeEventListener: vi.fn() },
      ),
    )
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
  afterEach(() => vi.unstubAllGlobals())

  it('owns a full-width shared header before the nav-only sidebar and content offset', async () => {
    const { container } = renderShell()
    await screen.findByRole('heading', { name: 'Session content' })
    const header = container.querySelector('[data-platform-header]')!
    expect(header).toBe(container.firstElementChild?.firstElementChild)
    expect(
      [...header.querySelectorAll('[data-platform-header-slot]')].map((slot) =>
        slot.getAttribute('data-platform-header-slot'),
      ),
    ).toEqual(['leading', 'services', 'actions'])
    const sidebar = container.querySelector('aside')!
    expect(sidebar).toHaveClass('top-[var(--shell-header-height)]')
    expect(sidebar).not.toHaveTextContent('Fleet Control')
    expect(sidebar).not.toHaveTextContent('Fleet Operator')
    expect(screen.getByRole('link', { name: 'Fleet Control' })).toHaveAttribute('href', '/')
    expect(
      screen.getByRole('button', { name: 'Открыть список сервисов: Fleet Control' }),
    ).toBeInTheDocument()
    expect(screen.queryByText('Fleet Operator')).not.toBeInTheDocument()
  })

  it.each([
    ['/agents/new', 'Create agent content', 'reading'],
    ['/agents/agent-1/edit', 'Edit agent content', 'reading'],
    ['/settings', 'Settings content', 'reading'],
    ['/', 'Dashboard content', 'wide'],
  ])('preserves the page mode for %s', async (path, title, mode) => {
    renderShell(path)
    expect((await screen.findByRole('heading', { name: title })).parentElement).toHaveAttribute(
      'data-page-layout',
      mode,
    )
  })

  it.each([
    ['Fleet Operator', 'operator@example.test', 'Fleet Operator', true],
    [null, 'operator@example.test', 'operator@example.test', false],
    ['   ', 'operator@example.test', 'operator@example.test', false],
    ['operator@example.test', 'operator@example.test', 'operator@example.test', false],
    [null, null, 'Оператор Fleet Control', false],
    ['A'.repeat(180), 'operator@example.test', 'A'.repeat(180), true],
  ])(
    'shows identity once in a bounded account menu (%s)',
    async (name, email, identity, distinct) => {
      useAuthStore.setState({ displayName: name, email })
      renderShell()
      fireEvent.keyDown(await screen.findByRole('button', { name: 'Аккаунт' }), {
        key: 'ArrowDown',
      })
      const menu = await screen.findByRole('menu')
      expect(within(menu).getAllByText(identity!, { exact: true })).toHaveLength(1)
      expect(menu).toHaveClass('max-w-[calc(100vw-2rem)]')
      expect(menu.querySelectorAll('.break-words')).toHaveLength(distinct ? 2 : 1)
      expect(screen.getByRole('menuitem', { name: 'Выйти' })).toHaveClass('min-h-11', 'md:min-h-10')
    },
  )

  it('keeps direct-route navigation active and permission aware', async () => {
    renderShell()

    expect(await screen.findByRole('heading', { name: 'Session content' })).toBeVisible()
    expect(screen.getByRole('heading', { name: 'Session content' }).parentElement).toHaveAttribute(
      'data-page-layout',
      'detail-with-aside',
    )
    const sessions = screen.getByRole('link', { name: 'Сессии' })
    expect(sessions).toHaveClass('bg-surface-raised')
    expect(screen.getByRole('link', { name: 'Агенты' })).toBeVisible()
    expect(screen.queryByRole('link', { name: 'Настройки' })).not.toBeInTheDocument()
    expect(screen.queryByRole('contentinfo')).not.toBeInTheDocument()
  })

  it('keeps nested entity tabs in the shared detail mode', async () => {
    renderShell('/agents/agent-1/runtime')

    expect(await screen.findByRole('heading', { name: 'Runtime content' })).toBeVisible()
    expect(screen.getByRole('heading', { name: 'Runtime content' }).parentElement).toHaveAttribute(
      'data-page-layout',
      'detail-with-aside',
    )
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

  it('starts central sign-out before any local login reroute', async () => {
    renderShell()
    fireEvent.keyDown(await screen.findByRole('button', { name: 'Аккаунт' }), { key: 'ArrowDown' })
    fireEvent.click(await screen.findByRole('menuitem', { name: 'Выйти' }))

    expect(useAuthStore.getState().token).toBe('test-token')
    expect(endSso).toHaveBeenCalledWith(expect.objectContaining({ clientId: 'fleet-control' }))
  })

  it('closes the drawer after navigation without a duplicate profile', async () => {
    renderShell()
    fireEvent.click(await screen.findByRole('button', { name: 'Открыть навигацию' }))
    const dialog = await screen.findByRole('dialog')
    expect(dialog).not.toHaveTextContent('Fleet Operator')
    const dashboard = within(dialog).getByRole('link', { name: 'Обзор' })
    expect(dashboard).toHaveClass('min-h-11')
    fireEvent.click(dashboard)
    await screen.findByRole('heading', { name: 'Dashboard content' })
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument())
  })

  it('closes at the desktop breakpoint and cleans up its media listener', async () => {
    const { unmount } = renderShell()
    fireEvent.click(await screen.findByRole('button', { name: 'Открыть навигацию' }))
    await screen.findByRole('dialog')
    const listener = desktop.addEventListener.mock.calls[0]?.[1] as (() => void) | undefined
    expect(listener).toBeTypeOf('function')
    act(() => {
      desktop.matches = true
      listener?.()
    })
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument())
    unmount()
    expect(desktop.removeEventListener).toHaveBeenCalledWith('change', listener)
  })
})
