import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import {
  getWorkflowCatalog,
  listAgents,
  listWorkflowBindings,
  rebindWorkflowBinding,
} from '@/api/fleet'
import type {
  Agent,
  WorkflowBinding,
  WorkflowCatalog,
  WorkflowNamespaceCatalogEntry,
} from '@/api/types'
import { Button, Input, Label } from '@sdlc/ui/ui'
import { EmptyState, ErrorState, PageHeader, StatusBadge } from '../common'

const statuses = ['all', 'stale', 'connected', 'unbound', 'pending'] as const

export function WorkflowsPage() {
  const { t } = useTranslation()
  const [search, setSearch] = useState('')
  const [status, setStatus] = useState<(typeof statuses)[number]>('all')
  const bindings = useQuery({ queryKey: ['workflow-bindings'], queryFn: listWorkflowBindings })
  const agents = useQuery({ queryKey: ['agents'], queryFn: listAgents })
  const catalog = useQuery({ queryKey: ['workflow-catalog'], queryFn: getWorkflowCatalog })
  const agentsById = useMemo(
    () => new Map((agents.data ?? []).map((agent) => [agent.id, agent])),
    [agents.data],
  )
  const visibleBindings = useMemo(() => {
    const term = search.trim().toLocaleLowerCase()
    return (bindings.data ?? []).filter((binding) => {
      if (status !== 'all' && binding.binding_status !== status) return false
      if (!term) return true
      const agent = agentsById.get(binding.agent_id)
      return [
        agent?.name,
        agent?.display_name,
        binding.agent_id,
        binding.workflow_name,
        binding.namespace_name,
      ].some((value) => value?.toLocaleLowerCase().includes(term))
    })
  }, [agentsById, bindings.data, search, status])

  return (
    <>
      <PageHeader title={t('workflows.title')} description={t('workflows.description')} />
      {bindings.isError ? (
        <div className="space-y-2">
          <ErrorState message={`${t('workflows.loadError')} ${bindings.error.message}`} />
          <Button type="button" variant="outline" onClick={() => void bindings.refetch()}>
            {t('workflows.retry')}
          </Button>
        </div>
      ) : (
        <>
          {agents.isError ? (
            <div className="mb-3 space-y-2">
              <ErrorState message={t('workflows.agentsError')} />
              <Button type="button" variant="outline" onClick={() => void agents.refetch()}>
                {t('workflows.retry')}
              </Button>
            </div>
          ) : null}
          {catalog.isError ? (
            <div className="mb-3 space-y-2">
              <ErrorState message={t('workflows.catalogError')} />
              <Button type="button" variant="outline" onClick={() => void catalog.refetch()}>
                {t('workflows.retry')}
              </Button>
            </div>
          ) : null}
          <div className="mb-3 flex flex-wrap items-end gap-3">
            <div className="min-w-0 flex-1 basis-60">
              <Label htmlFor="workflow-search">{t('workflows.search')}</Label>
              <Input
                id="workflow-search"
                className="mt-1 h-10"
                value={search}
                onChange={(event) => setSearch(event.target.value)}
              />
            </div>
            <div>
              <Label htmlFor="workflow-status">{t('workflows.status')}</Label>
              <select
                id="workflow-status"
                className="mt-1 h-10 rounded-md border border-border bg-background px-3 text-sm text-text-primary"
                value={status}
                onChange={(event) => setStatus(event.target.value as (typeof statuses)[number])}
              >
                {statuses.map((item) => (
                  <option key={item} value={item}>
                    {t(`workflows.filters.${item}`)}
                  </option>
                ))}
              </select>
            </div>
            <p className="pb-2 text-sm text-text-muted">
              {t('workflows.count', {
                shown: visibleBindings.length,
                total: bindings.data?.length ?? 0,
              })}
            </p>
          </div>
          {bindings.isLoading ? <EmptyState title={t('workflows.loading')} /> : null}
          {!bindings.isLoading && !bindings.data?.length ? (
            <EmptyState title={t('workflows.empty')} />
          ) : null}
          {bindings.data?.length && !visibleBindings.length ? (
            <EmptyState title={t('workflows.noMatches')} />
          ) : null}
          {visibleBindings.length ? (
            <div className="divide-y divide-border border-y border-border">
              {visibleBindings.map((binding) => (
                <BindingRow
                  key={binding.id}
                  binding={binding}
                  agent={agentsById.get(binding.agent_id)}
                  catalog={catalog.data}
                  catalogLoading={catalog.isLoading}
                />
              ))}
            </div>
          ) : null}
        </>
      )}
    </>
  )
}

function BindingRow({
  binding,
  agent,
  catalog,
  catalogLoading,
}: {
  binding: WorkflowBinding
  agent?: Agent
  catalog?: WorkflowCatalog
  catalogLoading: boolean
}) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const [selectedNamespaceId, setSelectedNamespaceId] = useState('')
  const namespaceId = selectedNamespaceId || catalog?.namespaces[0]?.id || ''
  const selectedNamespace = catalog?.namespaces.find((item) => item.id === namespaceId)
  const rebind = useMutation({
    mutationFn: (namespace: WorkflowNamespaceCatalogEntry) =>
      rebindWorkflowBinding(binding.agent_id, {
        namespace_id: namespace.id,
        workflow_id: namespace.workflow_id,
      }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ['workflow-bindings'] })
      toast.success(t('workflows.rebindSuccess'))
    },
  })

  return (
    <div className="grid min-w-0 gap-3 py-3 md:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto] md:items-start">
      <div className="min-w-0">
        <p className="break-words font-medium text-text-primary">
          {agent?.display_name ?? binding.agent_id}
        </p>
        <p className="break-all text-xs text-text-muted">{agent?.name ?? binding.agent_id}</p>
      </div>
      <div className="min-w-0 text-sm">
        <p className="break-words text-text-primary">
          {binding.workflow_name ?? binding.workflow_id ?? t('agent.unbound')}
        </p>
        <p className="break-words text-text-muted">
          {t('workflows.namespace')}:{' '}
          {binding.namespace_name ?? binding.namespace_id ?? t('agent.unbound')}
        </p>
        {binding.binding_status === 'stale' ? (
          <div className="mt-2 flex flex-wrap items-end gap-2">
            <div className="min-w-0 flex-1 basis-40">
              <Label htmlFor={`namespace-${binding.id}`}>
                {t('workflows.targetNamespace', { name: agent?.display_name ?? binding.agent_id })}
              </Label>
              <select
                id={`namespace-${binding.id}`}
                className="mt-1 h-10 w-full min-w-0 rounded-md border border-border bg-background px-2 text-sm text-text-primary"
                value={namespaceId}
                disabled={!catalog?.namespaces.length || rebind.isPending}
                onChange={(event) => setSelectedNamespaceId(event.target.value)}
              >
                {!namespaceId ? (
                  <option value="">
                    {catalogLoading ? t('workflows.catalogLoading') : t('workflows.noNamespaces')}
                  </option>
                ) : null}
                {catalog?.namespaces.map((namespace) => (
                  <option key={namespace.id} value={namespace.id}>
                    {namespace.name}
                  </option>
                ))}
              </select>
            </div>
            <Button
              type="button"
              variant="outline"
              disabled={!selectedNamespace || rebind.isPending}
              aria-busy={rebind.isPending}
              onClick={() => {
                if (selectedNamespace) rebind.mutate(selectedNamespace)
              }}
            >
              {rebind.isPending ? t('workflows.rebinding') : t('workflows.rebind')}
            </Button>
          </div>
        ) : null}
        {rebind.isError ? (
          <div className="mt-2">
            <ErrorState message={`${t('workflows.rebindError')} ${rebind.error.message}`} />
          </div>
        ) : null}
      </div>
      <StatusBadge value={binding.binding_status} />
    </div>
  )
}
