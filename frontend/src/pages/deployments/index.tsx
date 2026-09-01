import { useQuery } from '@tanstack/react-query'
import { listAgents, listRuntimeTemplates } from '@/api/fleet'
import { Card, CardContent, CardHeader, CardTitle } from '@/shared/ui/card'
import { EmptyState, ErrorState, JsonBlock, KindBadge, PageHeader, StatusBadge } from '../common'

export function DeploymentsPage() {
  const templates = useQuery({ queryKey: ['runtime-templates'], queryFn: listRuntimeTemplates })
  const agents = useQuery({ queryKey: ['agents'], queryFn: listAgents })

  if (templates.isError) return <ErrorState message={templates.error.message} />

  return (
    <>
      <PageHeader
        title="Deployments"
        description="Runtime templates, source references, ports and installed agent copies."
      />
      <div className="grid gap-4 xl:grid-cols-[1fr_1fr]">
        <Card>
          <CardHeader>
            <CardTitle>Runtime templates</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            {templates.data?.length ? (
              templates.data.map((template) => (
                <div key={template.kind} className="rounded-md border border-border p-3">
                  <div className="flex flex-wrap items-center gap-2">
                    <KindBadge kind={template.kind} />
                    <p className="font-medium text-text-primary">{template.display_name}</p>
                    <StatusBadge value={template.implemented ? 'implemented' : 'planned'} />
                  </div>
                  <p className="mt-2 text-sm text-text-muted">{template.description}</p>
                  <div className="mt-3">
                    <JsonBlock value={template.capabilities} />
                  </div>
                </div>
              ))
            ) : (
              <EmptyState
                title={
                  templates.isLoading ? 'Loading runtime templates...' : 'No runtime templates'
                }
              />
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Installed agents</CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            {agents.data?.length ? (
              agents.data.map((agent) => (
                <div key={agent.id} className="rounded-md border border-border p-3">
                  <div className="flex flex-wrap items-center gap-2">
                    <p className="font-medium text-text-primary">{agent.name}</p>
                    <KindBadge kind={agent.kind} />
                    <StatusBadge value={agent.status} />
                  </div>
                  <p className="mt-1 break-all text-xs text-text-muted">{agent.paths.runtime}</p>
                </div>
              ))
            ) : (
              <EmptyState title={agents.isLoading ? 'Loading agents...' : 'No installed agents'} />
            )}
          </CardContent>
        </Card>
      </div>
    </>
  )
}
