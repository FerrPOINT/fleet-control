import { useEffect, useRef, useState } from 'react'
import { Link, useBlocker, useParams, useSearchParams } from 'react-router'
import {
  useInfiniteQuery,
  useMutation,
  useQuery,
  useQueryClient,
  type QueryClient,
} from '@tanstack/react-query'
import {
  ArrowDown,
  ArrowLeft,
  Bot,
  Check,
  ClipboardList,
  FileText,
  Info,
  MessageSquare,
  RotateCw,
  Send,
  ShieldCheck,
  Square,
} from 'lucide-react'
import {
  Button,
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  Label,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
  Textarea,
} from '@sdlc/ui/ui'
import { connectAuthenticatedEventStream } from '@sdlc/ui/lib'
import { ApiError } from '@sdlc/ui/lib'
import { apiBaseUrl } from '@/api/client'
import {
  createSessionMessage,
  getSession,
  listAgentDirectory,
  listSessionAgentRuns,
  steerSessionRun,
  stopSessionRun,
} from '@/api/fleet'
import {
  answerClarification,
  blockedLabels,
  canSubmitAnswer,
  confirmRequirements,
  getChatControls,
  getChatHistory,
  getClarifications,
  getRequirements,
  getTaskContext,
  type RequirementsRevision,
  type AnswerInput,
} from '@/api/task-chats'
import { useAuthStore } from '@/shared/auth/store'
import { TaskApprovalsPanel } from './approvals'
import { RuntimeControlsPanel } from './runtime-controls'
import { ControlRecovery } from './control-recovery'
import { useControlJournal, type ControlHandle } from './control-journal'
import { UserAvatar } from '@/shared/ui/user-avatar'
import { EmptyState, ErrorState, StatusBadge, formatDate } from '../common'
import './chat.css'

function requestKey() {
  return crypto.randomUUID()
}
const steerByteLimit = 64 * 1024
const exceedsSteerLimit = (input: string) =>
  new TextEncoder().encode(input).byteLength > steerByteLimit
async function refreshHistory(client: QueryClient, id: string) {
  const queryKey = ['chat-history', id]
  const alreadyFetching = client.isFetching({ queryKey, exact: true }) > 0
  await client.invalidateQueries({ queryKey, exact: true }, { cancelRefetch: false })
  // A pending page began before this event: read again after it without cancelling it.
  if (alreadyFetching) {
    await client.invalidateQueries({ queryKey, exact: true }, { cancelRefetch: false })
  }
}
function ReadableError({ error }: { error: unknown }) {
  return <ErrorState message={error instanceof Error ? error.message : 'Данные недоступны'} />
}

export function ChatDetailPage() {
  const { sessionId = '' } = useParams()
  const actorId = useAuthStore((state) => state.userId)
  return <ChatWorkspace key={`${actorId}:${sessionId}`} id={sessionId} />
}

function ChatWorkspace({ id }: { id: string }) {
  const [params, setParams] = useSearchParams()
  const client = useQueryClient()
  const token = useAuthStore((state) => state.token)
  const userId = useAuthStore((state) => state.userId)
  const journal = useControlJournal(userId, id)
  const controlCallbacks = useRef<Partial<Record<ControlHandle['operation'], string>>>({})
  const session = useQuery({ queryKey: ['session', id], queryFn: () => getSession(id) })
  const agents = useQuery({ queryKey: ['agent-directory'], queryFn: listAgentDirectory })
  const task = useQuery({
    queryKey: ['task-context', id],
    queryFn: () => getTaskContext(id),
    refetchInterval: 10000,
  })
  const controls = useQuery({
    queryKey: ['chat-controls', id],
    queryFn: () => getChatControls(id),
    refetchInterval: 10000,
  })
  const runs = useQuery({
    queryKey: ['session-runs', id],
    queryFn: () => listSessionAgentRuns(id),
    refetchInterval: 10000,
  })
  const history = useInfiniteQuery({
    queryKey: ['chat-history', id],
    queryFn: ({ pageParam }) => getChatHistory(id, pageParam),
    initialPageParam: undefined as string | undefined,
    getNextPageParam: (last) => last.next_before ?? undefined,
  })
  const bound = Boolean(task.data?.binding)
  const questions = useQuery({
    queryKey: ['clarifications', id],
    queryFn: () => getClarifications(id),
    enabled: bound,
    refetchInterval: 10000,
  })
  const requirements = useQuery({
    queryKey: ['requirements', id],
    queryFn: () => getRequirements(id),
    enabled: bound,
    refetchInterval: 10000,
  })
  const [contextOpen, setContextOpen] = useState(false)
  const contextTrigger = useRef<HTMLButtonElement>(null)
  const [drafts, setDrafts] = useState<
    Record<string, { selected: string[]; text: string; comment: string; key: string }>
  >({})
  const [body, setBody] = useState('')
  const [messageKey, setMessageKey] = useState(requestKey)
  const [delta, setDelta] = useState<Record<string, string>>({})
  const [receipt, setReceipt] = useState<string | null>(null)
  const transcript = useRef<HTMLDivElement>(null)
  const following = useRef(true)
  const [newMessages, setNewMessages] = useState(false)
  const tab = ['dialogue', 'clarification', 'requirements'].includes(params.get('tab') ?? '')
    ? params.get('tab')!
    : 'dialogue'
  const owner = session.data?.user_id === userId
  const canResolveApprovals =
    useAuthStore((state) => state.permissions.includes('agents:manage')) || owner
  const context = task.data?.tracker
  const questionList = questions.data?.questions ?? []
  const selectedQuestion =
    questionList.find((question) => question.id === params.get('question')) ??
    questionList.find((question) => question.state === 'open') ??
    questionList[0]
  const questionKey = selectedQuestion ? `${selectedQuestion.id}:${selectedQuestion.version}` : ''
  const draft = drafts[questionKey] ?? { selected: [], text: '', comment: '', key: '' }
  const dirty =
    Boolean(body.trim()) ||
    Object.values(drafts).some((value) =>
      Boolean(value.selected.length || value.text.trim() || value.comment.trim()),
    )
  const staleDrafts = Object.entries(drafts).filter(
    ([key]) => selectedQuestion && key.startsWith(`${selectedQuestion.id}:`) && key !== questionKey,
  )
  const blocker = useBlocker(
    ({ currentLocation, nextLocation }) =>
      dirty && currentLocation.pathname !== nextLocation.pathname,
  )
  useEffect(() => {
    const warn = (event: BeforeUnloadEvent) => {
      if (dirty) {
        event.preventDefault()
        event.returnValue = ''
      }
    }
    window.addEventListener('beforeunload', warn)
    return () => window.removeEventListener('beforeunload', warn)
  }, [dirty])
  const invalidate = async () => {
    await Promise.all(
      [
        'session',
        'chat-history',
        'session-runs',
        'runtime-controls',
        'runtime-control-recovery',
        'chat-controls',
        'task-context',
        'clarifications',
        'requirements',
        'task-approvals',
        'task-approval-decision',
      ].map((key) =>
        key === 'chat-history'
          ? refreshHistory(client, id)
          : client.invalidateQueries({ queryKey: [key, id] }),
      ),
    )
  }
  useEffect(() => {
    if (!token) return
    return connectAuthenticatedEventStream({
      url: `${apiBaseUrl}/api/v1/sessions/${id}/stream`,
      token,
      eventTypes: ['session'],
      onOpen: () => {
        void refreshHistory(client, id)
      },
      onEvent: (_type, data) => {
        if (
          data &&
          typeof data === 'object' &&
          'type' in data &&
          data.type === 'session_run_delta' &&
          'run_id' in data &&
          typeof data.run_id === 'string'
        ) {
          const runId = data.run_id
          if ('text' in data && typeof data.text === 'string') {
            const text = data.text
            setDelta((current) => ({ ...current, [runId]: text }))
          } else if ('delta' in data && typeof data.delta === 'string') {
            const text = data.delta
            setDelta((current) => ({ ...current, [runId]: (current[runId] ?? '') + text }))
          }
        } else {
          ;[
            'chat-history',
            'session-runs',
            'runtime-controls',
            'runtime-control-recovery',
            'chat-controls',
            'task-context',
            'clarifications',
            'requirements',
            'task-approvals',
            'task-approval-decision',
          ].forEach((key) => {
            if (key === 'chat-history') void refreshHistory(client, id)
            else void client.invalidateQueries({ queryKey: [key, id] })
          })
        }
      },
    })
  }, [client, id, token])
  const messageMap = new Map(
    [...(history.data?.pages ?? [])]
      .reverse()
      .flatMap((page) => page.items)
      .map((message) => [message.id, message]),
  )
  const messages = [...messageMap.values()]
  const lastMessage = messages.at(-1)?.id
  useEffect(() => {
    if (!transcript.current) return
    if (following.current) transcript.current.scrollTop = transcript.current.scrollHeight
    else if (lastMessage) setNewMessages(true)
  }, [lastMessage, delta])
  const message = useMutation({
    mutationFn: async (
      command:
        | { kind: 'steer'; runId: string; input: string; key: string }
        | { kind: 'prompt'; input: string; key: string },
    ) => {
      if (command.kind === 'steer') {
        if (exceedsSteerLimit(command.input))
          throw new ApiError(400, 'Уточнение не должно превышать 64 КиБ в UTF-8.')
        journal.save({ operation: 'steer', runId: command.runId, key: command.key })
        controlCallbacks.current.steer = command.key
        return steerSessionRun(id, command.runId, { input: command.input }, command.key)
      }
      return createSessionMessage(id, { body: command.input, idempotency_key: command.key })
    },
    onSuccess: async (result, command) => {
      if (command.kind === 'steer' && controlCallbacks.current.steer !== command.key) return
      if ('accepted' in result && !result.accepted) {
        setReceipt('Принятие команды не подтверждено. Текст и ключ команды сохранены.')
        await invalidate()
        return
      }
      if (command.kind === 'steer') {
        if (!('run_id' in result) || result.run_id !== command.runId || result.session_id !== id)
          throw new Error('Ответ runtime не соответствует исходной команде')
        journal.clear({ operation: 'steer', runId: command.runId, key: command.key })
        delete controlCallbacks.current.steer
      }
      setBody('')
      setMessageKey(requestKey())
      setReceipt(
        'delivery_state' in result && result.delivery_state === 'pending'
          ? 'Сообщение сохранено. Доставка ожидается.'
          : 'Команда принята. Выполнение проверяется по статусу.',
      )
      await invalidate()
    },
  })
  const answer = useMutation({
    mutationFn: (command: {
      questionId: string
      questionKey: string
      questionTitle: string
      payload: AnswerInput
    }) => answerClarification(id, command.questionId, command.payload),
    onSuccess: async (_result, command) => {
      setReceipt('Ответ сохранён. Требования ещё не опубликованы.')
      setDrafts((current) => {
        const next = { ...current }
        delete next[command.questionKey]
        return next
      })
      await invalidate()
    },
    onError: () => {
      void questions.refetch()
      void task.refetch()
    },
  })
  const stop = useMutation({
    mutationFn: (command: { runId: string; key: string }) => {
      journal.save({ operation: 'stop', ...command })
      controlCallbacks.current.stop = command.key
      return stopSessionRun(id, command.runId, command.key)
    },
    onSuccess: async (result, command) => {
      if (controlCallbacks.current.stop !== command.key) return
      if (result.accepted) {
        if (result.run_id !== command.runId || result.session_id !== id)
          throw new Error('Ответ runtime не соответствует исходной команде')
        journal.clear({ operation: 'stop', ...command })
        delete controlCallbacks.current.stop
      }
      await invalidate()
    },
  })
  const answerUncertain =
    answer.isError && (!(answer.error instanceof ApiError) || answer.error.status >= 500)
  const messageUncertain =
    (message.isError && (!(message.error instanceof ApiError) || message.error.status >= 500)) ||
    (message.isSuccess && 'accepted' in message.data && !message.data.accepted)
  const stopUnacknowledged =
    Boolean(journal.entries.stop) || (stop.isSuccess && !stop.data.accepted)
  const uncertainSteer =
    Boolean(journal.entries.steer) || (messageUncertain && message.variables?.kind === 'steer')
  const readbackRunIds = [
    ...new Set(
      [
        stop.isPending || stop.isError || stopUnacknowledged ? stop.variables?.runId : null,
        message.variables?.kind === 'steer' && (message.isPending || messageUncertain)
          ? message.variables.runId
          : null,
        controls.data?.active_run_id,
        journal.entries.stop?.runId,
        journal.entries.steer?.runId,
      ].filter((runId): runId is string => Boolean(runId)),
    ),
  ]
  if (!readbackRunIds.length && runs.data?.[0]) readbackRunIds.push(runs.data[0].id)
  const steerTooLarge = Boolean(controls.data?.can_steer) && exceedsSteerLimit(body.trim())
  const canSubmitMessage =
    owner &&
    !controls.isError &&
    !message.isPending &&
    !journal.error &&
    !uncertainSteer &&
    !steerTooLarge &&
    Boolean(body.trim()) &&
    (messageUncertain || controls.data?.can_send || controls.data?.can_steer)
  const submitMessage = () => {
    if (!canSubmitMessage) return
    if (messageUncertain && message.variables) {
      message.mutate(message.variables)
      return
    }
    message.mutate(
      controls.data?.can_steer && controls.data.active_run_id
        ? { kind: 'steer', runId: controls.data.active_run_id, input: body.trim(), key: messageKey }
        : { kind: 'prompt', input: body.trim(), key: messageKey },
    )
  }
  const settleControl = (handle: ControlHandle) => {
    try {
      journal.clear(handle)
    } catch {
      setReceipt('Не удалось закрыть сверку. Новая отправка остаётся заблокированной.')
      return
    }
    delete controlCallbacks.current[handle.operation]
    if (handle.operation === 'steer') {
      setBody('')
      setMessageKey(requestKey())
      message.reset()
    } else stop.reset()
    setReceipt(
      'Сверка закрыта. Состояние выполнения проверяется отдельно; команда не отправляется повторно.',
    )
    void invalidate()
  }
  const updateDraft = (change: Partial<typeof draft>) =>
    setDrafts((current) => ({
      ...current,
      [questionKey]: {
        ...(current[questionKey] ?? { selected: [], text: '', comment: '', key: requestKey() }),
        key: requestKey(),
        ...change,
      },
    }))
  const switchTab = (value: string) =>
    setParams((current) => {
      const next = new URLSearchParams(current)
      next.set('tab', value)
      return next
    })
  if (session.isPending) return <EmptyState title="Загрузка чата" />
  if (session.isError || !session.data) return <ReadableError error={session.error} />
  const agent = agents.data?.find((item) => item.id === session.data.primary_agent_id)
  const activeRun = runs.data?.find((run) => run.id === controls.data?.active_run_id)
  const waiting = context?.waiting_reason
  const returnTo = params.get('returnTo')
  const backTo =
    returnTo === '/chats' || returnTo?.startsWith('/chats?')
      ? returnTo
      : `/chats${params.get('agent') ? `?agent=${encodeURIComponent(params.get('agent')!)}` : ''}`
  const contextContent = (
    <>
      <h2>Контекст задачи</h2>
      <dl className="fc-chat-fields">
        <div>
          <dt>Задача</dt>
          <dd>{task.data?.binding?.task_id ?? 'Свободный чат'}</dd>
        </div>
        <div>
          <dt>Владелец</dt>
          <dd>
            <UserAvatar userId={session.data.user_id} name={session.data.user_display_name} />
            {session.data.user_display_name}
          </dd>
        </div>
        <div>
          <dt>Требования</dt>
          <dd>
            {context?.requirement_revision
              ? `Редакция ${context.requirement_revision}`
              : 'Пока нет редакции'}
          </dd>
        </div>
        <div>
          <dt>Выполнение</dt>
          <dd>{activeRun ? <StatusBadge value={activeRun.state} /> : 'Нет активного запуска'}</dd>
        </div>
        <div>
          <dt>Execution</dt>
          <dd>{context?.assignment?.execution_id ?? 'Не назначен'}</dd>
        </div>
        <div>
          <dt>Checkpoint</dt>
          <dd>Нет подтверждённых данных</dd>
        </div>
      </dl>
      <h3>Уточнения</h3>
      <p>
        {questions.isError
          ? 'Источник недоступен'
          : bound
            ? `${questionList.filter((question) => question.state === 'answered').length} / ${questionList.length}`
            : 'Нет привязки к SDLC'}
      </p>
      <h3>Запуски</h3>
      {runs.isError ? (
        <ReadableError error={runs.error} />
      ) : (
        (runs.data ?? []).map((run) => (
          <details key={run.id}>
            <summary>
              {formatDate(run.created_at)} · <StatusBadge value={run.state} />
            </summary>
            <dl>
              <dt>Run</dt>
              <dd>{run.id}</dd>
              <dt>Runtime</dt>
              <dd>{run.runtime_run_id ?? 'Не подтверждён'}</dd>
              <dt>Модель</dt>
              <dd>
                {run.model ?? 'Неизвестно'} · {run.provider ?? 'Неизвестно'}
              </dd>
            </dl>
            {run.last_error && <p role="alert">{run.last_error}</p>}
          </details>
        ))
      )}
    </>
  )
  return (
    <section className="fc-chat-workbench">
      <header className="fc-chat-header">
        <div>
          <Link to={backTo} aria-label="Вернуться к чатам">
            <ArrowLeft size={18} />
          </Link>
          <span>{session.data.task_key}</span>
          <h1>{session.data.title}</h1>
          <StatusBadge
            value={session.data.visibility === 'private' ? 'private' : 'leader_scoped'}
          />
        </div>
        <div>
          {context?.stage && <StatusBadge value={context.stage} />}
          {waiting && <span className="fc-chat-wait">{waiting}</span>}
          <Button
            ref={contextTrigger}
            variant="ghost"
            className="fc-chat-context-button"
            aria-label="Контекст задачи"
            title="Контекст задачи"
            onClick={() => setContextOpen(true)}
          >
            <Info size={18} />
          </Button>
        </div>
      </header>
      <div className="fc-chat-agent">
        <Bot size={16} />
        <strong>{agent?.display_name ?? session.data.primary_agent_name}</strong>
        <span>{agent?.kind ?? 'Runtime неизвестен'}</span>
        <StatusBadge value={agent?.status} />
        {controls.data?.blocked_reason && (
          <span>{blockedLabels[controls.data.blocked_reason] ?? controls.data.blocked_reason}</span>
        )}
      </div>
      <div className="fc-chat-grid">
        <div className="fc-chat-main">
          <Tabs value={tab} onValueChange={switchTab} className="fc-chat-tabs">
            <TabsList className="fc-chat-tab-list" aria-label="Разделы задачи">
              <TabsTrigger value="dialogue">
                <MessageSquare size={15} />
                Диалог
              </TabsTrigger>
              <TabsTrigger value="clarification">
                <ClipboardList size={15} />
                Уточнения
              </TabsTrigger>
              <TabsTrigger value="requirements">
                <FileText size={15} />
                Требования
              </TabsTrigger>
            </TabsList>
            {task.isError && <ReadableError error={task.error} />}
            {receipt && (
              <p role="status" className="fc-chat-notice">
                {receipt}
              </p>
            )}
            <TabsContent value="dialogue" className="fc-chat-panel">
              <div
                ref={transcript}
                className="fc-chat-scroll"
                onScroll={() => {
                  const node = transcript.current
                  if (node)
                    following.current = node.scrollHeight - node.scrollTop - node.clientHeight < 48
                }}
              >
                {history.isError && <ReadableError error={history.error} />}
                {history.isPending && <p>Загрузка истории</p>}
                {history.hasNextPage && (
                  <Button
                    variant="outline"
                    disabled={history.isFetchingNextPage}
                    onClick={() => {
                      const node = transcript.current
                      const before = node?.scrollHeight ?? 0
                      void history.fetchNextPage().then(() => {
                        requestAnimationFrame(() => {
                          if (node) node.scrollTop += node.scrollHeight - before
                        })
                      })
                    }}
                  >
                    Предыдущие сообщения
                  </Button>
                )}
                {!history.isPending && !messages.length && (
                  <EmptyState title="Сообщений пока нет" />
                )}
                {messages.map((item) => (
                  <article className="fc-chat-message" key={item.id}>
                    <div>
                      <UserAvatar
                        userId={item.author_user_id ?? item.author_agent_id ?? 'system'}
                        name={item.author_display_name}
                      />
                      <strong>{item.author_display_name}</strong>
                      <time>{formatDate(item.created_at)}</time>
                      {item.author_type === 'user' && <StatusBadge value={item.delivery_state} />}
                    </div>
                    <p>{item.body}</p>
                    {item.delivery_error && <p role="alert">{item.delivery_error}</p>}
                  </article>
                ))}
                {activeRun && delta[activeRun.id] && (
                  <article className="fc-chat-message">
                    <strong>{activeRun.agent_name}</strong>
                    <p>{delta[activeRun.id]}</p>
                  </article>
                )}
                <TaskApprovalsPanel sessionId={id} canResolve={canResolveApprovals} hideWhenEmpty />
                {readbackRunIds.map((runId) => (
                  <RuntimeControlsPanel key={runId} sessionId={id} runId={runId} />
                ))}
                {userId &&
                  Object.values(journal.entries).map((handle) => (
                    <ControlRecovery
                      key={handle.key}
                      actorId={userId}
                      sessionId={id}
                      handle={handle}
                      onSettled={() => settleControl(handle)}
                    />
                  ))}
              </div>
              {newMessages && (
                <Button
                  variant="outline"
                  onClick={() => {
                    following.current = true
                    setNewMessages(false)
                    if (transcript.current)
                      transcript.current.scrollTop = transcript.current.scrollHeight
                  }}
                >
                  <ArrowDown size={15} />
                  Новые сообщения
                </Button>
              )}
              {bound && questionList.some((question) => question.state === 'open') && (
                <Button variant="outline" onClick={() => switchTab('clarification')}>
                  <ClipboardList size={15} />
                  Ответить на уточнение
                </Button>
              )}
              <form
                className="fc-chat-composer"
                onSubmit={(event) => {
                  event.preventDefault()
                  submitMessage()
                }}
              >
                <Label htmlFor="chat-prompt">
                  {controls.data?.can_steer ? 'Уточнение активному запуску' : 'Сообщение агенту'}
                </Label>
                <Textarea
                  id="chat-prompt"
                  value={body}
                  disabled={
                    !owner ||
                    (!controls.data?.can_send && !controls.data?.can_steer) ||
                    message.isPending ||
                    messageUncertain ||
                    uncertainSteer ||
                    journal.error
                  }
                  onChange={(event) => {
                    setBody(event.target.value)
                    setMessageKey(requestKey())
                  }}
                  rows={2}
                />
                {message.isError && <ReadableError error={message.error} />}
                {steerTooLarge && <p role="alert">Уточнение не должно превышать 64 КиБ в UTF-8.</p>}
                {(messageUncertain || uncertainSteer) && !message.isPending && (
                  <p role="status">
                    {uncertainSteer
                      ? 'Исход уточнения запуску неизвестен. Нельзя повторить его как новый prompt; требуется сверка runtime.'
                      : 'Исход команды проверяется. Повтор сообщения использует прежний ключ; текст пока нельзя менять.'}
                  </p>
                )}
                {controls.isError && <ReadableError error={controls.error} />}
                {journal.error && (
                  <p role="alert">
                    Метаданные исходной команды недоступны. Новая отправка заблокирована.
                  </p>
                )}
                <div>
                  <span>
                    <ShieldCheck size={14} />
                    {owner ? 'Ваш чат' : 'Только чтение'}
                  </span>
                  <Button
                    type="submit"
                    aria-label={
                      controls.data?.can_steer
                        ? 'Передать уточнение запуску'
                        : 'Отправить сообщение'
                    }
                    title={
                      controls.data?.can_steer
                        ? 'Передать уточнение запуску'
                        : 'Отправить сообщение'
                    }
                    disabled={!canSubmitMessage}
                  >
                    <Send size={16} />
                  </Button>
                  {controls.data?.can_stop && (
                    <Button
                      type="button"
                      variant="outline"
                      aria-label="Остановить запуск"
                      title="Остановить запуск"
                      disabled={stop.isPending || journal.error}
                      onClick={() => {
                        const command =
                          journal.entries.stop ??
                          ((stop.isError || stopUnacknowledged) && stop.variables
                            ? stop.variables
                            : { runId: controls.data!.active_run_id!, key: requestKey() })
                        stop.mutate(command)
                      }}
                    >
                      <Square size={15} />
                    </Button>
                  )}
                </div>
                {stop.isError && <ReadableError error={stop.error} />}
                {stopUnacknowledged && (
                  <p role="status">
                    Принятие остановки не подтверждено. Повтор проверяет исходную команду, а не
                    останавливает другой запуск.
                  </p>
                )}
              </form>
            </TabsContent>
            <TabsContent value="clarification" className="fc-chat-panel">
              <div className="fc-chat-scroll">
                {!bound ? (
                  <EmptyState
                    title={
                      task.isError ? 'Контекст недоступен' : 'Свободный чат: уточнения не привязаны'
                    }
                  />
                ) : questions.isPending ? (
                  <p>Загрузка уточнений</p>
                ) : questions.isError ? (
                  <ReadableError error={questions.error} />
                ) : !questionList.length ? (
                  <EmptyState title="Уточнений пока нет" />
                ) : (
                  <>
                    <nav className="fc-chat-question-list" aria-label="Вопросы">
                      {questionList.map((question, index) => (
                        <button
                          key={question.id}
                          aria-current={question.id === selectedQuestion?.id ? 'true' : undefined}
                          onClick={() =>
                            setParams((current) => {
                              const next = new URLSearchParams(current)
                              next.set('question', question.id)
                              return next
                            })
                          }
                        >
                          {index + 1}. {question.text}
                          <StatusBadge value={question.state} />
                        </button>
                      ))}
                    </nav>
                    {selectedQuestion && (
                      <fieldset
                        className="fc-chat-question"
                        disabled={
                          !owner ||
                          task.isError ||
                          questions.isError ||
                          !context?.permissions.can_answer ||
                          selectedQuestion.state !== 'open' ||
                          answer.isPending ||
                          answerUncertain
                        }
                      >
                        <legend>{selectedQuestion.text}</legend>
                        <p>{selectedQuestion.rationale}</p>
                        <p className="fc-chat-muted">
                          {selectedQuestion.required
                            ? 'Обязательный вопрос'
                            : 'Необязательный вопрос'}{' '}
                          · Требования: ред. {selectedQuestion.requirement_revision} ·{' '}
                          {selectedQuestion.requirement_reference}
                        </p>
                        {selectedQuestion.mode !== 'text' &&
                          selectedQuestion.options.map((option) => (
                            <label className="fc-chat-option" key={option.id}>
                              <input
                                type={selectedQuestion.mode === 'multiple' ? 'checkbox' : 'radio'}
                                name={`question-${selectedQuestion.id}`}
                                checked={
                                  selectedQuestion.state === 'answered'
                                    ? (selectedQuestion.answer?.selected_option_ids.includes(
                                        option.id,
                                      ) ?? false)
                                    : draft.selected.includes(option.id)
                                }
                                onChange={(event) =>
                                  updateDraft({
                                    selected:
                                      selectedQuestion.mode === 'single'
                                        ? [option.id]
                                        : event.target.checked
                                          ? [...draft.selected, option.id]
                                          : draft.selected.filter((value) => value !== option.id),
                                  })
                                }
                              />
                              <span>
                                <strong>{option.label}</strong>
                                {selectedQuestion.recommended_option_id === option.id && (
                                  <small>Рекомендация PM</small>
                                )}
                                <span>{option.consequences}</span>
                              </span>
                            </label>
                          ))}
                        <Label htmlFor="clarification-text">
                          {selectedQuestion.mode === 'text'
                            ? 'Ваш ответ'
                            : 'Свой вариант или детали'}
                        </Label>
                        <Textarea
                          id="clarification-text"
                          value={
                            selectedQuestion.state === 'answered'
                              ? (selectedQuestion.answer?.text ?? '')
                              : draft.text
                          }
                          onChange={(event) => updateDraft({ text: event.target.value })}
                        />
                        <Label htmlFor="clarification-comment">Комментарий</Label>
                        <Textarea
                          id="clarification-comment"
                          value={
                            selectedQuestion.state === 'answered'
                              ? (selectedQuestion.answer?.comment ?? '')
                              : draft.comment
                          }
                          onChange={(event) => updateDraft({ comment: event.target.value })}
                        />
                      </fieldset>
                    )}
                    {staleDrafts.map(([key, previous]) => (
                      <section key={key} className="fc-chat-question" role="alert">
                        <h3>Вопрос изменился. Несохранённый ответ сохранён отдельно.</h3>
                        <p>{previous.text}</p>
                        <p>{previous.comment}</p>
                        <p>
                          {previous.selected
                            .map(
                              (optionId) =>
                                selectedQuestion?.options.find((option) => option.id === optionId)
                                  ?.label ?? optionId,
                            )
                            .join(', ')}
                        </p>
                        <Button
                          variant="outline"
                          disabled={
                            !owner ||
                            task.isError ||
                            answer.isPending ||
                            answerUncertain ||
                            Boolean(
                              draft.text.trim() || draft.comment.trim() || draft.selected.length,
                            )
                          }
                          onClick={() =>
                            setDrafts((current) => {
                              const next = { ...current }
                              delete next[key]
                              next[questionKey] = {
                                ...previous,
                                key: requestKey(),
                                selected: previous.selected.filter((id) =>
                                  selectedQuestion?.options.some((option) => option.id === id),
                                ),
                              }
                              return next
                            })
                          }
                        >
                          Перенести черновик и проверить новый вопрос
                        </Button>
                      </section>
                    ))}
                  </>
                )}
                {answer.isError && <ReadableError error={answer.error} />}
                {answerUncertain && (
                  <>
                    <p>Непроверенный ответ: {answer.variables?.questionTitle}</p>
                    <p role="status">
                      Неизвестен исход сохранения. Новые ответы заблокированы до сверки исходной
                      команды.
                    </p>
                    <Button
                      variant="outline"
                      disabled={
                        !owner ||
                        !bound ||
                        !context ||
                        !answer.variables ||
                        task.isError ||
                        questions.isError
                      }
                      onClick={() => {
                        // New-answer permission can close before the original key is acknowledged.
                        if (answerUncertain && answer.variables) answer.mutate(answer.variables)
                      }}
                    >
                      <RotateCw size={15} />
                      Повторить исходный ответ
                    </Button>
                  </>
                )}
              </div>
              <footer className="fc-chat-answer-footer">
                <p>Ответ закрывает вопрос, но не публикует требования.</p>
                <Button
                  disabled={
                    !selectedQuestion ||
                    !owner ||
                    task.isError ||
                    questions.isError ||
                    !context?.permissions.can_answer ||
                    !draft.key ||
                    !canSubmitAnswer(selectedQuestion, draft.selected, draft.text) ||
                    answer.isPending ||
                    answerUncertain
                  }
                  onClick={() => {
                    if (!selectedQuestion) return
                    answer.mutate({
                      questionId: selectedQuestion.id,
                      questionKey,
                      questionTitle: selectedQuestion.text,
                      payload: {
                        expected_question_version: selectedQuestion.version,
                        requirement_revision: selectedQuestion.requirement_revision,
                        selected_option_ids: draft.selected,
                        text: draft.text.trim() || null,
                        comment: draft.comment.trim() || null,
                        idempotency_key: draft.key,
                      },
                    })
                  }}
                >
                  <Check size={15} />
                  Сохранить ответ
                </Button>
              </footer>
            </TabsContent>
            <TabsContent value="requirements" className="fc-chat-panel">
              <div className="fc-chat-scroll">
                {!bound ? (
                  <EmptyState title="Нет привязки к SDLC" />
                ) : requirements.isPending ? (
                  <p>Загрузка требований</p>
                ) : requirements.isError ? (
                  <ReadableError error={requirements.error} />
                ) : (
                  <RequirementsView
                    revisions={requirements.data?.revisions ?? []}
                    currentRevision={context?.requirement_revision ?? null}
                    canConfirm={owner && !task.isError && Boolean(context?.permissions.can_confirm)}
                    sessionId={id}
                    onSaved={invalidate}
                  />
                )}
              </div>
            </TabsContent>
          </Tabs>
        </div>
        <aside className="fc-chat-context">{contextContent}</aside>
      </div>
      <Dialog open={contextOpen} onOpenChange={setContextOpen}>
        <DialogContent
          onCloseAutoFocus={(event) => {
            event.preventDefault()
            contextTrigger.current?.focus()
          }}
        >
          <DialogHeader>
            <DialogTitle>Контекст задачи</DialogTitle>
          </DialogHeader>
          <div className="fc-chat-drawer-content">{contextContent}</div>
        </DialogContent>
      </Dialog>
      <Dialog
        open={blocker.state === 'blocked'}
        onOpenChange={(open) => {
          if (!open && blocker.state === 'blocked') blocker.reset()
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Остались несохранённые изменения</DialogTitle>
          </DialogHeader>
          <p>Черновик хранится только в этом чате и будет потерян при переходе.</p>
          <Button variant="outline" onClick={() => blocker.state === 'blocked' && blocker.reset()}>
            Остаться
          </Button>
          <Button onClick={() => blocker.state === 'blocked' && blocker.proceed()}>
            Уйти без сохранения
          </Button>
        </DialogContent>
      </Dialog>
    </section>
  )
}

const documentLabels = {
  scope: 'Входит в объём',
  exclusions: 'Не входит в объём',
  scenarios: 'Сценарии',
  acceptance_criteria: 'Критерии приёмки',
  constraints: 'Ограничения',
  dependencies: 'Зависимости',
  assumptions: 'Допущения',
  checklist: 'Checklist',
  prerequisites: 'Технические prerequisites',
}
function RequirementsBody({ revision }: { revision: RequirementsRevision }) {
  return (
    <article className="fc-chat-requirements">
      <h2>Редакция {revision.revision}</h2>
      <p>{revision.goal}</p>
      {Object.entries(documentLabels).map(([key, label]) => (
        <section key={key}>
          <h3>{label}</h3>
          <ul>
            {(revision[key as keyof typeof documentLabels] ?? []).map((item, index) => (
              <li key={index}>{item}</li>
            ))}
          </ul>
        </section>
      ))}
    </article>
  )
}
function RequirementsView({
  revisions,
  currentRevision,
  canConfirm,
  sessionId,
  onSaved,
}: {
  revisions: RequirementsRevision[]
  currentRevision: number | null
  canConfirm: boolean
  sessionId: string
  onSaved: () => Promise<void>
}) {
  const [params, setParams] = useSearchParams()
  const selected =
    revisions.find((revision) => String(revision.revision) === params.get('revision')) ??
    revisions.find((revision) => revision.revision === currentRevision) ??
    revisions.at(-1)
  const previous = revisions
    .filter((revision) => selected && revision.revision < selected.revision)
    .sort((a, b) => b.revision - a.revision)[0]
  if (!selected) return <EmptyState title="PM пока не подготовил требования" />
  return (
    <>
      <Label htmlFor="requirement-revision">Редакция требований</Label>
      <select
        id="requirement-revision"
        value={selected.revision}
        onChange={(event) =>
          setParams((current) => {
            const next = new URLSearchParams(current)
            next.set('revision', event.target.value)
            return next
          })
        }
      >
        {revisions.map((revision) => (
          <option key={revision.revision} value={revision.revision}>
            Редакция {revision.revision}
          </option>
        ))}
      </select>
      <RequirementsBody revision={selected} />
      {previous && (
        <details>
          <summary>Сравнить с редакцией {previous.revision}</summary>
          <div className="fc-chat-revision-comparison">
            <RequirementsBody revision={previous} />
            <RequirementsBody revision={selected} />
          </div>
        </details>
      )}
      <RevisionConfirmation
        key={`${selected.revision}:${selected.content_hash}`}
        revision={selected}
        enabled={canConfirm && selected.revision === currentRevision}
        sessionId={sessionId}
        onSaved={onSaved}
      />
    </>
  )
}
function RevisionConfirmation({
  revision,
  enabled,
  sessionId,
  onSaved,
}: {
  revision: RequirementsRevision
  enabled: boolean
  sessionId: string
  onSaved: () => Promise<void>
}) {
  const [checked, setChecked] = useState(false)
  const [key] = useState(requestKey)
  const mutation = useMutation({
    mutationFn: () => confirmRequirements(sessionId, revision.revision, revision.content_hash, key),
    onSuccess: onSaved,
  })
  return (
    <div className="fc-chat-confirm">
      <label>
        <input
          type="checkbox"
          checked={checked}
          disabled={!enabled || mutation.isPending || mutation.isSuccess}
          onChange={(event) => setChecked(event.target.checked)}
        />
        Подтверждаю цель, границы и критерии приёмки редакции {revision.revision}
      </label>
      {!enabled && <p>Подтверждение недоступно: проверьте актуальную редакцию и prerequisites.</p>}
      <Button
        disabled={!enabled || !checked || mutation.isPending || mutation.isSuccess}
        onClick={() => mutation.mutate()}
      >
        <Check size={15} />
        Подтвердить редакцию {revision.revision}
      </Button>
      {mutation.isError && <ReadableError error={mutation.error} />}
      {mutation.isSuccess && (
        <p role="status">Подтверждение сохранено. Следующее назначение проверяется отдельно.</p>
      )}
    </div>
  )
}
