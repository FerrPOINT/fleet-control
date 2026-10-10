import { useEffect, useRef, useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Button, Label, Textarea } from '@sdlc/ui/ui'
import { getSession, listSessionAgentRuns, steerSessionRun, stopSessionRun } from '@/api/fleet'
import { getChatControls } from '@/api/task-chats'
import { apiBaseUrl } from '@/api/client'
import { isCurrentAuth, ssoConfig, useAuthStore } from '@/shared/auth/store'
import {
  clearControlRecovery,
  controlRecoveryService,
  readControlRecovery,
  writeControlRecovery,
  type ControlRecoveryRecord,
} from '@/shared/chat-control-recovery'
import { dispatchHeld, payloadDigest, unknownOutcome } from './core'

export function ChatRunControls({
  sessionId,
  taskBound,
  onHeldChange,
}: {
  sessionId: string
  taskBound: boolean
  onHeldChange?: (held: boolean) => void
}) {
  const auth = useAuthStore()
  const client = useQueryClient()
  const [input, setInput] = useState('')
  const [sending, setSending] = useState(false)
  const [held, setHeld] = useState(() => readControlRecovery(sessionId).state !== 'none')
  const [notice, setNotice] = useState('')
  const inFlight = useRef(false)
  const live = useRef(true)
  useEffect(() => {
    live.current = true
    return () => {
      live.current = false
    }
  }, [])
  const controls = useQuery({
    queryKey: ['chat-controls', sessionId],
    queryFn: () => getChatControls(sessionId),
    enabled: Boolean(auth.token && auth.userId && !auth.signingOut),
    retry: false,
    refetchInterval: 10_000,
  })
  const allowed = Boolean(
    auth.token &&
    auth.userId &&
    auth.permissions.includes('sessions:write_own') &&
    !auth.signingOut &&
    !sending &&
    !held &&
    !dispatchHeld(sessionId) &&
    controls.isSuccess &&
    !controls.isFetching &&
    controls.data.active_run_id,
  )

  async function submit(operation: 'steer' | 'stop') {
    if (!allowed || inFlight.current || (operation === 'steer' && (!input.trim() || taskBound)))
      return
    inFlight.current = true
    setSending(true)
    setNotice('')
    const scope = useAuthStore.getState()
    let marker: ControlRecoveryRecord | undefined
    let posted = false
    const text = input.trim()
    try {
      const [session, current, runs] = await Promise.all([
        getSession(sessionId),
        getChatControls(sessionId),
        listSessionAgentRuns(sessionId),
      ])
      const run = runs.find((item) => item.id === current.active_run_id)
      if (
        !live.current ||
        !isCurrentAuth(scope) ||
        session.user_id !== scope.userId ||
        !scope.permissions.includes('sessions:write_own') ||
        session.task_bound !== taskBound ||
        dispatchHeld(sessionId) ||
        readControlRecovery(sessionId).state !== 'none' ||
        current.active_run_id !== controls.data?.active_run_id ||
        !run ||
        run.session_id !== sessionId ||
        run.agent_id !== session.primary_agent_id ||
        !run.runtime_run_id ||
        !['pending', 'running', 'waiting', 'stopping'].includes(run.state) ||
        (operation === 'steer' ? !current.can_steer || taskBound : !current.can_stop)
      )
        throw new Error('Доступ или запуск изменился. Команда не отправлена.')
      const body = operation === 'steer' ? { input: text } : {}
      marker = {
        version: 1,
        service: controlRecoveryService(apiBaseUrl, ssoConfig.issuer),
        sessionId,
        runId: run.id,
        agentId: run.agent_id,
        actorId: scope.userId!,
        key: crypto.randomUUID(),
        operation,
        payloadSha256: await payloadDigest(body),
      }
      if (!live.current || !isCurrentAuth(scope))
        throw new Error('Доступ изменился. Команда не отправлена.')
      writeControlRecovery(marker)
      setHeld(true)
      onHeldChange?.(true)
      posted = true
      const receipt =
        operation === 'steer'
          ? await steerSessionRun(sessionId, run.id, { input: text })
          : await stopSessionRun(sessionId, run.id)
      if (!live.current || !isCurrentAuth(scope)) return
      if (
        receipt.session_id !== sessionId ||
        receipt.run_id !== run.id ||
        receipt.runtime_run_id !== run.runtime_run_id ||
        typeof receipt.accepted !== 'boolean'
      )
        throw new Error('Не удалось проверить подтверждение исходной команды.')
      if (!clearControlRecovery(marker)) throw new Error('Исходная команда требует сверки.')
      setHeld(false)
      onHeldChange?.(dispatchHeld(sessionId))
      if (receipt.accepted) {
        setInput('')
        setNotice('Команда принята. Состояние запуска проверяется отдельно.')
      } else setNotice('Запуск отклонил команду.')
      void client.invalidateQueries({ queryKey: ['session-runs', sessionId] })
      void client.invalidateQueries({ queryKey: ['chat-core-runs', sessionId] })
      void controls.refetch()
    } catch (error) {
      if (!live.current || !isCurrentAuth(scope)) return
      if (marker && (!posted || !unknownOutcome(error)) && clearControlRecovery(marker)) {
        setHeld(false)
        onHeldChange?.(dispatchHeld(sessionId))
      }
      setNotice(error instanceof Error ? error.message : 'Исход команды неизвестен.')
    } finally {
      inFlight.current = false
      if (live.current && isCurrentAuth(scope)) setSending(false)
    }
  }

  return (
    <section aria-label="Управление запуском" className="space-y-3">
      {held && !onHeldChange ? (
        <p role="alert">Исходная команда требует сверки. Повтор и новая команда заблокированы.</p>
      ) : null}
      {controls.isError ? <p role="alert">Управление запуском недоступно.</p> : null}
      {!taskBound && controls.data?.can_steer ? (
        <>
          <Label htmlFor={`run-steer-${sessionId}`}>Уточнение активному запуску</Label>
          <Textarea
            id={`run-steer-${sessionId}`}
            value={input}
            maxLength={20000}
            disabled={!allowed}
            onChange={(event) => setInput(event.target.value)}
          />
          <Button
            type="button"
            disabled={!allowed || !input.trim()}
            onClick={() => void submit('steer')}
          >
            Передать уточнение запуску
          </Button>
        </>
      ) : null}
      {controls.data?.can_stop ? (
        <Button
          type="button"
          variant="outline"
          disabled={!allowed}
          onClick={() => void submit('stop')}
        >
          Остановить запуск
        </Button>
      ) : null}
      {notice ? <p role="status">{notice}</p> : null}
    </section>
  )
}
