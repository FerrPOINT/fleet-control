import { FormEvent, useMemo, useState } from 'react'
import { Link, useNavigate } from 'react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Bot, Coffee, Plus, Rocket } from 'lucide-react'
import {
  createAgent,
  listAgents,
  listExecutors,
  listRuntimeTemplates,
  listSessions,
} from '@/api/fleet'
import type {
  AgentKind,
  AgentProductRole,
  AgentRole,
  AgentSession,
  CreateAgentRequest,
} from '@/api/types'
import { SessionUserFilter, useSessionUserFilter } from '@/shared/session-user-filter'
import { Button } from '@sdlc/ui/ui'
import { Card, CardContent, CardHeader, CardTitle } from '@sdlc/ui/ui'
import { Input } from '@sdlc/ui/ui'
import { Label } from '@sdlc/ui/ui'
import { Textarea } from '@sdlc/ui/ui'
import { UserAvatar } from '@/shared/ui/user-avatar'
import {
  AgentIdentity,
  EmptyState,
  ErrorState,
  JsonBlock,
  PageHeader,
  StatusBadge,
} from '../common'

export function AgentsPage({
  createMode = false,
  defaultProductRole = 'executor',
}: {
  createMode?: boolean
  defaultProductRole?: AgentProductRole
}) {
  const agents = useQuery({ queryKey: ['agents'], queryFn: listAgents })
  const templates = useQuery({ queryKey: ['runtime-templates'], queryFn: listRuntimeTemplates })
  const userFilter = useSessionUserFilter()
  const sessions = useQuery({
    queryKey: ['sessions', 'agents', userFilter.selectedUserIds],
    queryFn: () => listSessions(undefined, userFilter.selectedUserIds),
    enabled: !createMode,
  })
  const sessionsByAgent = useMemo(() => groupSessionsByAgent(sessions.data ?? []), [sessions.data])

  return (
    <>
      <PageHeader
        title={createMode ? 'Create agent' : 'Agents'}
        description="Managed Hermes and Java Agent runtimes with isolated runtime, config, workspace and logs."
        actions={
          !createMode ? (
            <Button asChild>
              <Link to="/agents/new">
                <Plus className="h-4 w-4" />
                New agent
              </Link>
            </Button>
          ) : null
        }
      />
      {agents.isError ? <ErrorState message={agents.error.message} /> : null}
      {createMode ? (
        <CreateAgentPanel
          templates={templates.data ?? []}
          defaultProductRole={defaultProductRole}
        />
      ) : null}
      {!createMode ? <SessionUserFilter filter={userFilter} className="mb-4" /> : null}
      <div className="mt-4 grid gap-3 xl:grid-cols-2">
        {agents.data?.length ? (
          agents.data.map((agent) => (
            <Card key={agent.id}>
              <CardContent className="pt-4">
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <AgentIdentity agent={agent} />
                  <Button asChild variant="outline" size="sm">
                    <Link to={`/agents/${agent.id}`}>Open</Link>
                  </Button>
                </div>
                <dl className="mt-4 grid gap-3 text-sm sm:grid-cols-3">
                  <div>
                    <dt className="text-xs text-text-muted">API port</dt>
                    <dd className="font-medium text-text-primary">{agent.api_port ?? 'n/a'}</dd>
                  </div>
                  <div>
                    <dt className="text-xs text-text-muted">Dashboard</dt>
                    <dd className="font-medium text-text-primary">
                      {agent.dashboard_port ?? 'n/a'}
                    </dd>
                  </div>
                  <div>
                    <dt className="text-xs text-text-muted">Workflow</dt>
                    <dd className="font-medium text-text-primary">
                      {agent.workflow_id ?? 'unbound'}
                    </dd>
                  </div>
                </dl>
                <div className="mt-4 rounded-md border border-border bg-background p-3">
                  <div className="flex items-center justify-between gap-3">
                    <p className="text-xs font-medium uppercase text-text-muted">Sessions</p>
                    <span className="text-xs font-medium text-text-secondary">
                      {(sessionsByAgent.get(agent.id) ?? []).length}
                    </span>
                  </div>
                  <div className="mt-3 space-y-2">
                    {sessions.isLoading ? (
                      <p className="text-xs text-text-muted">Loading sessions...</p>
                    ) : (sessionsByAgent.get(agent.id) ?? []).length ? (
                      (sessionsByAgent.get(agent.id) ?? [])
                        .slice(0, 2)
                        .map((session) => <SessionPreview key={session.id} session={session} />)
                    ) : (
                      <p className="text-xs text-text-muted">
                        {userFilter.selectedUserIds.length
                          ? 'No sessions for selected users'
                          : 'No sessions yet'}
                      </p>
                    )}
                  </div>
                </div>
              </CardContent>
            </Card>
          ))
        ) : (
          <div className="xl:col-span-2">
            <EmptyState title={agents.isLoading ? 'Loading agents...' : 'No agents yet'} />
          </div>
        )}
      </div>
    </>
  )
}

function groupSessionsByAgent(sessions: AgentSession[]) {
  const grouped = new Map<string, AgentSession[]>()
  for (const session of sessions) {
    grouped.set(session.primary_agent_id, [
      ...(grouped.get(session.primary_agent_id) ?? []),
      session,
    ])
  }
  return grouped
}

function SessionPreview({ session }: { session: AgentSession }) {
  return (
    <Link
      to={`/sessions/${session.id}`}
      className="flex min-w-0 items-center gap-2 rounded-md border border-border p-2 hover:bg-surface-raised"
    >
      <UserAvatar name={session.user_display_name} userId={session.user_id} />
      <span className="min-w-0 flex-1">
        <span className="block truncate text-sm font-medium text-text-primary">
          {session.title}
        </span>
        <span className="block truncate text-xs text-text-muted">
          {session.user_display_name} - {session.leader_agent_name ?? 'private'} -{' '}
          {session.task_key ?? 'No task key'}
        </span>
      </span>
    </Link>
  )
}

function CreateAgentPanel({
  templates,
  defaultProductRole,
}: {
  templates: Awaited<ReturnType<typeof listRuntimeTemplates>>
  defaultProductRole: AgentProductRole
}) {
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const executors = useQuery({ queryKey: ['executors'], queryFn: listExecutors })
  const [kind, setKind] = useState<AgentKind>('hermes')
  const [productRole, setProductRole] = useState<AgentProductRole>(defaultProductRole)
  const [role, setRole] = useState<AgentRole>(
    defaultProductRole === 'leader' ? 'it_lead' : 'developer',
  )
  const [displayName, setDisplayName] = useState(
    defaultProductRole === 'leader' ? 'IT Lead Hermes' : 'Developer Hermes',
  )
  const [description, setDescription] = useState(
    defaultProductRole === 'leader'
      ? 'Team lead agent coordinating managed executors'
      : 'Primary development workflow agent',
  )
  const [namespaceId, setNamespaceId] = useState(defaultProductRole === 'leader' ? 'lead' : 'dev')
  const [namespaceName, setNamespaceName] = useState(
    defaultProductRole === 'leader' ? 'Leadership' : 'Development',
  )
  const [workflowId, setWorkflowId] = useState(
    defaultProductRole === 'leader' ? 'workflow-lead' : 'workflow-dev',
  )
  const [workflowName, setWorkflowName] = useState(
    defaultProductRole === 'leader' ? 'Leadership Workflow' : 'Developer Workflow',
  )
  const [executorIds, setExecutorIds] = useState<string[]>([])

  const selectedTemplate = useMemo(
    () => templates.find((template) => template.kind === kind),
    [kind, templates],
  )

  const mutation = useMutation({
    mutationFn: (payload: CreateAgentRequest) => createAgent(payload),
    onSuccess: async (agent) => {
      await queryClient.invalidateQueries({ queryKey: ['agents'] })
      await queryClient.invalidateQueries({
        queryKey: [agent.product_role === 'leader' ? 'leaders' : 'executors'],
      })
      navigate(agent.product_role === 'leader' ? `/leaders/${agent.id}` : `/executors/${agent.id}`)
    },
  })

  function selectKind(nextKind: AgentKind) {
    setKind(nextKind)
    if (nextKind === 'java_agent') {
      setDisplayName('Java Agent')
      setDescription('Spring Boot runtime contract for phase 2')
      setNamespaceId('java')
      setNamespaceName('Java Agent')
      setWorkflowId('workflow-java')
      setWorkflowName('Java Agent Workflow')
    } else {
      applyRoleDefaults(productRole, role)
    }
  }

  function handleProductRole(nextProductRole: AgentProductRole) {
    setProductRole(nextProductRole)
    const nextRole =
      nextProductRole === 'leader' ? 'it_lead' : role === 'it_lead' ? 'developer' : role
    setRole(nextRole)
    applyRoleDefaults(nextProductRole, nextRole)
  }

  function handleRole(nextRole: AgentRole) {
    setRole(nextRole)
    if (kind !== 'hermes') return
    applyRoleDefaults(productRole, nextRole)
  }

  function applyRoleDefaults(nextProductRole: AgentProductRole, nextRole: AgentRole) {
    if (nextProductRole === 'leader') {
      setDisplayName('IT Lead Hermes')
      setDescription('Team lead agent coordinating managed executors')
      setNamespaceId('lead')
      setNamespaceName('Leadership')
      setWorkflowId('workflow-lead')
      setWorkflowName('Leadership Workflow')
    } else if (nextRole === 'tester') {
      setDisplayName('Tester Hermes')
      setDescription('QA and verification workflow agent')
      setNamespaceId('qa')
      setNamespaceName('Quality Assurance')
      setWorkflowId('workflow-qa')
      setWorkflowName('Tester Workflow')
    } else if (nextRole === 'developer') {
      setDisplayName('Developer Hermes')
      setDescription('Primary development workflow agent')
      setNamespaceId('dev')
      setNamespaceName('Development')
      setWorkflowId('workflow-dev')
      setWorkflowName('Developer Workflow')
    }
  }

  function toggleExecutor(executorId: string) {
    setExecutorIds((current) =>
      current.includes(executorId)
        ? current.filter((id) => id !== executorId)
        : [...current, executorId],
    )
  }

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    mutation.mutate({
      kind,
      product_role: productRole,
      role,
      display_name: displayName,
      description,
      namespace_id: namespaceId,
      namespace_name: namespaceName,
      workflow_id: workflowId,
      workflow_name: workflowName,
      executor_ids: productRole === 'leader' ? executorIds : [],
    })
  }

  return (
    <Card className="mb-5">
      <CardHeader>
        <CardTitle>Provision wizard</CardTitle>
      </CardHeader>
      <CardContent>
        <form className="grid gap-4 xl:grid-cols-[1fr_1.2fr]" onSubmit={submit}>
          <div className="grid gap-3">
            {templates.map((template) => (
              <button
                type="button"
                key={template.kind}
                onClick={() => selectKind(template.kind)}
                className={`rounded-md border p-4 text-left transition-colors ${
                  kind === template.kind
                    ? 'border-accent bg-accent/10'
                    : 'border-border bg-background hover:bg-surface-raised'
                }`}
              >
                <div className="flex items-center justify-between gap-2">
                  <div className="flex items-center gap-2">
                    {template.kind === 'hermes' ? (
                      <Bot className="h-4 w-4" />
                    ) : (
                      <Coffee className="h-4 w-4" />
                    )}
                    <span className="font-medium text-text-primary">{template.display_name}</span>
                  </div>
                  <StatusBadge value={template.implemented ? 'implemented' : 'planned'} />
                </div>
                <p className="mt-2 text-sm text-text-muted">{template.description}</p>
              </button>
            ))}
            <JsonBlock value={selectedTemplate?.capabilities ?? {}} />
          </div>

          <div className="grid gap-3">
            <div className="grid gap-2">
              <Label htmlFor="product-role">Product role</Label>
              <select
                id="product-role"
                value={productRole}
                onChange={(event) => handleProductRole(event.target.value as AgentProductRole)}
                className="h-9 rounded-md border border-border bg-background px-3 text-sm"
              >
                <option value="executor">Executor</option>
                <option value="leader">Leader</option>
              </select>
            </div>
            <div className="grid gap-2">
              <Label htmlFor="role">Profile</Label>
              <select
                id="role"
                value={role}
                onChange={(event) => handleRole(event.target.value as AgentRole)}
                className="h-9 rounded-md border border-border bg-background px-3 text-sm"
              >
                <option value="developer">Developer</option>
                <option value="tester">Tester</option>
                <option value="it_lead">IT lead</option>
                <option value="custom">Custom</option>
              </select>
            </div>
            {productRole === 'leader' ? (
              <div className="grid gap-2">
                <Label>Managed executors</Label>
                <div className="grid gap-2 rounded-md border border-border bg-background p-3">
                  {executors.data?.length ? (
                    executors.data.map((executor) => (
                      <label key={executor.id} className="flex items-center gap-2 text-sm">
                        <input
                          type="checkbox"
                          checked={executorIds.includes(executor.id)}
                          onChange={() => toggleExecutor(executor.id)}
                        />
                        <span className="min-w-0 truncate">
                          {executor.display_name} - {executor.name}
                        </span>
                      </label>
                    ))
                  ) : (
                    <p className="text-sm text-text-muted">
                      {executors.isLoading ? 'Loading executors...' : 'No executors yet'}
                    </p>
                  )}
                </div>
              </div>
            ) : null}
            <div className="grid gap-2">
              <Label htmlFor="display-name">Display name</Label>
              <Input
                id="display-name"
                value={displayName}
                onChange={(event) => setDisplayName(event.target.value)}
              />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="description">Description</Label>
              <Textarea
                id="description"
                value={description}
                onChange={(event) => setDescription(event.target.value)}
              />
            </div>
            <div className="grid gap-3 md:grid-cols-2">
              <div className="grid gap-2">
                <Label htmlFor="namespace-id">Namespace ID</Label>
                <Input
                  id="namespace-id"
                  value={namespaceId}
                  onChange={(event) => setNamespaceId(event.target.value)}
                />
              </div>
              <div className="grid gap-2">
                <Label htmlFor="namespace-name">Namespace name</Label>
                <Input
                  id="namespace-name"
                  value={namespaceName}
                  onChange={(event) => setNamespaceName(event.target.value)}
                />
              </div>
              <div className="grid gap-2">
                <Label htmlFor="workflow-id">Workflow ID</Label>
                <Input
                  id="workflow-id"
                  value={workflowId}
                  onChange={(event) => setWorkflowId(event.target.value)}
                />
              </div>
              <div className="grid gap-2">
                <Label htmlFor="workflow-name">Workflow name</Label>
                <Input
                  id="workflow-name"
                  value={workflowName}
                  onChange={(event) => setWorkflowName(event.target.value)}
                />
              </div>
            </div>
            {mutation.isError ? <ErrorState message={mutation.error.message} /> : null}
            <Button
              type="submit"
              disabled={mutation.isPending || selectedTemplate?.implemented === false}
            >
              <Rocket className="h-4 w-4" />
              Create agent
            </Button>
          </div>
        </form>
      </CardContent>
    </Card>
  )
}
