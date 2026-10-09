import { useId } from 'react'
import { useQuery } from '@tanstack/react-query'
import { RefreshCw } from 'lucide-react'
import { Button } from '@sdlc/ui/ui'
import { listRuntimeControls, type RuntimeControlReceipt } from '@/api/runtime-controls'
import { formatDate } from '../common'

const stateLabels: Record<RuntimeControlReceipt['state'], string> = {
  reserved: 'Команда зарезервирована. Отправка не подтверждена.',
  submitted: 'Обработка команды начата. Доставка не подтверждена.',
  acknowledged: 'Команда принята runtime.',
  uncertain: 'Исход команды неизвестен. Повторная отправка не разрешена.',
  rejected: 'Команда не отправлена.',
  terminal_observed: 'Запуск завершён. Доставка команды не подтверждена.',
}

function label(receipt: RuntimeControlReceipt) {
  if (receipt.state === 'acknowledged' && receipt.operation === 'stop') {
    return receipt.acknowledgement === 'already_terminal'
      ? 'Runtime сообщает о завершённом запуске. Остановка процесса не подтверждена.'
      : 'Запрос остановки принят. Завершение запуска и остановка процесса не подтверждены.'
  }
  return stateLabels[receipt.state]
}

export function RuntimeControlsPanel({
  sessionId,
  runId,
}: {
  sessionId: string
  runId?: string | null
}) {
  const titleId = useId()
  const controls = useQuery({
    queryKey: ['runtime-controls', sessionId, runId],
    queryFn: () => listRuntimeControls(sessionId, runId!),
    enabled: Boolean(runId),
    retry: false,
    refetchInterval: 5000,
  })
  if (!runId || (controls.isSuccess && controls.data.length === 0)) return null
  const wrongTarget = controls.data?.some(
    (receipt) => receipt.session_id !== sessionId || receipt.session_run_id !== runId,
  )
  return (
    <section aria-labelledby={titleId} className="min-w-0 border-t border-border py-3">
      <header className="flex min-w-0 items-center justify-between gap-2">
        <h2 id={titleId} className="text-sm font-semibold text-text-primary">
          Последние команды запуска
          <span className="sr-only"> {runId}</span>
        </h2>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          title="Проверить состояние команд"
          aria-label="Проверить состояние команд"
          disabled={controls.isFetching}
          onClick={() => void controls.refetch()}
        >
          <RefreshCw className="h-4 w-4" aria-hidden />
        </Button>
      </header>
      <p className="break-words text-xs text-text-muted [overflow-wrap:anywhere]">Запуск {runId}</p>
      {controls.isPending ? (
        <p role="status" className="text-sm text-text-muted">
          Загрузка состояния команд…
        </p>
      ) : controls.isError || wrongTarget ? (
        <p role="alert" className="text-sm text-text-muted">
          Состояние команд недоступно. Это не подтверждает доставку или завершение запуска.
        </p>
      ) : (
        <ul className="min-w-0 divide-y divide-border">
          {controls.data.map((receipt) => (
            <li key={receipt.id} className="min-w-0 space-y-1 py-2 text-sm">
              <div className="flex min-w-0 flex-wrap items-center gap-2">
                <strong>{receipt.operation === 'stop' ? 'Остановка' : 'Уточнение запуску'}</strong>
                <time className="text-xs text-text-muted" dateTime={receipt.updated_at}>
                  {formatDate(receipt.updated_at)}
                </time>
              </div>
              <p className="break-words [overflow-wrap:anywhere]">{label(receipt)}</p>
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}
