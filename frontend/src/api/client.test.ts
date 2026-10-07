import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { apiRequest } from './client'
import { useAuthStore } from '@/shared/auth/store'

function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((settle) => {
    resolve = settle
  })
  return { promise, resolve }
}

describe('request authorization isolation', () => {
  const navigate = vi.fn()
  beforeEach(() => {
    navigate.mockClear()
    useAuthStore
      .getState()
      .setAuth({ token: 'old-token', userId: 'owner', email: 'owner@example.test' })
    vi.stubGlobal('window', { location: { assign: navigate } })
  })
  afterEach(() => vi.unstubAllGlobals())

  it.each([
    { token: 'new-token', userId: 'other' },
    { token: 'new-token', userId: 'owner' },
    { token: 'old-token', userId: 'owner' },
  ])('does not deliver the prior login response after a new login: %j', async (login) => {
    const response = deferred<Response>()
    vi.stubGlobal(
      'fetch',
      vi.fn(() => response.promise),
    )
    const request = apiRequest('/api/v1/sessions')
    useAuthStore.getState().setAuth({ ...login, email: 'new@example.test' })
    response.resolve(Response.json({ privateTranscript: 'prior owner content' }))
    await expect(request).rejects.toMatchObject({ name: 'AuthContextChangedError' })
    expect(navigate).not.toHaveBeenCalled()
    const fetchCall = vi.mocked(fetch).mock.calls[0]
    expect(fetchCall).toBeDefined()
    if (!fetchCall) throw new Error('Expected the original authenticated request')
    expect(new Headers(fetchCall[1]?.headers).get('Authorization')).toBe('Bearer old-token')
  })

  it('does not let a late 401 clear a newer login', async () => {
    const response = deferred<Response>()
    vi.stubGlobal(
      'fetch',
      vi.fn(() => response.promise),
    )
    const request = apiRequest('/api/v1/sessions')
    useAuthStore
      .getState()
      .setAuth({ token: 'new-token', userId: 'other', email: 'other@example.test' })
    response.resolve(Response.json({ error: 'old login expired' }, { status: 401 }))
    await expect(request).rejects.toMatchObject({ name: 'AuthContextChangedError' })
    expect(useAuthStore.getState().token).toBe('new-token')
    expect(navigate).not.toHaveBeenCalled()
  })

  it('does not surface a prior login error body to the next user', async () => {
    const response = deferred<Response>()
    vi.stubGlobal(
      'fetch',
      vi.fn(() => response.promise),
    )
    const request = apiRequest('/api/v1/sessions')
    useAuthStore
      .getState()
      .setAuth({ token: 'new-token', userId: 'other', email: 'other@example.test' })
    response.resolve(Response.json({ error: 'private prior task details' }, { status: 403 }))
    const error = await request.catch((caught: unknown) => caught)
    expect(error).toMatchObject({ name: 'AuthContextChangedError' })
    expect(error).not.toHaveProperty('body')
    expect(String(error)).not.toContain('private prior task details')
  })

  it('checks authorization again after a delayed response body', async () => {
    const body = deferred<string>()
    const response = Response.json({ ignored: true })
    vi.spyOn(response, 'text').mockImplementation(() => body.promise)
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => response),
    )
    const request = apiRequest('/api/v1/sessions')
    await vi.waitFor(() => expect(response.text).toHaveBeenCalled())
    useAuthStore
      .getState()
      .setAuth({ token: 'new-token', userId: 'other', email: 'other@example.test' })
    body.resolve(JSON.stringify({ privateTranscript: 'prior owner content' }))
    await expect(request).rejects.toMatchObject({ name: 'AuthContextChangedError' })
  })

  it('does not start a request during central sign-out', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => Response.json({ accepted: true })),
    )
    useAuthStore.getState().startSignOut()
    await expect(
      apiRequest('/api/v1/sessions', { method: 'POST', body: '{}' }),
    ).rejects.toMatchObject({ name: 'AuthContextChangedError' })
    expect(fetch).not.toHaveBeenCalled()
  })

  it('expires only its own login once for concurrent 401 responses', async () => {
    const response = deferred<Response>()
    vi.stubGlobal(
      'fetch',
      vi.fn(() => response.promise.then((result) => result.clone())),
    )
    const first = apiRequest('/api/v1/sessions')
    const second = apiRequest('/api/v1/agents')
    response.resolve(Response.json({ error: 'expired' }, { status: 401 }))
    const outcomes = await Promise.allSettled([first, second])
    expect(outcomes.every((outcome) => outcome.status === 'rejected')).toBe(true)
    expect(useAuthStore.getState().token).toBeNull()
    expect(navigate).toHaveBeenCalledTimes(1)
    expect(navigate).toHaveBeenCalledWith('/login')
  })

  it('keeps an anonymous 401 local without starting a login navigation', async () => {
    useAuthStore.getState().logout()
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => Response.json({ error: 'not signed in' }, { status: 401 })),
    )
    await expect(
      apiRequest('/api/v1/auth/login', { method: 'POST', body: '{}' }),
    ).rejects.toMatchObject({ status: 401 })
    expect(navigate).not.toHaveBeenCalled()
  })
})
