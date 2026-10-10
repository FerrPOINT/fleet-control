import { act, renderHook } from '@testing-library/react'
import { beforeEach, describe, expect, it } from 'vitest'
import { useAuthStore } from '@/shared/auth/store'
import { markDispatch, payloadDigest } from './core'
import { useDispatchRecovery } from './dispatch-recovery'

const scope = 'session-one:answer'
const input = { text: 'Private original answer', idempotency_key: 'original-key' }
const marker = { actor: 'owner', agent: 'agent-one', service: 'original-service', key: 'original-key' }

beforeEach(async () => {
  sessionStorage.clear()
  useAuthStore.setState({ userId: 'owner', token: 'private-token', signingOut: false })
  markDispatch(scope, { ...marker, digest: await payloadDigest(input) })
})

describe('original answer metadata settlement', () => {
  it('releases restored metadata only for its exact authorized command body and key', async () => {
    const { result } = renderHook(() => useDispatchRecovery(scope))
    expect(result.current.restored).toBe(true)
    let settled = false
    await act(async () => {
      settled = await result.current.finishRecovered(input, marker.key, marker.agent, marker.service)
    })
    expect(settled).toBe(true)
    expect(result.current.held).toBe(false)
    expect(sessionStorage.getItem(`fleet-control.chat-dispatch.v1:${scope}`)).toBeNull()
  })

  it.each(['body', 'key', 'agent', 'service', 'actor'])(
    'retains custody when the recovered %s differs',
    async (changed) => {
      const original = sessionStorage.getItem(`fleet-control.chat-dispatch.v1:${scope}`)
      const { result } = renderHook(() => useDispatchRecovery(scope))
      if (changed === 'actor') useAuthStore.setState({ userId: 'another-owner' })
      let settled = true
      await act(async () => {
        settled = await result.current.finishRecovered(
          changed === 'body' ? { ...input, text: 'Changed' } : input,
          changed === 'key' ? 'new-key' : marker.key,
          changed === 'agent' ? 'another-agent' : marker.agent,
          changed === 'service' ? 'another-service' : marker.service,
        )
      })
      expect(settled).toBe(false)
      expect(result.current.held).toBe(true)
      expect(sessionStorage.getItem(`fleet-control.chat-dispatch.v1:${scope}`)).toBe(original)
      expect(original).not.toContain(input.text)
      expect(original).not.toContain('private-token')
    },
  )
})
