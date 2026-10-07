import { apiRequest } from './client'
import type { operations } from './generated'
import type { RuntimeControlReceipt } from './types'

export type RuntimeControlLookupQuery = operations['lookup_control']['parameters']['query']
export type OriginalRuntimeControl =
  { operation: 'steer'; input: string } | { operation: 'stop'; input: null }

// Rust str::trim uses Unicode White_Space; JS trim treats U+0085/U+FEFF differently.
export function trimRuntimeControlInput(input: string) {
  return input.replace(
    // eslint-disable-next-line no-control-regex -- Exact published Rust White_Space includes ASCII control spaces.
    /^[\u0009-\u000d\u0020\u0085\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000]+|[\u0009-\u000d\u0020\u0085\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000]+$/gu,
    '',
  )
}

export async function runtimeControlPayloadSha256(command: OriginalRuntimeControl) {
  const input = command.operation === 'stop' ? null : trimRuntimeControlInput(command.input)
  const bytes = new TextEncoder().encode(JSON.stringify({ input, operation: command.operation }))
  const hash = await crypto.subtle.digest('SHA-256', bytes)
  return Array.from(new Uint8Array(hash), (byte) => byte.toString(16).padStart(2, '0')).join('')
}

export async function lookupRuntimeControl(
  sessionId: string,
  runId: string,
  key: string,
  command: OriginalRuntimeControl,
) {
  return lookupRuntimeControlByDigest(sessionId, runId, key, {
    operation: command.operation,
    payload_sha256: await runtimeControlPayloadSha256(command),
  })
}

export function lookupRuntimeControlByDigest(
  sessionId: string,
  runId: string,
  key: string,
  query: RuntimeControlLookupQuery,
) {
  return apiRequest<RuntimeControlReceipt>(
    `/api/v1/sessions/${encodeURIComponent(sessionId)}/runs/${encodeURIComponent(runId)}/controls/lookup?${new URLSearchParams({ operation: query.operation, payload_sha256: query.payload_sha256 })}`,
    { method: 'GET', headers: { 'Idempotency-Key': key } },
  )
}
