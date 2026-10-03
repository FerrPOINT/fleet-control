import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter, useLocation, useNavigate } from 'react-router'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ChatsPage } from './index'
import * as fleet from '@/api/fleet'
import * as directory from '@/api/chats-directory'
import * as auth from '@/api/auth'
import type { AgentDirectoryItem, AgentSession, UserResponse } from '@/api/types'
import { useAuthStore } from '@/shared/auth/store'

vi.mock('@/api/fleet', () => ({ createSession: vi.fn() }))
vi.mock('@/api/chats-directory', () => ({ getChatsDirectory: vi.fn() }))
vi.mock('@/api/auth', () => ({ listUsers: vi.fn() }))
const owner = '00000000-0000-4000-8000-000000000001'
const other = '00000000-0000-4000-8000-000000000002'
const developer = {
  id: '00000000-0000-4000-8000-000000000010',
  name: 'agent1',
  display_name: 'Developer',
  kind: 'hermes',
  product_role: 'executor',
  role: 'developer',
  sdlc_role: 'developer',
  status: 'running',
} as AgentDirectoryItem
const tester = {
  ...developer,
  id: '00000000-0000-4000-8000-000000000011',
  name: 'agent2',
  display_name: 'Tester',
  sdlc_role: 'tester',
} as AgentDirectoryItem
const session = {
  id: '00000000-0000-4000-8000-000000000020',
  primary_agent_id: developer.id,
  user_id: owner,
  user_display_name: 'Owner',
  title: 'Implement login',
  task_key: 'TASK-1',
  state: 'active',
  visibility: 'private',
  created_at: '2026-10-01T12:00:00Z',
  updated_at: '2026-10-01T12:00:00Z',
  last_message_preview: 'Login requirements',
} as AgentSession
const second = { ...session, id: '00000000-0000-4000-8000-000000000021', title: 'Second page' }
const testerSession = {
  ...session,
  id: '00000000-0000-4000-8000-000000000022',
  primary_agent_id: tester.id,
  title: 'Test login',
}
function page(
  items = [session],
  selected = developer.id,
  next: string | null = null,
): directory.ChatsDirectoryPage {
  return {
    agents: [
      { agent: developer, matching_session_count: 7 },
      { agent: tester, matching_session_count: 2 },
    ],
    selected_agent_id: selected,
    items,
    next_before: next,
  }
}
function Location() {
  const location = useLocation()
  const navigate = useNavigate()
  return (
    <>
      <output data-testid="location">
        {location.pathname}
        {location.search}
      </output>
      <button onClick={() => navigate(-1)}>Browser back</button>
    </>
  )
}
function renderPage(url = '/chats') {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })
  render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[url]}>
        <ChatsPage />
        <Location />
      </MemoryRouter>
    </QueryClientProvider>,
  )
  return client
}
beforeEach(() => {
  vi.clearAllMocks()
  useAuthStore.setState({
    userId: owner,
    displayName: 'Owner',
    email: 'owner@example.test',
    systemRole: 'user',
    isSystemAdmin: false,
    permissions: ['sessions:write_own'],
  })
  vi.mocked(directory.getChatsDirectory).mockImplementation(async (query = {}) =>
    query.agentId === tester.id
      ? page([testerSession], tester.id)
      : query.before
        ? page([second])
        : page(),
  )
  vi.mocked(auth.listUsers).mockResolvedValue([
    { id: owner, display_name: 'Owner' },
    { id: other, display_name: 'Other' },
  ] as UserResponse[])
  vi.mocked(fleet.createSession).mockResolvedValue(session)
})
describe('server-scoped ChatsPage', () => {
  it('defaults to mine and uses server counts, not current page lengths', async () => {
    renderPage()
    const link = await screen.findByRole('link', { name: /Implement login/ })
    expect(directory.getChatsDirectory).toHaveBeenCalledWith({
      agentId: undefined,
      userIds: [owner],
      search: '',
      before: undefined,
      limit: 50,
    })
    expect(auth.listUsers).not.toHaveBeenCalled()
    expect(screen.getAllByLabelText('Количество сессий').map((item) => item.textContent)).toEqual([
      '7',
      '2',
    ])
    const returnTo = new URL(link.getAttribute('href')!, 'http://local').searchParams.get(
      'returnTo',
    )!
    expect(new URL(returnTo, 'http://local').searchParams.get('agent')).toBe(developer.id)
    expect(new URL(returnTo, 'http://local').searchParams.get('users')).toBe(owner)
  })
  it('keeps concrete sessions separate despite the same display task key', async () => {
    renderPage()
    await screen.findByRole('link', { name: /Implement login/ })
    fireEvent.click(await screen.findByRole('button', { name: /Tester/ }))
    expect(await screen.findByRole('link', { name: /Test login/ })).toBeVisible()
    expect(screen.queryByRole('link', { name: /Implement login/ })).not.toBeInTheDocument()
    expect(directory.getChatsDirectory).toHaveBeenLastCalledWith(
      expect.objectContaining({ agentId: tester.id }),
    )
  })
  it('searches server-side and displays exactly returned matches/counts', async () => {
    renderPage()
    await screen.findByRole('link', { name: /Implement login/ })
    vi.mocked(directory.getChatsDirectory).mockResolvedValue({
      ...page([second]),
      agents: [{ agent: developer, matching_session_count: 1 }],
    })
    fireEvent.change(screen.getByRole('textbox', { name: 'Поиск по задачам' }), {
      target: { value: 'Owner' },
    })
    expect(await screen.findByRole('link', { name: /Second page/ })).toBeVisible()
    expect(directory.getChatsDirectory).toHaveBeenLastCalledWith(
      expect.objectContaining({ search: 'Owner', before: undefined }),
    )
    expect(screen.getByLabelText('Количество сессий')).toHaveTextContent('1')
  })
  it('paginates with session cursors and preserves full filtered returnTo', async () => {
    vi.mocked(directory.getChatsDirectory).mockImplementation(async (query = {}) =>
      query.before ? page([second]) : page([session], developer.id, session.id),
    )
    renderPage(`/chats?agent=${developer.id}&q=login&users=${owner}`)
    fireEvent.click(await screen.findByRole('button', { name: 'Следующая страница' }))
    const link = await screen.findByRole('link', { name: /Second page/ })
    expect(directory.getChatsDirectory).toHaveBeenLastCalledWith(
      expect.objectContaining({
        agentId: developer.id,
        before: session.id,
        search: 'login',
        userIds: [owner],
      }),
    )
    expect(screen.getByRole('button', { name: 'Следующая страница' })).toBeDisabled()
    const returnTo = new URL(link.getAttribute('href')!, 'http://local').searchParams.get(
      'returnTo',
    )!
    expect(Object.fromEntries(new URL(returnTo, 'http://local').searchParams)).toEqual({
      agent: developer.id,
      q: 'login',
      users: owner,
      before: session.id,
    })
    fireEvent.click(screen.getByRole('button', { name: 'Первая страница' }))
    expect(await screen.findByRole('link', { name: /Implement login/ })).toBeVisible()
    expect(screen.getByTestId('location')).not.toHaveTextContent('before=')
  })
  it('resets cursor on search and agent changes', async () => {
    renderPage(`/chats?agent=${developer.id}&before=${session.id}`)
    await screen.findByRole('link', { name: /Second page/ })
    fireEvent.change(screen.getByRole('textbox', { name: 'Поиск по задачам' }), {
      target: { value: 'login' },
    })
    await waitFor(() =>
      expect(directory.getChatsDirectory).toHaveBeenLastCalledWith(
        expect.objectContaining({ before: undefined, search: 'login' }),
      ),
    )
    fireEvent.click(await screen.findByRole('button', { name: /Tester/ }))
    await screen.findByRole('link', { name: /Test login/ })
    expect(screen.getByTestId('location')).not.toHaveTextContent('before=')
  })
  it('permits multiuser/all only with read_all and follows URL scope on browser back', async () => {
    useAuthStore.setState({ systemRole: 'operator', permissions: ['sessions:read_all'] })
    renderPage(`/chats?users=${owner}&agent=${developer.id}&before=${session.id}`)
    await screen.findByRole('link', { name: /Second page/ })
    await screen.findByRole('option', { name: 'Other' })
    fireEvent.change(screen.getByRole('combobox', { name: 'Добавить пользователя' }), {
      target: { value: other },
    })
    await waitFor(() =>
      expect(directory.getChatsDirectory).toHaveBeenLastCalledWith(
        expect.objectContaining({ userIds: [owner, other], before: undefined }),
      ),
    )
    fireEvent.click(screen.getByRole('button', { name: 'Убрать фильтр по Owner' }))
    fireEvent.click(screen.getByRole('button', { name: 'Убрать фильтр по Other' }))
    await waitFor(() =>
      expect(directory.getChatsDirectory).toHaveBeenLastCalledWith(
        expect.objectContaining({ userIds: [] }),
      ),
    )
    fireEvent.click(await screen.findByRole('button', { name: /Tester/ }))
    await screen.findByRole('link', { name: /Test login/ })
    fireEvent.click(screen.getByRole('button', { name: 'Browser back' }))
    await waitFor(() =>
      expect(directory.getChatsDirectory).toHaveBeenLastCalledWith(
        expect.objectContaining({ agentId: developer.id, userIds: [] }),
      ),
    )
  })
  it('does not trust foreign URL scopes for read-own users', async () => {
    renderPage(`/chats?users=${other}`)
    await screen.findByRole('link', { name: /Implement login/ })
    expect(directory.getChatsDirectory).toHaveBeenLastCalledWith(
      expect.objectContaining({ userIds: [owner] }),
    )
    expect(screen.getByRole('button', { name: 'Убрать фильтр по Owner' })).toBeDisabled()
    expect(auth.listUsers).not.toHaveBeenCalled()
  })
  it('renders loading without false zero counts or empty results', () => {
    vi.mocked(directory.getChatsDirectory).mockReturnValue(new Promise(() => {}))
    renderPage()
    expect(screen.getByText('Загрузка чатов...')).toBeVisible()
    expect(screen.queryByLabelText('Количество сессий')).not.toBeInTheDocument()
    expect(screen.queryByText('Сессий по выбранному фильтру нет')).not.toBeInTheDocument()
  })
  it('renders true empty pages with server zero counts', async () => {
    vi.mocked(directory.getChatsDirectory).mockResolvedValue({
      ...page([]),
      agents: [{ agent: developer, matching_session_count: 0 }],
    })
    renderPage()
    expect(await screen.findByText('Сессий по выбранному фильтру нет')).toBeVisible()
    expect(screen.getByLabelText('Количество сессий')).toHaveTextContent('0')
  })
  it('does not show cached stale counts after a failed refetch', async () => {
    const client = renderPage()
    await screen.findByRole('link', { name: /Implement login/ })
    vi.mocked(directory.getChatsDirectory).mockRejectedValue(new Error('offline'))
    await client.invalidateQueries({ queryKey: ['chats-directory'] })
    expect(await screen.findByRole('alert')).toBeVisible()
    expect(screen.queryByRole('link', { name: /Implement login/ })).not.toBeInTheDocument()
    expect(screen.queryByLabelText('Количество сессий')).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Новый чат' })).not.toBeInTheDocument()
  })
  it('recovers an invalid cursor without losing other filters', async () => {
    vi.mocked(directory.getChatsDirectory).mockRejectedValueOnce(new Error('invalid cursor'))
    renderPage(`/chats?agent=${developer.id}&before=${session.id}&q=login&users=${owner}`)
    await screen.findByRole('alert')
    fireEvent.click(screen.getByRole('button', { name: 'Первая страница' }))
    await screen.findByRole('link', { name: /Implement login/ })
    expect(directory.getChatsDirectory).toHaveBeenLastCalledWith(
      expect.objectContaining({
        agentId: developer.id,
        userIds: [owner],
        search: 'login',
        before: undefined,
      }),
    )
  })
  it('recovers an unavailable agent without losing the search or user scope', async () => {
    vi.mocked(directory.getChatsDirectory).mockRejectedValueOnce(new Error('archived agent'))
    renderPage(`/chats?agent=${tester.id}&before=${session.id}&q=login&users=${owner}`)
    await screen.findByRole('alert')
    fireEvent.click(screen.getByRole('button', { name: 'Выберите агента' }))
    await screen.findByRole('link', { name: /Implement login/ })
    expect(directory.getChatsDirectory).toHaveBeenLastCalledWith(
      expect.objectContaining({
        agentId: undefined,
        before: undefined,
        search: 'login',
        userIds: [owner],
      }),
    )
  })
  it('defaults privileged users to mine until they explicitly select all', async () => {
    useAuthStore.setState({ systemRole: 'admin', permissions: ['sessions:read_all'] })
    renderPage()
    await screen.findByRole('link', { name: /Implement login/ })
    expect(directory.getChatsDirectory).toHaveBeenLastCalledWith(
      expect.objectContaining({ userIds: [owner] }),
    )
  })
  it('renders an empty agent directory without a create action', async () => {
    vi.mocked(directory.getChatsDirectory).mockResolvedValue({
      agents: [],
      selected_agent_id: null,
      items: [],
      next_before: null,
    })
    renderPage()
    expect(await screen.findByText('Доступных агентов нет')).toBeVisible()
    expect(screen.queryByRole('button', { name: 'Новый чат' })).not.toBeInTheDocument()
    expect(screen.queryByLabelText('Количество сессий')).not.toBeInTheDocument()
  })
  it('reuses private-chat idempotency after a failed create', async () => {
    vi.mocked(fleet.createSession).mockRejectedValueOnce(new Error('offline'))
    renderPage()
    fireEvent.click(await screen.findByRole('button', { name: 'Новый чат' }))
    fireEvent.change(screen.getByLabelText('Название'), { target: { value: '  Private work  ' } })
    fireEvent.click(screen.getByRole('button', { name: 'Создать сессию' }))
    await screen.findByRole('alert')
    const first = vi.mocked(fleet.createSession).mock.calls[0]?.[0]
    expect(first).toMatchObject({
      primary_agent_id: developer.id,
      title: 'Private work',
      leader_agent_id: null,
    })
    fireEvent.click(screen.getByRole('button', { name: 'Создать сессию' }))
    await waitFor(() => expect(fleet.createSession).toHaveBeenCalledTimes(2))
    expect(vi.mocked(fleet.createSession).mock.calls[1]?.[0].idempotency_key).toBe(
      first?.idempotency_key,
    )
  })
})
