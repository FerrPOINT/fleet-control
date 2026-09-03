import { FormEvent, useEffect, useMemo, useState } from 'react'
import { Link, useParams } from 'react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowRightLeft, Crown, Send, UsersRound } from 'lucide-react'
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
import type { SessionAgentRun } from '@/api/types'
import { useAuthStore } from '@/shared/auth/store'
import { Input } from '@/shared/ui/input'
import { Label } from '@/shared/ui/label'
import { Button } from '@/shared/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/shared/ui/card'
import { Textarea } from '@/shared/ui/textarea'
import { UserAvatar } from '@/shared/ui/user-avatar'
import { EmptyState, ErrorState, PageHeader, StatusBadge, formatDate } from '../common'

export function SessionDetailPage() {
  const { sessionId } = useParams()
  const queryClient = useQueryClient()
  const session = useQuery({
    queryKey: ['session', sessionId],
    queryFn: () => getSession(sessionId!),
    enabled: Boolean(sessionId),
  })
  const agents = useQuery({ queryKey: ['agent-directory'], queryFn: listAgentDirectory })
  const leaders = agents.data?.filter((agent) => agent.product_role === 'leader') ?? []
  const leaderTeams = useQuery({
    queryKey: ['leader-teams', leaders.map((leader) => leader.id).join(',')],
    enabled: Boolean(leaders.length),
    queryFn: async () => {
      return Promise.all(
        leaders.map(async (leader) => ({
          leader,
          executors: await listLeaderExecutors(leader.id),
        })),
      )
    },
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
  const canManageAgents = useAuthStore((state) => state.permissions.includes('agents:manage'))
  const [targetAgentId, setTargetAgentId] = useState('')
  const [leaderId, setLeaderId] = useState('')
  const [messageBody, setMessageBody] = useState('')
  const [authorMode, setAuthorMode] = useState<'user' | 'leader'>('user')
  const [delegationExecutorId, setDelegationExecutorId] = useState('')
  const [delegationTitle, setDelegationTitle] = useState('Delegated executor task')
  const [delegationMessage, setDelegationMessage] = useState('')
  const [steerDraftByRun, setSteerDraftByRun] = useState<Record<string, string>>({})
  const primaryAgent = agents.data?.find((agent) => agent.id === session.data?.primary_agent_id)
  const possibleLeaders =
    primaryAgent?.product_role === 'leader'
      ? leaders.filter((leader) => leader.id === primaryAgent.id)
      : (leaderTeams.data
          ?.filter((team) =>
            team.executors.some((executor) => executor.executor_agent_id === primaryAgent?.id),
          )
          .map((team) => team.leader) ?? [])
  const delegationExecutors = useMemo(
    () =>
      leaderTeams.data?.find((team) => team.leader.id === session.data?.leader_agent_id)
        ?.executors ?? [],
    [leaderTeams.data, session.data?.leader_agent_id],
  )

  useEffect(() => {
    setLeaderId(session.data?.leader_agent_id ?? '')
  }, [session.data?.leader_agent_id])

  useEffect(() => {
    if (!delegationExecutorId && delegationExecutors[0]) {
      setDelegationExecutorId(delegationExecutors[0].executor_agent_id)
    }
  }, [delegationExecutorId, delegationExecutors])

  const mutation = useMutation({
    mutationFn: () => handoffSession(sessionId!, { target_agent_id: targetAgentId }),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['session', sessionId] }),
        queryClient.invalidateQueries({ queryKey: ['sessions'] }),
      ])
    },
  })
  const leaderMutation = useMutation({
    mutationFn: () => assignSessionLeader(sessionId!, { leader_agent_id: leaderId || null }),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['session', sessionId] }),
        queryClient.invalidateQueries({ queryKey: ['session-messages', sessionId] }),
        queryClient.invalidateQueries({ queryKey: ['session-runs', sessionId] }),
        queryClient.invalidateQueries({ queryKey: ['sessions'] }),
      ])
    },
  })
  const messageMutation = useMutation({
    mutationFn: () =>
      createSessionMessage(sessionId!, {
        body: messageBody,
        author_agent_id:
          authorMode === 'leader' && session.data?.leader_agent_id
            ? session.data.leader_agent_id
            : null,
      }),
    onSuccess: async () => {
      setMessageBody('')
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['session', sessionId] }),
        queryClient.invalidateQueries({ queryKey: ['session-messages', sessionId] }),
        queryClient.invalidateQueries({ queryKey: ['session-runs', sessionId] }),
        queryClient.invalidateQueries({ queryKey: ['sessions'] }),
      ])
    },
  })
  const stopRunMutation = useMutation({
    mutationFn: (runId: string) => stopSessionRun(sessionId!, runId),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['session-runs', sessionId] }),
  })
  const steerRunMutation = useMutation({
    mutationFn: ({ runId, input }: { runId: string; input: string }) =>
      steerSessionRun(sessionId!, runId, { input }),
    onSuccess: async (_response, variables) => {
      setSteerDraftByRun((drafts) => ({ ...drafts, [variables.runId]: '' }))
      await queryClient.invalidateQueries({ queryKey: ['session-runs', sessionId] })
    },
  })
  const approvalMutation = useMutation({
    mutationFn: ({ runId, choice }: { runId: string; choice: string }) =>
      resolveSessionRunApproval(sessionId!, runId, { choice, resolve_all: true }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['session-runs', sessionId] }),
  })
  const delegationMutation = useMutation({
    mutationFn: () =>
      createSessionDelegation(sessionId!, {
        executor_agent_id: delegationExecutorId,
        title: delegationTitle,
        initial_message: delegationMessage || null,
        idempotency_key: globalThis.crypto?.randomUUID?.() ?? null,
      }),
    onSuccess: async () => {
      setDelegationMessage('')
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['sessions'] }),
        queryClient.invalidateQueries({ queryKey: ['session', sessionId] }),
        queryClient.invalidateQueries({ queryKey: ['session-messages', sessionId] }),
      ])
    },
  })

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (targetAgentId) mutation.mutate()
  }

  function submitLeader(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    leaderMutation.mutate()
  }

  function submitMessage(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (messageBody.trim()) messageMutation.mutate()
  }

  function submitDelegation(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (delegationExecutorId) delegationMutation.mutate()
  }

  if (!sessionId) return <ErrorState message="Session id is missing" />
  if (session.isError) return <ErrorState message={session.error.message} />

  return (
    <>
      <PageHeader
        title={session.data?.title ?? 'Session'}
        description="Task chat metadata and cross-agent handoff."
      />
      <div className="grid gap-4 xl:grid-cols-[1fr_360px]">
        <Card>
          <CardHeader>
            <CardTitle>Session state</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            {session.data ? (
              <>
                <div className="flex flex-wrap items-center gap-2">
                  <StatusBadge value={session.data.state} />
                  <StatusBadge value={session.data.visibility} />
                  <span className="text-sm text-text-muted">
                    {session.data.task_key ?? 'No task key'}
                  </span>
                </div>
                <div className="flex min-w-0 items-center gap-3 rounded-md border border-border bg-background p-3">
                  <UserAvatar
                    name={session.data.user_display_name}
                    userId={session.data.user_id}
                    size="md"
                  />
                  <div className="min-w-0">
                    <p className="truncate text-sm font-medium text-text-primary">
                      {session.data.user_display_name}
                    </p>
                    <p className="truncate text-xs text-text-muted">{session.data.user_email}</p>
                  </div>
                </div>
                <dl className="grid gap-3 text-sm md:grid-cols-2">
                  <Field label="Primary agent" value={session.data.primary_agent_name} />
                  <Field label="Leader" value={session.data.leader_agent_name ?? 'private'} />
                  <Field label="Namespace" value={session.data.namespace_id ?? 'unbound'} />
                  <Field label="Parent session" value={session.data.parent_session_id ?? 'none'} />
                  <Field
                    label="External session"
                    value={session.data.external_session_id ?? 'none'}
                  />
                  <Field label="Updated" value={formatDate(session.data.updated_at)} />
                </dl>
              </>
            ) : null}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Crown className="h-4 w-4" />
              Leader
            </CardTitle>
          </CardHeader>
          <CardContent>
            <form className="grid gap-3" onSubmit={submitLeader}>
              <select
                aria-label="Session leader"
                value={leaderId}
                onChange={(event) => setLeaderId(event.target.value)}
                className="h-9 rounded-md border border-border bg-background px-3 text-sm"
              >
                <option value="">Private chat</option>
                {possibleLeaders.map((leader) => (
                  <option key={leader.id} value={leader.id}>
                    {leader.name} - {leader.display_name}
                  </option>
                ))}
              </select>
              {leaderMutation.isError ? (
                <ErrorState message={leaderMutation.error.message} />
              ) : null}
              <Button type="submit" disabled={leaderMutation.isPending}>
                <Crown className="h-4 w-4" />
                Save leader
              </Button>
            </form>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Handoff</CardTitle>
          </CardHeader>
          <CardContent>
            <form className="grid gap-3" onSubmit={submit}>
              <select
                aria-label="Handoff target agent"
                value={targetAgentId}
                onChange={(event) => setTargetAgentId(event.target.value)}
                className="h-9 rounded-md border border-border bg-background px-3 text-sm"
              >
                <option value="">Select target agent</option>
                {agents.data
                  ?.filter((agent) => agent.id !== session.data?.primary_agent_id)
                  .map((agent) => (
                    <option key={agent.id} value={agent.id}>
                      {agent.name} - {agent.display_name}
                    </option>
                  ))}
              </select>
              {mutation.isError ? <ErrorState message={mutation.error.message} /> : null}
              <Button type="submit" disabled={!targetAgentId || mutation.isPending}>
                <ArrowRightLeft className="h-4 w-4" />
                Handoff session
              </Button>
              {session.data && canManageAgents ? (
                <Button asChild variant="outline">
                  <Link to={`/agents/${session.data.primary_agent_id}/sessions`}>
                    Open agent sessions
                  </Link>
                </Button>
              ) : null}
            </form>
          </CardContent>
        </Card>
      </div>
      <div className="mt-4 grid gap-4 xl:grid-cols-[1fr_360px]">
        <Card>
          <CardHeader>
            <CardTitle>Transcript mirror</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            {messages.data?.length ? (
              messages.data.map((message) => (
                <div key={message.id} className="rounded-md border border-border bg-background p-3">
                  <div className="flex flex-wrap items-center gap-2 text-xs text-text-muted">
                    <StatusBadge value={message.author_type} />
                    <StatusBadge value={message.message_kind} />
                    <StatusBadge value={message.delivery_state} />
                    <span>{message.author_display_name}</span>
                    <span>{formatDate(message.created_at)}</span>
                  </div>
                  <p className="mt-2 whitespace-pre-wrap text-sm text-text-secondary">
                    {message.body}
                  </p>
                  {message.runtime_message_id ? (
                    <p className="mt-2 break-all text-xs text-text-muted">
                      runtime {message.runtime_message_id}
                    </p>
                  ) : null}
                  {message.delivery_error ? (
                    <p className="mt-2 text-xs text-danger">{message.delivery_error}</p>
                  ) : null}
                </div>
              ))
            ) : (
              <EmptyState
                title={messages.isLoading ? 'Loading transcript...' : 'No mirrored messages'}
              />
            )}
            <form className="grid gap-3" onSubmit={submitMessage}>
              <select
                aria-label="Message author"
                value={authorMode}
                onChange={(event) => setAuthorMode(event.target.value as 'user' | 'leader')}
                className="h-9 rounded-md border border-border bg-background px-3 text-sm"
              >
                <option value="user">Send as me</option>
                <option value="leader" disabled={!session.data?.leader_agent_id}>
                  Send as selected leader
                </option>
              </select>
              <Textarea
                className="min-h-24"
                value={messageBody}
                onChange={(event) => setMessageBody(event.target.value)}
                placeholder="Write a session message"
              />
              {messageMutation.isError ? (
                <ErrorState message={messageMutation.error.message} />
              ) : null}
              <Button type="submit" disabled={messageMutation.isPending || !messageBody.trim()}>
                <Send className="h-4 w-4" />
                Send message
              </Button>
            </form>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Runtime runs</CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            {runs.data?.length ? (
              runs.data.map((run) => (
                <RuntimeRunCard
                  key={run.id}
                  run={run}
                  steerDraft={steerDraftByRun[run.id] ?? ''}
                  onSteerDraftChange={(value) =>
                    setSteerDraftByRun((drafts) => ({ ...drafts, [run.id]: value }))
                  }
                  onStop={() => stopRunMutation.mutate(run.id)}
                  onSteer={() =>
                    steerRunMutation.mutate({
                      runId: run.id,
                      input: steerDraftByRun[run.id] ?? '',
                    })
                  }
                  onApprove={() => approvalMutation.mutate({ runId: run.id, choice: 'always' })}
                  onDeny={() => approvalMutation.mutate({ runId: run.id, choice: 'deny' })}
                  isMutating={
                    stopRunMutation.isPending ||
                    steerRunMutation.isPending ||
                    approvalMutation.isPending
                  }
                />
              ))
            ) : (
              <EmptyState title={runs.isLoading ? 'Loading runs...' : 'No runtime runs'} />
            )}
            {stopRunMutation.isError ? (
              <ErrorState message={stopRunMutation.error.message} />
            ) : null}
            {steerRunMutation.isError ? (
              <ErrorState message={steerRunMutation.error.message} />
            ) : null}
            {approvalMutation.isError ? (
              <ErrorState message={approvalMutation.error.message} />
            ) : null}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <UsersRound className="h-4 w-4" />
              Participants
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            {participants.data?.length ? (
              participants.data.map((participant) => (
                <div key={participant.id} className="rounded-md border border-border p-3 text-sm">
                  <div className="flex flex-wrap items-center gap-2">
                    <StatusBadge value={participant.participant_type} />
                    <StatusBadge value={participant.session_role} />
                    <span className="font-medium text-text-primary">
                      {participant.display_name}
                    </span>
                  </div>
                </div>
              ))
            ) : (
              <EmptyState
                title={participants.isLoading ? 'Loading participants...' : 'No participants'}
              />
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Delegation</CardTitle>
          </CardHeader>
          <CardContent>
            <form className="grid gap-3" onSubmit={submitDelegation}>
              <div className="grid gap-2">
                <Label htmlFor="delegation-executor">Executor</Label>
                <select
                  id="delegation-executor"
                  value={delegationExecutorId}
                  onChange={(event) => setDelegationExecutorId(event.target.value)}
                  className="h-9 rounded-md border border-border bg-background px-3 text-sm"
                  disabled={!session.data?.leader_agent_id}
                >
                  <option value="">Select executor</option>
                  {delegationExecutors.map((executor) => (
                    <option key={executor.executor_agent_id} value={executor.executor_agent_id}>
                      {executor.executor_name} - {executor.executor_display_name}
                    </option>
                  ))}
                </select>
              </div>
              <div className="grid gap-2">
                <Label htmlFor="delegation-title">Title</Label>
                <Input
                  id="delegation-title"
                  value={delegationTitle}
                  onChange={(event) => setDelegationTitle(event.target.value)}
                />
              </div>
              <Textarea
                className="min-h-20"
                value={delegationMessage}
                onChange={(event) => setDelegationMessage(event.target.value)}
                placeholder="Initial task for the executor"
              />
              {delegationMutation.isError ? (
                <ErrorState message={delegationMutation.error.message} />
              ) : null}
              <Button
                type="submit"
                disabled={
                  delegationMutation.isPending ||
                  !session.data?.leader_agent_id ||
                  !delegationExecutorId
                }
              >
                <ArrowRightLeft className="h-4 w-4" />
                Delegate task
              </Button>
            </form>
          </CardContent>
        </Card>
      </div>
    </>
  )
}

function RuntimeRunCard({
  run,
  steerDraft,
  onSteerDraftChange,
  onStop,
  onSteer,
  onApprove,
  onDeny,
  isMutating,
}: {
  run: SessionAgentRun
  steerDraft: string
  onSteerDraftChange: (value: string) => void
  onStop: () => void
  onSteer: () => void
  onApprove: () => void
  onDeny: () => void
  isMutating: boolean
}) {
  const canControl = run.state === 'running' || run.state === 'waiting'
  return (
    <div className="rounded-md border border-border p-3 text-sm">
      <div className="flex flex-wrap items-center gap-2">
        <span className="font-medium text-text-primary">{run.agent_name}</span>
        <StatusBadge value={run.run_role} />
        <StatusBadge value={run.state} />
      </div>
      <p className="mt-1 break-all text-xs text-text-muted">
        session {run.runtime_session_id ?? 'pending'}
      </p>
      <p className="mt-1 break-all text-xs text-text-muted">
        run {run.runtime_run_id ?? 'not dispatched'}
      </p>
      {run.last_event_at ? (
        <p className="mt-1 text-xs text-text-muted">last event {formatDate(run.last_event_at)}</p>
      ) : null}
      {run.last_error ? <p className="mt-2 text-xs text-danger">{run.last_error}</p> : null}
      {canControl ? (
        <div className="mt-3 grid gap-2">
          <div className="flex flex-wrap gap-2">
            <Button size="sm" variant="outline" onClick={onStop} disabled={isMutating}>
              Stop
            </Button>
            {run.state === 'waiting' ? (
              <>
                <Button size="sm" variant="outline" onClick={onApprove} disabled={isMutating}>
                  Approve
                </Button>
                <Button size="sm" variant="outline" onClick={onDeny} disabled={isMutating}>
                  Deny
                </Button>
              </>
            ) : null}
          </div>
          <div className="grid gap-2">
            <Input
              value={steerDraft}
              onChange={(event) => onSteerDraftChange(event.target.value)}
              placeholder="Steer this run"
            />
            <Button
              size="sm"
              variant="outline"
              onClick={onSteer}
              disabled={isMutating || !steerDraft.trim()}
            >
              Steer
            </Button>
          </div>
        </div>
      ) : null}
    </div>
  )
}

function Field({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt className="text-xs text-text-muted">{label}</dt>
      <dd className="break-words font-medium text-text-primary">{value}</dd>
    </div>
  )
}
