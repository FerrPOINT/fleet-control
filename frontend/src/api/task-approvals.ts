import { apiRequest, jsonBody } from './client'
import type { components } from './generated'
import { ApiError } from '@sdlc/ui/lib'

export type RuntimeApprovalRequest = components['schemas']['RuntimeApprovalRequest']

export type ApprovalDecisionRequest = components['schemas']['ApprovalDecisionRequest']
export type ApprovalDecision = components['schemas']['ApprovalDecision']

const path = (sessionId: string) => `/api/v1/sessions/${encodeURIComponent(sessionId)}/approvals`

export async function getTaskApprovals(sessionId: string) {
  const approvals = await apiRequest<RuntimeApprovalRequest[]>(path(sessionId))
  if (
    !Array.isArray(approvals) ||
    approvals.some(
      (approval) =>
        !approval ||
        typeof approval !== 'object' ||
        [
          'id',
          'session_id',
          'session_run_id',
          'agent_id',
          'runtime_run_id',
          'prompt',
          'created_at',
        ].some((field) => typeof approval[field as keyof RuntimeApprovalRequest] !== 'string') ||
        !Number.isFinite(Date.parse(approval.created_at)) ||
        (approval.runtime_approval_id != null &&
          typeof approval.runtime_approval_id !== 'string') ||
        !['pending', 'approved', 'denied', 'cancelled'].includes(approval.state),
    )
  )
    throw new ApiError(502, 'Invalid runtime approval response')
  return approvals
}

export async function getTaskApprovalDecision(sessionId: string, approvalId: string) {
  try {
    return await apiRequest<ApprovalDecision>(
      `${path(sessionId)}/${encodeURIComponent(approvalId)}/decision`,
    )
  } catch (error) {
    if (error instanceof ApiError && error.status === 404) return null
    throw error
  }
}

export const decideTaskApproval = (
  sessionId: string,
  approvalId: string,
  request: ApprovalDecisionRequest,
) =>
  apiRequest<ApprovalDecision>(
    `${path(sessionId)}/${encodeURIComponent(approvalId)}/decision`,
    jsonBody({ choice: request.choice, idempotency_key: request.idempotency_key }),
  )
