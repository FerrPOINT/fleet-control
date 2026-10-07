import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  clearControlRecovery,
  controlRecoveryService,
  readControlRecovery,
  writeControlRecovery,
  type ControlRecoveryRecord,
} from './chat-control-recovery'

const command: ControlRecoveryRecord = {
  version: 1,
  service: controlRecoveryService('', 'http://localhost:7701'),
  sessionId: 'session1',
  runId: 'original-run',
  agentId: 'agent1',
  actorId: 'owner',
  key: 'original-key',
  operation: 'steer',
  payloadSha256: 'a'.repeat(64),
}
const storageKey = 'fleet-control.control-recovery.v1:session1'
beforeEach(() => sessionStorage.clear())
afterEach(() => vi.restoreAllMocks())

describe('tab-scoped original command metadata', () => {
  it('retains only the closed metadata shape and does not use localStorage', () => {
    const permanent = localStorage.getItem(storageKey)
    writeControlRecovery(command)
    expect(readControlRecovery('session1')).toEqual({ state: 'pending', command })
    expect(JSON.parse(sessionStorage.getItem(storageKey)!)).toEqual(command)
    expect(localStorage.getItem(storageKey)).toBe(permanent)
  })
  it.each(['input', 'body', 'transcript', 'token', 'bearer', 'runtimeCredential'])(
    'rejects an extra %s field',
    (field) => {
      expect(() => writeControlRecovery({ ...command, [field]: 'private-value' })).toThrow()
      expect(sessionStorage.getItem(storageKey)).toBeNull()
    },
  )
  it.each([
    '{invalid',
    '{}',
    JSON.stringify({ ...command, version: 2 }),
    JSON.stringify({ ...command, payloadSha256: 'A'.repeat(64) }),
    JSON.stringify({ ...command, sessionId: 'another-session' }),
    'x'.repeat(4097),
  ])('holds unreadable or incompatible metadata without deleting it', (raw) => {
    sessionStorage.setItem(storageKey, raw)
    expect(readControlRecovery('session1')).toEqual({ state: 'blocked' })
    expect(sessionStorage.getItem(storageKey)).toBe(raw)
  })
  it('does not overwrite an unresolved key or clear another scope', () => {
    writeControlRecovery(command)
    expect(() => writeControlRecovery({ ...command, key: 'new-key' })).toThrow()
    expect(clearControlRecovery({ ...command, actorId: 'another-actor' })).toBe(false)
    expect(clearControlRecovery({ ...command, payloadSha256: 'b'.repeat(64) })).toBe(false)
    expect(readControlRecovery('session1')).toEqual({ state: 'pending', command })
    expect(clearControlRecovery(command)).toBe(true)
    expect(readControlRecovery('session1')).toEqual({ state: 'none' })
  })
  it('fails closed when storage is unavailable or cannot retain a write', () => {
    vi.spyOn(Object.getPrototypeOf(sessionStorage), 'getItem').mockImplementation(() => {
      throw new Error('denied')
    })
    expect(readControlRecovery('session1')).toEqual({ state: 'blocked' })
    expect(() => writeControlRecovery(command)).toThrow()
    vi.restoreAllMocks()
    vi.spyOn(Object.getPrototypeOf(sessionStorage), 'setItem').mockImplementation(() => {})
    expect(() => writeControlRecovery(command)).toThrow()
  })
  it('binds public browser/API/issuer context without URL credentials or query values', () => {
    const first = controlRecoveryService(
      'https://api.example.test/fleet',
      'https://auth.example.test',
    )
    expect(first).not.toBe(
      controlRecoveryService('https://api.example.test/other', 'https://auth.example.test'),
    )
    expect(first).not.toBe(
      controlRecoveryService('https://api.example.test/fleet', 'https://other.example.test'),
    )
    const safe = controlRecoveryService(
      'https://user:secret@api.example.test/fleet?token=secret',
      'https://auth.example.test#secret',
    )
    expect(safe).toBe(first)
    expect(safe).not.toContain('secret')
  })
})
