import { useMemo, useState } from 'react'
import { Link, useParams } from 'react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Crown, Pencil, Plus, Save, Users } from 'lucide-react'
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
import { Button } from '@sdlc/ui/ui'
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
  const leaders = useQuery({ queryKey: ['leaders'], queryFn: listLeaders })
  const userFilter = useSessionUserFilter()
  const sessions = useQuery({
    queryKey: ['sessions', 'leaders', userFilter.selectedUserIds],
    queryFn: () => listSessions(undefined, userFilter.selectedUserIds),
  })

  if (leaders.isError) return <ErrorState message={leaders.error.message} />

  return (
    <>
      <PageHeader
        title="Leaders"
        description="Team lead agents coordinate executor sessions, prompts and workflow scopes."
        actions={
          <Button asChild>
            <Link to="/leaders/new">
              <Plus className="h-4 w-4" />
              New leader
            </Link>
          </Button>
        }
      />
      <SessionUserFilter filter={userFilter} className="mb-4" />
      <div className="grid gap-3 xl:grid-cols-2">
        {leaders.data?.length ? (
          leaders.data.map((leader) => (
            <LeaderCard
              key={leader.id}
              leader={leader}
              sessions={(sessions.data ?? []).filter(
                (session) => session.leader_agent_id === leader.id,
              )}
            />
          ))
        ) : (
          <div className="xl:col-span-2">
            <EmptyState title={leaders.isLoading ? 'Loading leaders...' : 'No leaders yet'} />
          </div>
        )}
      </div>
    </>
  )
}

function LeaderCard({ leader, sessions }: { leader: Agent; sessions: AgentSession[] }) {
  const team = useQuery({
    queryKey: ['leader-executors', leader.id],
    queryFn: () => listLeaderExecutors(leader.id),
  })

  return (
    <Card>
      <CardContent className="pt-4">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <AgentIdentity agent={leader} />
          <Button asChild variant="outline" size="sm">
            <Link to={`/leaders/${leader.id}`}>Open</Link>
          </Button>
        </div>
        <dl className="mt-4 grid gap-3 text-sm sm:grid-cols-3">
          <Metric label="Executors" value={team.data?.length ?? 0} />
          <Metric label="Sessions" value={sessions.length} />
          <Metric label="Namespace" value={leader.namespace_id ?? 'unbound'} />
        </dl>
        <div className="mt-4 rounded-md border border-border bg-background p-3">
          <p className="text-xs font-medium uppercase text-text-muted">Active team tasks</p>
          <div className="mt-3 space-y-2">
            {sessions.slice(0, 3).map((session) => (
              <SessionLine key={session.id} session={session} />
            ))}
            {!sessions.length ? (
              <p className="text-xs text-text-muted">No leader-scoped sessions</p>
            ) : null}
          </div>
        </div>
      </CardContent>
    </Card>
  )
}

export function LeaderDetailPage() {
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
  const mutation = useMutation({
    mutationFn: () => updateLeaderExecutors(leaderId!, { executor_ids: draftExecutorIds }),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['leader-executors', leaderId] }),
        queryClient.invalidateQueries({ queryKey: ['leaders'] }),
      ])
      setSelectedExecutorIds(null)
    },
  })

  function toggleExecutor(executorId: string) {
    setSelectedExecutorIds(
      draftExecutorIds.includes(executorId)
        ? draftExecutorIds.filter((id) => id !== executorId)
        : [...draftExecutorIds, executorId],
    )
  }

  if (!leaderId) return <ErrorState message="Leader id is missing" />
  if (agents.isError) return <ErrorState message={agents.error.message} />
  if (!leader)
    return <EmptyState title={agents.isLoading ? 'Loading leader...' : 'Leader not found'} />

  return (
    <>
      <PageHeader
        title={leader.display_name}
        description={`${leader.name} manages executor task sessions through Fleet Control.`}
        actions={
          <>
            <Button asChild variant="outline">
              <Link to={`/leaders/${leader.id}/edit`}>
                <Pencil className="h-4 w-4" />
                Edit leader
              </Link>
            </Button>
            <Button asChild variant="outline">
              <Link to={`/agents/${leader.id}`}>Technical details</Link>
            </Button>
          </>
        }
      />
      <div className="grid gap-4 xl:grid-cols-[420px_1fr]">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Users className="h-4 w-4" />
              Managed executors
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            {executors.data?.length ? (
              executors.data.map((executor) => (
                <label
                  key={executor.id}
                  className="flex items-center gap-3 rounded-md border border-border p-3 text-sm"
                >
                  <input
                    type="checkbox"
                    checked={draftExecutorIds.includes(executor.id)}
                    onChange={() => toggleExecutor(executor.id)}
                  />
                  <span className="min-w-0 flex-1">
                    <span className="block truncate font-medium text-text-primary">
                      {executor.display_name}
                    </span>
                    <span className="block truncate text-xs text-text-muted">
                      {executor.name} - {executor.role} - {executor.namespace_id ?? 'unbound'}
                    </span>
                  </span>
                </label>
              ))
            ) : (
              <EmptyState
                title={executors.isLoading ? 'Loading executors...' : 'No executors yet'}
              />
            )}
            {mutation.isError ? <ErrorState message={mutation.error.message} /> : null}
            <Button onClick={() => mutation.mutate()} disabled={mutation.isPending}>
              <Save className="h-4 w-4" />
              Save team
            </Button>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Crown className="h-4 w-4" />
              Leader sessions
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            <SessionUserFilter filter={userFilter} className="mb-3" />
            {sessions.data?.length ? (
              sessions.data.map((session) => <SessionLine key={session.id} session={session} />)
            ) : (
              <EmptyState
                title={sessions.isLoading ? 'Loading sessions...' : 'No sessions for this leader'}
              />
            )}
          </CardContent>
        </Card>
      </div>
    </>
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

function Metric({ label, value }: { label: string; value: string | number }) {
  return (
    <div>
      <dt className="text-xs text-text-muted">{label}</dt>
      <dd className="font-medium text-text-primary">{value}</dd>
    </div>
  )
}
