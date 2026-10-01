import { cleanup, render, screen } from '@testing-library/react'
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
      </Routes>
    </MemoryRouter>,
  )
}

function responses(role: 'user' | 'operator', permissionUser = 'local-user', status = 200) {
  vi.stubGlobal(
    'fetch',
    vi.fn(async (url: string) => {
      if (url.endsWith('/users/me/permissions')) {
        return new Response(
          JSON.stringify({
            user_id: permissionUser,
            role,
            is_system_admin: false,
            permissions: ['sessions:read_own', 'agents:read_directory'],
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
})
