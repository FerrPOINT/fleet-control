import { Link } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { ArrowRight, Bot, Crown, PlayCircle, UserRoundCheck, XCircle } from 'lucide-react'
import { getDashboard } from '@/api/fleet'
import { Button } from '@sdlc/ui/ui'
import { AgentIdentity, EmptyState, ErrorState, PageHeader, StatCard, formatDate } from '../common'

export function DashboardPage() {
  const { t } = useTranslation()
  const dashboard = useQuery({ queryKey: ['dashboard'], queryFn: getDashboard })

  if (dashboard.isError) return <ErrorState message={dashboard.error.message} />

  return (
    <>
      <PageHeader
        title={t('dashboard.title')}
        actions={
          <Button asChild>
            <Link to="/agents/new">
              <Bot className="h-4 w-4" />
              {t('dashboard.newAgent')}
            </Link>
          </Button>
        }
      />

      <div className="grid grid-cols-2 border-l border-t border-border sm:grid-cols-4">
        <StatCard label={t('dashboard.agents')} value={dashboard.data?.total_agents ?? 0} />
        <StatCard label={t('dashboard.leaders')} value={dashboard.data?.leader_agents ?? 0} />
        <StatCard label={t('dashboard.executors')} value={dashboard.data?.executor_agents ?? 0} />
        <StatCard
          label={t('dashboard.running')}
          value={dashboard.data?.running_agents ?? 0}
          tone="text-success"
        />
        <StatCard
          label={t('dashboard.failed')}
          value={dashboard.data?.failed_agents ?? 0}
          tone="text-danger"
        />
        <StatCard
          label={t('dashboard.activeSessions')}
          value={dashboard.data?.active_sessions ?? 0}
        />
        <StatCard
          label={t('dashboard.privateSessions')}
          value={dashboard.data?.private_sessions ?? 0}
        />
        <StatCard
          label={t('dashboard.leaderSessions')}
          value={dashboard.data?.leader_scoped_sessions ?? 0}
        />
      </div>

      <div className="mt-5 grid gap-4 xl:grid-cols-[1.5fr_1fr]">
        <section className="min-w-0" aria-labelledby="fleet-agents-heading">
          <h2
            id="fleet-agents-heading"
            className="mb-3 border-b border-border pb-2 text-base font-semibold"
          >
            {t('dashboard.agents')}
          </h2>
          <div className="space-y-2">
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
              <EmptyState
                title={dashboard.isLoading ? t('dashboard.loadingAgents') : t('dashboard.noAgents')}
              />
            )}
          </div>
        </section>

        <section className="min-w-0" aria-labelledby="fleet-events-heading">
          <h2
            id="fleet-events-heading"
            className="mb-3 border-b border-border pb-2 text-base font-semibold"
          >
            {t('dashboard.recentEvents')}
          </h2>
          <div className="space-y-2">
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
              <EmptyState title={t('dashboard.noEvents')} />
            )}
          </div>
        </section>
      </div>
    </>
  )
}
