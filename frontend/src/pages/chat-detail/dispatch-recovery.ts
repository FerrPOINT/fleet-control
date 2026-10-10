import { useEffect, useRef, useState } from 'react'
import { ApiError } from '@sdlc/ui/lib'
import { isCurrentAuth, useAuthStore, type AuthScope } from '@/shared/auth/store'
import { ControlPreparationError } from '@/shared/chat-control-recovery'

// Reuses PR59's tab-scoped digest marker; private payloads remain in memory.
type Marker = { actor: string; agent: string; service: string; key: string; digest: string }
const storageKey = (scope: string) => `fleet-control.chat-dispatch.v1:${scope}`
function stored(scope: string) {
  try {
    return sessionStorage.getItem(storageKey(scope))
  } catch {
    return 'unavailable'
  }
}
function clear(scope: string, marker: Marker) {
  try {
    if (stored(scope) !== JSON.stringify(marker)) return false
    sessionStorage.removeItem(storageKey(scope))
    return stored(scope) === null
  } catch {
    return false
  }
}
export function useDispatchRecovery(scope: string) {
  const live = useRef(true)
  useEffect(() => {
    live.current = true
    return () => {
      live.current = false
    }
  }, [])
  const [held, setHeld] = useState(() => stored(scope) !== null)
  const original = useRef<{ marker: Marker; auth: AuthScope; uncertain: boolean }>(undefined)
  const restored = held && !original.current
  const matches = () => live.current && original.current && isCurrentAuth(original.current.auth)
  return {
    held,
    restored,
    async prepare(
      payload: unknown,
      key: string,
      agent: string,
      service: string,
      allowed: () => boolean,
    ) {
      const auth = useAuthStore.getState()
      const bytes = await crypto.subtle.digest(
        'SHA-256',
        new TextEncoder().encode(JSON.stringify(payload)),
      )
      const digest = Array.from(new Uint8Array(bytes), (byte) =>
        byte.toString(16).padStart(2, '0'),
      ).join('')
      if (!live.current || !allowed() || !auth.userId || !auth.token || !isCurrentAuth(auth))
        throw new ControlPreparationError('Доступ изменился. Команда не отправлена.')
      const marker = { actor: auth.userId, agent, service, key, digest }
      const raw = JSON.stringify(marker)
      if (original.current) {
        if (JSON.stringify(original.current.marker) !== raw || stored(scope) !== raw)
          throw new ControlPreparationError(
            'Нужна сверка исходной команды. Новый запрос не отправлен.',
          )
        original.current.auth = auth
        return
      }
      if (stored(scope) !== null)
        throw new ControlPreparationError('После перезагрузки нужен результат исходной команды.')
      try {
        sessionStorage.setItem(storageKey(scope), raw)
        if (stored(scope) !== raw) throw new Error()
      } catch {
        setHeld(true)
        throw new ControlPreparationError('Не удалось сохранить metadata. Команда не отправлена.')
      }
      original.current = { marker, auth, uncertain: false }
      setHeld(true)
    },
    finish(allowed = true) {
      if (!allowed || !matches() || !original.current || !clear(scope, original.current.marker)) {
        if (original.current) original.current.uncertain = true
        return false
      }
      original.current = undefined
      setHeld(false)
      return true
    },
    async finishRecovered(payload: unknown, key: string, agent: string, service: string) {
      const auth = useAuthStore.getState()
      const bytes = await crypto.subtle.digest(
        'SHA-256',
        new TextEncoder().encode(JSON.stringify(payload)),
      )
      const digest = Array.from(new Uint8Array(bytes), (byte) =>
        byte.toString(16).padStart(2, '0'),
      ).join('')
      if (!live.current || !auth.userId || !auth.token || !isCurrentAuth(auth)) return false
      if (!clear(scope, { actor: auth.userId, agent, service, key, digest })) return false
      original.current = undefined
      setHeld(false)
      return true
    },
    fail(error: unknown) {
      if (!original.current || error instanceof ControlPreparationError) return
      if (
        !matches() ||
        !(error instanceof ApiError) ||
        error.status < 400 ||
        error.status === 408 ||
        error.status >= 500
      )
        original.current.uncertain = true
      if (!original.current.uncertain && matches() && clear(scope, original.current.marker)) {
        original.current = undefined
        setHeld(false)
      }
    },
  }
}
