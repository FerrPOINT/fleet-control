import { useId } from 'react'
import { useQuery } from '@tanstack/react-query'
import { RefreshCw } from 'lucide-react'
import { Button } from '@sdlc/ui/ui'
import { lookupRuntimeControl, type RuntimeControlReceipt } from '@/api/runtime-controls'
import type { ControlHandle } from './control-journal'

export function ControlRecovery({
  actorId,
  sessionId,
  handle,
  onSettled,
}: {
  actorId: string
  sessionId: string
  handle: ControlHandle
  onSettled: (receipt: RuntimeControlReceipt) => void
}) {
  const titleId = useId()
  const query = useQuery({
    queryKey: ['runtime-control-recovery', sessionId, actorId, handle.runId, handle.key],
    queryFn: () => lookupRuntimeControl(sessionId, handle.runId, handle.key),
    retry: false,
    refetchInterval: 5000,
  })
  const receipt = query.data
  const exact =
    receipt &&
    typeof receipt.id === 'string' &&
    Boolean(receipt.id.trim()) &&
    receipt.actor_user_id === actorId &&
    receipt.session_id === sessionId &&
    receipt.session_run_id === handle.runId &&
    receipt.operation === handle.operation
  const settled =
    !query.isError &&
    !query.isFetching &&
    exact &&
    ['acknowledged', 'rejected', 'terminal_observed'].includes(receipt.state)
  return (
    <section aria-labelledby={titleId} className="min-w-0 border-t border-border py-3 text-sm">
      <header className="flex min-w-0 items-center justify-between gap-2">
        <h2 id={titleId} className="font-semibold">
          Сверка исходной команды: {handle.operation === 'stop' ? 'остановка' : 'уточнение'}
        </h2>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          aria-label={`Сверить исходную команду ${handle.operation}`}
          title="Сверить исходную команду"
          disabled={query.isFetching}
          onClick={() => void query.refetch()}
        >
          <RefreshCw className="h-4 w-4" aria-hidden />
        </Button>
      </header>
      <p className="break-words text-xs text-text-muted [overflow-wrap:anywhere]">
        Запуск {handle.runId}
      </p>
      <p role="status" className="my-2 break-words">
        {query.isPending
          ? 'Проверка исходной команды…'
          : query.isError || !exact
            ? 'Доставка не подтверждена. Отсутствие квитанции не разрешает новую отправку.'
            : receipt.state === 'acknowledged'
              ? 'Команда принята runtime. Это не подтверждает завершение или остановку процесса.'
              : receipt.state === 'rejected'
                ? 'Команда не была отправлена.'
                : receipt.state === 'terminal_observed'
                  ? 'Запуск завершён. Доставка команды не подтверждена.'
                  : 'Исход команды ещё не подтверждён. Новая отправка заблокирована.'}
      </p>
      {settled && (
        <Button type="button" variant="outline" onClick={() => onSettled(receipt)}>
          Закрыть сверку {handle.operation === 'stop' ? 'остановки' : 'уточнения'}
        </Button>
      )}
    </section>
  )
}
