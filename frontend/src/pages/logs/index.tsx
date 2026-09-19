import { FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { ChevronRight, Search, X } from 'lucide-react'
import { listAgents, listAuditLog, listEvents, listLogs } from '@/api/fleet'
import { Button, Input, Label } from '@sdlc/ui/ui'
import { EmptyState, ErrorState, PageHeader } from '../common'

const tabs = ['process', 'events', 'audit'] as const
type LogsTab = (typeof tabs)[number]
const API_LIMIT = 150
const PAGE_SIZE = 25

type LogRow = {
  id: string
  primary: string
  secondary: string
  body: string
  payload?: unknown
}

export function LogsPage() {
  const { t } = useTranslation()
  const [params, setParams] = useSearchParams()
  const selectedTab = tabs.includes(params.get('tab') as LogsTab)
    ? (params.get('tab') as LogsTab)
    : 'process'

  return (
    <>
      <PageHeader title={t('logs.title')} description={t('logs.description')} />
      <div className="mb-4 flex flex-wrap gap-2" aria-label={t('logs.sections')}>
        {tabs.map((tab) => (
          <Button
            key={tab}
            type="button"
            variant={selectedTab === tab ? 'default' : 'outline'}
            aria-pressed={selectedTab === tab}
            onClick={() =>
              setParams((current) => {
                const next = new URLSearchParams(current)
                next.set('tab', tab)
                return next
              })
            }
          >
            {t(`logs.tabs.${tab}`)}
          </Button>
        ))}
      </div>
      {selectedTab === 'process' ? <ProcessLogs /> : null}
      {selectedTab === 'events' ? <EventsLog /> : null}
      {selectedTab === 'audit' ? <AuditTrail /> : null}
    </>
  )
}

function ProcessLogs() {
  const { t, i18n } = useTranslation()
  const [agentId, setAgentId] = useState('')
  const agents = useQuery({ queryKey: ['agents'], queryFn: listAgents })
  const logs = useQuery({
    queryKey: ['logs', agentId],
    queryFn: () => listLogs(agentId || undefined, API_LIMIT),
  })

  return (
    <section aria-label={t('logs.tabs.process')}>
      <div className="mb-3 max-w-sm">
        <Label htmlFor="process-log-agent">{t('logs.agent')}</Label>
        <select
          id="process-log-agent"
          value={agentId}
          onChange={(event) => setAgentId(event.target.value)}
          className="mt-1 h-10 w-full rounded-md border border-border bg-background px-3 text-sm text-text-primary"
        >
          <option value="">{t('logs.allAgents')}</option>
          {agents.data?.map((agent) => (
            <option key={agent.id} value={agent.id}>
              {agent.name} - {agent.display_name}
            </option>
          ))}
        </select>
      </div>
      {agents.isError ? (
        <div className="mb-3">
          <LoadError message={t('logs.agentsError')} retry={() => void agents.refetch()} />
        </div>
      ) : null}
      {logs.isError ? (
        <LoadError message={logs.error.message} retry={() => void logs.refetch()} />
      ) : (
        <LogList
          key={agentId}
          loading={logs.isLoading}
          empty={t('logs.emptyProcess')}
          rows={(logs.data ?? []).map((entry) => ({
            id: entry.id,
            primary: entry.stream,
            secondary: formatTimestamp(entry.created_at, i18n.language),
            body: entry.message,
          }))}
        />
      )}
    </section>
  )
}

function EventsLog() {
  const { t, i18n } = useTranslation()
  const events = useQuery({ queryKey: ['events', 'recent'], queryFn: () => listEvents(API_LIMIT) })
  return (
    <section aria-label={t('logs.tabs.events')}>
      {events.isError ? (
        <LoadError message={events.error.message} retry={() => void events.refetch()} />
      ) : (
        <LogList
          loading={events.isLoading}
          empty={t('logs.emptyEvents')}
          rows={(events.data ?? []).map((event) => ({
            id: event.id,
            primary: event.event_type,
            secondary: formatTimestamp(event.created_at, i18n.language),
            body: event.message,
            payload: event.payload,
          }))}
        />
      )}
    </section>
  )
}

function AuditTrail() {
  const { t, i18n } = useTranslation()
  const [action, setAction] = useState('')
  const [entityType, setEntityType] = useState('')
  const [applied, setApplied] = useState({ action: '', entityType: '' })
  const audit = useQuery({
    queryKey: ['audit-log', applied.action, applied.entityType],
    queryFn: () =>
      listAuditLog({
        action: applied.action || undefined,
        entity_type: applied.entityType || undefined,
        limit: API_LIMIT,
      }),
  })

  function applyFilters(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setApplied({ action: action.trim(), entityType: entityType.trim() })
  }

  function resetFilters() {
    setAction('')
    setEntityType('')
    setApplied({ action: '', entityType: '' })
  }

  return (
    <section aria-label={t('logs.tabs.audit')}>
      <form
        className="mb-3 grid grid-cols-2 gap-3 sm:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto_auto] sm:items-end"
        onSubmit={applyFilters}
      >
        <div className="col-span-2 min-w-0 sm:col-span-1">
          <Label htmlFor="audit-action">{t('logs.action')}</Label>
          <Input
            id="audit-action"
            className="mt-1 h-10"
            value={action}
            onChange={(event) => setAction(event.target.value)}
            placeholder="session.create"
          />
        </div>
        <div className="col-span-2 min-w-0 sm:col-span-1">
          <Label htmlFor="audit-entity">{t('logs.entityType')}</Label>
          <Input
            id="audit-entity"
            className="mt-1 h-10"
            value={entityType}
            onChange={(event) => setEntityType(event.target.value)}
            placeholder="agent"
          />
        </div>
        <Button type="submit" className="w-full">
          <Search className="h-4 w-4" />
          {t('logs.apply')}
        </Button>
        <Button
          type="button"
          variant="outline"
          className="w-full"
          disabled={!action && !entityType && !applied.action && !applied.entityType}
          onClick={resetFilters}
        >
          <X className="h-4 w-4" />
          {t('logs.clear')}
        </Button>
      </form>
      {audit.isError ? (
        <LoadError message={audit.error.message} retry={() => void audit.refetch()} />
      ) : (
        <LogList
          key={`${applied.action}|${applied.entityType}`}
          loading={audit.isLoading}
          empty={applied.action || applied.entityType ? t('logs.noMatches') : t('logs.emptyAudit')}
          rows={(audit.data ?? []).map((entry) => ({
            id: entry.id,
            primary: entry.action,
            secondary: formatTimestamp(entry.created_at, i18n.language),
            body: `${entry.entity_type} · ${entry.entity_id ?? 'fleet'}`,
            payload: entry.payload,
          }))}
        />
      )}
    </section>
  )
}

function LoadError({ message, retry }: { message: string; retry: () => void }) {
  const { t } = useTranslation()
  return (
    <div className="space-y-2">
      <ErrorState message={message} />
      <Button type="button" variant="outline" onClick={retry}>
        {t('logs.retry')}
      </Button>
    </div>
  )
}

function LogList({ rows, loading, empty }: { rows: LogRow[]; loading: boolean; empty: string }) {
  const { t } = useTranslation()
  const [visibleCount, setVisibleCount] = useState(PAGE_SIZE)
  if (!rows.length) return <EmptyState title={loading ? t('logs.loading') : empty} />
  return (
    <>
      <p className="mb-2 text-sm text-text-muted">
        {t('logs.count', { shown: Math.min(visibleCount, rows.length), total: rows.length })}
      </p>
      <div className="divide-y divide-border border-y border-border">
        {rows.slice(0, visibleCount).map((row) => (
          <details key={row.id} className="group min-w-0">
            <summary className="flex min-h-12 cursor-pointer list-none flex-wrap items-center gap-x-3 gap-y-1 py-2 text-sm hover:bg-surface-raised focus-visible:outline-2 focus-visible:outline-focus">
              <ChevronRight className="h-4 w-4 shrink-0 text-text-muted transition-transform group-open:rotate-90" />
              <code className="max-w-full break-all text-xs font-medium text-text-primary">
                {row.primary}
              </code>
              <span className="min-w-0 flex-1 basis-40 truncate text-text-secondary">
                {row.body}
              </span>
              <time className="text-xs text-text-muted">{row.secondary}</time>
            </summary>
            <div className="mb-3 ml-7 min-w-0 space-y-2">
              <p className="whitespace-pre-wrap break-all text-sm text-text-secondary">
                {row.body}
              </p>
              {row.payload !== undefined ? (
                <pre className="max-w-full whitespace-pre-wrap break-all rounded-md border border-border bg-surface-raised p-3 text-xs text-text-secondary">
                  {JSON.stringify(row.payload, null, 2)}
                </pre>
              ) : null}
            </div>
          </details>
        ))}
      </div>
      {visibleCount < rows.length ? (
        <Button
          type="button"
          className="mt-3"
          variant="outline"
          onClick={() => setVisibleCount((count) => count + PAGE_SIZE)}
        >
          {t('logs.showMore')}
        </Button>
      ) : null}
      {rows.length >= API_LIMIT ? (
        <p className="mt-2 text-xs text-text-muted">
          {t('logs.limitNotice', { limit: API_LIMIT })}
        </p>
      ) : null}
    </>
  )
}

function formatTimestamp(value: string, language: string) {
  const locale = language.startsWith('ru') ? 'ru-RU' : 'en-US'
  return new Intl.DateTimeFormat(locale, { dateStyle: 'short', timeStyle: 'short' }).format(
    new Date(value),
  )
}
