import { act, cleanup, render, screen, waitFor } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router'
import { completeSso } from '@sdlc/ui/sso'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { SsoCallbackPage } from './index'
import { useAuthStore } from '@/shared/auth/store'

vi.mock('@sdlc/ui/sso', () => ({ completeSso: vi.fn() }))

function renderCallback() {
  return render(
    <MemoryRouter initialEntries={['/sso/callback']}>
      <Routes>
        <Route path="/sso/callback" element={<SsoCallbackPage />} />
        <Route path="/chats" element={<h1>Chats destination</h1>} />
        <Route path="/" element={<h1>Existing login destination</h1>} />
      </Routes>
    </MemoryRouter>,
  )
}

function responses(
  role: 'user' | 'operator',
  permissionUser = 'local-user',
  status = 200,
  permissions = ['sessions:read_own', 'agents:read_directory'],
) {
  vi.stubGlobal(
    'fetch',
    vi.fn(async (url: string) => {
      if (url.endsWith('/users/me/permissions')) {
        return new Response(
          JSON.stringify({
            user_id: permissionUser,
            role,
            is_system_admin: false,
            permissions,
          }),
          { status },
        )
      }
      return new Response(
        JSON.stringify({
          id: 'local-user',
          email: 'member@example.test',
          username: 'member',
          display_name: 'Member',
        }),
      )
    }),
  )
}

describe('SSO callback authorization', () => {
  it('does not share an unfinished SSO exchange across a logout and remount', async () => {
    let resolve!: (value: Awaited<ReturnType<typeof completeSso>>) => void
    vi.mocked(completeSso).mockImplementationOnce(
      () =>
        new Promise((settle) => {
          resolve = settle
        }),
    )
    responses('user')
    const original = renderCallback()
    await waitFor(() => expect(completeSso).toHaveBeenCalledTimes(1))
    act(() => useAuthStore.getState().logout())
    original.unmount()
    renderCallback()
    await screen.findByRole('alert')
    await act(async () =>
      resolve({
        accessToken: 'old-token',
        expiresAt: Date.now() + 60_000,
        subject: 'old-subject',
        email: 'old@example.test',
        name: 'Old profile',
        returnTo: '/chats',
      }),
    )
    expect(useAuthStore.getState().token).toBeNull()
    expect(completeSso).toHaveBeenCalledTimes(1)
    expect(fetch).not.toHaveBeenCalled()
  })

  it('does not replace an existing login from a stale callback URL', async () => {
    responses('user')
    useAuthStore
      .getState()
      .setAuth({ token: 'new-token', userId: 'new-user', email: 'new@example.test' })
    renderCallback()
    await screen.findByRole('heading', { name: 'Existing login destination' })
    expect(completeSso).not.toHaveBeenCalled()
    expect(fetch).not.toHaveBeenCalled()
    expect(useAuthStore.getState().token).toBe('new-token')
  })

  it.each(['new-login', 'logout'] as const)(
    'does not apply old callback profile after %s',
    async (change) => {
      let resolve!: (value: Response) => void
      const profile = new Promise<Response>((settle) => {
        resolve = settle
      })
      responses('user')
      vi.mocked(fetch).mockImplementationOnce(() => profile)
      renderCallback()
      await waitFor(() => expect(fetch).toHaveBeenCalledTimes(1))
      act(() => {
        if (change === 'new-login')
          useAuthStore
            .getState()
            .setAuth({ token: 'new-token', userId: 'new-user', email: 'new@example.test' })
        else useAuthStore.getState().logout()
      })
      await act(async () =>
        resolve(
          Response.json({
            id: 'local-user',
            email: 'member@example.test',
            username: 'member',
            display_name: 'Member',
          }),
        ),
      )
      expect(useAuthStore.getState().token).toBe(change === 'new-login' ? 'new-token' : null)
      expect(fetch).toHaveBeenCalledTimes(1)
      expect(screen.queryByRole('heading', { name: 'Chats destination' })).not.toBeInTheDocument()
    },
  )

  beforeEach(() => {
    useAuthStore.getState().logout()
    vi.mocked(completeSso).mockResolvedValue({
      accessToken: 'verified-token',
      expiresAt: Date.now() + 60_000,
      subject: 'central-subject',
      email: 'member@example.test',
      name: 'Member',
      returnTo: '/chats',
    })
  })

  afterEach(() => {
    cleanup()
    vi.unstubAllGlobals()
    vi.clearAllMocks()
  })

  it.each(['user', 'operator'] as const)(
    'preserves backend %s role without admin promotion',
    async (role) => {
      responses(role)
      renderCallback()
      expect(await screen.findByRole('heading', { name: 'Chats destination' })).toBeVisible()
      expect(useAuthStore.getState()).toMatchObject({
        userId: 'local-user',
        token: 'verified-token',
        systemRole: role,
        isSystemAdmin: false,
      })
      expect(fetch).toHaveBeenCalledWith(expect.stringContaining('/users/me/permissions'), {
        headers: { Authorization: 'Bearer verified-token' },
      })
    },
  )

  it('rejects permissions belonging to another profile', async () => {
    responses('user', 'other-user')
    renderCallback()
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Профиль и права пользователя не совпадают',
    )
    expect(useAuthStore.getState().token).toBeNull()
  })

  it('fails closed when permissions cannot be verified', async () => {
    responses('operator', 'local-user', 503)
    renderCallback()
    expect(await screen.findByRole('alert')).toHaveTextContent('Не удалось проверить права')
    expect(useAuthStore.getState().token).toBeNull()
  })

  it('uses central backend permissions without rewriting the historical user role', async () => {
    responses('user', 'local-user', 200, ['agents:manage', 'settings:manage', 'sessions:read_all'])
    renderCallback()
    expect(await screen.findByRole('heading', { name: 'Chats destination' })).toBeVisible()
    expect(useAuthStore.getState()).toMatchObject({
      systemRole: 'user',
      isSystemAdmin: false,
      permissions: ['agents:manage', 'settings:manage', 'sessions:read_all'],
    })
  })
})
