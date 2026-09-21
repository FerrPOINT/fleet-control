import { useMemo, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Check, RefreshCw } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router'
import { toast } from 'sonner'
import {
  acknowledgeFleetAlert,
  listAgentDirectory,
  listFleetAlerts,
  type FleetAlert,
} from '@/api/fleet'
import type { AgentDirectoryItem } from '@/api/types'
import { Button } from '@sdlc/ui/ui'
import { EmptyState, ErrorState, JsonBlock, PageHeader, labelize } from '../common'

const stateFilters = ['all', 'open', 'acknowledged', 'resolved'] as const
type StateFilter = (typeof stateFilters)[number]
const PAGE_SIZE = 25

const severityTone: Record<string, string> = {
  critical: 'border-danger/40 bg-danger/10',
  warning: 'border-warning/40 bg-warning/10',
  info: 'border-accent/40 bg-accent/10',
}

const stateTone: Record<FleetAlert['state'], string> = {
  open: 'border-danger/40 bg-danger/10',
  acknowledged: 'border-warning/40 bg-warning/10',
  resolved: 'border-success/40 bg-success/10',
}

export function AlertsPage() {
  const { t } = useTranslation()
  const [params, setParams] = useSearchParams()
  const filter = stateFilters.includes(params.get('state') as StateFilter)
    ? (params.get('state') as StateFilter)
    : 'all'

  function selectFilter(state: StateFilter) {
    setParams((current) => {
      const next = new URLSearchParams(current)
      if (state === 'all') next.delete('state')
      else next.set('state', state)
      return next
    })
  }

  return (
    <>
      <PageHeader title={t('alerts.title')} description={t('alerts.description')} />
      <nav className="mb-4 flex flex-wrap gap-2" aria-label={t('alerts.filtersLabel')}>
        {stateFilters.map((state) => (
          <Button
            key={state}
            type="button"
            className="h-10"
            variant={filter === state ? 'default' : 'outline'}
            aria-pressed={filter === state}
            onClick={() => selectFilter(state)}
          >
            {t(`alerts.filters.${state}`)}
          </Button>
        ))}
      </nav>
      <AlertsList key={filter} filter={filter} />
    </>
  )
}

function AlertsList({ filter }: { filter: StateFilter }) {
  const { t, i18n } = useTranslation()
  const [visibleCount, setVisibleCount] = useState(PAGE_SIZE)
  const alerts = useQuery({
    queryKey: ['fleet-alerts', filter],
    queryFn: () => listFleetAlerts(filter === 'all' ? undefined : filter),
    refetchInterval: 30_000,
  })
  const agents = useQuery({ queryKey: ['agent-directory'], queryFn: listAgentDirectory })
  const agentsById = useMemo(
    () => new Map((agents.data ?? []).map((agent) => [agent.id, agent])),
    [agents.data],
  )

  if (alerts.isError && !alerts.data) {
    return <LoadError message={t('alerts.loadError')} retry={() => void alerts.refetch()} />
  }
  if (alerts.isPending) return <EmptyState title={t('alerts.loading')} />

  const rows = alerts.data ?? []
  const hasAgentReferences = rows.some((alert) => alert.agent_id)

  return (
    <section aria-labelledby="alerts-list-title">
      <div className="mb-3 flex flex-wrap items-center justify-between gap-3">
        <h2 id="alerts-list-title" className="text-base font-semibold text-text-primary">
          {t('alerts.listTitle', { count: rows.length })}
        </h2>
        <Button
          type="button"
          className="h-10"
          variant="outline"
          disabled={alerts.isFetching}
          onClick={() => void alerts.refetch()}
        >
          <RefreshCw className={`h-4 w-4 ${alerts.isFetching ? 'animate-spin' : ''}`} />
          {alerts.isFetching ? t('alerts.refreshing') : t('alerts.refresh')}
        </Button>
      </div>

      {alerts.isError ? (
        <div className="mb-3">
          <LoadError message={t('alerts.refreshError')} retry={() => void alerts.refetch()} />
        </div>
      ) : null}
      {agents.isError && hasAgentReferences ? (
        <div className="mb-3">
          <LoadError message={t('alerts.directoryError')} retry={() => void agents.refetch()} />
        </div>
      ) : null}

      {!rows.length ? (
        <EmptyState title={filter === 'all' ? t('alerts.emptyAll') : t('alerts.emptyFiltered')} />
      ) : (
        <>
          <ul className="divide-y divide-border border-y border-border">
            {rows.slice(0, visibleCount).map((alert) => (
              <AlertRow
                key={alert.id}
                alert={alert}
                agent={alert.agent_id ? agentsById.get(alert.agent_id) : undefined}
                language={i18n.resolvedLanguage ?? i18n.language}
              />
            ))}
          </ul>
          {visibleCount < rows.length ? (
            <Button
              type="button"
              className="mt-4 h-10"
              variant="outline"
              onClick={() => setVisibleCount((current) => current + PAGE_SIZE)}
            >
              {t('alerts.showMore')}
            </Button>
          ) : null}
        </>
      )}
    </section>
  )
}

function AlertRow({
  alert,
  agent,
  language,
}: {
  alert: FleetAlert
  agent?: AgentDirectoryItem
  language: string
}) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const acknowledge = useMutation({
    mutationFn: () => acknowledgeFleetAlert(alert.id),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ['fleet-alerts'] })
      toast.success(t('alerts.acknowledgedSuccess'))
    },
  })
  const severityClass = severityTone[alert.severity] ?? 'border-border bg-surface-raised'
  const alertStateClass = stateTone[alert.state] ?? 'border-border bg-surface-raised'

  return (
    <li className="min-w-0 py-3" aria-busy={acknowledge.isPending}>
      <div className="grid min-w-0 gap-3 lg:grid-cols-[minmax(0,1.25fr)_minmax(0,1fr)_auto] lg:items-start">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <span
              className={`inline-flex min-h-6 items-center rounded-md border px-2 text-xs font-medium text-text-primary ${severityClass}`}
            >
              {t(`alerts.severity.${alert.severity}`, {
                defaultValue: labelize(alert.severity),
              })}
            </span>
            <span
              className={`inline-flex min-h-6 items-center rounded-md border px-2 text-xs font-medium text-text-primary ${alertStateClass}`}
            >
              {t(`alerts.states.${alert.state}`, { defaultValue: labelize(alert.state) })}
            </span>
          </div>
          <h3 className="mt-2 font-medium text-text-primary">
            {t(`alerts.kinds.${alert.kind}`, { defaultValue: labelize(alert.kind) })}
          </h3>
          <p className="mt-1 text-sm text-text-secondary">
            {agent
              ? `${agent.display_name} · ${agent.name}`
              : alert.agent_id
                ? t('alerts.unknownAgent')
                : t('alerts.fleetScope')}
          </p>
          {alert.agent_id ? (
            <code className="mt-1 block break-all text-xs text-text-muted">{alert.agent_id}</code>
          ) : null}
        </div>

        <dl className="grid gap-x-4 gap-y-2 text-sm sm:grid-cols-2 lg:grid-cols-1 xl:grid-cols-2">
          <Timestamp label={t('alerts.opened')} value={alert.opened_at} language={language} />
          {alert.acknowledged_at ? (
            <Timestamp
              label={t('alerts.acknowledgedAt')}
              value={alert.acknowledged_at}
              language={language}
            />
          ) : null}
          {alert.resolved_at ? (
            <Timestamp
              label={t('alerts.resolvedAt')}
              value={alert.resolved_at}
              language={language}
            />
          ) : null}
        </dl>

        {alert.state === 'open' ? (
          <Button
            type="button"
            className="h-10 w-full lg:w-auto"
            variant="outline"
            disabled={acknowledge.isPending}
            onClick={() => acknowledge.mutate()}
          >
            <Check className="h-4 w-4" />
            {acknowledge.isPending ? t('alerts.acknowledging') : t('alerts.acknowledge')}
          </Button>
        ) : null}
      </div>

      <details className="mt-2 max-w-3xl">
        <summary className="inline-flex min-h-10 cursor-pointer items-center text-sm font-medium text-accent underline-offset-2 hover:underline focus-visible:outline focus-visible:outline-2 focus-visible:outline-focus">
          {t('alerts.details')}
        </summary>
        <JsonBlock value={alert.detail} />
      </details>

      {acknowledge.isError ? (
        <div className="mt-2 flex flex-wrap items-center gap-2">
          <ErrorState message={t('alerts.acknowledgeError')} />
          <Button
            type="button"
            className="h-10"
            variant="outline"
            onClick={() => acknowledge.mutate()}
          >
            {t('alerts.retryAcknowledge')}
          </Button>
        </div>
      ) : null}
    </li>
  )
}

function Timestamp({ label, value, language }: { label: string; value: string; language: string }) {
  return (
    <div>
      <dt className="text-xs text-text-muted">{label}</dt>
      <dd className="mt-0.5 whitespace-nowrap text-text-secondary">
        <time dateTime={value}>{formatTimestamp(value, language)}</time>
      </dd>
    </div>
  )
}

function LoadError({ message, retry }: { message: string; retry: () => void }) {
  const { t } = useTranslation()
  return (
    <div className="flex flex-wrap items-center gap-2">
      <ErrorState message={message} />
      <Button type="button" className="h-10" variant="outline" onClick={retry}>
        {t('alerts.retry')}
      </Button>
    </div>
  )
}

function formatTimestamp(value: string, language: string) {
  const locale = language.startsWith('ru') ? 'ru-RU' : 'en-US'
  return new Intl.DateTimeFormat(locale, { dateStyle: 'short', timeStyle: 'short' }).format(
    new Date(value),
  )
}
