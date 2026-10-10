import { beforeEach, describe, expect, it, vi } from 'vitest'
import { apiRequest } from './client'
import { ApiError } from '@sdlc/ui/lib'
import {
  decideTaskApproval,
  getTaskApprovalDecision,
  getTaskApprovals,
  type ApprovalDecision,
  type RuntimeApprovalRequest,
} from './task-approvals'

vi.mock('./client', async (original) => ({
  ...(await original<typeof import('./client')>()),
  apiRequest: vi.fn(),
}))

beforeEach(() => vi.clearAllMocks())

const receipt: ApprovalDecision = {
  id: 'decision',
  session_id: 'session/1',
  approval_id: 'approval/2',
  session_run_id: 'run',
  actor_user_id: 'actor',
  choice: 'once',
  state: 'pending',
  created_at: '2026-10-01T12:00:00Z',
}
const approval: RuntimeApprovalRequest = {
  id: 'approval',
  session_id: 'session',
  session_run_id: 'run',
  agent_id: 'agent',
  runtime_run_id: 'hermes-run',
  prompt: 'Bounded action',
  created_at: '2026-10-01T12:00:00Z',
  detail: {},
  state: 'pending',
  runtime_approval_id: 'exact-action',
}

describe('targeted approval API fixture contract', () => {
  it('preserves a valid exact approval list', async () => {
    vi.mocked(apiRequest).mockResolvedValue([approval])
    expect(await getTaskApprovals('session')).toEqual([approval])
  })
  it.each([
    { ...approval, created_at: 'not-a-date' },
    { ...approval, state: 'always' },
    { ...approval, runtime_approval_id: 42 },
  ])('rejects unsafe rendered approval fields: %j', async (payload) => {
    vi.mocked(apiRequest).mockResolvedValue([payload])
    await expect(getTaskApprovals('session')).rejects.toThrow('Invalid runtime approval response')
  })
  it.each([{}, null, 'invalid', [null]])(
    'rejects malformed approval lists: %j',
    async (payload) => {
      vi.mocked(apiRequest).mockResolvedValue(payload)
      await expect(getTaskApprovals('session')).rejects.toThrow('Invalid runtime approval response')
      expect(apiRequest).toHaveBeenCalledTimes(1)
    },
  )

  it('reads approvals only within the encoded session path', async () => {
    vi.mocked(apiRequest).mockResolvedValue([])
    expect(await getTaskApprovals('session/with space')).toEqual([])
    expect(apiRequest).toHaveBeenCalledWith('/api/v1/sessions/session%2Fwith%20space/approvals')
  })

  it.each(['once', 'deny'] as const)(
    'posts an explicit %s choice to one approval',
    async (choice) => {
      vi.mocked(apiRequest).mockResolvedValue({ ...receipt, choice })
      const response = await decideTaskApproval('session/1', 'approval/2', {
        choice,
        idempotency_key: 'stable-key',
        ...{ resolve_all: true, run_id: 'unrelated' },
      })
      expect(response).toEqual({ ...receipt, choice })
      expect(apiRequest).toHaveBeenCalledExactlyOnceWith(
        '/api/v1/sessions/session%2F1/approvals/approval%2F2/decision',
        { method: 'POST', body: JSON.stringify({ choice, idempotency_key: 'stable-key' }) },
      )
    },
  )

  it.each(['pending', 'delivered', 'uncertain', 'failed'] as const)(
    'preserves the %s delivery receipt without inferring approval resolution',
    async (state) => {
      vi.mocked(apiRequest).mockResolvedValue({ ...receipt, state })
      expect(
        await decideTaskApproval('session', 'approval', { choice: 'once', idempotency_key: 'key' }),
      ).toEqual({ ...receipt, state })
    },
  )

  it('propagates a lost response without a second request', async () => {
    vi.mocked(apiRequest).mockRejectedValue(new TypeError('fetch failed'))
    await expect(
      decideTaskApproval('session', 'approval', { choice: 'deny', idempotency_key: 'key' }),
    ).rejects.toThrow('fetch failed')
    expect(apiRequest).toHaveBeenCalledTimes(1)
  })

  it('reads the historical decision for one exact approval without submitting', async () => {
    vi.mocked(apiRequest).mockResolvedValue({ ...receipt, state: 'uncertain' })
    expect(await getTaskApprovalDecision('session/1', 'approval/2')).toEqual({
      ...receipt,
      state: 'uncertain',
    })
    expect(apiRequest).toHaveBeenCalledExactlyOnceWith(
      '/api/v1/sessions/session%2F1/approvals/approval%2F2/decision',
    )
  })

  it('treats only a real 404 as the absence of a historical command', async () => {
    vi.mocked(apiRequest).mockRejectedValueOnce(new ApiError(404, 'no command'))
    expect(await getTaskApprovalDecision('session', 'approval')).toBeNull()
    for (const error of [
      new ApiError(403, 'denied'),
      new ApiError(503, 'offline'),
      new TypeError('network'),
    ]) {
      vi.mocked(apiRequest).mockRejectedValueOnce(error)
      await expect(getTaskApprovalDecision('session', 'approval')).rejects.toBe(error)
    }
  })
})
