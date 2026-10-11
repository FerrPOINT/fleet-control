import { ApiError } from '@sdlc/ui/lib'
import type {
  AgentSession,
  CreateSessionMessageRequest,
  SessionAgentRun,
  SessionMessage,
} from '@/api/types'

export function unknownOutcome(error: unknown) {
  return (
    !(error instanceof ApiError) ||
    error.status < 400 ||
    error.status === 408 ||
    error.status >= 500
  )
}

export function chatBackTo(value: string | null) {
  if (!value) return '/chats'
  try {
    const url = new URL(value, 'https://fleet.invalid')
    return url.origin === 'https://fleet.invalid' && url.pathname === '/chats'
      ? url.pathname + url.search
      : '/chats'
  } catch {
    return '/chats'
  }
}

export function chatActivity(
  session: AgentSession,
  messages: SessionMessage[],
  runs: SessionAgentRun[],
) {
  const relevantRuns = runs.filter(
    (run) => run.session_id === session.id && run.agent_id === session.primary_agent_id,
  )
  return {
    // Preserve the server's (created_at, id) ordering; do not order by client clocks.
    messages: messages.filter(
      (message) =>
        message.session_id === session.id &&
        (message.author_type !== 'agent' || message.author_agent_id === session.primary_agent_id),
    ),
    runs: relevantRuns,
    busy:
      relevantRuns.some((run) =>
        ['pending', 'running', 'waiting', 'stopping'].includes(run.state),
      ) ||
      messages.some(
        (message) =>
          message.session_id === session.id &&
          ['pending', 'dispatched'].includes(message.delivery_state),
      ),
  }
}

// A tab-scoped dispatch marker. Private text and credentials stay only in memory.
export type DispatchMarker = {
  actor: string
  agent: string
  service: string
  key: string
  digest: string
}
export function commandService(api: string, issuer: string) {
  const root = (value: string) => {
    const url = new URL(value || '/', window.location.origin)
    return url.origin + url.pathname.replace(/\/$/, '')
  }
  return JSON.stringify([window.location.origin, root(api), root(issuer)])
}
const markerKey = (scope: string) => `fleet-control.chat-dispatch.v1:${scope}`
export function dispatchHeld(scope: string) {
  try {
    return sessionStorage.getItem(markerKey(scope)) !== null
  } catch {
    return true
  }
}
export function markDispatch(scope: string, marker: DispatchMarker) {
  if (dispatchHeld(scope)) throw new Error('Original dispatch requires reconciliation')
  const { actor, agent, service, key, digest } = marker
  const raw = JSON.stringify({ actor, agent, service, key, digest })
  sessionStorage.setItem(markerKey(scope), raw)
  if (sessionStorage.getItem(markerKey(scope)) !== raw)
    throw new Error('Dispatch metadata unavailable')
}
export function clearDispatch(scope: string, marker: DispatchMarker) {
  try {
    if (sessionStorage.getItem(markerKey(scope)) !== JSON.stringify(marker)) return false
    sessionStorage.removeItem(markerKey(scope))
    return !dispatchHeld(scope)
  } catch {
    return false
  }
}
export function legacyControlHeld(sessionId: string) {
  try {
    return sessionStorage.getItem(`fleet-control.control-recovery.v1:${sessionId}`) !== null
  } catch {
    return true
  }
}
export function chatMessageRequest(body: string, key: string): CreateSessionMessageRequest {
  // Match the stored serde_json request digest: sorted keys and explicit nulls.
  return {
    author_agent_id: null,
    body,
    idempotency_key: key,
    message_kind: null,
    runtime_message_id: null,
  }
}
export async function payloadDigest(value: unknown) {
  const digest = await crypto.subtle.digest(
    'SHA-256',
    new TextEncoder().encode(JSON.stringify(value)),
  )
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, '0')).join('')
}
