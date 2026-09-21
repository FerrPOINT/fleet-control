import { FormEvent, useEffect, useState } from 'react'
import { Link } from 'react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { MessageSquarePlus } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import { createSession, listAgentDirectory, listLeaderExecutors, listSessions } from '@/api/fleet'
import { SessionUserFilter, useSessionUserFilter } from '@/shared/session-user-filter'
import { Button } from '@sdlc/ui/ui'
import { Card, CardContent, CardHeader, CardTitle } from '@sdlc/ui/ui'
import { Input } from '@sdlc/ui/ui'
import { Label } from '@sdlc/ui/ui'
import { UserAvatar } from '@/shared/ui/user-avatar'
import { EmptyState, ErrorState, PageHeader, StatusBadge, formatDate } from '../common'

function newIdempotencyKey(): string | null {
  return globalThis.crypto?.randomUUID?.() ?? null
}

export function SessionsPage() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const agents = useQuery({ queryKey: ['agent-directory'], queryFn: listAgentDirectory })
  const leaders = agents.data?.filter((agent) => agent.product_role === 'leader') ?? []
  const leaderTeams = useQuery({
    queryKey: ['leader-teams', leaders.map((leader) => leader.id).join(',')],
    enabled: Boolean(leaders.length),
    queryFn: async () => {
      const teams = await Promise.all(
        leaders.map(async (leader) => ({
          leader,
          executors: await listLeaderExecutors(leader.id),
        })),
      )
      return teams
    },
  })
  const userFilter = useSessionUserFilter()
  const sessions = useQuery({
    queryKey: ['sessions', 'list', userFilter.selectedUserIds],
    queryFn: () => listSessions(undefined, userFilter.selectedUserIds),
  })
  const [agentId, setAgentId] = useState('')
  const [leaderId, setLeaderId] = useState('')
  const [title, setTitle] = useState(() => t('sessions.defaultTitle'))
  const [taskKey, setTaskKey] = useState('')
  const [titleTouched, setTitleTouched] = useState(false)
  const [idempotencyKey, setIdempotencyKey] = useState(newIdempotencyKey)
  const selectedAgent = agents.data?.find((agent) => agent.id === agentId) ?? agents.data?.[0]
  const possibleLeaders =
    selectedAgent?.product_role === 'leader'
      ? leaders.filter((leader) => leader.id === selectedAgent.id)
      : (leaderTeams.data
          ?.filter((team) =>
            team.executors.some((executor) => executor.executor_agent_id === selectedAgent?.id),
          )
          .map((team) => team.leader) ?? [])
  const needsLeaderDirectory = selectedAgent?.product_role === 'executor' && leaders.length > 0
  const leaderDirectoryUnavailable = needsLeaderDirectory && leaderTeams.isError
  const leaderDirectoryLoading = needsLeaderDirectory && leaderTeams.isPending
  const titleIsValid = Boolean(title.trim())
  const formUnavailable =
    agents.isPending || agents.isError || !selectedAgent || leaderDirectoryLoading

  const mutation = useMutation({
    mutationFn: () =>
      createSession({
        primary_agent_id: selectedAgent?.id || '',
        title: title.trim(),
        task_key: taskKey.trim() || null,
        leader_agent_id:
          selectedAgent?.product_role === 'leader' ? selectedAgent.id : leaderId || null,
        idempotency_key: idempotencyKey,
      }),
    onSuccess: async (createdSession) => {
      await queryClient.invalidateQueries({ queryKey: ['sessions'] })
      setTitle(t('sessions.defaultTitle'))
      setTaskKey('')
      setTitleTouched(false)
      setIdempotencyKey(newIdempotencyKey())
      toast.success(t('sessions.created', { title: createdSession.title }))
    },
  })

  function resetDraftMutation() {
    mutation.reset()
    setIdempotencyKey(newIdempotencyKey())
  }

  useEffect(() => {
    if (!agentId && agents.data?.[0]) setAgentId(agents.data[0].id)
  }, [agentId, agents.data])

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setTitleTouched(true)
    if (!titleIsValid || formUnavailable || mutation.isPending) return
    mutation.mutate()
  }

  return (
    <>
      <PageHeader title={t('sessions.title')} description={t('sessions.description')} />
      <div className="grid gap-4 xl:grid-cols-[380px_1fr]">
        <Card>
          <CardHeader>
            <CardTitle>{t('sessions.createTitle')}</CardTitle>
          </CardHeader>
          <CardContent>
            <form className="grid gap-3" onSubmit={submit} aria-busy={mutation.isPending}>
              <div className="grid gap-2">
                <Label htmlFor="session-agent">{t('sessions.agent')}</Label>
                <select
                  id="session-agent"
                  value={agentId}
                  disabled={
                    agents.isPending || agents.isError || !agents.data?.length || mutation.isPending
                  }
                  onChange={(event) => {
                    setAgentId(event.target.value)
                    setLeaderId('')
                    resetDraftMutation()
                  }}
                  className="h-10 rounded-md border border-border bg-background px-3 text-sm disabled:cursor-not-allowed disabled:opacity-60"
                >
                  {!agents.data?.length ? (
                    <option value="">
                      {agents.isPending ? t('sessions.loadingAgents') : t('sessions.noAgents')}
                    </option>
                  ) : null}
                  {agents.data?.map((agent) => (
                    <option key={agent.id} value={agent.id}>
                      {agent.display_name} ({agent.name})
                    </option>
                  ))}
                </select>
                {agents.isError ? (
                  <div className="space-y-2">
                    <ErrorState message={t('sessions.agentsError')} />
                    <Button
                      type="button"
                      size="sm"
                      variant="outline"
                      onClick={() => void agents.refetch()}
                    >
                      {t('sessions.retry')}
                    </Button>
                  </div>
                ) : null}
              </div>
              <div className="grid gap-2">
                <Label htmlFor="session-leader">{t('sessions.leader')}</Label>
                <select
                  id="session-leader"
                  value={selectedAgent?.product_role === 'leader' ? selectedAgent.id : leaderId}
                  disabled={
                    !selectedAgent ||
                    selectedAgent.product_role === 'leader' ||
                    leaderDirectoryLoading ||
                    leaderDirectoryUnavailable ||
                    mutation.isPending
                  }
                  onChange={(event) => {
                    setLeaderId(event.target.value)
                    resetDraftMutation()
                  }}
                  aria-describedby="session-leader-help"
                  className="h-10 rounded-md border border-border bg-background px-3 text-sm disabled:cursor-not-allowed disabled:opacity-60"
                >
                  {selectedAgent?.product_role === 'leader' ? null : (
                    <option value="">{t('sessions.privateSession')}</option>
                  )}
                  {possibleLeaders.map((leader) => (
                    <option key={leader.id} value={leader.id}>
                      {leader.display_name} ({leader.name})
                    </option>
                  ))}
                </select>
                <p id="session-leader-help" className="text-xs text-text-muted">
                  {selectedAgent?.product_role === 'leader'
                    ? t('sessions.directLeaderHelp')
                    : leaderDirectoryLoading
                      ? t('sessions.loadingLeaders')
                      : possibleLeaders.length
                        ? t('sessions.leaderHelp')
                        : t('sessions.privateOnlyHelp')}
                </p>
                {leaderDirectoryUnavailable ? (
                  <div className="space-y-2">
                    <ErrorState message={t('sessions.leadersError')} />
                    <Button
                      type="button"
                      size="sm"
                      variant="outline"
                      onClick={() => void leaderTeams.refetch()}
                    >
                      {t('sessions.retry')}
                    </Button>
                  </div>
                ) : null}
              </div>
              <div className="grid gap-2">
                <Label htmlFor="session-title">{t('sessions.sessionTitle')}</Label>
                <Input
                  id="session-title"
                  value={title}
                  className="h-10"
                  required
                  disabled={mutation.isPending}
                  aria-invalid={titleTouched && !titleIsValid}
                  aria-describedby={
                    titleTouched && !titleIsValid ? 'session-title-error' : undefined
                  }
                  onBlur={() => setTitleTouched(true)}
                  onChange={(event) => {
                    setTitle(event.target.value)
                    resetDraftMutation()
                  }}
                />
                {titleTouched && !titleIsValid ? (
                  <p id="session-title-error" role="alert" className="text-sm text-danger">
                    {t('sessions.titleRequired')}
                  </p>
                ) : null}
              </div>
              <div className="grid gap-2">
                <Label htmlFor="task-key">{t('sessions.taskKey')}</Label>
                <Input
                  id="task-key"
                  value={taskKey}
                  className="h-10"
                  disabled={mutation.isPending}
                  onChange={(event) => {
                    setTaskKey(event.target.value)
                    resetDraftMutation()
                  }}
                  placeholder="CARD-123"
                />
                <p className="text-xs text-text-muted">{t('sessions.taskKeyHelp')}</p>
              </div>
              {mutation.isError ? <ErrorState message={t('sessions.createError')} /> : null}
              <Button
                type="submit"
                className="h-10"
                disabled={mutation.isPending || formUnavailable || !titleIsValid}
              >
                <MessageSquarePlus className="h-4 w-4" />
                {mutation.isPending ? t('sessions.creating') : t('sessions.create')}
              </Button>
            </form>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>
              {t('sessions.listTitle')}
              {sessions.data ? (
                <span className="ml-2 text-sm font-normal text-text-muted">
                  {sessions.data.length}
                </span>
              ) : null}
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            <SessionUserFilter filter={userFilter} className="mb-3" />
            {sessions.isError ? (
              <div className="space-y-2">
                <ErrorState message={t('sessions.listError')} />
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  onClick={() => void sessions.refetch()}
                >
                  {t('sessions.retry')}
                </Button>
              </div>
            ) : sessions.isPending ? (
              <EmptyState title={t('sessions.loadingSessions')} />
            ) : sessions.data.length ? (
              <ul className="space-y-2" aria-label={t('sessions.listLabel')}>
                {sessions.data.map((session) => (
                  <li key={session.id}>
                    <Link
                      to={`/sessions/${session.id}`}
                      className="block rounded-md border border-border p-3 transition-colors hover:bg-surface-raised focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
                    >
                      <div className="flex min-w-0 items-start gap-3">
                        <UserAvatar
                          name={session.user_display_name}
                          userId={session.user_id}
                          size="md"
                        />
                        <div className="min-w-0 flex-1">
                          <div className="flex flex-wrap items-center gap-2">
                            <p className="min-w-0 break-words font-medium text-text-primary">
                              {session.title}
                            </p>
                            <StatusBadge value={session.state} />
                            <StatusBadge value={session.visibility} />
                            {session.task_key ? (
                              <span className="font-mono text-xs text-text-muted">
                                {session.task_key}
                              </span>
                            ) : null}
                          </div>
                          <div className="mt-1 flex flex-wrap gap-x-3 gap-y-1 text-xs text-text-muted">
                            <span>
                              {t('sessions.userMeta', { name: session.user_display_name })}
                            </span>
                            <span>{t('sessions.agentMeta', { name: session.agent_name })}</span>
                            <span>
                              {t('sessions.leaderMeta', {
                                name: session.leader_agent_name ?? t('sessions.privateValue'),
                              })}
                            </span>
                            <span>
                              {t('sessions.namespaceMeta', {
                                name: session.namespace_id ?? t('sessions.unboundValue'),
                              })}
                            </span>
                            <span>
                              {t('sessions.updatedMeta', { date: formatDate(session.updated_at) })}
                            </span>
                          </div>
                        </div>
                      </div>
                    </Link>
                  </li>
                ))}
              </ul>
            ) : (
              <EmptyState title={t('sessions.noSessions')} />
            )}
          </CardContent>
        </Card>
      </div>
    </>
  )
}
