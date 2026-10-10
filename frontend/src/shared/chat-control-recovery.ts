// Tab-scoped reconciliation metadata, never authorization or a prompt backup.
export type ControlRecoveryRecord = {
  version: 1
  service: string
  sessionId: string
  runId: string
  agentId: string
  actorId: string
  key: string
  operation: 'steer' | 'stop'
  payloadSha256: string
}
export type ControlRecovery =
  { state: 'none' } | { state: 'pending'; command: ControlRecoveryRecord } | { state: 'blocked' }

export class ControlPreparationError extends Error {}
const fields = [
  'version',
  'service',
  'sessionId',
  'runId',
  'agentId',
  'actorId',
  'key',
  'operation',
  'payloadSha256',
]
const storageKey = (sessionId: string) => `fleet-control.control-recovery.v1:${sessionId}`

export function controlRecoveryService(apiBase: string, issuer: string) {
  const publicRoot = (url: string) => {
    const parsed = new URL(url || '/', window.location.origin)
    return parsed.origin + parsed.pathname.replace(/\/$/, '')
  }
  return JSON.stringify({
    service: 'fleet-control',
    origin: window.location.origin,
    api: publicRoot(apiBase),
    issuer: publicRoot(issuer),
  })
}

function validRecord(value: unknown, sessionId: string): value is ControlRecoveryRecord {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false
  const record = value as Record<string, unknown>
  return (
    Object.keys(record).length === fields.length &&
    fields.every((field) => Object.hasOwn(record, field)) &&
    record.version === 1 &&
    record.sessionId === sessionId &&
    typeof record.service === 'string' &&
    record.service.length > 0 &&
    record.service.length <= 1024 &&
    ['sessionId', 'runId', 'agentId', 'actorId', 'key'].every(
      (field) => typeof record[field] === 'string' && /^[\w.:-]{1,128}$/.test(record[field]),
    ) &&
    (record.operation === 'steer' || record.operation === 'stop') &&
    typeof record.payloadSha256 === 'string' &&
    /^[a-f0-9]{64}$/.test(record.payloadSha256)
  )
}

export function readControlRecovery(sessionId: string): ControlRecovery {
  try {
    const raw = sessionStorage.getItem(storageKey(sessionId))
    if (raw === null) return { state: 'none' }
    if (raw.length > 4096) return { state: 'blocked' }
    const command: unknown = JSON.parse(raw)
    return validRecord(command, sessionId) ? { state: 'pending', command } : { state: 'blocked' }
  } catch {
    return { state: 'blocked' }
  }
}

export function writeControlRecovery(command: ControlRecoveryRecord) {
  if (
    !validRecord(command, command.sessionId) ||
    readControlRecovery(command.sessionId).state !== 'none'
  )
    throw new ControlPreparationError(
      'Исходная команда требует сверки. Новая команда не отправлена.',
    )
  try {
    // Explicit allowlist prevents accidental persistence of input/token fields.
    const safe = Object.fromEntries(
      fields.map((field) => [field, command[field as keyof ControlRecoveryRecord]]),
    )
    sessionStorage.setItem(storageKey(command.sessionId), JSON.stringify(safe))
    const stored = readControlRecovery(command.sessionId)
    if (stored.state !== 'pending' || !sameRecord(stored.command, command)) throw new Error()
  } catch {
    throw new ControlPreparationError('Не удалось сохранить ключ сверки. Команда не отправлена.')
  }
}

function sameRecord(left: ControlRecoveryRecord, right: ControlRecoveryRecord) {
  return fields.every(
    (field) =>
      left[field as keyof ControlRecoveryRecord] === right[field as keyof ControlRecoveryRecord],
  )
}

export function clearControlRecovery(command: ControlRecoveryRecord) {
  try {
    const stored = readControlRecovery(command.sessionId)
    if (stored.state !== 'pending' || !sameRecord(stored.command, command)) return false
    sessionStorage.removeItem(storageKey(command.sessionId))
    return readControlRecovery(command.sessionId).state === 'none'
  } catch {
    return false
  }
}
