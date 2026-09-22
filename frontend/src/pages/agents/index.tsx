import { FormEvent, type ReactNode, useMemo, useState } from 'react'
import { Link, useNavigate } from 'react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { Archive, Bot, Coffee, HardDrive, Plus, Rocket, ShieldAlert } from 'lucide-react'
import {
  createAgent,
  getAgentStorageReview,
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
  AgentStorageReview,
  AgentStorageReviewItem,
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
  formatDate,
} from '../common'

export function AgentsPage({
  createMode = false,
  defaultProductRole = 'executor',
}: {
  createMode?: boolean
  defaultProductRole?: AgentProductRole
}) {
  const { t } = useTranslation()
  const agents = useQuery({ queryKey: ['agents'], queryFn: listAgents })
  const templates = useQuery({ queryKey: ['runtime-templates'], queryFn: listRuntimeTemplates })
  const userFilter = useSessionUserFilter()
  const sessions = useQuery({
    queryKey: ['sessions', 'agents', userFilter.selectedUserIds],
    queryFn: () => listSessions(undefined, userFilter.selectedUserIds),
    enabled: !createMode,
  })
  const storageReview = useQuery({
    queryKey: ['agent-storage-review'],
    queryFn: getAgentStorageReview,
    enabled: !createMode,
  })
  const sessionsByAgent = useMemo(() => groupSessionsByAgent(sessions.data ?? []), [sessions.data])

  return (
    <>
      <PageHeader
        title={createMode ? t('agents.createTitle') : t('agents.title')}
        description={t('agents.description')}
        actions={
          !createMode ? (
            <Button asChild>
              <Link to="/agents/new">
                <Plus className="h-4 w-4" />
                {t('agents.new')}
              </Link>
            </Button>
          ) : null
        }
      />
      {agents.isError ? (
        <RetryState message={t('agents.loadError')} onRetry={() => void agents.refetch()} />
      ) : null}
      {createMode && templates.isError ? (
        <RetryState message={t('agents.templatesError')} onRetry={() => void templates.refetch()} />
      ) : null}
      {createMode && templates.isPending ? (
        <EmptyState title={t('agents.loadingTemplates')} />
      ) : null}
      {createMode && templates.isSuccess && templates.data.length ? (
        <CreateAgentPanel templates={templates.data} defaultProductRole={defaultProductRole} />
      ) : null}
      {createMode && templates.isSuccess && !templates.data.length ? (
        <EmptyState title={t('agents.noTemplates')} />
      ) : null}
      {!createMode ? <SessionUserFilter filter={userFilter} className="mb-4" /> : null}
      {!createMode && sessions.isError ? (
        <RetryState message={t('agents.sessionsError')} onRetry={() => void sessions.refetch()} />
      ) : null}
      {!createMode ? (
        <StorageReviewPanel
          review={storageReview.data}
          isLoading={storageReview.isLoading}
          isError={storageReview.isError}
          onRetry={() => void storageReview.refetch()}
        />
      ) : null}
      {!agents.isError ? (
        agents.data?.length ? (
          <div className="mt-4 divide-y divide-border rounded-md border border-border bg-surface">
            {agents.data.map((agent) => {
              const agentSessions = sessionsByAgent.get(agent.id) ?? []
              return (
                <article key={agent.id} className="p-3 sm:p-4">
                  <div className="flex flex-wrap items-start justify-between gap-3">
                    <AgentIdentity agent={agent} />
                    <Button asChild variant="outline" size="sm" className="h-10">
                      <Link to={`/agents/${agent.id}`}>{t('agents.open')}</Link>
                    </Button>
                  </div>
                  <dl className="mt-3 flex flex-wrap gap-x-6 gap-y-2 border-t border-border pt-3 text-sm">
                    <div className="min-w-24">
                      <dt className="text-xs text-text-muted">{t('agents.apiPort')}</dt>
                      <dd className="font-medium text-text-primary">
                        {agent.api_port ?? t('agents.notAvailable')}
                      </dd>
                    </div>
                    <div className="min-w-24">
                      <dt className="text-xs text-text-muted">{t('agents.dashboard')}</dt>
                      <dd className="font-medium text-text-primary">
                        {agent.dashboard_port ?? t('agents.notAvailable')}
                      </dd>
                    </div>
                    <div className="min-w-0 flex-1 basis-40">
                      <dt className="text-xs text-text-muted">{t('agents.workflow')}</dt>
                      <dd className="truncate font-medium text-text-primary">
                        {agent.workflow_id ?? t('agents.unbound')}
                      </dd>
                    </div>
                  </dl>
                  <section className="mt-3 border-t border-border pt-3">
                    <div className="flex items-center justify-between gap-3">
                      <h3 className="text-xs font-medium uppercase text-text-muted">
                        {t('agents.sessions')}
                      </h3>
                      <span className="text-xs font-medium text-text-secondary">
                        {agentSessions.length}
                      </span>
                    </div>
                    <div className="mt-2 space-y-2">
                      {sessions.isLoading ? (
                        <p className="text-xs text-text-muted">{t('agents.loadingSessions')}</p>
                      ) : sessions.isError ? (
                        <p className="text-xs text-text-muted">{t('agents.sessionsUnavailable')}</p>
                      ) : agentSessions.length ? (
                        agentSessions
                          .slice(0, 2)
                          .map((session) => <SessionPreview key={session.id} session={session} />)
                      ) : (
                        <p className="text-xs text-text-muted">
                          {userFilter.selectedUserIds.length
                            ? t('agents.noFilteredSessions')
                            : t('agents.noSessions')}
                        </p>
                      )}
                    </div>
                  </section>
                </article>
              )
            })}
          </div>
        ) : (
          <EmptyState title={agents.isLoading ? t('agents.loading') : t('agents.empty')} />
        )
      ) : null}
    </>
  )
}

function StorageReviewPanel({
  review,
  isLoading,
  isError,
  onRetry,
}: {
  review?: AgentStorageReview
  isLoading: boolean
  isError: boolean
  onRetry: () => void
}) {
  const { t, i18n } = useTranslation()
  const candidates = review?.items.filter((item) => item.purge_eligible) ?? []
  const issues =
    review?.items.filter(
      (item) => !item.root_exists || (item.root_exists && !item.marker_verified),
    ) ?? []

  return (
    <section className="mb-4 space-y-4 border-y border-border py-4" aria-labelledby="storage-title">
      <h2
        id="storage-title"
        className="flex items-center gap-2 text-base font-semibold text-text-primary"
      >
        <HardDrive className="h-4 w-4" />
        {t('agents.storageTitle')}
      </h2>
      {isError ? <RetryState message={t('agents.storageError')} onRetry={onRetry} /> : null}
      {isLoading ? <p className="text-sm text-text-muted">{t('agents.reviewingStorage')}</p> : null}
      {review ? (
        <>
          <div className="grid grid-cols-2 gap-2 xl:grid-cols-5">
            <StorageMetric
              label={t('agents.managedSize')}
              value={formatBytes(review.total_bytes)}
            />
            <StorageMetric
              label={t('agents.archivedSize')}
              value={formatBytes(review.archived_bytes)}
            />
            <StorageMetric label={t('agents.totalAgents')} value={String(review.total_agents)} />
            <StorageMetric
              label={t('agents.purgeReady')}
              value={String(review.purge_eligible_agents)}
            />
            <StorageMetric
              label={t('agents.folderIssues')}
              value={String(review.marker_issue_agents + review.missing_root_agents)}
            />
          </div>
          <div className="grid gap-3 xl:grid-cols-2">
            <StorageList
              icon={<Archive className="h-4 w-4" />}
              title={t('agents.purgeCandidates')}
              empty={t('agents.noPurgeCandidates')}
              items={candidates}
            />
            <StorageList
              icon={<ShieldAlert className="h-4 w-4" />}
              title={t('agents.storageIssues')}
              empty={t('agents.noStorageIssues')}
              items={issues}
            />
          </div>
          <p className="text-xs text-text-muted">
            {t('agents.reviewedAt', {
              date: formatDate(review.reviewed_at, i18n.resolvedLanguage),
            })}
          </p>
        </>
      ) : null}
    </section>
  )
}

function StorageMetric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border border-border bg-background p-3">
      <p className="text-xs text-text-muted">{label}</p>
      <p className="mt-1 font-medium text-text-primary">{value}</p>
    </div>
  )
}

function StorageList({
  icon,
  title,
  empty,
  items,
}: {
  icon: ReactNode
  title: string
  empty: string
  items: AgentStorageReviewItem[]
}) {
  const { t } = useTranslation()

  function localizeRetentionHint(hint: string) {
    const staleDays = hint.match(/archived agent is stale \(over (\d+) days\)/)?.[1]
    if (staleDays) return t('agents.retentionStale', { days: staleDays })
    const key = {
      'agent folder is already absent': 'agents.retentionAbsent',
      'folder marker must match this agent before purge is allowed': 'agents.retentionMarker',
      'archived agent files can be purged explicitly by an operator': 'agents.retentionArchived',
      'archive the agent before physical purge': 'agents.retentionActive',
    }[hint]
    return key ? t(key) : hint
  }

  return (
    <div className="rounded-md border border-border bg-background p-3">
      <div className="flex items-center gap-2 text-sm font-medium text-text-primary">
        {icon}
        {title}
      </div>
      <div className="mt-3 grid gap-2">
        {items.length ? (
          items.slice(0, 3).map((item) => (
            <Link
              key={item.agent_id}
              to={`/agents/${item.agent_id}/workspace`}
              className="rounded-md border border-border p-2 hover:bg-surface-raised"
            >
              <div className="flex flex-wrap items-center justify-between gap-2">
                <span className="font-medium text-text-primary">{item.display_name}</span>
                <StatusBadge value={item.status} />
              </div>
              <p className="mt-1 text-xs text-text-muted">
                {item.agent_name} - {formatBytes(item.total_bytes)} -{' '}
                {localizeRetentionHint(item.retention_hint)}
              </p>
            </Link>
          ))
        ) : (
          <p className="text-sm text-text-muted">{empty}</p>
        )}
      </div>
    </div>
  )
}

function RetryState({ message, onRetry }: { message: string; onRetry: () => void }) {
  const { t } = useTranslation()
  return (
    <div className="space-y-2">
      <ErrorState message={message} />
      <Button type="button" variant="outline" className="h-10" onClick={onRetry}>
        {t('agents.retry')}
      </Button>
    </div>
  )
}

function formatBytes(bytes: number) {
  if (bytes < 1024) return `${bytes} B`
  const units = ['KB', 'MB', 'GB', 'TB']
  let value = bytes / 1024
  let unit = units[0]
  for (let index = 1; index < units.length && value >= 1024; index += 1) {
    value /= 1024
    unit = units[index]
  }
  return `${value.toFixed(value >= 10 ? 1 : 2)} ${unit}`
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
  const { t } = useTranslation()
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
          {session.user_display_name} - {session.leader_agent_name ?? t('agents.privateSession')} -{' '}
          {session.task_key ?? t('agents.noTaskKey')}
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
  const { t } = useTranslation()
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
      setDescription('Spring Boot runtime with process control and actuator health')
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
        <CardTitle>{t('agents.provisionWizard')}</CardTitle>
      </CardHeader>
      <CardContent>
        <form
          className="grid gap-4 xl:grid-cols-[1fr_1.2fr]"
          onSubmit={submit}
          aria-busy={mutation.isPending}
        >
          <fieldset className="contents" disabled={mutation.isPending}>
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
                <Label htmlFor="product-role">{t('agents.productRole')}</Label>
                <select
                  id="product-role"
                  value={productRole}
                  onChange={(event) => handleProductRole(event.target.value as AgentProductRole)}
                  className="h-10 rounded-md border border-border bg-background px-3 text-sm"
                >
                  <option value="executor">{t('agents.executor')}</option>
                  <option value="leader">{t('agents.leader')}</option>
                </select>
              </div>
              <div className="grid gap-2">
                <Label htmlFor="role">{t('agents.profile')}</Label>
                <select
                  id="role"
                  value={role}
                  onChange={(event) => handleRole(event.target.value as AgentRole)}
                  className="h-10 rounded-md border border-border bg-background px-3 text-sm"
                >
                  <option value="developer">{t('agents.developer')}</option>
                  <option value="tester">{t('agents.tester')}</option>
                  <option value="it_lead">{t('agents.itLead')}</option>
                  <option value="custom">{t('agents.custom')}</option>
                </select>
              </div>
              {productRole === 'leader' ? (
                <div className="grid gap-2">
                  <Label>{t('agents.managedExecutors')}</Label>
                  <div className="grid gap-2 rounded-md border border-border bg-background p-3">
                    {executors.isError ? (
                      <RetryState
                        message={t('agents.executorsError')}
                        onRetry={() => void executors.refetch()}
                      />
                    ) : executors.data?.length ? (
                      executors.data.map((executor) => (
                        <label
                          key={executor.id}
                          className="flex min-h-10 items-center gap-2 text-sm"
                        >
                          <input
                            type="checkbox"
                            className="h-4 w-4"
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
                        {executors.isLoading
                          ? t('agents.loadingExecutors')
                          : t('agents.noExecutors')}
                      </p>
                    )}
                  </div>
                </div>
              ) : null}
              <div className="grid gap-2">
                <Label htmlFor="display-name">{t('agents.displayName')}</Label>
                <Input
                  id="display-name"
                  value={displayName}
                  onChange={(event) => setDisplayName(event.target.value)}
                />
              </div>
              <div className="grid gap-2">
                <Label htmlFor="description">{t('agents.details')}</Label>
                <Textarea
                  id="description"
                  value={description}
                  onChange={(event) => setDescription(event.target.value)}
                />
              </div>
              <div className="grid gap-3 md:grid-cols-2">
                <div className="grid gap-2">
                  <Label htmlFor="namespace-id">{t('agents.namespaceId')}</Label>
                  <Input
                    id="namespace-id"
                    value={namespaceId}
                    onChange={(event) => setNamespaceId(event.target.value)}
                  />
                </div>
                <div className="grid gap-2">
                  <Label htmlFor="namespace-name">{t('agents.namespaceName')}</Label>
                  <Input
                    id="namespace-name"
                    value={namespaceName}
                    onChange={(event) => setNamespaceName(event.target.value)}
                  />
                </div>
                <div className="grid gap-2">
                  <Label htmlFor="workflow-id">{t('agents.workflowId')}</Label>
                  <Input
                    id="workflow-id"
                    value={workflowId}
                    onChange={(event) => setWorkflowId(event.target.value)}
                  />
                </div>
                <div className="grid gap-2">
                  <Label htmlFor="workflow-name">{t('agents.workflowName')}</Label>
                  <Input
                    id="workflow-name"
                    value={workflowName}
                    onChange={(event) => setWorkflowName(event.target.value)}
                  />
                </div>
              </div>
              {mutation.isError ? <ErrorState message={t('agents.createError')} /> : null}
              <Button
                type="submit"
                disabled={mutation.isPending || selectedTemplate?.implemented === false}
                aria-busy={mutation.isPending}
              >
                <Rocket className="h-4 w-4" />
                {mutation.isPending ? t('agents.creating') : t('agents.create')}
              </Button>
            </div>
          </fieldset>
        </form>
      </CardContent>
    </Card>
  )
}
