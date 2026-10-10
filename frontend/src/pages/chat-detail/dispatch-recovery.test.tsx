import { act, renderHook } from '@testing-library/react'
import { beforeEach, describe, expect, it } from 'vitest'
import { useAuthStore } from '@/shared/auth/store'
import { markDispatch, payloadDigest } from './core'
import { useDispatchRecovery } from './dispatch-recovery'
import { canonicalAnswerPayload } from './answer-payload'

const scope = 'session-one:answer'
const input = { text: 'Private original answer', idempotency_key: 'original-key' }
const marker = {
  actor: 'owner',
  agent: 'agent-one',
  service: 'original-service',
  key: 'original-key',
}

const low = '00000000-0000-4000-8000-000000000001'
const high = '00000000-0000-4000-8000-000000000002'
const multipleAnswer = {
  expected_question_version: 7,
  requirement_revision: 11,
  selected_option_ids: [high, low],
  text: 'Original custom answer',
  comment: 'Original comment',
  idempotency_key: marker.key,
}

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
      settled = await result.current.finishRecovered(
        input,
        marker.key,
        marker.agent,
        marker.service,
      )
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

describe('multiple-answer lost ACK recovery', () => {
  beforeEach(() => sessionStorage.clear())

  async function loseAckAndReload() {
    const initial = renderHook(() => useDispatchRecovery(scope))
    await act(async () => {
      await initial.result.current.prepare(
        canonicalAnswerPayload(multipleAnswer),
        marker.key,
        marker.agent,
        marker.service,
        () => true,
      )
      initial.result.current.fail(new Error('Lost answer ACK'))
    })
    expect(initial.result.current.held).toBe(true)
    initial.unmount()
    return renderHook(() => useDispatchRecovery(scope))
  }

  it('settles sorted readback after reverse-order clicks, lost ACK and reload', async () => {
    const serverRequest = { ...multipleAnswer, selected_option_ids: [low, high] }
    expect(await payloadDigest(multipleAnswer)).not.toBe(await payloadDigest(serverRequest))
    const { result } = await loseAckAndReload()
    expect(result.current.restored).toBe(true)
    const raw = sessionStorage.getItem(`fleet-control.chat-dispatch.v1:${scope}`)!
    expect(raw).not.toContain(multipleAnswer.text)
    expect(raw).not.toContain(multipleAnswer.comment)
    expect(raw).not.toContain('private-token')
    let settled = false
    await act(async () => {
      settled = await result.current.finishRecovered(
        canonicalAnswerPayload(serverRequest),
        marker.key,
        marker.agent,
        marker.service,
      )
    })
    expect(settled).toBe(true)
    expect(result.current.held).toBe(false)
    expect(sessionStorage.getItem(`fleet-control.chat-dispatch.v1:${scope}`)).toBeNull()
  })

  it.each([
    { expected_question_version: 8 },
    { requirement_revision: 12 },
    { selected_option_ids: [low] },
    { selected_option_ids: [low, high, high] },
    { text: 'Changed answer' },
    { text: null },
    { comment: 'Changed comment' },
    { comment: null },
    { idempotency_key: 'new-key' },
  ])('retains the original marker when recovered answer identity changes: %j', async (changed) => {
    const { result } = await loseAckAndReload()
    const raw = sessionStorage.getItem(`fleet-control.chat-dispatch.v1:${scope}`)
    const request = canonicalAnswerPayload({ ...multipleAnswer, ...changed })
    let settled = true
    await act(async () => {
      settled = await result.current.finishRecovered(
        request,
        request.idempotency_key,
        marker.agent,
        marker.service,
      )
    })
    expect(settled).toBe(false)
    expect(result.current.held).toBe(true)
    expect(sessionStorage.getItem(`fleet-control.chat-dispatch.v1:${scope}`)).toBe(raw)
  })
})
