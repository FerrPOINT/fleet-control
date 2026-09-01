import { useQuery } from '@tanstack/react-query'
import { listAgents, listWorkflowBindings } from '@/api/fleet'
import { Card, CardContent, CardHeader, CardTitle } from '@/shared/ui/card'
import { EmptyState, ErrorState, PageHeader, StatusBadge } from '../common'

export function WorkflowsPage() {
  const bindings = useQuery({ queryKey: ['workflow-bindings'], queryFn: listWorkflowBindings })
  const agents = useQuery({ queryKey: ['agents'], queryFn: listAgents })

  if (bindings.isError) return <ErrorState message={bindings.error.message} />

  return (
    <>
      <PageHeader
        title="Workflow bindings"
        description="Fleet Control stores per-agent namespace bindings while project-workflow remains the workflow source of truth."
      />
      <Card>
        <CardHeader>
          <CardTitle>Bindings</CardTitle>
        </CardHeader>
        <CardContent className="space-y-2">
          {bindings.data?.length ? (
            bindings.data.map((binding) => {
              const agent = agents.data?.find((item) => item.id === binding.agent_id)
              return (
                <div
                  key={binding.id}
                  className="grid gap-3 rounded-md border border-border p-3 md:grid-cols-[1fr_1fr_auto]"
                >
                  <div>
                    <p className="font-medium text-text-primary">
                      {agent?.display_name ?? binding.agent_id}
                    </p>
                    <p className="text-xs text-text-muted">{agent?.name ?? 'agent not loaded'}</p>
                  </div>
                  <div>
                    <p className="text-sm text-text-primary">
                      {binding.workflow_name ?? binding.workflow_id ?? 'unbound'}
                    </p>
                    <p className="text-xs text-text-muted">
                      namespace {binding.namespace_name ?? binding.namespace_id ?? 'unbound'}
                    </p>
                  </div>
                  <StatusBadge value={binding.binding_status} />
                </div>
              )
            })
          ) : (
            <EmptyState
              title={bindings.isLoading ? 'Loading bindings...' : 'No workflow bindings'}
            />
          )}
        </CardContent>
      </Card>
    </>
  )
}
