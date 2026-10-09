import { useCallback, useEffect, useRef, useState } from 'react'
import { Link, useBlocker, useParams, useSearchParams } from 'react-router'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowDown, ArrowLeft, Bot, Info, Send } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { ApiError, connectAuthenticatedEventStream } from '@sdlc/ui/lib'
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
import { apiBaseUrl } from '@/api/client'
import {
  createSessionMessage,
  getSession,
  listAgentDirectory,
  listSessionAgentRuns,
  listSessionMessages,
} from '@/api/fleet'
import { isCurrentAuth, ssoConfig, useAuthStore } from '@/shared/auth/store'
import { UserAvatar } from '@/shared/ui/user-avatar'
import { EmptyState, ErrorState, StatusBadge, formatDate } from '../common'
import {
  chatActivity,
  chatBackTo,
  chatMessageRequest,
  clearDispatch,
  commandService,
  dispatchHeld,
  legacyControlHeld,
  markDispatch,
  payloadDigest,
  unknownOutcome,
  type DispatchMarker,
} from './core'
import './chat.css'

export function ChatDetailPage() {
  const { sessionId } = useParams()
  return sessionId ? <ChatWorkspace key={sessionId} sessionId={sessionId} /> : null
}

function ChatWorkspace({ sessionId }: { sessionId: string }) {
  const { t } = useTranslation()
  const [params, setParams] = useSearchParams()
  const tab = ['dialogue', 'clarification', 'requirements'].includes(params.get('tab') ?? '')
    ? params.get('tab')!
    : 'dialogue'
  const client = useQueryClient()
  const auth = useAuthStore()
  const [draft, setDraft] = useState('')
  const [sending, setSending] = useState(false)
  const [error, setError] = useState('')
  const [held, setHeld] = useState(() => dispatchHeld(sessionId) || legacyControlHeld(sessionId))
  const [contextOpen, setContextOpen] = useState(false)
  const [stream, setStream] = useState<Record<string, string>>({})
  const [newMessages, setNewMessages] = useState(false)
  const scroll = useRef<HTMLDivElement>(null)
  const atBottom = useRef(true)
  const readingTop = useRef(0)
  const restoringScroll = useRef(false)
  const dialogueActive = useRef(tab === 'dialogue')
  const previousContent = useRef('')
  const live = useRef(true)
  const dispatching = useRef(false)
  const command = useRef<{ body: string; marker: DispatchMarker; uncertain: boolean } | null>(null)
  const authorized = Boolean(auth.token && auth.userId && !auth.signingOut)
  const session = useQuery({
    queryKey: ['chat-core-session', sessionId],
    queryFn: () => getSession(sessionId),
    enabled: authorized,
    retry: false,
    refetchInterval: 10_000,
  })
  const canRead = authorized && session.isSuccess && !session.isFetching
  const directory = useQuery({
    queryKey: ['agent-directory'],
    queryFn: listAgentDirectory,
    enabled: authorized,
    retry: false,
  })
  const messages = useQuery({
    queryKey: ['chat-core-messages', sessionId],
    queryFn: () => listSessionMessages(sessionId),
    enabled: authorized && session.isSuccess,
    retry: false,
    refetchInterval: 10_000,
  })
  const runs = useQuery({
    queryKey: ['chat-core-runs', sessionId],
    queryFn: () => listSessionAgentRuns(sessionId),
    enabled: authorized && session.isSuccess,
    retry: false,
    refetchInterval: 10_000,
  })
  const { refetch: refetchSession } = session
  const { refetch: refetchDirectory } = directory
  const { refetch: refetchMessages } = messages
  const { refetch: refetchRuns } = runs
  const refresh = useCallback(async () => {
    await Promise.all([refetchSession(), refetchDirectory(), refetchMessages(), refetchRuns()])
  }, [refetchSession, refetchDirectory, refetchMessages, refetchRuns])
  const denied = [session.error, messages.error, runs.error].some(
    (failure) => failure instanceof ApiError && [401, 403, 404].includes(failure.status),
  )
  useEffect(() => {
    live.current = true
    return () => {
      live.current = false
    }
  }, [])
  useEffect(() => {
    if (!authorized || !auth.token || !session.isSuccess || denied) return
    const scope = useAuthStore.getState()
    return connectAuthenticatedEventStream({
      url: `${apiBaseUrl}/api/v1/sessions/${sessionId}/stream`,
      token: auth.token,
      eventTypes: ['session'],
      onOpen: () => {
        if (isCurrentAuth(scope)) void refresh()
      },
      onEvent: (_type, data) => {
        if (!live.current || !isCurrentAuth(scope)) return
        if (
          data &&
          typeof data === 'object' &&
          'type' in data &&
          data.type === 'session_run_delta' &&
          'run_id' in data &&
          typeof data.run_id === 'string'
        ) {
          const id = data.run_id
          if ('text' in data && typeof data.text === 'string') {
            const text = data.text
            setStream((current) => ({ ...current, [id]: text }))
          } else if ('delta' in data && typeof data.delta === 'string') {
            const delta = data.delta
            setStream((current) => ({ ...current, [id]: (current[id] ?? '') + delta }))
          }
        } else void refresh()
      },
    })
  }, [authorized, auth.token, session.isSuccess, denied, sessionId, refresh])
  const activity = session.data
    ? chatActivity(session.data, messages.data ?? [], runs.data ?? [])
    : null
  const activeRuns =
    activity?.runs.filter((run) => ['running', 'waiting'].includes(run.state)) ?? []
  const content = JSON.stringify([
    activity?.messages.map((message) => [message.id, message.body]),
    activeRuns.map((run) => stream[run.id]),
  ])
  useEffect(() => {
    const changed = Boolean(previousContent.current && previousContent.current !== content)
    previousContent.current = content
    if (tab !== 'dialogue') {
      if (changed) setNewMessages(true)
      return
    }
    // Radix completes panel visibility after the controlled tab changes.
    // Restore after layout, and ignore the temporary hidden-panel scroll event.
    const frame = requestAnimationFrame(() => {
      const node = scroll.current
      const pinned = atBottom.current
      if (node) {
        node.scrollTop = pinned ? node.scrollHeight : readingTop.current
        if (pinned) setNewMessages(false)
        else if (changed) setNewMessages(true)
      }
      restoringScroll.current = false
    })
    return () => cancelAnimationFrame(frame)
  }, [content, tab])
  const blocker = useBlocker(
    ({ currentLocation, nextLocation }) =>
      Boolean(draft || sending) && currentLocation.pathname !== nextLocation.pathname,
  )
  useEffect(() => {
    if (!draft && !sending) return
    const warn = (event: BeforeUnloadEvent) => {
      event.preventDefault()
      event.returnValue = ''
    }
    window.addEventListener('beforeunload', warn)
    return () => window.removeEventListener('beforeunload', warn)
  }, [draft, sending])
  const agent = directory.data?.find((entry) => entry.id === session.data?.primary_agent_id)
  const owns =
    session.data?.user_id === auth.userId && auth.permissions.includes('sessions:write_own')
  const healthy =
    canRead &&
    messages.isSuccess &&
    runs.isSuccess &&
    directory.isSuccess &&
    !messages.isFetching &&
    !runs.isFetching &&
    !directory.isFetching
  const writable =
    healthy &&
    owns &&
    agent?.product_role === 'executor' &&
    agent.status !== 'archived' &&
    ['draft', 'active'].includes(session.data?.state ?? '')
  const canSend = writable && !sending && ((!held && !activity?.busy) || Boolean(command.current))

  async function send() {
    if (!canSend || !draft.trim() || dispatching.current) return
    dispatching.current = true
    setSending(true)
    setError('')
    const scope = useAuthStore.getState()
    let sent = false
    let original = command.current
    try {
      // Check the actual ACL/agent/run/delivery immediately before each dispatch.
      const [fresh, agents, history, freshRuns] = await Promise.all([
        getSession(sessionId),
        listAgentDirectory(),
        listSessionMessages(sessionId),
        listSessionAgentRuns(sessionId),
      ])
      const primary = agents.find((entry) => entry.id === fresh.primary_agent_id)
      if (
        !isCurrentAuth(scope) ||
        fresh.user_id !== scope.userId ||
        !scope.permissions.includes('sessions:write_own') ||
        primary?.product_role !== 'executor' ||
        primary.status === 'archived' ||
        !['draft', 'active'].includes(fresh.state) ||
        legacyControlHeld(sessionId) ||
        fresh.primary_agent_id !== session.data?.primary_agent_id ||
        (!original && chatActivity(fresh, history, freshRuns).busy)
      )
        throw new Error(t('chatCore.readOnly'))
      if (
        original &&
        (original.marker.actor !== scope.userId ||
          original.marker.agent !== fresh.primary_agent_id ||
          original.marker.service !== commandService(apiBaseUrl, ssoConfig.issuer))
      )
        throw new Error(t('chatCore.held'))
      if (!original) {
        if (dispatchHeld(sessionId)) throw new Error(t('chatCore.held'))
        const body = draft.trim()
        const key = crypto.randomUUID()
        const marker = {
          actor: scope.userId!,
          agent: fresh.primary_agent_id,
          service: commandService(apiBaseUrl, ssoConfig.issuer),
          key,
          digest: await payloadDigest(chatMessageRequest(body, key)),
        }
        if (!isCurrentAuth(scope) || !live.current) return
        markDispatch(sessionId, marker)
        original = { body, marker, uncertain: false }
        command.current = original
        setHeld(true)
      }
      sent = true
      const response = await createSessionMessage(
        sessionId,
        chatMessageRequest(original.body, original.marker.key),
      )
      if (!live.current || !isCurrentAuth(scope)) return
      if (
        !response.id ||
        response.session_id !== sessionId ||
        response.author_user_id !== scope.userId ||
        response.author_type !== 'user' ||
        response.author_agent_id !== null ||
        response.message_kind !== 'user_prompt' ||
        response.request_payload_hash !== original.marker.digest ||
        !clearDispatch(sessionId, original.marker)
      )
        throw new Error(t('chatCore.held'))
      command.current = null
      setHeld(false)
      setDraft('')
      await refresh()
      void client.invalidateQueries({ queryKey: ['sessions'] })
    } catch (failure) {
      if (!live.current || !isCurrentAuth(scope)) return
      if (sent && original && !original.uncertain && !unknownOutcome(failure)) {
        if (clearDispatch(sessionId, original.marker)) {
          command.current = null
          setHeld(false)
        }
      }
      if (sent && original && unknownOutcome(failure)) original.uncertain = true
      setError(sent && unknownOutcome(failure) ? t('chatCore.unknown') : t('chatCore.sendError'))
    } finally {
      dispatching.current = false
      if (live.current) setSending(false)
    }
  }

  const context = (
    <>
      <h2>{t('chatCore.context')}</h2>
      <dl className="fc-chat-fields">
        <div>
          <dt>{t('chatCore.owner')}</dt>
          <dd>{session.data?.user_display_name}</dd>
        </div>
        <div>
          <dt>{t('chatCore.task')}</dt>
          <dd>{session.data?.task_key ?? t('chats.freeChat')}</dd>
        </div>
        <div>
          <dt>{t('chatCore.runtime')}</dt>
          <dd>{agent?.kind ?? t('chatCore.unavailable')}</dd>
        </div>
      </dl>
      <p className="fc-chat-notice">{t('chatCore.pmUnavailable')}</p>
      <h3>{t('chatCore.runs')}</h3>
      {activity?.runs.length ? (
        activity.runs.map((run) => (
          <div key={run.id} className="mb-3">
            <StatusBadge value={run.state} />
            <p className="fc-chat-muted">
              {run.provider ?? '—'} · {run.model ?? '—'}
            </p>
          </div>
        ))
      ) : (
        <p className="fc-chat-muted">{t('chatCore.noRuns')}</p>
      )}
    </>
  )
  if (denied || !authorized)
    return (
      <>
        <Link to={chatBackTo(params.get('backTo'))}>{t('chatCore.back')}</Link>
        <ErrorState message={t('chatCore.denied')} />
      </>
    )
  if (session.isPending) return <EmptyState title={t('chats.loading')} />
  if (!session.data || session.isError)
    return (
      <>
        <ErrorState message={t('chats.loadError')} />
        <Button onClick={() => void refresh()}>{t('sessions.retry')}</Button>
      </>
    )
  return (
    <div className="fc-chat-workbench">
      <header className="fc-chat-header">
        <div>
          <Link to={chatBackTo(params.get('backTo'))} aria-label={t('chatCore.back')}>
            <ArrowLeft className="h-5 w-5" />
          </Link>
          <h1>{session.data.title}</h1>
          <StatusBadge value={session.data.state} />
          <StatusBadge value={session.data.visibility} />
        </div>
        <Button
          variant="outline"
          className="fc-chat-context-button"
          onClick={() => setContextOpen(true)}
        >
          <Info className="h-4 w-4" />
          {t('chatCore.context')}
        </Button>
      </header>
      <div className="fc-chat-agent">
        <Bot className="h-5 w-5" />
        <strong>{agent?.display_name ?? session.data.primary_agent_name}</strong>
        {agent ? <StatusBadge value={agent.status} /> : null}
        <span>{session.data.task_key ?? t('chats.freeChat')}</span>
      </div>
      <div className="fc-chat-grid">
        <section className="fc-chat-main">
          <Tabs
            value={tab}
            onValueChange={(value) => {
              const node = scroll.current
              if (value !== 'dialogue' && dialogueActive.current && node && node.clientHeight > 0)
                readingTop.current = node.scrollTop
              dialogueActive.current = value === 'dialogue'
              restoringScroll.current = value === 'dialogue'
              setParams((current) => {
                const next = new URLSearchParams(current)
                next.set('tab', value)
                return next
              })
            }}
            className="fc-chat-tabs"
          >
            <TabsList className="fc-chat-tab-list">
              {(['dialogue', 'clarification', 'requirements'] as const).map((value) => (
                <TabsTrigger key={value} value={value}>
                  {t(`chatCore.${value}`)}
                </TabsTrigger>
              ))}
            </TabsList>
            <TabsContent
              value="dialogue"
              forceMount
              hidden={tab !== 'dialogue'}
              style={tab === 'dialogue' ? undefined : { display: 'none' }}
              className="fc-chat-panel"
            >
              <div
                className="fc-chat-scroll"
                ref={scroll}
                onScroll={() => {
                  const node = scroll.current
                  if (
                    !node ||
                    restoringScroll.current ||
                    !dialogueActive.current ||
                    node.clientHeight === 0
                  )
                    return
                  readingTop.current = node.scrollTop
                  atBottom.current = node.scrollHeight - node.clientHeight - node.scrollTop < 40
                  if (atBottom.current) setNewMessages(false)
                }}
              >
                {messages.isError || runs.isError || directory.isError ? (
                  <ErrorState message={t('chats.loadError')} />
                ) : messages.isPending || runs.isPending ? (
                  <EmptyState title={t('chats.loading')} />
                ) : (
                  <>
                    <p className="fc-chat-notice">{t('chatCore.historyLimit')}</p>
                    {activity?.messages.map((message) => (
                      <article className="fc-chat-message" key={message.id}>
                        <div>
                          <UserAvatar
                            userId={message.author_user_id ?? undefined}
                            name={message.author_display_name}
                          />
                          <strong>{message.author_display_name}</strong>
                          <time>{formatDate(message.created_at)}</time>
                          <StatusBadge value={message.delivery_state} />
                        </div>
                        <p>{message.body}</p>
                        {message.delivery_error ? (
                          <p className="text-danger">{message.delivery_error}</p>
                        ) : null}
                      </article>
                    ))}
                    {!activity?.messages.length ? (
                      <EmptyState title={t('chatCore.emptyDialogue')} />
                    ) : null}
                    {activeRuns.map((run) =>
                      stream[run.id] ? (
                        <article className="fc-chat-message" key={run.id}>
                          <div>
                            <Bot className="h-5 w-5" />
                            <strong>{agent?.display_name}</strong>
                            <StatusBadge value={run.state} />
                          </div>
                          <p>{stream[run.id]}</p>
                        </article>
                      ) : null,
                    )}
                  </>
                )}
              </div>
              {newMessages ? (
                <Button
                  variant="outline"
                  onClick={() => {
                    const node = scroll.current
                    if (node) node.scrollTop = node.scrollHeight
                    atBottom.current = true
                    setNewMessages(false)
                  }}
                >
                  <ArrowDown className="h-4 w-4" />
                  {t('chatCore.newMessages')}
                </Button>
              ) : null}
              <form
                className="fc-chat-composer"
                onSubmit={(event) => {
                  event.preventDefault()
                  void send()
                }}
              >
                <Label htmlFor="chat-message">{t('chatCore.message')}</Label>
                <Textarea
                  id="chat-message"
                  value={draft}
                  maxLength={20000}
                  disabled={sending || held || !owns}
                  onChange={(event) => {
                    setDraft(event.target.value)
                    setError('')
                  }}
                />
                {held ? (
                  <p role="alert" className="fc-chat-notice">
                    {t('chatCore.held')}
                  </p>
                ) : !owns ? (
                  <p className="fc-chat-notice">{t('chatCore.readOnly')}</p>
                ) : activity?.busy ? (
                  <p className="fc-chat-notice">{t('chatCore.activeRun')}</p>
                ) : null}
                {error ? (
                  <p role="alert" className="text-sm text-danger">
                    {error}
                  </p>
                ) : null}
                <div>
                  <Button
                    type="button"
                    variant="outline"
                    onClick={() => void refresh()}
                    disabled={sending}
                  >
                    {t('sessions.retry')}
                  </Button>
                  <Button type="submit" disabled={!canSend || !draft.trim()}>
                    <Send className="h-4 w-4" />
                    {t(command.current ? 'chatCore.retryOriginal' : 'chatCore.send')}
                  </Button>
                </div>
              </form>
            </TabsContent>
            {(['clarification', 'requirements'] as const).map((value) => (
              <TabsContent value={value} className="fc-chat-panel" key={value}>
                <div className="fc-chat-scroll">
                  <h2 className="text-lg font-semibold">{t(`chatCore.${value}`)}</h2>
                  <p className="fc-chat-notice">{t('chatCore.pmUnavailable')}</p>
                  <Button disabled>
                    {t(value === 'clarification' ? 'chatCore.answer' : 'chatCore.confirm')}
                  </Button>
                </div>
              </TabsContent>
            ))}
          </Tabs>
        </section>
        <aside className="fc-chat-context">{context}</aside>
      </div>
      <Dialog open={contextOpen} onOpenChange={setContextOpen}>
        <DialogContent className="fc-chat-drawer-content">
          <DialogHeader>
            <DialogTitle>{t('chatCore.context')}</DialogTitle>
          </DialogHeader>
          {context}
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
            <DialogTitle>{t('chatCore.unsaved')}</DialogTitle>
          </DialogHeader>
          <p>{t('chatCore.leaveWarning')}</p>
          <Button variant="outline" onClick={() => blocker.state === 'blocked' && blocker.reset()}>
            {t('chatCore.stay')}
          </Button>
          <Button onClick={() => blocker.state === 'blocked' && blocker.proceed()}>
            {t('chatCore.leave')}
          </Button>
        </DialogContent>
      </Dialog>
    </div>
  )
}
