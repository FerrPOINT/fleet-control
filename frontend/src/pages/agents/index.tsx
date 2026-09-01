import { FormEvent, useMemo, useState } from 'react'
import { Link, useNavigate } from 'react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Bot, Coffee, Plus, Rocket } from 'lucide-react'
import { createAgent, listAgents, listRuntimeTemplates, listSessions } from '@/api/fleet'
import type { AgentKind, AgentRole, AgentSession, CreateAgentRequest } from '@/api/types'
import { SessionUserFilter, useSessionUserFilter } from '@/shared/session-user-filter'
import { Button } from '@/shared/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/shared/ui/card'
import { Input } from '@/shared/ui/input'
import { Label } from '@/shared/ui/label'
import { Textarea } from '@/shared/ui/textarea'
import { UserAvatar } from '@/shared/ui/user-avatar'
import {
  AgentIdentity,
  EmptyState,
  ErrorState,
  JsonBlock,
  PageHeader,
  StatusBadge,
} from '../common'

export function AgentsPage({ createMode = false }: { createMode?: boolean }) {
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
      {createMode ? <CreateAgentPanel templates={templates.data ?? []} /> : null}
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
    grouped.set(session.agent_id, [...(grouped.get(session.agent_id) ?? []), session])
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
          {session.user_display_name} - {session.task_key ?? 'No task key'}
        </span>
      </span>
    </Link>
  )
}

function CreateAgentPanel({
  templates,
}: {
  templates: Awaited<ReturnType<typeof listRuntimeTemplates>>
}) {
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [kind, setKind] = useState<AgentKind>('hermes')
  const [role, setRole] = useState<AgentRole>('developer')
  const [displayName, setDisplayName] = useState('Developer Hermes')
  const [description, setDescription] = useState('Primary development workflow agent')
  const [namespaceId, setNamespaceId] = useState('dev')
  const [namespaceName, setNamespaceName] = useState('Development')
  const [workflowId, setWorkflowId] = useState('workflow-dev')
  const [workflowName, setWorkflowName] = useState('Developer Workflow')

  const selectedTemplate = useMemo(
    () => templates.find((template) => template.kind === kind),
    [kind, templates],
  )

  const mutation = useMutation({
    mutationFn: (payload: CreateAgentRequest) => createAgent(payload),
    onSuccess: async (agent) => {
      await queryClient.invalidateQueries({ queryKey: ['agents'] })
      navigate(`/agents/${agent.id}`)
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
      setDisplayName(role === 'tester' ? 'Tester Hermes' : 'Developer Hermes')
      setDescription(
        role === 'tester'
          ? 'QA and verification workflow agent'
          : 'Primary development workflow agent',
      )
    }
  }

  function handleRole(nextRole: AgentRole) {
    setRole(nextRole)
    if (kind !== 'hermes') return
    if (nextRole === 'tester') {
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

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    mutation.mutate({
      kind,
      role,
      display_name: displayName,
      description,
      namespace_id: namespaceId,
      namespace_name: namespaceName,
      workflow_id: workflowId,
      workflow_name: workflowName,
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
              <Label htmlFor="role">Role</Label>
              <select
                id="role"
                value={role}
                onChange={(event) => handleRole(event.target.value as AgentRole)}
                className="h-9 rounded-md border border-border bg-background px-3 text-sm"
              >
                <option value="developer">Developer</option>
                <option value="tester">Tester</option>
                <option value="custom">Custom</option>
              </select>
            </div>
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
