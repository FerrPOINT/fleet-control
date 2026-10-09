import { apiRequest } from './client'
import type { components } from './generated'

export type RuntimeControlReceipt = components['schemas']['RuntimeControlReceipt']

export function listRuntimeControls(sessionId: string, runId: string) {
  return apiRequest<RuntimeControlReceipt[]>(
    `/api/v1/sessions/${encodeURIComponent(sessionId)}/runs/${encodeURIComponent(runId)}/controls`,
  )
}

export function lookupRuntimeControl(sessionId: string, runId: string, key: string) {
  return apiRequest<RuntimeControlReceipt>(
    `/api/v1/sessions/${encodeURIComponent(sessionId)}/runs/${encodeURIComponent(runId)}/controls/lookup`,
    { headers: { 'Idempotency-Key': key } },
  )
}
