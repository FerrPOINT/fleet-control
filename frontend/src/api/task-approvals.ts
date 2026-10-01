import { apiRequest, jsonBody } from './client'
import type { components } from './generated'
import { ApiError } from '@sdlc/ui/lib'

export type RuntimeApprovalRequest = components['schemas']['RuntimeApprovalRequest']

export type ApprovalDecisionRequest = components['schemas']['ApprovalDecisionRequest']
export type ApprovalDecision = components['schemas']['ApprovalDecision']

const path = (sessionId: string) => `/api/v1/sessions/${encodeURIComponent(sessionId)}/approvals`

export const getTaskApprovals = (sessionId: string) =>
  apiRequest<RuntimeApprovalRequest[]>(path(sessionId))

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
