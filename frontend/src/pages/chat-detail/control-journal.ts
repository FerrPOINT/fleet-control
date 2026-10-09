import { useState } from 'react'
import type { RuntimeControlReceipt } from '@/api/runtime-controls'

export type ControlHandle = {
  operation: RuntimeControlReceipt['operation']
  runId: string
  key: string
}
type Entries = Partial<Record<ControlHandle['operation'], ControlHandle>>
const validRef = (value: unknown): value is string =>
  typeof value === 'string' && /^[A-Za-z0-9._:-]{1,128}$/.test(value)
const storageKey = (actorId: string, sessionId: string) =>
  `fleet-runtime-controls:v1:${encodeURIComponent(actorId)}:${encodeURIComponent(sessionId)}`

export function readControlJournal(actorId: string, sessionId: string): Entries {
  const raw = sessionStorage.getItem(storageKey(actorId, sessionId))
  if (!raw) return {}
  if (raw.length > 2048) throw new Error('Invalid control journal')
  const value: unknown = JSON.parse(raw)
  if (!value || typeof value !== 'object' || Array.isArray(value))
    throw new Error('Invalid control journal')
  const entries: Entries = {}
  for (const [operation, handle] of Object.entries(value)) {
    if (
      (operation !== 'stop' && operation !== 'steer') ||
      !handle ||
      typeof handle !== 'object' ||
      Array.isArray(handle) ||
      Object.keys(handle).sort().join(',') !== 'key,operation,runId' ||
      handle.operation !== operation ||
      !validRef(handle.runId) ||
      !validRef(handle.key)
    )
      throw new Error('Invalid control journal')
    entries[operation] = { operation, runId: handle.runId, key: handle.key }
  }
  return entries
}

export function saveControlHandle(actorId: string, sessionId: string, handle: ControlHandle) {
  if (
    !validRef(actorId) ||
    !validRef(sessionId) ||
    !validRef(handle.runId) ||
    !validRef(handle.key)
  )
    throw new Error('Invalid control identity')
  const entries = readControlJournal(actorId, sessionId)
  const prior = entries[handle.operation]
  if (prior && (prior.runId !== handle.runId || prior.key !== handle.key))
    throw new Error('An original control still requires readback')
  const next = {
    ...entries,
    [handle.operation]: {
      operation: handle.operation,
      runId: handle.runId,
      key: handle.key,
    },
  }
  sessionStorage.setItem(storageKey(actorId, sessionId), JSON.stringify(next))
  return next
}

export function clearControlHandle(actorId: string, sessionId: string, handle: ControlHandle) {
  const entries = readControlJournal(actorId, sessionId)
  const prior = entries[handle.operation]
  if (prior && (prior.runId !== handle.runId || prior.key !== handle.key))
    throw new Error('Original control identity changed')
  delete entries[handle.operation]
  sessionStorage.setItem(storageKey(actorId, sessionId), JSON.stringify(entries))
  return entries
}

export function useControlJournal(actorId: string | null, sessionId: string) {
  const [state, setState] = useState(() => {
    try {
      return { entries: actorId ? readControlJournal(actorId, sessionId) : {}, error: false }
    } catch {
      return { entries: {} as Entries, error: true }
    }
  })
  const update = (handle: ControlHandle, clear: boolean) => {
    if (!actorId || state.error) throw new Error('Сверка исходной команды недоступна')
    try {
      const entries = clear
        ? clearControlHandle(actorId, sessionId, handle)
        : saveControlHandle(actorId, sessionId, handle)
      setState({ entries, error: false })
    } catch {
      setState((current) => ({ ...current, error: true }))
      throw new Error('Не удалось сохранить исходную команду. Повторная отправка заблокирована.')
    }
  }
  return {
    ...state,
    save: (handle: ControlHandle) => update(handle, false),
    clear: (handle: ControlHandle) => update(handle, true),
  }
}
