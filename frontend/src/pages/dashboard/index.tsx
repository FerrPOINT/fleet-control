import { Link } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { ArrowRight, Bot, Crown, PlayCircle, UserRoundCheck, XCircle } from 'lucide-react'
import { getDashboard } from '@/api/fleet'
import { Button } from '@sdlc/ui/ui'
import { Card, CardContent, CardHeader, CardTitle } from '@sdlc/ui/ui'
import { AgentIdentity, EmptyState, ErrorState, PageHeader, StatCard, formatDate } from '../common'

export function DashboardPage() {
  const dashboard = useQuery({ queryKey: ['dashboard'], queryFn: getDashboard })

  if (dashboard.isError) return <ErrorState message={dashboard.error.message} />

  return (
    <>
      <PageHeader
        title="Fleet dashboard"
        description="Live overview of managed agent runtimes, task sessions and workflow bindings."
        actions={
          <Button asChild>
            <Link to="/agents/new">
              <Bot className="h-4 w-4" />
              New agent
            </Link>
          </Button>
        }
      />

      <div className="grid gap-3 md:grid-cols-4">
        <StatCard label="Agents" value={dashboard.data?.total_agents ?? 0} />
        <StatCard label="Leaders" value={dashboard.data?.leader_agents ?? 0} />
        <StatCard label="Executors" value={dashboard.data?.executor_agents ?? 0} />
        <StatCard label="Running" value={dashboard.data?.running_agents ?? 0} tone="text-success" />
      </div>

      <div className="mt-3 grid gap-3 md:grid-cols-4">
        <StatCard label="Failed" value={dashboard.data?.failed_agents ?? 0} tone="text-danger" />
        <StatCard label="Active sessions" value={dashboard.data?.active_sessions ?? 0} />
        <StatCard label="Private sessions" value={dashboard.data?.private_sessions ?? 0} />
        <StatCard label="Leader sessions" value={dashboard.data?.leader_scoped_sessions ?? 0} />
      </div>

      <div className="mt-5 grid gap-4 xl:grid-cols-[1.5fr_1fr]">
        <Card>
          <CardHeader>
            <CardTitle>Agents</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            {dashboard.data?.agents.length ? (
              dashboard.data.agents.map((agent) => (
                <Link
                  key={agent.id}
                  to={
                    agent.product_role === 'leader' ? `/leaders/${agent.id}` : `/agents/${agent.id}`
                  }
                  className="flex items-center justify-between gap-3 rounded-md border border-border p-3 transition-colors hover:bg-surface-raised"
                >
                  <div className="flex min-w-0 items-center gap-3">
                    {agent.product_role === 'leader' ? (
                      <Crown className="h-4 w-4 shrink-0 text-text-muted" />
                    ) : (
                      <UserRoundCheck className="h-4 w-4 shrink-0 text-text-muted" />
                    )}
                    <AgentIdentity agent={agent} />
                  </div>
                  <ArrowRight className="h-4 w-4 text-text-muted" />
                </Link>
              ))
            ) : (
              <EmptyState title={dashboard.isLoading ? 'Loading fleet...' : 'No agents yet'} />
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Recent events</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            {dashboard.data?.recent_events.length ? (
              dashboard.data.recent_events.map((event) => (
                <div key={event.id} className="rounded-md border border-border p-3">
                  <div className="flex items-center gap-2 text-sm text-text-primary">
                    {event.event_type.includes('failed') ? (
                      <XCircle className="h-4 w-4 text-danger" />
                    ) : (
                      <PlayCircle className="h-4 w-4 text-success" />
                    )}
                    {event.message}
                  </div>
                  <p className="mt-1 text-xs text-text-muted">{formatDate(event.created_at)}</p>
                </div>
              ))
            ) : (
              <EmptyState title="No events recorded" />
            )}
          </CardContent>
        </Card>
      </div>
    </>
  )
}
