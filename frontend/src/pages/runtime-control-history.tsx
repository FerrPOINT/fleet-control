import { useQuery } from '@tanstack/react-query'
import { RefreshCw } from 'lucide-react'
import { Button } from '@sdlc/ui/ui'
import { listRuntimeControls } from '@/api/fleet'
import type { RuntimeControlReceipt } from '@/api/types'
import { UserAvatar } from '@/shared/ui/user-avatar'
import { ErrorState, formatDate } from './common'

const labels: Record<RuntimeControlReceipt['state'], string> = {
  reserved: 'Подготовлена, ещё не отправлена',
  submitted: 'Отправка начата, исход проверяется',
  acknowledged: 'Runtime подтвердил команду',
  uncertain: 'Исход неизвестен, повторная отправка запрещена',
  rejected: 'Не отправлена',
  terminal_observed: 'Запуск завершён; принятие команды неизвестно',
}

export function isUnresolvedControl(command: RuntimeControlReceipt) {
  return ['reserved', 'submitted', 'uncertain'].includes(command.state)
}

export function useRuntimeControls(sessionId: string, runId?: string) {
  return useQuery({
    queryKey: ['runtime-controls', sessionId, runId],
    queryFn: () => listRuntimeControls(sessionId, runId!),
    enabled: Boolean(runId),
    refetchInterval: (query) => (query.state.data?.some(isUnresolvedControl) ? 3000 : false),
  })
}

export function RuntimeControlHistory({ sessionId, runId }: { sessionId: string; runId: string }) {
  const commands = useRuntimeControls(sessionId, runId)
  return (
    <section className="mt-3 grid min-w-0 gap-2" aria-label="Команды запуска">
      <div className="flex items-center justify-between gap-2">
        <h3 className="text-sm font-medium">Команды запуска</h3>
        <Button
          variant="ghost"
          size="icon"
          title="Проверить команды"
          aria-label="Проверить команды"
          disabled={commands.isFetching}
          onClick={() => void commands.refetch()}
        >
          <RefreshCw size={14} />
        </Button>
      </div>
      {commands.isPending ? (
        <p className="text-xs">Загрузка команд</p>
      ) : commands.isError ? (
        <ErrorState message="Не удалось проверить команды запуска" />
      ) : !commands.data?.length ? (
        <p className="text-xs text-text-muted">Команд пока нет</p>
      ) : (
        <ul className="grid gap-2 text-xs">
          {commands.data.map((command) => (
            <li key={command.id} className="grid min-w-0 gap-1 border-l border-border pl-2">
              <div className="flex flex-wrap items-center gap-2">
                <UserAvatar userId={command.actor_user_id} name={command.actor_user_id} />
                <strong>{command.operation === 'stop' ? 'Остановка' : 'Уточнение'}</strong>
                <time dateTime={command.created_at}>{formatDate(command.created_at)}</time>
              </div>
              <p role={isUnresolvedControl(command) ? 'status' : undefined}>
                {labels[command.state]}
              </p>
              {command.acknowledgement === 'stopping' && <p>Ожидается завершение запуска.</p>}
              <details>
                <summary>Идентификатор команды</summary>
                <p className="break-all">{command.id}</p>
              </details>
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}
