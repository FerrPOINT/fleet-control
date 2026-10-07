import { createHash, webcrypto } from 'node:crypto'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { apiRequest } from './client'
import {
  lookupRuntimeControl,
  lookupRuntimeControlByDigest,
  type RuntimeControlLookupQuery,
  runtimeControlPayloadSha256,
  trimRuntimeControlInput,
} from './runtime-control-lookup'

vi.mock('./client', () => ({ apiRequest: vi.fn() }))
beforeEach(() => {
  vi.clearAllMocks()
  vi.stubGlobal('crypto', webcrypto)
})
afterEach(() => vi.unstubAllGlobals())

describe('published original-key control lookup', () => {
  it('reads an already computed digest with a closed query and no original text', async () => {
    await lookupRuntimeControlByDigest('session1', 'original-run', 'original-key', {
      operation: 'stop',
      payload_sha256: 'ea123901799860e917ce433b72c621c4afffe4c866497c1bfb38507816f8048f',
      input: 'must-not-enter-query',
      actorId: 'must-not-enter-query',
    } as RuntimeControlLookupQuery)
    expect(apiRequest).toHaveBeenCalledOnce()
    const [path, init] = vi.mocked(apiRequest).mock.calls[0]!
    const query = new URL(path, 'http://fixture.invalid').searchParams
    expect([...query.keys()]).toEqual(['operation', 'payload_sha256'])
    expect(query.get('operation')).toBe('stop')
    expect(path).not.toContain('must-not-enter-query')
    expect(init).toEqual({ method: 'GET', headers: { 'Idempotency-Key': 'original-key' } })
  })
  it('matches the published stop-null digest', async () => {
    expect(await runtimeControlPayloadSha256({ operation: 'stop', input: null })).toBe(
      'ea123901799860e917ce433b72c621c4afffe4c866497c1bfb38507816f8048f',
    )
  })
  it.each(['keep scope', '\u0085 keep scope\u0085'])(
    'matches the published steer digest: %s',
    async (input) => {
      expect(await runtimeControlPayloadSha256({ operation: 'steer', input })).toBe(
        '836755e924913fa3776aeec3253eb2f9ba7c4d473e44deb16e87bbdd93f9f1b2',
      )
    },
  )
  it('trims every published Rust boundary space and preserves FEFF and internal text', async () => {
    const spaces =
      '\t\n\v\f\r \u0085\u00a0\u1680\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007\u2008\u2009\u200a\u2028\u2029\u202f\u205f\u3000'
    expect(trimRuntimeControlInput(`${spaces}keep scope${spaces}`)).toBe('keep scope')
    const semantic = '\ufeffkeep\u0085 scope\n"Текст"\ufeff'
    expect(trimRuntimeControlInput(`${spaces}${semantic}${spaces}`)).toBe(semantic)
    expect(
      await runtimeControlPayloadSha256({
        operation: 'steer',
        input: `${spaces}${semantic}${spaces}`,
      }),
    ).toBe(
      createHash('sha256')
        .update(JSON.stringify({ input: semantic, operation: 'steer' }), 'utf8')
        .digest('hex'),
    )
  })
  it('uses GET, one key header and only operation/digest query fields', async () => {
    vi.mocked(apiRequest).mockResolvedValue({ id: 'historical-command' })
    expect(
      await lookupRuntimeControl('session/original', 'run/original', 'original-key', {
        operation: 'steer',
        input: '\u0085keep scope\u0085',
      }),
    ).toEqual({ id: 'historical-command' })
    expect(apiRequest).toHaveBeenCalledOnce()
    const [path, init] = vi.mocked(apiRequest).mock.calls[0]!
    const url = new URL(path, 'http://fixture.invalid')
    expect(url.pathname).toBe(
      '/api/v1/sessions/session%2Foriginal/runs/run%2Foriginal/controls/lookup',
    )
    expect([...url.searchParams.keys()]).toEqual(['operation', 'payload_sha256'])
    expect(url.searchParams.get('payload_sha256')).toBe(
      '836755e924913fa3776aeec3253eb2f9ba7c4d473e44deb16e87bbdd93f9f1b2',
    )
    expect(init).toEqual({ method: 'GET', headers: { 'Idempotency-Key': 'original-key' } })
    expect(path).not.toContain('original-key')
    expect(path).not.toContain('keep scope')
  })
  it.each([404, 409])('propagates lookup %s without another request', async (status) => {
    const error = new Error(`HTTP ${status}`)
    vi.mocked(apiRequest).mockRejectedValue(error)
    await expect(
      lookupRuntimeControl('session', 'run', 'key', { operation: 'stop', input: null }),
    ).rejects.toBe(error)
    expect(apiRequest).toHaveBeenCalledOnce()
  })
})
