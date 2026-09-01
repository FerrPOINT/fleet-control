import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { listAgents, listLogs } from '@/api/fleet'
import { Card, CardContent, CardHeader, CardTitle } from '@/shared/ui/card'
import { EmptyState, ErrorState, PageHeader, formatDate } from '../common'

export function LogsPage() {
  const [agentId, setAgentId] = useState('')
  const agents = useQuery({ queryKey: ['agents'], queryFn: listAgents })
  const logs = useQuery({
    queryKey: ['logs', agentId],
    queryFn: () => listLogs(agentId || undefined, 150),
  })

  if (logs.isError) return <ErrorState message={logs.error.message} />

  return (
    <>
      <PageHeader
        title="Logs"
        description="Global and per-agent process logs with secret redaction at write time."
      />
      <Card>
        <CardHeader>
          <CardTitle>Log stream</CardTitle>
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
          <div className="space-y-2">
            {logs.data?.length ? (
              logs.data.map((entry) => (
                <div
                  key={entry.id}
                  className="rounded-md border border-border bg-background p-3 font-mono text-xs"
                >
                  <span className="text-text-muted">{formatDate(entry.created_at)}</span>
                  <span className="ml-2 text-warning">{entry.stream}</span>
                  <p className="mt-1 whitespace-pre-wrap text-text-secondary">{entry.message}</p>
                </div>
              ))
            ) : (
              <EmptyState title={logs.isLoading ? 'Loading logs...' : 'No logs recorded'} />
            )}
          </div>
        </CardContent>
      </Card>
    </>
  )
}
