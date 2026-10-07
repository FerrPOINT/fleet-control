import { beforeEach, describe, expect, it, vi } from 'vitest'

describe('Fleet auth storage migration', () => {
  it('keeps sign-out authority suspension in memory and never persists it or credentials', async () => {
    const { useAuthStore } = await import('./store')
    useAuthStore
      .getState()
      .setAuth({ token: 'private-token', userId: 'owner', email: 'owner@example.test' })
    useAuthStore.getState().startSignOut()
    expect(useAuthStore.getState().signingOut).toBe(true)
    expect(useAuthStore.getState().token).toBe('private-token')
    const stored = JSON.parse(localStorage.getItem('fleet-control-auth')!)
    expect(stored.state).not.toHaveProperty('signingOut')
    expect(stored.state).not.toHaveProperty('token')
    useAuthStore.getState().logout()
    expect(useAuthStore.getState().token).toBeNull()
  })
  beforeEach(() => {
    vi.resetModules()
    localStorage.clear()
  })

  it.each([
    { state: { token: 'legacy-secret', userId: 'user-1', email: 'user@example.test' }, version: 0 },
    { token: 'legacy-secret', userId: 'user-1', email: 'user@example.test' },
  ])('removes a legacy access token without losing the cached profile', async (legacy) => {
    localStorage.setItem('fleet-control-auth', JSON.stringify(legacy))

    const { useAuthStore } = await import('./store')

    expect(useAuthStore.getState().token).toBeNull()
    expect(useAuthStore.getState().userId).toBe('user-1')
    expect(useAuthStore.getState().email).toBe('user@example.test')
    const stored = localStorage.getItem('fleet-control-auth')
    expect(stored).not.toContain('legacy-secret')
    const parsed = JSON.parse(stored!)
    expect(parsed.state ?? parsed).not.toHaveProperty('token')
  })

  it('drops an unreadable legacy auth record', async () => {
    localStorage.setItem('fleet-control-auth', '{"token":"legacy-secret"')

    const { useAuthStore } = await import('./store')

    expect(useAuthStore.getState().token).toBeNull()
    expect(localStorage.getItem('fleet-control-auth')).toBeNull()
  })
})
