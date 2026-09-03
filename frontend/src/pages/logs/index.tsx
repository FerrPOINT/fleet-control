import { useState } from 'react'
import { useSearchParams } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { listAgents, listAuditLog, listEvents, listLogs } from '@/api/fleet'
import { Button } from '@/shared/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/shared/ui/card'
import { Input } from '@/shared/ui/input'
import { Label } from '@/shared/ui/label'
import { EmptyState, ErrorState, JsonBlock, PageHeader, StatusBadge, formatDate } from '../common'

const tabs = ['process', 'events', 'audit'] as const
type LogsTab = (typeof tabs)[number]

export function LogsPage() {
  const [params, setParams] = useSearchParams()
  const selectedTab = tabs.includes(params.get('tab') as LogsTab)
    ? (params.get('tab') as LogsTab)
    : 'process'

  return (
    <>
      <PageHeader
        title="Logs"
        description="Process output, control-plane events and redacted audit trail."
      />
      <div className="mb-4 flex flex-wrap gap-2">
        {tabs.map((tab) => (
          <Button
            key={tab}
            type="button"
            variant={selectedTab === tab ? 'default' : 'outline'}
            onClick={() => setParams({ tab })}
          >
            {tab}
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
  const [agentId, setAgentId] = useState('')
  const agents = useQuery({ queryKey: ['agents'], queryFn: listAgents })
  const logs = useQuery({
    queryKey: ['logs', agentId],
    queryFn: () => listLogs(agentId || undefined, 150),
  })

  if (logs.isError) return <ErrorState message={logs.error.message} />

  return (
    <Card>
      <CardHeader>
        <CardTitle>Process logs</CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        <select
          value={agentId}
          onChange={(event) => setAgentId(event.target.value)}
          className="h-9 rounded-md border border-border bg-background px-3 text-sm"
        >
          <option value="">All agents</option>
          {agents.data?.map((agent) => (
            <option key={agent.id} value={agent.id}>
              {agent.name} - {agent.display_name}
            </option>
          ))}
        </select>
        <LogList
          loading={logs.isLoading}
          empty="No process logs recorded"
          rows={(logs.data ?? []).map((entry) => ({
            id: entry.id,
            primary: entry.stream,
            secondary: formatDate(entry.created_at),
            body: entry.message,
          }))}
        />
      </CardContent>
    </Card>
  )
}

function EventsLog() {
  const events = useQuery({ queryKey: ['events', 'recent'], queryFn: () => listEvents(150) })
  if (events.isError) return <ErrorState message={events.error.message} />
  return (
    <Card>
      <CardHeader>
        <CardTitle>Control-plane events</CardTitle>
      </CardHeader>
      <CardContent>
        <LogList
          loading={events.isLoading}
          empty="No events recorded"
          rows={(events.data ?? []).map((event) => ({
            id: event.id,
            primary: event.event_type,
            secondary: formatDate(event.created_at),
            body: event.message,
            payload: event.payload,
          }))}
        />
      </CardContent>
    </Card>
  )
}

function AuditTrail() {
  const [action, setAction] = useState('')
  const [entityType, setEntityType] = useState('')
  const audit = useQuery({
    queryKey: ['audit-log', action, entityType],
    queryFn: () =>
      listAuditLog({
        action: action || undefined,
        entity_type: entityType || undefined,
        limit: 150,
      }),
  })
  if (audit.isError) return <ErrorState message={audit.error.message} />
  return (
    <Card>
      <CardHeader>
        <CardTitle>Audit trail</CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="grid gap-3 md:grid-cols-2">
          <div className="grid gap-2">
            <Label htmlFor="audit-action">Action</Label>
            <Input
              id="audit-action"
              value={action}
              onChange={(event) => setAction(event.target.value)}
              placeholder="session.create"
            />
          </div>
          <div className="grid gap-2">
            <Label htmlFor="audit-entity">Entity type</Label>
            <Input
              id="audit-entity"
              value={entityType}
              onChange={(event) => setEntityType(event.target.value)}
              placeholder="agent"
            />
          </div>
        </div>
        <div className="space-y-2">
          {audit.data?.length ? (
            audit.data.map((entry) => (
              <div key={entry.id} className="rounded-md border border-border bg-background p-3">
                <div className="flex flex-wrap items-center gap-2 text-xs text-text-muted">
                  <StatusBadge value={entry.action} />
                  <span>{entry.entity_type}</span>
                  <span>{entry.entity_id ?? 'fleet'}</span>
                  <span>{formatDate(entry.created_at)}</span>
                </div>
                <div className="mt-2">
                  <JsonBlock value={entry.payload} />
                </div>
              </div>
            ))
          ) : (
            <EmptyState title={audit.isLoading ? 'Loading audit log...' : 'No audit entries'} />
          )}
        </div>
      </CardContent>
    </Card>
  )
}

function LogList({
  rows,
  loading,
  empty,
}: {
  rows: Array<{ id: string; primary: string; secondary: string; body: string; payload?: unknown }>
  loading: boolean
  empty: string
}) {
  if (!rows.length) return <EmptyState title={loading ? 'Loading logs...' : empty} />
  return (
    <div className="space-y-2">
      {rows.map((row) => (
        <div key={row.id} className="rounded-md border border-border bg-background p-3">
          <div className="flex flex-wrap items-center gap-2 font-mono text-xs">
            <span className="text-warning">{row.primary}</span>
            <span className="text-text-muted">{row.secondary}</span>
          </div>
          <p className="mt-1 whitespace-pre-wrap text-sm text-text-secondary">{row.body}</p>
          {row.payload ? (
            <div className="mt-2">
              <JsonBlock value={row.payload} />
            </div>
          ) : null}
        </div>
      ))}
    </div>
  )
}
