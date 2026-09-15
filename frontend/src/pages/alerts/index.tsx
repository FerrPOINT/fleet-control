import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useSearchParams } from 'react-router'
import { acknowledgeFleetAlert, listFleetAlerts, type FleetAlert } from '@/api/fleet'
import { Button } from '@sdlc/ui/ui'
import { Card, CardContent, CardHeader, CardTitle } from '@sdlc/ui/ui'
import { EmptyState, ErrorState, PageHeader, formatDate } from '../common'

const stateFilters = ['all', 'open', 'acknowledged', 'resolved'] as const
type StateFilter = (typeof stateFilters)[number]

const severityBadge: Record<string, string> = {
  critical: 'bg-red-100 text-red-800',
  warning: 'bg-amber-100 text-amber-800',
  info: 'bg-sky-100 text-sky-800',
}

const stateBadge: Record<FleetAlert['state'], string> = {
  open: 'bg-red-100 text-red-800',
  acknowledged: 'bg-amber-100 text-amber-800',
  resolved: 'bg-emerald-100 text-emerald-800',
}

export function AlertsPage() {
  const [params, setParams] = useSearchParams()
  const filter = stateFilters.includes(params.get('state') as StateFilter)
    ? (params.get('state') as StateFilter)
    : 'all'

  return (
    <>
      <PageHeader
        title="Fleet alerts"
        description="Health-transition alerts across the fleet: agents going down, recoveries and their acknowledgement state."
      />
      <div className="mb-4 flex flex-wrap gap-2">
        {stateFilters.map((state) => (
          <Button
            key={state}
            type="button"
            variant={filter === state ? 'default' : 'outline'}
            onClick={() => setParams(state === 'all' ? {} : { state })}
          >
            {state}
          </Button>
        ))}
      </div>
      <AlertsTable filter={filter} />
    </>
  )
}

function AlertsTable({ filter }: { filter: StateFilter }) {
  const queryClient = useQueryClient()
  const alerts = useQuery({
    queryKey: ['fleet-alerts', filter],
    queryFn: () => listFleetAlerts(filter === 'all' ? undefined : filter),
  })
  const acknowledge = useMutation({
    mutationFn: (alertId: string) => acknowledgeFleetAlert(alertId),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['fleet-alerts'] }),
  })

  if (alerts.isError) return <ErrorState message={alerts.error.message} />
  if (alerts.isPending) return null

  const rows = alerts.data

  return (
    <Card>
      <CardHeader>
        <CardTitle>Alerts ({rows.length})</CardTitle>
      </CardHeader>
      <CardContent>
        {rows.length === 0 ? (
          <EmptyState title="No fleet alerts in this state." />
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-sm" data-testid="fleet-alerts-table">
              <thead>
                <tr className="border-b text-left text-muted-foreground">
                  <th className="py-2 pr-4">Severity</th>
                  <th className="py-2 pr-4">Kind</th>
                  <th className="py-2 pr-4">Agent</th>
                  <th className="py-2 pr-4">State</th>
                  <th className="py-2 pr-4">Opened</th>
                  <th className="py-2 pr-4">Resolved</th>
                  <th className="py-2 pr-4">Actions</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((alert) => (
                  <tr key={alert.id} className="border-b last:border-b-0">
                    <td className="py-2 pr-4">
                      <span
                        className={`inline-flex rounded-full px-2 py-0.5 text-xs font-medium ${
                          severityBadge[alert.severity] ?? 'bg-slate-100 text-slate-800'
                        }`}
                      >
                        {alert.severity}
                      </span>
                    </td>
                    <td className="py-2 pr-4 font-medium">{alert.kind}</td>
                    <td className="py-2 pr-4 font-mono text-xs">
                      {alert.agent_id ?? '—'}
                    </td>
                    <td className="py-2 pr-4">
                      <span
                        className={`inline-flex rounded-full px-2 py-0.5 text-xs font-medium ${stateBadge[alert.state]}`}
                      >
                        {alert.state}
                      </span>
                    </td>
                    <td className="py-2 pr-4 whitespace-nowrap">{formatDate(alert.opened_at)}</td>
                    <td className="py-2 pr-4 whitespace-nowrap">
                      {alert.resolved_at ? formatDate(alert.resolved_at) : '—'}
                    </td>
                    <td className="py-2 pr-4">
                      {alert.state === 'open' ? (
                        <Button
                          type="button"
                          size="sm"
                          variant="outline"
                          disabled={acknowledge.isPending}
                          onClick={() => acknowledge.mutate(alert.id)}
                        >
                          Acknowledge
                        </Button>
                      ) : null}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </CardContent>
    </Card>
  )
}
