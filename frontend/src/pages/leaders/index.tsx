import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useParams } from 'react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Crown, Pencil, Plus, Save, Search, Users } from 'lucide-react'
import { toast } from 'sonner'
import {
  listAgents,
  listExecutors,
  listLeaderExecutors,
  listLeaders,
  listSessions,
  updateLeaderExecutors,
} from '@/api/fleet'
import type { Agent, AgentSession } from '@/api/types'
import { SessionUserFilter, useSessionUserFilter } from '@/shared/session-user-filter'
import { Button, Input } from '@sdlc/ui/ui'
import { Card, CardContent, CardHeader, CardTitle } from '@sdlc/ui/ui'
import { UserAvatar } from '@/shared/ui/user-avatar'
import {
  AgentIdentity,
  EmptyState,
  ErrorState,
  PageHeader,
  StatusBadge,
  formatDate,
} from '../common'

export function LeadersPage() {
  const { t } = useTranslation()
  const leaders = useQuery({ queryKey: ['leaders'], queryFn: listLeaders })
  const userFilter = useSessionUserFilter()
  const sessions = useQuery({
    queryKey: ['sessions', 'leaders', userFilter.selectedUserIds],
    queryFn: () => listSessions(undefined, userFilter.selectedUserIds),
  })
  const [search, setSearch] = useState('')
  const [visibleCount, setVisibleCount] = useState(25)
  const sessionsByLeader = useMemo(() => {
    const grouped = new Map<string, AgentSession[]>()
    for (const session of sessions.data ?? []) {
      if (!session.leader_agent_id) continue
      grouped.set(session.leader_agent_id, [
        ...(grouped.get(session.leader_agent_id) ?? []),
        session,
      ])
    }
    return grouped
  }, [sessions.data])
  const matchingLeaders = useMemo(() => {
    const query = search.trim().toLocaleLowerCase()
    return (leaders.data ?? [])
      .filter((leader) =>
        `${leader.display_name} ${leader.name} ${leader.role}`.toLocaleLowerCase().includes(query),
      )
      .sort((a, b) => a.display_name.localeCompare(b.display_name))
  }, [leaders.data, search])

  return (
    <>
      <PageHeader
        title={t('leaders.title')}
        description={t('leaders.description')}
        actions={
          <Button asChild>
            <Link to="/leaders/new">
              <Plus className="h-4 w-4" />
              {t('leaders.new')}
            </Link>
          </Button>
        }
      />
      <SessionUserFilter filter={userFilter} className="mb-4" />
      <div className="mb-3 flex flex-wrap items-end justify-between gap-3">
        <label className="relative block w-full max-w-sm">
          <span className="sr-only">{t('leaders.search')}</span>
          <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-text-muted" />
          <Input
            value={search}
            onChange={(event) => {
              setSearch(event.target.value)
              setVisibleCount(25)
            }}
            placeholder={t('leaders.search')}
            className="h-10 pl-9"
          />
        </label>
        {!leaders.isLoading && !leaders.isError ? (
          <p className="text-sm text-text-muted">
            {t('leaders.count', {
              shown: Math.min(visibleCount, matchingLeaders.length),
              total: matchingLeaders.length,
            })}
          </p>
        ) : null}
      </div>
      {leaders.isError ? (
        <LoadError message={t('leaders.loadError')} retry={() => void leaders.refetch()} />
      ) : leaders.isLoading ? (
        <EmptyState title={t('leaders.loadingList')} />
      ) : !leaders.data?.length ? (
        <EmptyState title={t('leaders.empty')} />
      ) : !matchingLeaders.length ? (
        <EmptyState title={t('leaders.noMatches')} />
      ) : (
        <>
          {sessions.isError ? (
            <div className="mb-3">
              <LoadError
                message={t('leaders.listSessionsError')}
                retry={() => void sessions.refetch()}
              />
            </div>
          ) : null}
          <ul className="divide-y divide-border border-y border-border">
            {matchingLeaders.slice(0, visibleCount).map((leader) => (
              <LeaderRow
                key={leader.id}
                leader={leader}
                sessions={sessionsByLeader.get(leader.id) ?? []}
                sessionsLoading={sessions.isLoading}
                sessionsFailed={sessions.isError}
              />
            ))}
          </ul>
          {visibleCount < matchingLeaders.length ? (
            <Button
              type="button"
              variant="outline"
              className="mt-4"
              onClick={() => setVisibleCount((current) => current + 25)}
            >
              {t('leaders.showMore')}
            </Button>
          ) : null}
        </>
      )}
    </>
  )
}

function LeaderRow({
  leader,
  sessions,
  sessionsLoading,
  sessionsFailed,
}: {
  leader: Agent
  sessions: AgentSession[]
  sessionsLoading: boolean
  sessionsFailed: boolean
}) {
  const { t } = useTranslation()
  return (
    <li className="min-w-0 py-3">
      <div className="flex min-w-0 flex-wrap items-start justify-between gap-3">
        <AgentIdentity agent={leader} />
        <Button asChild variant="outline" size="sm" className="min-h-10">
          <Link to={`/leaders/${leader.id}`}>{t('leaders.open')}</Link>
        </Button>
      </div>
      {sessionsFailed ? null : sessionsLoading ? (
        <p className="mt-2 text-xs text-text-muted">{t('leaders.loadingSessions')}</p>
      ) : sessions.length ? (
        <details className="mt-2">
          <summary className="inline-flex min-h-10 cursor-pointer items-center text-sm text-accent underline-offset-2 hover:underline focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus">
            {t('leaders.sessionsCount', { count: sessions.length })}
          </summary>
          <div className="mt-2 grid gap-2 xl:grid-cols-2">
            {sessions.map((session) => (
              <SessionLine key={session.id} session={session} />
            ))}
          </div>
        </details>
      ) : (
        <p className="mt-2 text-xs text-text-muted">{t('leaders.noListSessions')}</p>
      )}
    </li>
  )
}

export function LeaderDetailPage() {
  const { t } = useTranslation()
  const { leaderId } = useParams()
  const queryClient = useQueryClient()
  const agents = useQuery({ queryKey: ['agents'], queryFn: listAgents })
  const executors = useQuery({ queryKey: ['executors'], queryFn: listExecutors })
  const team = useQuery({
    queryKey: ['leader-executors', leaderId],
    queryFn: () => listLeaderExecutors(leaderId!),
    enabled: Boolean(leaderId),
  })
  const userFilter = useSessionUserFilter()
  const sessions = useQuery({
    queryKey: ['sessions', 'leader-detail', leaderId, userFilter.selectedUserIds],
    queryFn: () => listSessions(undefined, userFilter.selectedUserIds, leaderId),
    enabled: Boolean(leaderId),
  })
  const [selectedExecutorIds, setSelectedExecutorIds] = useState<string[] | null>(null)
  const leader = agents.data?.find((agent) => agent.id === leaderId) ?? null
  const currentTeamIds = useMemo(
    () => team.data?.map((item) => item.executor_agent_id) ?? [],
    [team.data],
  )
  const draftExecutorIds = selectedExecutorIds ?? currentTeamIds
  const hasTeamChanges =
    selectedExecutorIds !== null &&
    (selectedExecutorIds.length !== currentTeamIds.length ||
      currentTeamIds.some((id) => !selectedExecutorIds.includes(id)))
  const teamReady = team.isSuccess && executors.isSuccess
  const mutation = useMutation({
    mutationFn: (executorIds: string[]) =>
      updateLeaderExecutors(leaderId!, { executor_ids: executorIds }),
    onSuccess: async (updatedTeam) => {
      queryClient.setQueryData(['leader-executors', leaderId], updatedTeam)
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['leader-executors', leaderId] }),
        queryClient.invalidateQueries({ queryKey: ['leaders'] }),
      ])
      setSelectedExecutorIds(null)
      toast.success(t('leaders.teamSaved'))
    },
  })

  function toggleExecutor(executorId: string) {
    setSelectedExecutorIds(
      draftExecutorIds.includes(executorId)
        ? draftExecutorIds.filter((id) => id !== executorId)
        : [...draftExecutorIds, executorId],
    )
  }

  if (!leaderId) return <ErrorState message={t('leaders.missingId')} />
  if (agents.isError) return <ErrorState message={agents.error.message} />
  if (!leader)
    return <EmptyState title={agents.isLoading ? t('leaders.loading') : t('leaders.notFound')} />

  return (
    <>
      <PageHeader
        title={leader.display_name}
        description={t('leaders.detailDescription', { name: leader.name })}
        actions={
          <>
            <Button asChild variant="outline">
              <Link to={`/leaders/${leader.id}/edit`}>
                <Pencil className="h-4 w-4" />
                {t('leaders.edit')}
              </Link>
            </Button>
            <Button asChild variant="outline">
              <Link to={`/agents/${leader.id}`}>{t('leaders.technicalDetails')}</Link>
            </Button>
          </>
        }
      />
      <div className="grid gap-4 xl:grid-cols-[420px_1fr]">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Users className="h-4 w-4" />
              {t('leaders.managedExecutors')}
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            {team.isError ? (
              <LoadError message={t('leaders.teamError')} retry={() => void team.refetch()} />
            ) : null}
            {executors.isError ? (
              <LoadError
                message={t('leaders.executorsError')}
                retry={() => void executors.refetch()}
              />
            ) : null}
            {teamReady && executors.data?.length ? (
              executors.data.map((executor) => (
                <label
                  key={executor.id}
                  className="flex items-center gap-3 rounded-md border border-border p-3 text-sm"
                >
                  <input
                    type="checkbox"
                    checked={draftExecutorIds.includes(executor.id)}
                    onChange={() => toggleExecutor(executor.id)}
                    disabled={mutation.isPending}
                  />
                  <span className="min-w-0 flex-1">
                    <span className="block truncate font-medium text-text-primary">
                      {executor.display_name}
                    </span>
                    <span className="block truncate text-xs text-text-muted">
                      {executor.name} -{' '}
                      {t(`agentRoles.${executor.role}`, { defaultValue: executor.role })} -{' '}
                      {executor.namespace_id ?? t('agent.unbound')}
                    </span>
                  </span>
                </label>
              ))
            ) : !team.isError && !executors.isError ? (
              <EmptyState
                title={!teamReady ? t('leaders.loadingTeam') : t('leaders.noExecutors')}
              />
            ) : null}
            {mutation.isError ? <ErrorState message={mutation.error.message} /> : null}
            <Button
              onClick={() => mutation.mutate([...draftExecutorIds])}
              disabled={!teamReady || !hasTeamChanges || mutation.isPending}
            >
              <Save className="h-4 w-4" />
              {mutation.isPending ? t('leaders.savingTeam') : t('leaders.saveTeam')}
            </Button>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Crown className="h-4 w-4" />
              {t('leaders.sessions')}
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            <SessionUserFilter filter={userFilter} className="mb-3 border-0 bg-transparent p-0" />
            {sessions.isError ? (
              <LoadError
                message={t('leaders.sessionsError')}
                retry={() => void sessions.refetch()}
              />
            ) : sessions.data?.length ? (
              sessions.data.map((session) => <SessionLine key={session.id} session={session} />)
            ) : (
              <EmptyState
                title={sessions.isLoading ? t('leaders.loadingSessions') : t('leaders.noSessions')}
              />
            )}
          </CardContent>
        </Card>
      </div>
    </>
  )
}

function LoadError({ message, retry }: { message: string; retry: () => void }) {
  const { t } = useTranslation()
  return (
    <div className="space-y-2">
      <ErrorState message={message} />
      <Button type="button" variant="outline" size="sm" onClick={retry}>
        {t('leaders.retry')}
      </Button>
    </div>
  )
}

function SessionLine({ session }: { session: AgentSession }) {
  return (
    <Link
      to={`/sessions/${session.id}`}
      className="flex min-w-0 items-start gap-3 rounded-md border border-border p-3 hover:bg-surface-raised"
    >
      <UserAvatar name={session.user_display_name} userId={session.user_id} size="md" />
      <span className="min-w-0 flex-1">
        <span className="flex flex-wrap items-center gap-2">
          <span className="font-medium text-text-primary">{session.title}</span>
          <StatusBadge value={session.visibility} />
          <StatusBadge value={session.state} />
        </span>
        <span className="mt-1 block truncate text-xs text-text-muted">
          {session.user_display_name} - {session.agent_name} - {formatDate(session.updated_at)}
        </span>
      </span>
    </Link>
  )
}
