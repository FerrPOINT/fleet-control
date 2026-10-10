import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest'
import { clearControlHandle, readControlJournal, saveControlHandle } from './control-journal'

const steer = { operation: 'steer', runId: 'run-a', key: 'original-key' } as const
const stop = { operation: 'stop', runId: 'run-b', key: 'stop-key' } as const
const key = 'fleet-runtime-controls:v1:owner:session'
beforeEach(() => sessionStorage.clear())
afterEach(() => vi.restoreAllMocks())

describe('runtime command metadata journal', () => {
  it('restores only original metadata, never message input or credentials', () => {
    saveControlHandle('owner', 'session', {
      ...steer,
      ...JSON.parse('{"input":"private guidance","token":"private-token"}'),
    })
    expect(readControlJournal('owner', 'session')).toEqual({ steer })
    expect(sessionStorage.getItem(key)).not.toContain('private')
    expect(sessionStorage.getItem(key)).not.toContain('input')
    expect(sessionStorage.getItem(key)).not.toContain('token')
  })
  it('scopes handles to both the actor and session', () => {
    saveControlHandle('owner', 'session', steer)
    expect(readControlJournal('other', 'session')).toEqual({})
    expect(readControlJournal('owner', 'other')).toEqual({})
  })
  it('retains independent stop and steer handles', () => {
    saveControlHandle('owner', 'session', steer)
    saveControlHandle('owner', 'session', stop)
    expect(readControlJournal('owner', 'session')).toEqual({ steer, stop })
  })
  it('allows only the original key and run on replay', () => {
    saveControlHandle('owner', 'session', steer)
    saveControlHandle('owner', 'session', steer)
    for (const change of [{ key: 'fresh-key' }, { runId: 'new-run' }])
      expect(() => saveControlHandle('owner', 'session', { ...steer, ...change })).toThrow(
        /original control/,
      )
    expect(readControlJournal('owner', 'session')).toEqual({ steer })
  })
  it('only clears the exact settled handle and preserves the other operation', () => {
    saveControlHandle('owner', 'session', steer)
    saveControlHandle('owner', 'session', stop)
    expect(() => clearControlHandle('owner', 'session', { ...steer, key: 'wrong' })).toThrow(
      /identity changed/,
    )
    clearControlHandle('owner', 'session', steer)
    expect(readControlJournal('owner', 'session')).toEqual({ stop })
  })
  it.each([
    '[]',
    'null',
    '{"steer":{"operation":"stop","runId":"a","key":"b"}}',
    '{"steer":{"operation":"steer","runId":"a","key":"b","input":"secret"}}',
    '{"stop":{"operation":"stop","runId":"bad/path","key":"b"}}',
    '{"unknown":{}}',
    'not json',
  ])('fails closed on invalid metadata: %s', (raw) => {
    sessionStorage.setItem(key, raw)
    expect(() => readControlJournal('owner', 'session')).toThrow()
    expect(() => saveControlHandle('owner', 'session', steer)).toThrow()
    expect(sessionStorage.getItem(key)).toBe(raw)
  })
  it('bounds the stored document before parsing', () => {
    sessionStorage.setItem(key, ' '.repeat(2049))
    expect(() => readControlJournal('owner', 'session')).toThrow(/Invalid/)
  })
  it('does not claim durability when the browser rejects a write', () => {
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('Quota')
    })
    expect(() => saveControlHandle('owner', 'session', steer)).toThrow('Quota')
    expect(readControlJournal('owner', 'session')).toEqual({})
  })
  it('holds an initial save when the browser silently discards the write', () => {
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {})
    expect(() => saveControlHandle('owner', 'session', steer)).toThrow(/not retained/)
    expect(sessionStorage.getItem(key)).toBeNull()
  })
  it('holds a clear when the browser silently retains the unresolved metadata', () => {
    saveControlHandle('owner', 'session', steer)
    saveControlHandle('owner', 'session', stop)
    const original = sessionStorage.getItem(key)
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {})
    expect(() => clearControlHandle('owner', 'session', steer)).toThrow(/not retained/)
    expect(sessionStorage.getItem(key)).toBe(original)
    expect(readControlJournal('owner', 'session')).toEqual({ steer, stop })
  })
  it.each([
    { operation: 'save', readback: '{}' },
    { operation: 'save', readback: `${JSON.stringify({ steer, stop })}\n` },
    { operation: 'clear', readback: '{}' },
    { operation: 'clear', readback: `${JSON.stringify({ stop })}\n` },
  ])('holds $operation on wrong or byte-mutated readback: $readback', ({ operation, readback }) => {
    saveControlHandle('owner', 'session', steer)
    saveControlHandle('owner', 'session', stop)
    const original = sessionStorage.getItem(key)
    vi.spyOn(Storage.prototype, 'getItem')
      .mockReturnValueOnce(original)
      .mockReturnValueOnce(readback)
    const write = operation === 'save' ? saveControlHandle : clearControlHandle
    expect(() => write('owner', 'session', steer)).toThrow(/not retained/)
  })
})
