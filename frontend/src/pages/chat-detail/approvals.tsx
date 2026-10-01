import { useId, useRef } from 'react'
import {
  useIsMutating,
  useMutation,
  useMutationState,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query'
import { Check, RefreshCw, ShieldCheck, X } from 'lucide-react'
import { Button } from '@sdlc/ui/ui'
import {
  decideTaskApproval,
  getTaskApprovalDecision,
  getTaskApprovals,
  type ApprovalDecision,
  type ApprovalDecisionRequest,
  type RuntimeApprovalRequest,
} from '@/api/task-approvals'
import { EmptyState, ErrorState } from '../common'

const approvalLabels: Record<RuntimeApprovalRequest['state'], string> = {
  pending: 'Ожидает решения',
  approved: 'Разрешено',
  denied: 'Отклонено',
  cancelled: 'Отменено',
}

const detailLabels: Record<string, string> = {
  action: 'Действие',
  tool: 'Инструмент',
  tool_name: 'Инструмент',
  path: 'Путь',
  command: 'Команда',
  arguments: 'Параметры',
  args: 'Аргументы',
  cwd: 'Рабочая папка',
  working_directory: 'Рабочая папка',
  url: 'Адрес',
  host: 'Хост',
  reason: 'Причина',
  token: 'Токен',
}

function ActionDetail({ value }: { value: unknown }) {
  if (Array.isArray(value))
    return (
      <ul className="min-w-0 list-inside list-disc space-y-1">
        {value.map((item, index) => (
          <li key={index} className="min-w-0">
            <ActionDetail value={item} />
          </li>
        ))}
      </ul>
    )
  if (value && typeof value === 'object')
    return (
      <dl className="grid min-w-0 gap-2">
        {Object.entries(value).map(([key, item]) => (
          <div key={key} className="min-w-0">
            <dt className="break-words text-text-muted">
              {detailLabels[key] ?? key.replaceAll('_', ' ')}
            </dt>
            <dd className="min-w-0 whitespace-pre-wrap break-words [overflow-wrap:anywhere]">
              <ActionDetail value={item} />
            </dd>
          </div>
        ))}
      </dl>
    )
  return (
    <>
      {value == null
        ? 'Нет данных'
        : typeof value === 'boolean'
          ? value
            ? 'Да'
            : 'Нет'
          : String(value)}
    </>
  )
}

function deliveryLabel(state: ApprovalDecision['state'] | 'sending' | undefined) {
  switch (state) {
    case 'sending':
      return 'Отправка решения…'
    case 'pending':
      return 'Решение принято. Ожидается доставка агенту.'
    case 'delivered':
      return 'Решение доставлено. Ожидается обновление состояния.'
    case 'failed':
      return 'Не удалось доставить решение. Требуется сверка состояния.'
    default:
      return 'Доставка решения не подтверждена. Требуется сверка состояния.'
  }
}

export function TaskApprovalsPanel({
  sessionId,
  canResolve,
  hideWhenEmpty = false,
}: {
  sessionId: string
  canResolve: boolean
  hideWhenEmpty?: boolean
}) {
  const titleId = useId()
  const client = useQueryClient()
  const approvals = useQuery({
    queryKey: ['task-approvals', sessionId],
    queryFn: () => getTaskApprovals(sessionId),
    enabled: Boolean(sessionId),
    retry: false,
    refetchInterval: 10000,
  })
  const mutating = useIsMutating({ mutationKey: ['task-approval-decision', sessionId] }) > 0
  const wrongSession = approvals.data?.some((approval) => approval.session_id !== sessionId)
  const unavailable = approvals.isError || wrongSession

  if (hideWhenEmpty && approvals.isSuccess && approvals.data.length === 0) return null

  return (
    <section aria-labelledby={titleId} className="min-w-0 border-t border-border py-4">
      <header className="mb-3 flex min-w-0 flex-wrap items-center justify-between gap-2">
        <h2
          id={titleId}
          className="flex min-w-0 items-center gap-2 text-sm font-semibold text-text-primary"
        >
          <ShieldCheck className="h-4 w-4 shrink-0" aria-hidden />
          <span className="break-words">Разрешения действий агента</span>
        </h2>
        <Button
          variant="ghost"
          size="icon"
          type="button"
          title="Обновить разрешения"
          aria-label="Обновить разрешения"
          disabled={approvals.isFetching || mutating}
          onClick={() =>
            void Promise.allSettled([
              approvals.refetch(),
              client.invalidateQueries({ queryKey: ['task-approval-decision', sessionId] }),
            ])
          }
        >
          <RefreshCw className="h-4 w-4" aria-hidden />
        </Button>
      </header>
      {!canResolve && <p className="mb-3 text-xs text-text-muted">Только просмотр</p>}
      {approvals.isPending ? (
        <p role="status" className="text-sm text-text-muted">
          Загрузка разрешений…
        </p>
      ) : unavailable ? (
        <ErrorState message="Не удалось загрузить разрешения действий агента" />
      ) : !approvals.data?.length ? (
        <EmptyState title="Запросов на разрешение нет" />
      ) : (
        <ul className="min-w-0 divide-y divide-border">
          {approvals.data.map((approval) => (
            <li key={approval.id} className="min-w-0 py-3">
              <ApprovalRow
                approval={approval}
                canResolve={canResolve}
                busy={approvals.isFetching || mutating}
              />
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}

function ApprovalRow({
  approval,
  canResolve,
  busy,
}: {
  approval: RuntimeApprovalRequest
  canResolve: boolean
  busy: boolean
}) {
  const titleId = useId()
  const result = useRef<HTMLDivElement>(null)
  const submitted = useRef(false)
  const client = useQueryClient()
  const mutationKey = ['task-approval-decision', approval.session_id, approval.id]
  const pending = approval.state === 'pending'
  const hasExactTarget = Boolean(approval.runtime_approval_id?.trim())
  const decision = useQuery({
    queryKey: mutationKey,
    queryFn: () => getTaskApprovalDecision(approval.session_id, approval.id),
    enabled: pending,
    retry: false,
    refetchInterval: 10000,
  })
  // Keep an ambiguous attempt through remounts until the server mirror confirms resolution.
  const attempt = useMutationState({
    filters: { mutationKey, exact: true },
    select: (entry) => ({
      status: entry.state.status,
      response: entry.state.data as ApprovalDecision | undefined,
      choice: (entry.state.variables as ApprovalDecisionRequest | undefined)?.choice,
    }),
  }).at(-1)
  const mutation = useMutation({
    mutationKey,
    gcTime: Infinity,
    retry: false,
    mutationFn: (request: ApprovalDecisionRequest) =>
      decideTaskApproval(approval.session_id, approval.id, request),
    onSettled: async () => {
      await Promise.allSettled(
        [
          'task-approvals',
          'task-approval-decision',
          'chat-controls',
          'session-runs',
          'chat-history',
          'session',
        ].map((key) => client.invalidateQueries({ queryKey: [key, approval.session_id] })),
      )
      result.current?.focus({ preventScroll: true })
    },
  })
  const command = decision.data ?? attempt?.response
  const wrongCommand =
    command &&
    (command.session_id !== approval.session_id ||
      command.approval_id !== approval.id ||
      command.session_run_id !== approval.session_run_id)
  const disabled =
    !canResolve ||
    !hasExactTarget ||
    busy ||
    decision.isPending ||
    decision.isFetching ||
    decision.isError ||
    Boolean(command) ||
    Boolean(attempt)
  const choice = command?.choice ?? attempt?.choice
  const delivery =
    attempt?.status === 'pending'
      ? 'sending'
      : (command?.state ?? (attempt?.status === 'error' ? 'uncertain' : undefined))
  const action =
    approval.detail && typeof approval.detail === 'object' && 'action' in approval.detail
      ? approval.detail.action
      : null
  const submit = (choice: ApprovalDecisionRequest['choice']) => {
    if (
      !pending ||
      disabled ||
      submitted.current ||
      client.isMutating({ mutationKey: ['task-approval-decision', approval.session_id] })
    )
      return
    submitted.current = true
    mutation.mutate({ choice, idempotency_key: crypto.randomUUID() })
  }

  return (
    <article aria-labelledby={titleId} className="min-w-0 space-y-3">
      <h3 id={titleId} className="break-all text-sm font-medium text-text-primary">
        {typeof action === 'string' && action ? action : 'Действие агента'}
      </h3>
      <dl className="grid min-w-0 gap-2 text-xs text-text-secondary">
        <div className="min-w-0">
          <dt className="text-text-muted">Запуск</dt>
          <dd className="break-all font-mono">{approval.session_run_id}</dd>
        </div>
        <div className="min-w-0">
          <dt className="text-text-muted">Runtime run</dt>
          <dd className="break-all font-mono">{approval.runtime_run_id}</dd>
        </div>
        <div className="min-w-0">
          <dt className="text-text-muted">Запрос</dt>
          <dd className="break-all font-mono">{approval.id}</dd>
        </div>
      </dl>
      <p className="whitespace-pre-wrap break-words text-sm text-text-primary [overflow-wrap:anywhere]">
        {approval.prompt}
      </p>
      {approval.detail != null && (
        <div aria-label="Детали действия" className="min-w-0 text-xs text-text-secondary">
          <ActionDetail value={approval.detail} />
        </div>
      )}
      <div
        ref={result}
        role="status"
        aria-label="Результат решения"
        aria-live="polite"
        tabIndex={-1}
        className="space-y-1 text-sm text-text-secondary focus-visible:outline-focus"
      >
        <p>
          {pending && (command || attempt) ? 'Выбор отправлен' : approvalLabels[approval.state]}
        </p>
        {pending && !hasExactTarget && (
          <p>Идентификатор точного запроса агента отсутствует. Решение недоступно.</p>
        )}
        {pending && decision.isPending && <p>Проверка сохранённого решения…</p>}
        {pending && (decision.isError || wrongCommand) && (
          <ErrorState message="Не удалось сверить сохранённое решение" />
        )}
        {pending && !wrongCommand && (attempt || command) && (
          <>
            <p>{choice === 'once' ? 'Разрешить один раз' : 'Запретить'}</p>
            <p>{deliveryLabel(delivery)}</p>
          </>
        )}
      </div>
      {pending && (
        <div className="flex min-w-0 flex-wrap gap-2" aria-busy={mutation.isPending}>
          <Button type="button" disabled={disabled} onClick={() => submit('once')}>
            <Check className="h-4 w-4 shrink-0" aria-hidden />
            Разрешить один раз
          </Button>
          <Button
            type="button"
            variant="outline"
            disabled={disabled}
            onClick={() => submit('deny')}
          >
            <X className="h-4 w-4 shrink-0" aria-hidden />
            Запретить
          </Button>
        </div>
      )}
    </article>
  )
}
