import { type FormEvent, type ReactNode, useEffect, useMemo, useState } from 'react'
import { Link, useParams } from 'react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  ArrowRightLeft,
  Crown,
  ExternalLink,
  MessageSquareText,
  RefreshCw,
  Send,
  Square,
  UsersRound,
} from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import {
  assignSessionLeader,
  createSessionDelegation,
  createSessionMessage,
  getSession,
  handoffSession,
  listAgentDirectory,
  listLeaderExecutors,
  listSessionAgentRuns,
  listSessionMessages,
  listSessionParticipants,
  resolveSessionRunApproval,
  steerSessionRun,
  stopSessionRun,
} from '@/api/fleet'
import type { AgentSession, SessionAgentRun, SessionMessage, SessionParticipant } from '@/api/types'
import { useAuthStore } from '@/shared/auth/store'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
  Button,
  Input,
  Label,
  Textarea,
} from '@sdlc/ui/ui'
import { UserAvatar } from '@/shared/ui/user-avatar'
import { EmptyState, ErrorState, PageHeader, StatusBadge, formatDate } from '../common'

let fallbackRequestSequence = 0

function newIdempotencyKey() {
  return (
    globalThis.crypto?.randomUUID?.() ?? `fleet-ui-${Date.now()}-${(fallbackRequestSequence += 1)}`
  )
}

export function SessionDetailPage() {
  const { t } = useTranslation()
  const { sessionId } = useParams()
  const queryClient = useQueryClient()
  const canManageAgents = useAuthStore((state) => state.permissions.includes('agents:manage'))
  const session = useQuery({
    queryKey: ['session', sessionId],
    queryFn: () => getSession(sessionId!),
    enabled: Boolean(sessionId),
  })
  const agents = useQuery({ queryKey: ['agent-directory'], queryFn: listAgentDirectory })
  const leaders = useMemo(
    () => agents.data?.filter((agent) => agent.product_role === 'leader') ?? [],
    [agents.data],
  )
  const leaderTeams = useQuery({
    queryKey: ['leader-teams', leaders.map((leader) => leader.id).join(',')],
    enabled: Boolean(leaders.length),
    queryFn: async () =>
      Promise.all(
        leaders.map(async (leader) => ({
          leader,
          executors: await listLeaderExecutors(leader.id),
        })),
      ),
  })
  const messages = useQuery({
    queryKey: ['session-messages', sessionId],
    queryFn: () => listSessionMessages(sessionId!),
    enabled: Boolean(sessionId),
    refetchInterval: 2000,
  })
  const runs = useQuery({
    queryKey: ['session-runs', sessionId],
    queryFn: () => listSessionAgentRuns(sessionId!),
    enabled: Boolean(sessionId),
    refetchInterval: 2000,
  })
  const participants = useQuery({
    queryKey: ['session-participants', sessionId],
    queryFn: () => listSessionParticipants(sessionId!),
    enabled: Boolean(sessionId),
  })

  const [targetAgentId, setTargetAgentId] = useState('')
  const [leaderId, setLeaderId] = useState('')
  const [messageBody, setMessageBody] = useState('')
  const [messageRequestKey, setMessageRequestKey] = useState(newIdempotencyKey)
  const [authorMode, setAuthorMode] = useState<'user' | 'leader'>('user')
  const [delegationExecutorId, setDelegationExecutorId] = useState('')
  const [delegationTitle, setDelegationTitle] = useState(() => t('sessionDetail.delegationDefault'))
  const [delegationMessage, setDelegationMessage] = useState('')
  const [delegationRequestKey, setDelegationRequestKey] = useState(newIdempotencyKey)

  const primaryAgent = agents.data?.find((agent) => agent.id === session.data?.primary_agent_id)
  const possibleLeaders = useMemo(() => {
    const eligible =
      primaryAgent?.product_role === 'leader'
        ? leaders.filter((leader) => leader.id === primaryAgent.id)
        : (leaderTeams.data
            ?.filter((team) =>
              team.executors.some((executor) => executor.executor_agent_id === primaryAgent?.id),
            )
            .map((team) => team.leader) ?? [])
    const current = leaders.find((leader) => leader.id === session.data?.leader_agent_id)
    return current && !eligible.some((leader) => leader.id === current.id)
      ? [current, ...eligible]
      : eligible
  }, [leaderTeams.data, leaders, primaryAgent, session.data?.leader_agent_id])
  const delegationExecutors = useMemo(
    () =>
      leaderTeams.data?.find((team) => team.leader.id === session.data?.leader_agent_id)
        ?.executors ?? [],
    [leaderTeams.data, session.data?.leader_agent_id],
  )
  const teamsLoading = Boolean(leaders.length) && leaderTeams.isPending

  useEffect(() => {
    setLeaderId(session.data?.leader_agent_id ?? '')
  }, [session.data?.leader_agent_id])

  useEffect(() => {
    if (!session.data?.leader_agent_id) setAuthorMode('user')
  }, [session.data?.leader_agent_id])

  useEffect(() => {
    const executorStillAvailable = delegationExecutors.some(
      (executor) => executor.executor_agent_id === delegationExecutorId,
    )
    if (!executorStillAvailable) {
      setDelegationExecutorId(delegationExecutors[0]?.executor_agent_id ?? '')
    }
  }, [delegationExecutorId, delegationExecutors])

  const handoffMutation = useMutation({
    mutationFn: () => handoffSession(sessionId!, { target_agent_id: targetAgentId }),
    onSuccess: async (updated) => {
      setTargetAgentId('')
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['session', sessionId] }),
        queryClient.invalidateQueries({ queryKey: ['sessions'] }),
      ])
      toast.success(t('sessionDetail.handoffSuccess', { agent: updated.primary_agent_name }))
    },
  })
  const leaderMutation = useMutation({
    mutationFn: () => assignSessionLeader(sessionId!, { leader_agent_id: leaderId || null }),
    onSuccess: async (updated) => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['session', sessionId] }),
        queryClient.invalidateQueries({ queryKey: ['session-messages', sessionId] }),
        queryClient.invalidateQueries({ queryKey: ['session-runs', sessionId] }),
        queryClient.invalidateQueries({ queryKey: ['sessions'] }),
      ])
      toast.success(
        updated.leader_agent_name
          ? t('sessionDetail.leaderSuccess', { leader: updated.leader_agent_name })
          : t('sessionDetail.privateSuccess'),
      )
    },
  })
  const messageMutation = useMutation({
    mutationFn: () =>
      createSessionMessage(sessionId!, {
        body: messageBody.trim(),
        author_agent_id:
          authorMode === 'leader' && session.data?.leader_agent_id
            ? session.data.leader_agent_id
            : null,
        idempotency_key: messageRequestKey,
      }),
    onSuccess: async () => {
      setMessageBody('')
      setMessageRequestKey(newIdempotencyKey())
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['session', sessionId] }),
        queryClient.invalidateQueries({ queryKey: ['session-messages', sessionId] }),
        queryClient.invalidateQueries({ queryKey: ['session-runs', sessionId] }),
        queryClient.invalidateQueries({ queryKey: ['sessions'] }),
      ])
      toast.success(t('sessionDetail.messageSuccess'))
    },
  })
  const delegationMutation = useMutation({
    mutationFn: () =>
      createSessionDelegation(sessionId!, {
        executor_agent_id: delegationExecutorId,
        title: delegationTitle.trim(),
        initial_message: delegationMessage.trim() || null,
        idempotency_key: delegationRequestKey,
      }),
    onSuccess: async (created) => {
      setDelegationMessage('')
      setDelegationRequestKey(newIdempotencyKey())
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['sessions'] }),
        queryClient.invalidateQueries({ queryKey: ['session', sessionId] }),
        queryClient.invalidateQueries({ queryKey: ['session-messages', sessionId] }),
      ])
      toast.success(t('sessionDetail.delegationSuccess', { title: created.title }))
    },
  })

  function changeMessageBody(value: string) {
    setMessageBody(value)
    messageMutation.reset()
    setMessageRequestKey(newIdempotencyKey())
  }

  function resetDelegationRequest() {
    delegationMutation.reset()
    setDelegationRequestKey(newIdempotencyKey())
  }

  function submitHandoff(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (targetAgentId && !handoffMutation.isPending) handoffMutation.mutate()
  }

  function submitLeader(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!leaderMutation.isPending) leaderMutation.mutate()
  }

  function submitMessage(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (messageBody.trim() && !messageMutation.isPending) messageMutation.mutate()
  }

  function submitDelegation(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (delegationExecutorId && delegationTitle.trim() && !delegationMutation.isPending) {
      delegationMutation.mutate()
    }
  }

  if (!sessionId) return <ErrorState message={t('sessionDetail.missingId')} />

  if (session.isPending && !session.data) {
    return (
      <>
        <PageHeader title={t('sessionDetail.title')} description={t('sessionDetail.description')} />
        <EmptyState title={t('sessionDetail.loadingSession')} />
      </>
    )
  }

  if (session.isError && !session.data) {
    return (
      <>
        <PageHeader title={t('sessionDetail.title')} description={t('sessionDetail.description')} />
        <RetryState
          message={t('sessionDetail.sessionError')}
          onRetry={() => void session.refetch()}
        />
      </>
    )
  }

  if (!session.data) return null

  return (
    <>
      <PageHeader title={session.data.title} description={t('sessionDetail.description')} />
      {session.isError ? (
        <div className="mb-4">
          <RetryState
            message={t('sessionDetail.sessionStale')}
            onRetry={() => void session.refetch()}
          />
        </div>
      ) : null}

      <SessionSummary session={session.data} />

      <div className="mt-6 grid min-w-0 gap-6 xl:grid-cols-[minmax(0,1fr)_360px]">
        <div className="min-w-0 space-y-8">
          <section aria-labelledby="session-transcript-title">
            <SectionHeading
              id="session-transcript-title"
              icon={<MessageSquareText className="h-4 w-4" />}
              title={t('sessionDetail.transcript')}
              count={messages.data?.length}
              refreshing={messages.isFetching && Boolean(messages.data)}
              refreshLabel={t('sessionDetail.refreshing')}
            />
            <MessagesSection
              messages={messages.data}
              pending={messages.isPending}
              failed={messages.isError}
              onRetry={() => void messages.refetch()}
            />
            <form
              className="mt-4 grid gap-3 rounded-md border border-border bg-surface p-4"
              onSubmit={submitMessage}
              aria-busy={messageMutation.isPending}
            >
              <div className="grid max-w-sm gap-2">
                <Label htmlFor="session-message-author">{t('sessionDetail.messageAuthor')}</Label>
                <select
                  id="session-message-author"
                  value={authorMode}
                  disabled={messageMutation.isPending}
                  onChange={(event) => {
                    setAuthorMode(event.target.value as 'user' | 'leader')
                    messageMutation.reset()
                    setMessageRequestKey(newIdempotencyKey())
                  }}
                  className="h-10 min-w-0 rounded-md border border-border bg-background px-3 text-sm disabled:cursor-not-allowed disabled:opacity-60"
                >
                  <option value="user">{t('sessionDetail.sendAsMe')}</option>
                  <option value="leader" disabled={!session.data.leader_agent_id}>
                    {t('sessionDetail.sendAsLeader')}
                  </option>
                </select>
              </div>
              <div className="grid gap-2">
                <Label htmlFor="session-message-body">{t('sessionDetail.message')}</Label>
                <Textarea
                  id="session-message-body"
                  className="min-h-24"
                  value={messageBody}
                  disabled={messageMutation.isPending}
                  onChange={(event) => changeMessageBody(event.target.value)}
                  placeholder={t('sessionDetail.messagePlaceholder')}
                />
              </div>
              {messageMutation.isError ? (
                <ErrorState message={t('sessionDetail.messageError')} />
              ) : null}
              <div className="flex justify-end">
                <Button
                  type="submit"
                  className="h-10"
                  disabled={messageMutation.isPending || !messageBody.trim()}
                >
                  <Send className="h-4 w-4" />
                  {messageMutation.isPending
                    ? t('sessionDetail.sending')
                    : t('sessionDetail.sendMessage')}
                </Button>
              </div>
            </form>
          </section>

          <section aria-labelledby="session-runs-title">
            <SectionHeading
              id="session-runs-title"
              title={t('sessionDetail.runs')}
              count={runs.data?.length}
              refreshing={runs.isFetching && Boolean(runs.data)}
              refreshLabel={t('sessionDetail.refreshing')}
            />
            {runs.isError && runs.data ? (
              <div className="mb-3">
                <RetryState
                  message={t('sessionDetail.runsStale')}
                  onRetry={() => void runs.refetch()}
                />
              </div>
            ) : null}
            {runs.isError && !runs.data ? (
              <RetryState
                message={t('sessionDetail.runsError')}
                onRetry={() => void runs.refetch()}
              />
            ) : runs.isPending ? (
              <EmptyState title={t('sessionDetail.loadingRuns')} />
            ) : runs.data?.length ? (
              <ul className="divide-y divide-border rounded-md border border-border bg-surface">
                {runs.data.map((run) => (
                  <RuntimeRunRow key={run.id} run={run} sessionId={sessionId} />
                ))}
              </ul>
            ) : (
              <EmptyState title={t('sessionDetail.noRuns')} />
            )}
          </section>
        </div>

        <aside className="min-w-0 space-y-6">
          <section
            className="rounded-md border border-border bg-surface p-4"
            aria-labelledby="session-controls-title"
          >
            <h2 id="session-controls-title" className="text-base font-semibold text-text-primary">
              {t('sessionDetail.controls')}
            </h2>
            {agents.isError ? (
              <div className="mt-3">
                <RetryState
                  message={
                    agents.data ? t('sessionDetail.agentsStale') : t('sessionDetail.agentsError')
                  }
                  onRetry={() => void agents.refetch()}
                />
              </div>
            ) : null}

            <form className="mt-4 grid gap-3" onSubmit={submitLeader}>
              <div className="flex items-center gap-2 text-sm font-medium text-text-primary">
                <Crown className="h-4 w-4" />
                {t('sessionDetail.leader')}
              </div>
              <Label className="sr-only" htmlFor="session-leader">
                {t('sessionDetail.sessionLeader')}
              </Label>
              <select
                id="session-leader"
                value={leaderId}
                disabled={!agents.data || teamsLoading || leaderMutation.isPending}
                onChange={(event) => {
                  setLeaderId(event.target.value)
                  leaderMutation.reset()
                }}
                className="h-10 min-w-0 rounded-md border border-border bg-background px-3 text-sm disabled:cursor-not-allowed disabled:opacity-60"
              >
                <option value="">{t('sessionDetail.privateChat')}</option>
                {possibleLeaders.map((leader) => (
                  <option key={leader.id} value={leader.id}>
                    {leader.display_name} ({leader.name})
                  </option>
                ))}
              </select>
              {teamsLoading ? (
                <p className="text-xs text-text-muted">{t('sessionDetail.loadingLeaders')}</p>
              ) : leaderTeams.isError ? (
                <RetryState
                  message={t('sessionDetail.leadersError')}
                  onRetry={() => void leaderTeams.refetch()}
                />
              ) : null}
              {leaderMutation.isError ? (
                <ErrorState message={t('sessionDetail.leaderError')} />
              ) : null}
              <Button
                type="submit"
                variant="outline"
                className="h-10"
                disabled={
                  !agents.data ||
                  teamsLoading ||
                  leaderMutation.isPending ||
                  leaderId === (session.data.leader_agent_id ?? '')
                }
              >
                <Crown className="h-4 w-4" />
                {leaderMutation.isPending
                  ? t('sessionDetail.saving')
                  : t('sessionDetail.saveLeader')}
              </Button>
            </form>

            <form className="mt-5 grid gap-3 border-t border-border pt-5" onSubmit={submitHandoff}>
              <div className="flex items-center gap-2 text-sm font-medium text-text-primary">
                <ArrowRightLeft className="h-4 w-4" />
                {t('sessionDetail.handoff')}
              </div>
              <Label className="sr-only" htmlFor="session-handoff-agent">
                {t('sessionDetail.handoffTarget')}
              </Label>
              <select
                id="session-handoff-agent"
                value={targetAgentId}
                disabled={!agents.data || handoffMutation.isPending}
                onChange={(event) => {
                  setTargetAgentId(event.target.value)
                  handoffMutation.reset()
                }}
                className="h-10 min-w-0 rounded-md border border-border bg-background px-3 text-sm disabled:cursor-not-allowed disabled:opacity-60"
              >
                <option value="">{t('sessionDetail.selectAgent')}</option>
                {agents.data
                  ?.filter((agent) => agent.id !== session.data.primary_agent_id)
                  .map((agent) => (
                    <option key={agent.id} value={agent.id}>
                      {agent.display_name} ({agent.name})
                    </option>
                  ))}
              </select>
              <p className="text-xs text-text-muted">{t('sessionDetail.handoffHelp')}</p>
              {handoffMutation.isError ? (
                <ErrorState message={t('sessionDetail.handoffError')} />
              ) : null}
              <Button
                type="submit"
                variant="outline"
                className="h-10"
                disabled={!targetAgentId || handoffMutation.isPending}
              >
                <ArrowRightLeft className="h-4 w-4" />
                {handoffMutation.isPending
                  ? t('sessionDetail.handingOff')
                  : t('sessionDetail.handoffAction')}
              </Button>
            </form>

            {canManageAgents ? (
              <Button asChild variant="ghost" className="mt-3 h-10 w-full">
                <Link to={`/agents/${session.data.primary_agent_id}/sessions`}>
                  <ExternalLink className="h-4 w-4" />
                  {t('sessionDetail.openAgentSessions')}
                </Link>
              </Button>
            ) : null}
          </section>

          <section aria-labelledby="session-participants-title">
            <SectionHeading
              id="session-participants-title"
              icon={<UsersRound className="h-4 w-4" />}
              title={t('sessionDetail.participants')}
              count={participants.data?.length}
            />
            <ParticipantsSection
              participants={participants.data}
              pending={participants.isPending}
              failed={participants.isError}
              onRetry={() => void participants.refetch()}
            />
          </section>

          <section
            className="rounded-md border border-border bg-surface p-4"
            aria-labelledby="session-delegation-title"
          >
            <h2 id="session-delegation-title" className="text-base font-semibold text-text-primary">
              {t('sessionDetail.delegation')}
            </h2>
            <p className="mt-1 text-xs text-text-muted">{t('sessionDetail.delegationHelp')}</p>
            <form
              className="mt-4 grid gap-3"
              onSubmit={submitDelegation}
              aria-busy={delegationMutation.isPending}
            >
              <div className="grid gap-2">
                <Label htmlFor="delegation-executor">{t('sessionDetail.executor')}</Label>
                <select
                  id="delegation-executor"
                  value={delegationExecutorId}
                  disabled={
                    !session.data.leader_agent_id ||
                    teamsLoading ||
                    (leaderTeams.isError && !leaderTeams.data) ||
                    delegationMutation.isPending
                  }
                  onChange={(event) => {
                    setDelegationExecutorId(event.target.value)
                    resetDelegationRequest()
                  }}
                  className="h-10 min-w-0 rounded-md border border-border bg-background px-3 text-sm disabled:cursor-not-allowed disabled:opacity-60"
                >
                  <option value="">{t('sessionDetail.selectExecutor')}</option>
                  {delegationExecutors.map((executor) => (
                    <option key={executor.executor_agent_id} value={executor.executor_agent_id}>
                      {executor.executor_display_name} ({executor.executor_name})
                    </option>
                  ))}
                </select>
              </div>
              <div className="grid gap-2">
                <Label htmlFor="delegation-title">{t('sessionDetail.delegationTitle')}</Label>
                <Input
                  id="delegation-title"
                  className="h-10"
                  value={delegationTitle}
                  required
                  disabled={delegationMutation.isPending}
                  onChange={(event) => {
                    setDelegationTitle(event.target.value)
                    resetDelegationRequest()
                  }}
                />
              </div>
              <div className="grid gap-2">
                <Label htmlFor="delegation-message">{t('sessionDetail.delegationMessage')}</Label>
                <Textarea
                  id="delegation-message"
                  className="min-h-20"
                  value={delegationMessage}
                  disabled={delegationMutation.isPending}
                  onChange={(event) => {
                    setDelegationMessage(event.target.value)
                    resetDelegationRequest()
                  }}
                  placeholder={t('sessionDetail.delegationPlaceholder')}
                />
              </div>
              {!session.data.leader_agent_id ? (
                <p className="text-xs text-text-muted">
                  {t('sessionDetail.delegationNeedsLeader')}
                </p>
              ) : !delegationExecutors.length && !teamsLoading && !leaderTeams.isError ? (
                <p className="text-xs text-text-muted">{t('sessionDetail.noExecutors')}</p>
              ) : null}
              {delegationMutation.isError ? (
                <ErrorState message={t('sessionDetail.delegationError')} />
              ) : null}
              <Button
                type="submit"
                className="h-10"
                disabled={
                  delegationMutation.isPending ||
                  !session.data.leader_agent_id ||
                  !delegationExecutorId ||
                  !delegationTitle.trim()
                }
              >
                <ArrowRightLeft className="h-4 w-4" />
                {delegationMutation.isPending
                  ? t('sessionDetail.delegating')
                  : t('sessionDetail.delegate')}
              </Button>
            </form>
          </section>
        </aside>
      </div>
    </>
  )
}

function SessionSummary({ session }: { session: AgentSession }) {
  const { t } = useTranslation()
  return (
    <section className="border-y border-border py-4" aria-label={t('sessionDetail.summary')}>
      <div className="flex min-w-0 flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
        <div className="flex min-w-0 items-center gap-3">
          <UserAvatar name={session.user_display_name} userId={session.user_id} size="md" />
          <div className="min-w-0">
            <p className="truncate text-sm font-medium text-text-primary">
              {session.user_display_name}
            </p>
            <p className="truncate text-xs text-text-muted">{session.user_email}</p>
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <StatusBadge value={session.state} />
          <StatusBadge value={session.visibility} />
          {session.task_key ? (
            <span className="text-sm font-medium text-text-secondary">{session.task_key}</span>
          ) : null}
        </div>
      </div>
      <dl className="mt-4 grid gap-x-6 gap-y-3 text-sm sm:grid-cols-2 xl:grid-cols-3">
        <Field label={t('sessionDetail.primaryAgent')} value={session.primary_agent_name} />
        <Field
          label={t('sessionDetail.leader')}
          value={session.leader_agent_name ?? t('sessionDetail.privateValue')}
        />
        <Field
          label={t('sessionDetail.namespace')}
          value={session.namespace_id ?? t('sessionDetail.unboundValue')}
        />
        <Field
          label={t('sessionDetail.parentSession')}
          value={session.parent_session_id ?? t('sessionDetail.noneValue')}
          technical={Boolean(session.parent_session_id)}
        />
        <Field
          label={t('sessionDetail.externalSession')}
          value={session.external_session_id ?? t('sessionDetail.noneValue')}
          technical={Boolean(session.external_session_id)}
        />
        <Field label={t('sessionDetail.updated')} value={formatDate(session.updated_at)} />
      </dl>
    </section>
  )
}

function MessagesSection({
  messages,
  pending,
  failed,
  onRetry,
}: {
  messages: SessionMessage[] | undefined
  pending: boolean
  failed: boolean
  onRetry: () => void
}) {
  const { t } = useTranslation()
  if (failed && !messages) {
    return <RetryState message={t('sessionDetail.messagesError')} onRetry={onRetry} />
  }
  if (pending) return <EmptyState title={t('sessionDetail.loadingMessages')} />
  return (
    <>
      {failed ? (
        <div className="mb-3">
          <RetryState message={t('sessionDetail.messagesStale')} onRetry={onRetry} />
        </div>
      ) : null}
      {messages?.length ? (
        <ol className="divide-y divide-border rounded-md border border-border bg-surface">
          {messages.map((message) => (
            <li key={message.id} className="min-w-0 p-4">
              <div className="flex flex-wrap items-center gap-2 text-xs text-text-muted">
                <span className="font-medium text-text-primary">{message.author_display_name}</span>
                <StatusBadge value={message.author_type} />
                <StatusBadge value={message.message_kind} />
                <StatusBadge value={message.delivery_state} />
                <time dateTime={message.created_at}>{formatDate(message.created_at)}</time>
              </div>
              <p className="mt-2 whitespace-pre-wrap break-words text-sm text-text-secondary">
                {message.body}
              </p>
              {message.runtime_message_id || message.delivery_error ? (
                <details className="mt-2">
                  <summary className="flex min-h-10 cursor-pointer items-center text-xs font-medium text-accent">
                    {t('sessionDetail.technicalDetails')}
                  </summary>
                  <div className="space-y-1 border-l border-border pl-3 text-xs text-text-muted">
                    {message.runtime_message_id ? (
                      <p className="break-all">
                        {t('sessionDetail.runtimeMessage')}: {message.runtime_message_id}
                      </p>
                    ) : null}
                    {message.delivery_error ? (
                      <p className="break-words text-danger">{message.delivery_error}</p>
                    ) : null}
                  </div>
                </details>
              ) : null}
            </li>
          ))}
        </ol>
      ) : (
        <EmptyState title={t('sessionDetail.noMessages')} />
      )}
    </>
  )
}

function ParticipantsSection({
  participants,
  pending,
  failed,
  onRetry,
}: {
  participants: SessionParticipant[] | undefined
  pending: boolean
  failed: boolean
  onRetry: () => void
}) {
  const { t } = useTranslation()
  if (failed && !participants) {
    return <RetryState message={t('sessionDetail.participantsError')} onRetry={onRetry} />
  }
  if (pending) return <EmptyState title={t('sessionDetail.loadingParticipants')} />
  return (
    <>
      {failed ? (
        <div className="mb-3">
          <RetryState message={t('sessionDetail.participantsStale')} onRetry={onRetry} />
        </div>
      ) : null}
      {participants?.length ? (
        <ul className="divide-y divide-border rounded-md border border-border bg-surface">
          {participants.map((participant) => (
            <li key={participant.id} className="min-w-0 p-3">
              <p className="break-words text-sm font-medium text-text-primary">
                {participant.display_name}
              </p>
              <div className="mt-2 flex flex-wrap gap-2">
                <StatusBadge value={participant.participant_type} />
                <StatusBadge value={participant.session_role} />
              </div>
            </li>
          ))}
        </ul>
      ) : (
        <EmptyState title={t('sessionDetail.noParticipants')} />
      )}
    </>
  )
}

function RuntimeRunRow({ run, sessionId }: { run: SessionAgentRun; sessionId: string }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const [steerDraft, setSteerDraft] = useState('')
  const canControl = run.state === 'running' || run.state === 'waiting'
  const stopMutation = useMutation({
    mutationFn: () => stopSessionRun(sessionId, run.id),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ['session-runs', sessionId] })
      toast.success(t('sessionDetail.stopSuccess', { agent: run.agent_name }))
    },
  })
  const steerMutation = useMutation({
    mutationFn: () => steerSessionRun(sessionId, run.id, { input: steerDraft.trim() }),
    onSuccess: async () => {
      setSteerDraft('')
      await queryClient.invalidateQueries({ queryKey: ['session-runs', sessionId] })
      toast.success(t('sessionDetail.steerSuccess', { agent: run.agent_name }))
    },
  })
  const approvalMutation = useMutation({
    mutationFn: (choice: 'approve' | 'deny') =>
      resolveSessionRunApproval(sessionId, run.id, { choice, resolve_all: true }),
    onSuccess: async (_response, choice) => {
      await queryClient.invalidateQueries({ queryKey: ['session-runs', sessionId] })
      toast.success(
        choice === 'approve'
          ? t('sessionDetail.approveSuccess', { agent: run.agent_name })
          : t('sessionDetail.denySuccess', { agent: run.agent_name }),
      )
    },
  })
  const actionPending =
    stopMutation.isPending || steerMutation.isPending || approvalMutation.isPending
  const actionFailed = stopMutation.isError || steerMutation.isError || approvalMutation.isError

  function submitSteer(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (steerDraft.trim() && !actionPending) steerMutation.mutate()
  }

  return (
    <li className="min-w-0 p-4">
      <div className="flex min-w-0 flex-wrap items-center gap-2">
        <span className="break-words font-medium text-text-primary">{run.agent_name}</span>
        <StatusBadge value={run.run_role} />
        <StatusBadge value={run.state} />
      </div>
      {run.last_error ? (
        <p className="mt-2 break-words text-xs text-danger">{run.last_error}</p>
      ) : null}
      <details className="mt-2">
        <summary className="flex min-h-10 cursor-pointer items-center text-xs font-medium text-accent">
          {t('sessionDetail.technicalDetails')}
        </summary>
        <dl className="grid gap-2 border-l border-border pl-3 text-xs sm:grid-cols-2">
          <Field
            label={t('sessionDetail.runtimeSession')}
            value={run.runtime_session_id ?? t('sessionDetail.pendingValue')}
            technical
          />
          <Field
            label={t('sessionDetail.runtimeRun')}
            value={run.runtime_run_id ?? t('sessionDetail.notDispatchedValue')}
            technical
          />
          <Field
            label={t('sessionDetail.model')}
            value={run.model ?? t('sessionDetail.noneValue')}
          />
          <Field
            label={t('sessionDetail.lastEvent')}
            value={
              run.last_event_at ? formatDate(run.last_event_at) : t('sessionDetail.neverValue')
            }
          />
        </dl>
      </details>
      {canControl ? (
        <div className="mt-3 grid gap-3">
          <div className="flex flex-wrap gap-2">
            <AlertDialog>
              <AlertDialogTrigger asChild>
                <Button
                  type="button"
                  variant="destructive"
                  className="h-10"
                  disabled={actionPending}
                >
                  <Square className="h-4 w-4" />
                  {t('sessionDetail.stop')}
                </Button>
              </AlertDialogTrigger>
              <AlertDialogContent>
                <AlertDialogHeader>
                  <AlertDialogTitle>{t('sessionDetail.stopConfirmTitle')}</AlertDialogTitle>
                  <AlertDialogDescription>
                    {t('sessionDetail.stopConfirmDescription', { agent: run.agent_name })}
                  </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                  <AlertDialogCancel>{t('sessionDetail.cancel')}</AlertDialogCancel>
                  <AlertDialogAction onClick={() => stopMutation.mutate()}>
                    {t('sessionDetail.stopAction')}
                  </AlertDialogAction>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
            {run.state === 'waiting' ? (
              <>
                <Button
                  type="button"
                  variant="outline"
                  className="h-10"
                  onClick={() => approvalMutation.mutate('approve')}
                  disabled={actionPending}
                >
                  {t('sessionDetail.approve')}
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  className="h-10"
                  onClick={() => approvalMutation.mutate('deny')}
                  disabled={actionPending}
                >
                  {t('sessionDetail.deny')}
                </Button>
              </>
            ) : null}
          </div>
          <form className="grid gap-2 sm:grid-cols-[1fr_auto]" onSubmit={submitSteer}>
            <Label className="sr-only" htmlFor={`steer-${run.id}`}>
              {t('sessionDetail.steerLabel', { agent: run.agent_name })}
            </Label>
            <Input
              id={`steer-${run.id}`}
              className="h-10 min-w-0"
              value={steerDraft}
              disabled={actionPending}
              onChange={(event) => {
                setSteerDraft(event.target.value)
                steerMutation.reset()
              }}
              placeholder={t('sessionDetail.steerPlaceholder')}
            />
            <Button
              type="submit"
              variant="outline"
              className="h-10"
              disabled={actionPending || !steerDraft.trim()}
            >
              {steerMutation.isPending ? t('sessionDetail.sending') : t('sessionDetail.steer')}
            </Button>
          </form>
          {actionFailed ? <ErrorState message={t('sessionDetail.runActionError')} /> : null}
        </div>
      ) : null}
    </li>
  )
}

function SectionHeading({
  id,
  icon,
  title,
  count,
  refreshing,
  refreshLabel,
}: {
  id: string
  icon?: ReactNode
  title: string
  count?: number
  refreshing?: boolean
  refreshLabel?: string
}) {
  return (
    <div className="mb-3 flex min-h-10 flex-wrap items-center justify-between gap-2">
      <h2 id={id} className="flex items-center gap-2 text-base font-semibold text-text-primary">
        {icon}
        {title}
        {typeof count === 'number' ? (
          <span className="text-sm font-normal text-text-muted">({count})</span>
        ) : null}
      </h2>
      {refreshing ? (
        <span className="inline-flex items-center gap-2 text-xs text-text-muted">
          <RefreshCw className="h-3.5 w-3.5 animate-spin" />
          {refreshLabel}
        </span>
      ) : null}
    </div>
  )
}

function RetryState({ message, onRetry }: { message: string; onRetry: () => void }) {
  const { t } = useTranslation()
  return (
    <div className="space-y-2">
      <ErrorState message={message} />
      <Button type="button" variant="outline" className="h-10" onClick={onRetry}>
        <RefreshCw className="h-4 w-4" />
        {t('sessionDetail.retry')}
      </Button>
    </div>
  )
}

function Field({
  label,
  value,
  technical = false,
}: {
  label: string
  value: string
  technical?: boolean
}) {
  return (
    <div className="min-w-0">
      <dt className="text-xs text-text-muted">{label}</dt>
      <dd
        className={
          technical
            ? 'break-all font-mono text-xs font-medium text-text-primary'
            : 'break-words font-medium text-text-primary'
        }
      >
        {value}
      </dd>
    </div>
  )
}
