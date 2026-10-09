import { beforeEach, describe, expect, it, vi } from 'vitest'
import { apiRequest } from './client'
import { steerSessionRun, stopSessionRun } from './fleet'

vi.mock('./client', () => ({ apiRequest: vi.fn() }))

beforeEach(() => vi.clearAllMocks())

describe('runtime control command identity', () => {
  it('sends the caller-owned stable key on every stop replay', async () => {
    await stopSessionRun('session', 'original-run', 'stop-key')
    await stopSessionRun('session', 'original-run', 'stop-key')
    expect(apiRequest).toHaveBeenCalledTimes(2)
    for (const [path, options] of vi.mocked(apiRequest).mock.calls) {
      expect(path).toBe('/api/v1/sessions/session/runs/original-run/stop')
      expect(options).toEqual({
        method: 'POST',
        headers: { 'Idempotency-Key': 'stop-key' },
        body: '{}',
      })
    }
  })

  it('keeps steer payload separate from the required command header', async () => {
    await steerSessionRun('session', 'run', { input: 'Original instruction' }, 'steer-key')
    expect(apiRequest).toHaveBeenCalledWith('/api/v1/sessions/session/runs/run/steer', {
      method: 'POST',
      headers: { 'Idempotency-Key': 'steer-key' },
      body: JSON.stringify({ input: 'Original instruction' }),
    })
  })
})
