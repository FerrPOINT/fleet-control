import { useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  getWorkflowCatalog,
  listAgents,
  listWorkflowBindings,
  rebindWorkflowBinding,
} from '@/api/fleet'
import type { WorkflowNamespaceCatalogEntry } from '@/api/types'
import { Button, Card, CardContent, CardHeader, CardTitle } from '@sdlc/ui/ui'
import { EmptyState, ErrorState, PageHeader, StatusBadge } from '../common'

export function WorkflowsPage() {
  const queryClient = useQueryClient()
  const [selectedNamespaces, setSelectedNamespaces] = useState<Record<string, string>>({})
  const bindings = useQuery({ queryKey: ['workflow-bindings'], queryFn: listWorkflowBindings })
  const agents = useQuery({ queryKey: ['agents'], queryFn: listAgents })
  const catalog = useQuery({ queryKey: ['workflow-catalog'], queryFn: getWorkflowCatalog })
  const rebind = useMutation({
    mutationFn: ({
      agentId,
      namespace,
    }: {
      agentId: string
      namespace: WorkflowNamespaceCatalogEntry
    }) =>
      rebindWorkflowBinding(agentId, {
        namespace_id: namespace.id,
        workflow_id: namespace.workflow_id,
      }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['workflow-bindings'] }),
  })

  if (bindings.isError) return <ErrorState message={bindings.error.message} />

  return (
    <>
      <PageHeader
        title="Workflow bindings"
        description="Project Workflow owns the catalog. Rebinding is an explicit operator action and never retargets stale agents automatically."
      />
      {catalog.isError ? <ErrorState message={catalog.error.message} /> : null}
      <Card>
        <CardHeader>
          <CardTitle>Bindings</CardTitle>
        </CardHeader>
        <CardContent className="space-y-2">
          {bindings.data?.length ? (
            bindings.data.map((binding) => {
              const agent = agents.data?.find((item) => item.id === binding.agent_id)
              const selectedNamespaceId =
                selectedNamespaces[binding.agent_id] ?? catalog.data?.namespaces[0]?.id
              const selectedNamespace = catalog.data?.namespaces.find(
                (namespace) => namespace.id === selectedNamespaceId,
              )
              const workflowName = selectedNamespace
                ? catalog.data?.workflows.find(
                    (workflow) => workflow.id === selectedNamespace.workflow_id,
                  )?.name
                : undefined
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
                    {binding.binding_status === 'stale' && catalog.data?.namespaces.length ? (
                      <div className="mt-3 flex flex-wrap items-center gap-2">
                        <label className="sr-only" htmlFor={`namespace-${binding.agent_id}`}>
                          Workflow namespace for {agent?.display_name ?? binding.agent_id}
                        </label>
                        <select
                          id={`namespace-${binding.agent_id}`}
                          className="h-9 rounded-md border border-border bg-surface px-2 text-sm text-text-primary"
                          value={selectedNamespaceId}
                          onChange={(event) =>
                            setSelectedNamespaces((current) => ({
                              ...current,
                              [binding.agent_id]: event.target.value,
                            }))
                          }
                        >
                          {catalog.data.namespaces.map((namespace) => (
                            <option key={namespace.id} value={namespace.id}>
                              {namespace.name}
                            </option>
                          ))}
                        </select>
                        <Button
                          size="sm"
                          variant="outline"
                          disabled={!selectedNamespace || rebind.isPending}
                          onClick={() => {
                            if (selectedNamespace) {
                              rebind.mutate({
                                agentId: binding.agent_id,
                                namespace: selectedNamespace,
                              })
                            }
                          }}
                        >
                          {rebind.isPending
                            ? 'Rebinding...'
                            : `Rebind to ${workflowName ?? 'workflow'}`}
                        </Button>
                      </div>
                    ) : null}
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
