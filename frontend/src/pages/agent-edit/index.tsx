import { FormEvent, useEffect, useMemo, useState } from 'react'
import { Link, useNavigate, useParams } from 'react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { FileCode2, Save, Wrench } from 'lucide-react'
import { getAgent, listExecutors, listLeaderExecutors, updateAgent } from '@/api/fleet'
import type { AgentProductRole, AgentRole, UpdateAgentRequest } from '@/api/types'
import { Button } from '@/shared/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/shared/ui/card'
import { Input } from '@/shared/ui/input'
import { Label } from '@/shared/ui/label'
import { Textarea } from '@/shared/ui/textarea'
import { AgentIdentity, EmptyState, ErrorState, PageHeader, StatusBadge } from '../common'

export function AgentEditPage({ defaultProductRole }: { defaultProductRole?: AgentProductRole }) {
  const { agentId, leaderId } = useParams()
  const id = leaderId ?? agentId
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const agent = useQuery({
    queryKey: ['agent', id],
    queryFn: () => getAgent(id!),
    enabled: Boolean(id),
  })
  const executors = useQuery({ queryKey: ['executors'], queryFn: listExecutors })
  const team = useQuery({
    queryKey: ['leader-executors', id],
    queryFn: () => listLeaderExecutors(id!),
    enabled: Boolean(id),
  })
  const [productRole, setProductRole] = useState<AgentProductRole>(defaultProductRole ?? 'executor')
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
  const mutation = useMutation({
    mutationFn: () => {
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
    mutation.mutate()
  }

  if (!id) return <ErrorState message="Agent id is missing" />
  if (agent.isError) return <ErrorState message={agent.error.message} />
  if (!agent.data) return <EmptyState title="Loading agent..." />

  const roleOptions =
    productRole === 'leader'
      ? (['it_lead', 'custom'] as const)
      : (['developer', 'tester', 'custom'] as const)

  return (
    <>
      <PageHeader
        title={`Edit ${agent.data.display_name}`}
        description={`${agent.data.name} identity, product role and workflow binding.`}
        actions={
          <Button asChild variant="outline">
            <Link to={agent.data.product_role === 'leader' ? `/leaders/${id}` : `/executors/${id}`}>
              Back to overview
            </Link>
          </Button>
        }
      />
      <form className="grid gap-4 xl:grid-cols-[1fr_420px]" onSubmit={submit}>
        <Card>
          <CardHeader>
            <CardTitle>Identity</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-3">
            <AgentIdentity agent={agent.data} />
            <div className="grid gap-2">
              <Label htmlFor="edit-product-role">Product role</Label>
              <select
                id="edit-product-role"
                value={productRole}
                onChange={(event) => handleProductRole(event.target.value as AgentProductRole)}
                className="h-9 rounded-md border border-border bg-background px-3 text-sm"
              >
                <option value="executor">Executor</option>
                <option value="leader">Leader</option>
              </select>
            </div>
            <div className="grid gap-2">
              <Label htmlFor="edit-role">Profile</Label>
              <select
                id="edit-role"
                value={role}
                onChange={(event) => setRole(event.target.value as AgentRole)}
                className="h-9 rounded-md border border-border bg-background px-3 text-sm"
              >
                {roleOptions.map((option) => (
                  <option key={option} value={option}>
                    {option.replaceAll('_', ' ')}
                  </option>
                ))}
              </select>
            </div>
            <div className="grid gap-2">
              <Label htmlFor="edit-display-name">Display name</Label>
              <Input
                id="edit-display-name"
                value={displayName}
                onChange={(event) => setDisplayName(event.target.value)}
              />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="edit-description">Description</Label>
              <Textarea
                id="edit-description"
                value={description}
                onChange={(event) => setDescription(event.target.value)}
              />
            </div>
            <div className="grid gap-3 md:grid-cols-2">
              <div className="grid gap-2">
                <Label htmlFor="edit-namespace">Namespace ID</Label>
                <Input
                  id="edit-namespace"
                  value={namespaceId}
                  onChange={(event) => setNamespaceId(event.target.value)}
                />
              </div>
              <div className="grid gap-2">
                <Label htmlFor="edit-workflow">Workflow ID</Label>
                <Input
                  id="edit-workflow"
                  value={workflowId}
                  onChange={(event) => setWorkflowId(event.target.value)}
                />
              </div>
            </div>
            {mutation.isError ? <ErrorState message={mutation.error.message} /> : null}
            <Button type="submit" disabled={mutation.isPending || !displayName.trim()}>
              <Save className="h-4 w-4" />
              Save agent
            </Button>
          </CardContent>
        </Card>

        <div className="grid gap-4">
          {productRole === 'leader' ? (
            <Card>
              <CardHeader>
                <CardTitle>Managed executors</CardTitle>
              </CardHeader>
              <CardContent className="space-y-2">
                {executors.data?.filter((executor) => executor.id !== id).length ? (
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
                          onChange={() => toggleExecutor(executor.id)}
                        />
                        <span className="min-w-0 flex-1">
                          <span className="block truncate font-medium text-text-primary">
                            {executor.display_name}
                          </span>
                          <span className="block truncate text-xs text-text-muted">
                            {executor.name} - {executor.role} - {executor.namespace_id ?? 'unbound'}
                          </span>
                        </span>
                      </label>
                    ))
                ) : (
                  <EmptyState
                    title={executors.isLoading ? 'Loading executors...' : 'No executors yet'}
                  />
                )}
              </CardContent>
            </Card>
          ) : null}

          <Card>
            <CardHeader>
              <CardTitle>Prompt and skills</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <div className="flex flex-wrap gap-2">
                <StatusBadge value={agent.data.kind} />
                <StatusBadge value={agent.data.status} />
              </div>
              <Button asChild variant="outline">
                <Link to={`/agents/${id}/config`}>
                  <FileCode2 className="h-4 w-4" />
                  Open config
                </Link>
              </Button>
              <Button asChild variant="outline">
                <Link to={`/agents/${id}/skills`}>
                  <Wrench className="h-4 w-4" />
                  Open skills
                </Link>
              </Button>
            </CardContent>
          </Card>
        </div>
      </form>
    </>
  )
}
