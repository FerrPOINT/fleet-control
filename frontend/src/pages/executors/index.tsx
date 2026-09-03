import { useMemo } from 'react'
import { Link } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { Plus, UserRoundCheck } from 'lucide-react'
import { listExecutors, listLeaders, listSessions } from '@/api/fleet'
import type { AgentSession } from '@/api/types'
import { SessionUserFilter, useSessionUserFilter } from '@/shared/session-user-filter'
import { Button } from '@/shared/ui/button'
import { Card, CardContent } from '@/shared/ui/card'
import { UserAvatar } from '@/shared/ui/user-avatar'
import { AgentIdentity, EmptyState, ErrorState, PageHeader, StatusBadge } from '../common'

export function ExecutorsPage() {
  const executors = useQuery({ queryKey: ['executors'], queryFn: listExecutors })
  const leaders = useQuery({ queryKey: ['leaders'], queryFn: listLeaders })
  const userFilter = useSessionUserFilter()
  const sessions = useQuery({
    queryKey: ['sessions', 'executors', userFilter.selectedUserIds],
    queryFn: () => listSessions(undefined, userFilter.selectedUserIds),
  })
  const sessionsByExecutor = useMemo(
    () => groupSessionsByAgent(sessions.data ?? []),
    [sessions.data],
  )

  if (executors.isError) return <ErrorState message={executors.error.message} />

  return (
    <>
      <PageHeader
        title="Executors"
        description="Delivery agents with isolated Hermes workspaces, skills and namespace bindings."
        actions={
          <Button asChild>
            <Link to="/executors/new">
              <Plus className="h-4 w-4" />
              New executor
            </Link>
          </Button>
        }
      />
      <SessionUserFilter filter={userFilter} className="mb-4" />
      <div className="grid gap-3 xl:grid-cols-2">
        {executors.data?.length ? (
          executors.data.map((executor) => (
            <Card key={executor.id}>
              <CardContent className="pt-4">
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <AgentIdentity agent={executor} />
                  <Button asChild variant="outline" size="sm">
                    <Link to={`/executors/${executor.id}`}>Open</Link>
                  </Button>
                </div>
                <div className="mt-4 grid gap-3 text-sm sm:grid-cols-3">
                  <Metric label="Profile" value={executor.role} />
                  <Metric label="Namespace" value={executor.namespace_id ?? 'unbound'} />
                  <Metric label="Workflow" value={executor.workflow_id ?? 'unbound'} />
                </div>
                <div className="mt-4 rounded-md border border-border bg-background p-3">
                  <div className="mb-3 flex items-center gap-2 text-xs font-medium uppercase text-text-muted">
                    <UserRoundCheck className="h-4 w-4" />
                    Sessions
                  </div>
                  <div className="space-y-2">
                    {(sessionsByExecutor.get(executor.id) ?? []).slice(0, 3).map((session) => (
                      <SessionPreview key={session.id} session={session} />
                    ))}
                    {!(sessionsByExecutor.get(executor.id) ?? []).length ? (
                      <p className="text-xs text-text-muted">
                        {leaders.isLoading
                          ? 'Loading team context...'
                          : 'No sessions for selected users'}
                      </p>
                    ) : null}
                  </div>
                </div>
              </CardContent>
            </Card>
          ))
        ) : (
          <div className="xl:col-span-2">
            <EmptyState title={executors.isLoading ? 'Loading executors...' : 'No executors yet'} />
          </div>
        )}
      </div>
    </>
  )
}

function groupSessionsByAgent(sessions: AgentSession[]) {
  const grouped = new Map<string, AgentSession[]>()
  for (const session of sessions) {
    grouped.set(session.primary_agent_id, [
      ...(grouped.get(session.primary_agent_id) ?? []),
      session,
    ])
  }
  return grouped
}

function SessionPreview({ session }: { session: AgentSession }) {
  return (
    <Link
      to={`/sessions/${session.id}`}
      className="flex min-w-0 items-center gap-2 rounded-md border border-border p-2 hover:bg-surface-raised"
    >
      <UserAvatar name={session.user_display_name} userId={session.user_id} />
      <span className="min-w-0 flex-1">
        <span className="flex flex-wrap items-center gap-2">
          <span className="truncate text-sm font-medium text-text-primary">{session.title}</span>
          <StatusBadge value={session.visibility} />
        </span>
        <span className="block truncate text-xs text-text-muted">
          {session.user_display_name} - {session.leader_agent_name ?? 'private'}
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
