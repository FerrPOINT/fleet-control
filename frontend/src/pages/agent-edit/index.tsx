import { FormEvent, useEffect, useMemo, useState } from 'react'
import { Link, useNavigate, useParams } from 'react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { FileCode2, Save, Wrench } from 'lucide-react'
import { toast } from 'sonner'
import { getAgent, listExecutors, listLeaderExecutors, updateAgent } from '@/api/fleet'
import type { AgentProductRole, AgentRole, UpdateAgentRequest } from '@/api/types'
import { Button } from '@sdlc/ui/ui'
import { Card, CardContent, CardHeader, CardTitle } from '@sdlc/ui/ui'
import { Input } from '@sdlc/ui/ui'
import { Label } from '@sdlc/ui/ui'
import { Textarea } from '@sdlc/ui/ui'
import { AgentIdentity, EmptyState, ErrorState, PageHeader, StatusBadge } from '../common'

export function AgentEditPage({ defaultProductRole }: { defaultProductRole?: AgentProductRole }) {
  const { t } = useTranslation()
  const { agentId, leaderId } = useParams()
  const id = leaderId ?? agentId
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const agent = useQuery({
    queryKey: ['agent', id],
    queryFn: () => getAgent(id!),
    enabled: Boolean(id),
  })
  const [productRole, setProductRole] = useState<AgentProductRole>(defaultProductRole ?? 'executor')
  const executors = useQuery({
    queryKey: ['executors'],
    queryFn: listExecutors,
    enabled: productRole === 'leader',
  })
  const team = useQuery({
    queryKey: ['leader-executors', id],
    queryFn: () => listLeaderExecutors(id!),
    enabled: Boolean(id && agent.data?.product_role === 'leader'),
  })
  const [role, setRole] = useState<AgentRole>(
    defaultProductRole === 'leader' ? 'it_lead' : 'developer',
  )
  const [displayName, setDisplayName] = useState('')
  const [description, setDescription] = useState('')
  const [namespaceId, setNamespaceId] = useState('')
  const [workflowId, setWorkflowId] = useState('')
  const [selectedExecutorIds, setSelectedExecutorIds] = useState<string[] | null>(null)

  useEffect(() => {
    if (!agent.data) return
    setProductRole(agent.data.product_role)
    setRole(agent.data.role)
    setDisplayName(agent.data.display_name)
    setDescription(agent.data.description ?? '')
    setNamespaceId(agent.data.namespace_id ?? '')
    setWorkflowId(agent.data.workflow_id ?? '')
  }, [agent.data])

  const currentTeamIds = useMemo(
    () => team.data?.map((item) => item.executor_agent_id) ?? [],
    [team.data],
  )
  const draftExecutorIds = selectedExecutorIds ?? currentTeamIds
  const teamRequired = productRole === 'leader' && agent.data?.product_role === 'leader'
  const teamUnavailable =
    productRole === 'leader' &&
    ((teamRequired && (team.isError || !team.data)) || executors.isError || !executors.data)
  const mutation = useMutation({
    mutationFn: () => {
      if (teamUnavailable) throw new Error(t('agentEdit.teamUnavailable'))
      const payload: UpdateAgentRequest = {
        product_role: productRole,
        role,
        display_name: displayName,
        description,
        namespace_id: namespaceId,
        workflow_id: workflowId,
      }
      if (productRole === 'leader') payload.executor_ids = draftExecutorIds
      return updateAgent(id!, payload)
    },
    onSuccess: async (updated) => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['agents'] }),
        queryClient.invalidateQueries({ queryKey: ['agent', id] }),
        queryClient.invalidateQueries({ queryKey: ['leaders'] }),
        queryClient.invalidateQueries({ queryKey: ['executors'] }),
        queryClient.invalidateQueries({ queryKey: ['leader-executors', id] }),
      ])
      toast.success(t('agentEdit.saved'))
      navigate(
        updated.product_role === 'leader' ? `/leaders/${updated.id}` : `/executors/${updated.id}`,
      )
    },
  })

  function handleProductRole(nextProductRole: AgentProductRole) {
    setProductRole(nextProductRole)
    if (nextProductRole === 'leader' && role !== 'it_lead') setRole('it_lead')
    if (nextProductRole === 'executor' && role === 'it_lead') setRole('developer')
  }

  function toggleExecutor(executorId: string) {
    setSelectedExecutorIds(
      draftExecutorIds.includes(executorId)
        ? draftExecutorIds.filter((id) => id !== executorId)
        : [...draftExecutorIds, executorId],
    )
  }

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (teamUnavailable) return
    mutation.mutate()
  }

  if (!id) return <ErrorState message={t('agentEdit.missingId')} />
  if (agent.isError)
    return (
      <div className="space-y-3">
        <ErrorState message={t('agentEdit.loadError')} />
        <Button type="button" variant="outline" onClick={() => void agent.refetch()}>
          {t('agentEdit.retry')}
        </Button>
      </div>
    )
  if (!agent.data) return <EmptyState title={t('agentEdit.loading')} />

  const roleOptions =
    productRole === 'leader'
      ? (['it_lead', 'custom'] as const)
      : (['developer', 'tester', 'custom'] as const)

  return (
    <>
      <PageHeader
        title={t('agentEdit.title', { name: agent.data.display_name })}
        description={t('agentEdit.description', { name: agent.data.name })}
        actions={
          <Button asChild variant="outline">
            <Link to={agent.data.product_role === 'leader' ? `/leaders/${id}` : `/executors/${id}`}>
              {t('agentEdit.back')}
            </Link>
          </Button>
        }
      />
      <form
        className="grid min-w-0 gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(0,420px)]"
        onSubmit={submit}
        aria-busy={mutation.isPending}
      >
        <Card>
          <CardHeader>
            <CardTitle>{t('agentEdit.identity')}</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-3">
            <AgentIdentity agent={agent.data} />
            <div className="grid gap-2">
              <Label htmlFor="edit-product-role">{t('agentEdit.productRole')}</Label>
              <select
                id="edit-product-role"
                value={productRole}
                disabled={mutation.isPending}
                onChange={(event) => handleProductRole(event.target.value as AgentProductRole)}
                className="h-10 rounded-md border border-border bg-background px-3 text-sm"
              >
                <option value="executor">{t('productRoles.executor')}</option>
                <option value="leader">{t('productRoles.leader')}</option>
              </select>
            </div>
            <div className="grid gap-2">
              <Label htmlFor="edit-role">{t('agentEdit.profile')}</Label>
              <select
                id="edit-role"
                value={role}
                disabled={mutation.isPending}
                onChange={(event) => setRole(event.target.value as AgentRole)}
                className="h-10 rounded-md border border-border bg-background px-3 text-sm"
              >
                {roleOptions.map((option) => (
                  <option key={option} value={option}>
                    {t(`agentRoles.${option}`)}
                  </option>
                ))}
              </select>
            </div>
            <div className="grid gap-2">
              <Label htmlFor="edit-display-name">{t('agentEdit.displayName')}</Label>
              <Input
                id="edit-display-name"
                value={displayName}
                disabled={mutation.isPending}
                onChange={(event) => setDisplayName(event.target.value)}
              />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="edit-description">{t('agentEdit.details')}</Label>
              <Textarea
                id="edit-description"
                value={description}
                disabled={mutation.isPending}
                onChange={(event) => setDescription(event.target.value)}
              />
            </div>
            <div className="grid gap-3 md:grid-cols-2">
              <div className="grid gap-2">
                <Label htmlFor="edit-namespace">{t('agentEdit.namespaceId')}</Label>
                <Input
                  id="edit-namespace"
                  value={namespaceId}
                  disabled={mutation.isPending}
                  onChange={(event) => setNamespaceId(event.target.value)}
                />
              </div>
              <div className="grid gap-2">
                <Label htmlFor="edit-workflow">{t('agentEdit.workflowId')}</Label>
                <Input
                  id="edit-workflow"
                  value={workflowId}
                  disabled={mutation.isPending}
                  onChange={(event) => setWorkflowId(event.target.value)}
                />
              </div>
            </div>
            {mutation.isError ? <ErrorState message={t('agentEdit.saveError')} /> : null}
            <Button
              type="submit"
              disabled={mutation.isPending || !displayName.trim() || teamUnavailable}
            >
              <Save className="h-4 w-4" />
              {mutation.isPending ? t('agentEdit.saving') : t('agentEdit.save')}
            </Button>
          </CardContent>
        </Card>

        <div className="grid content-start gap-4">
          {productRole === 'leader' ? (
            <Card>
              <CardHeader>
                <CardTitle>{t('agentEdit.managedExecutors')}</CardTitle>
              </CardHeader>
              <CardContent className="space-y-2">
                {teamRequired && team.isError ? (
                  <div className="space-y-2">
                    <ErrorState message={t('agentEdit.teamError')} />
                    <Button type="button" variant="outline" onClick={() => void team.refetch()}>
                      {t('agentEdit.retry')}
                    </Button>
                  </div>
                ) : null}
                {teamRequired && team.isLoading ? (
                  <p className="text-sm text-text-muted">{t('agentEdit.loadingTeam')}</p>
                ) : null}
                {executors.isError ? (
                  <div className="space-y-2">
                    <ErrorState message={t('agentEdit.executorsError')} />
                    <Button
                      type="button"
                      variant="outline"
                      onClick={() => void executors.refetch()}
                    >
                      {t('agentEdit.retry')}
                    </Button>
                  </div>
                ) : null}
                {executors.isLoading && !executors.data ? (
                  <p className="text-sm text-text-muted">{t('agentEdit.loadingExecutors')}</p>
                ) : null}
                {!teamUnavailable &&
                !executors.isError &&
                executors.data?.filter((executor) => executor.id !== id).length ? (
                  executors.data
                    .filter((executor) => executor.id !== id)
                    .map((executor) => (
                      <label
                        key={executor.id}
                        className="flex items-center gap-3 rounded-md border border-border p-3 text-sm"
                      >
                        <input
                          type="checkbox"
                          checked={draftExecutorIds.includes(executor.id)}
                          disabled={mutation.isPending}
                          onChange={() => toggleExecutor(executor.id)}
                        />
                        <span className="min-w-0 flex-1">
                          <span className="block truncate font-medium text-text-primary">
                            {executor.display_name}
                          </span>
                          <span className="block truncate text-xs text-text-muted">
                            {executor.name} · {t(`agentRoles.${executor.role}`)} ·{' '}
                            {executor.namespace_id ?? t('agent.unbound')}
                          </span>
                        </span>
                      </label>
                    ))
                ) : !teamUnavailable && !executors.isError ? (
                  <EmptyState
                    title={
                      executors.isLoading
                        ? t('agentEdit.loadingExecutors')
                        : t('agentEdit.noExecutors')
                    }
                  />
                ) : null}
              </CardContent>
            </Card>
          ) : null}

          <Card>
            <CardHeader>
              <CardTitle>{t('agentEdit.promptAndSkills')}</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <div className="flex flex-wrap gap-2">
                <StatusBadge value={agent.data.kind} />
                <StatusBadge value={agent.data.status} />
              </div>
              <Button asChild variant="outline">
                <Link to={`/agents/${id}/config`}>
                  <FileCode2 className="h-4 w-4" />
                  {t('agentEdit.openConfig')}
                </Link>
              </Button>
              <Button asChild variant="outline">
                <Link to={`/agents/${id}/skills`}>
                  <Wrench className="h-4 w-4" />
                  {t('agentEdit.openSkills')}
                </Link>
              </Button>
            </CardContent>
          </Card>
        </div>
      </form>
    </>
  )
}
