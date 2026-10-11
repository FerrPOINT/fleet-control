import { useEffect, useRef, useState } from 'react'
import { Link, useBlocker, useParams, useSearchParams } from 'react-router'
import { useInfiniteQuery, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  ArrowDown,
  ArrowLeft,
  Bot,
  Check,
  ClipboardList,
  FileText,
  Info,
  MessageSquare,
  Send,
  ShieldCheck,
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
import { apiBaseUrl } from '@/api/client'
import { getSession, listAgentDirectory, listSessionAgentRuns } from '@/api/fleet'
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
import { UserAvatar } from '@/shared/ui/user-avatar'
import { EmptyState, ErrorState, StatusBadge, formatDate } from '../common'
import './task-chat.css'
import { ChatRunControls } from './run-controls'
import { useDispatchRecovery } from './dispatch-recovery'
import { refreshHistory } from './history'
import { commandService } from './core'
import { ssoConfig } from '@/shared/auth/store'

function requestKey() {
  return crypto.randomUUID()
}
function ReadableError({ error }: { error: unknown }) {
  return <ErrorState message={error instanceof Error ? error.message : 'Данные недоступны'} />
}

export function TaskChatDetailPage() {
  const { sessionId = '' } = useParams()
  return <ChatWorkspace key={sessionId} id={sessionId} />
}

function ChatWorkspace({ id }: { id: string }) {
  const [params, setParams] = useSearchParams()
  const client = useQueryClient()
  const token = useAuthStore((state) => state.token)
  const userId = useAuthStore((state) => state.userId)
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
  const [delta, setDelta] = useState<Record<string, string>>({})
  const [receipt, setReceipt] = useState<string | null>(null)
  const answerRecovery = useDispatchRecovery(`${id}:answer`)
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
  const dirty = Object.values(drafts).some((value) =>
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
  const answer = useMutation({
    mutationFn: async (command: {
      questionId: string
      questionKey: string
      payload: AnswerInput
    }) => {
      const fresh = await getSession(id)
      await answerRecovery.prepare(
        command.payload,
        command.payload.idempotency_key,
        fresh.primary_agent_id,
        commandService(apiBaseUrl, ssoConfig.issuer),
        () =>
          fresh.user_id === useAuthStore.getState().userId &&
          owner &&
          !task.isError &&
          Boolean(context?.permissions.can_answer),
      )
      const result = await answerClarification(id, command.questionId, command.payload)
      if (
        result.question_id !== command.questionId ||
        result.question_version !== command.payload.expected_question_version ||
        result.requirement_revision !== command.payload.requirement_revision ||
        result.author_subject !== task.data?.binding?.owner_subject
      )
        throw new Error('Не удалось проверить подтверждение сохранения ответа.')
      return result
    },
    onSuccess: async (_result, command) => {
      if (!answerRecovery.finish()) return
      setReceipt('Ответ сохранён. Требования ещё не опубликованы.')
      setDrafts((current) => {
        const next = { ...current }
        delete next[command.questionKey]
        return next
      })
      await invalidate()
    },
    onError: (error) => {
      answerRecovery.fail(error)
      void questions.refetch()
      void task.refetch()
    },
  })
  const answerUncertain = answerRecovery.held
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
  const returnTo = params.get('backTo')
  const backTo =
    returnTo === '/chats' || returnTo?.startsWith('/chats?')
      ? returnTo
      : `/chats${params.get('agent') ? `?agent=${encodeURIComponent(params.get('agent')!)}` : ''}`
  const contextContent = (
    <>
      <h2>Контекст задачи</h2>
      <dl className="fc-task-chat-fields">
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
    <section className="fc-task-chat-workbench">
      <header className="fc-task-chat-header">
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
          {waiting && <span className="fc-task-chat-wait">{waiting}</span>}
          <Button
            ref={contextTrigger}
            variant="ghost"
            className="fc-task-chat-context-button"
            aria-label="Контекст задачи"
            title="Контекст задачи"
            onClick={() => setContextOpen(true)}
          >
            <Info size={18} />
          </Button>
        </div>
      </header>
      <div className="fc-task-chat-agent">
        <Bot size={16} />
        <strong>{agent?.display_name ?? session.data.primary_agent_name}</strong>
        <span>{agent?.kind ?? 'Runtime неизвестен'}</span>
        <StatusBadge value={agent?.status} />
        {controls.data?.blocked_reason && (
          <span>{blockedLabels[controls.data.blocked_reason] ?? controls.data.blocked_reason}</span>
        )}
      </div>
      <div className="fc-task-chat-grid">
        <div className="fc-task-chat-main">
          <Tabs value={tab} onValueChange={switchTab} className="fc-task-chat-tabs">
            <TabsList className="fc-task-chat-tab-list" aria-label="Разделы задачи">
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
              <p role="status" className="fc-task-chat-notice">
                {receipt}
              </p>
            )}
            <TabsContent value="dialogue" className="fc-task-chat-panel">
              <div
                ref={transcript}
                className="fc-task-chat-scroll"
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
                  <article className="fc-task-chat-message" key={item.id}>
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
                  <article className="fc-task-chat-message">
                    <strong>{activeRun.agent_name}</strong>
                    <p>{delta[activeRun.id]}</p>
                  </article>
                )}
                <TaskApprovalsPanel sessionId={id} canResolve={canResolveApprovals} hideWhenEmpty />
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
              <div className="fc-task-chat-composer">
                <Label htmlFor="chat-prompt">Сообщение агенту</Label>
                <Textarea id="chat-prompt" value="" disabled rows={2} />
                {controls.isError && <ReadableError error={controls.error} />}
                <p>Обычное сообщение требует проверенного назначения Workflow.</p>
                <div>
                  <span>
                    <ShieldCheck size={14} />
                    {owner ? 'Ваш чат' : 'Только чтение'}
                  </span>
                  <Button disabled aria-label="Отправить сообщение">
                    <Send size={16} />
                  </Button>
                </div>
              </div>
              <ChatRunControls sessionId={id} taskBound={true} />
            </TabsContent>
            <TabsContent value="clarification" className="fc-task-chat-panel">
              <div className="fc-task-chat-scroll">
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
                    <nav className="fc-task-chat-question-list" aria-label="Вопросы">
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
                        className="fc-task-chat-question"
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
                        <p className="fc-task-chat-muted">
                          {selectedQuestion.required
                            ? 'Обязательный вопрос'
                            : 'Необязательный вопрос'}{' '}
                          · Требования: ред. {selectedQuestion.requirement_revision} ·{' '}
                          {selectedQuestion.requirement_reference}
                        </p>
                        {selectedQuestion.mode !== 'text' &&
                          selectedQuestion.options.map((option) => (
                            <label className="fc-task-chat-option" key={option.id}>
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
                      <section key={key} className="fc-task-chat-question" role="alert">
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
                            answerRecovery.held ||
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
                {answerRecovery.restored ? (
                  <p role="alert">
                    Исход сохранения ответа требует сверки после перезагрузки. Новый ответ
                    заблокирован.
                  </p>
                ) : (
                  answerUncertain && (
                    <p role="status">
                      Неизвестен исход сохранения. Проверьте состояние вопроса или повторите тот же
                      ответ без изменения ключа.
                    </p>
                  )
                )}
              </div>
              <footer className="fc-task-chat-answer-footer">
                <p>Ответ закрывает вопрос, но не публикует требования.</p>
                <Button
                  disabled={
                    !selectedQuestion ||
                    answerRecovery.restored ||
                    !owner ||
                    task.isError ||
                    questions.isError ||
                    !context?.permissions.can_answer ||
                    !draft.key ||
                    !canSubmitAnswer(selectedQuestion, draft.selected, draft.text) ||
                    answer.isPending
                  }
                  onClick={() => {
                    if (!selectedQuestion) return
                    if (answerRecovery.held && answer.variables) {
                      answer.mutate(answer.variables)
                      return
                    }
                    answer.mutate({
                      questionId: selectedQuestion.id,
                      questionKey,
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
            <TabsContent
              value="requirements"
              forceMount
              hidden={tab !== 'requirements'}
              style={tab === 'requirements' ? undefined : { display: 'none' }}
              className="fc-task-chat-panel"
            >
              <div className="fc-task-chat-scroll">
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
        <aside className="fc-task-chat-context">{contextContent}</aside>
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
          <div className="fc-task-chat-drawer-content">{contextContent}</div>
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
    <article className="fc-task-chat-requirements">
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
          <div className="fc-task-chat-revision-comparison">
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
  const recovery = useDispatchRecovery(`${sessionId}:confirmation`)
  const [verified, setVerified] = useState(false)
  const mutation = useMutation({
    mutationFn: async () => {
      const [session, context] = await Promise.all([
        getSession(sessionId),
        getTaskContext(sessionId),
      ])
      await recovery.prepare(
        { revision: revision.revision, content_hash: revision.content_hash, idempotency_key: key },
        key,
        session.primary_agent_id,
        commandService(apiBaseUrl, ssoConfig.issuer),
        () =>
          enabled && session.user_id === useAuthStore.getState().userId && Boolean(context.binding),
      )
      const result = await confirmRequirements(
        sessionId,
        revision.revision,
        revision.content_hash,
        key,
      )
      if (
        result.revision !== revision.revision ||
        result.content_hash !== revision.content_hash ||
        result.task_id !== context.binding?.task_id ||
        result.owner_subject !== context.binding?.owner_subject
      )
        throw new Error('Не удалось проверить подтверждение редакции.')
      return result
    },
    onSuccess: async () => {
      if (!recovery.finish()) return
      setVerified(true)
      await onSaved()
    },
    onError: (error) => recovery.fail(error),
  })
  return (
    <div className="fc-task-chat-confirm">
      <label>
        <input
          type="checkbox"
          checked={checked}
          disabled={!enabled || mutation.isPending || verified || recovery.held}
          onChange={(event) => setChecked(event.target.checked)}
        />
        Подтверждаю цель, границы и критерии приёмки редакции {revision.revision}
      </label>
      {!enabled && <p>Подтверждение недоступно: проверьте актуальную редакцию и prerequisites.</p>}
      <Button
        disabled={!enabled || !checked || mutation.isPending || verified || recovery.restored}
        onClick={() => mutation.mutate()}
      >
        <Check size={15} />
        Подтвердить редакцию {revision.revision}
      </Button>
      {mutation.isError && <ReadableError error={mutation.error} />}
      {recovery.restored && (
        <p role="alert">
          Исход подтверждения требует сверки после перезагрузки. Новая команда заблокирована.
        </p>
      )}
      {verified && (
        <p role="status">Подтверждение сохранено. Следующее назначение проверяется отдельно.</p>
      )}
    </div>
  )
}
