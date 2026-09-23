import { useMemo, useState } from 'react'
import { useSearchParams } from 'react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Ban, PackagePlus } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import {
  bulkCreateDeploymentJobs,
  cancelDeploymentJob,
  createDeploymentJob,
  getDeploymentJob,
  listAgents,
  listDeploymentJobs,
  listRuntimeTemplates,
} from '@/api/fleet'
import type { AgentKind, DeploymentJob, DeploymentJobKind } from '@/api/types'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  Button,
} from '@sdlc/ui/ui'
import { Card, CardContent, CardHeader, CardTitle } from '@sdlc/ui/ui'
import { Input } from '@sdlc/ui/ui'
import { Label } from '@sdlc/ui/ui'
import {
  EmptyState,
  ErrorState,
  JsonBlock,
  KindBadge,
  PageHeader,
  StatusBadge,
  formatDate,
} from '../common'

const tabs = ['overview', 'jobs', 'detail'] as const
type DeploymentTab = (typeof tabs)[number]

export function DeploymentsPage() {
  const { t } = useTranslation()
  const [params, setParams] = useSearchParams()
  const selectedTab = tabs.includes(params.get('tab') as DeploymentTab)
    ? (params.get('tab') as DeploymentTab)
    : 'overview'
  const selectedJobId = params.get('job_id')

  function selectTab(tab: DeploymentTab) {
    if (tab === 'detail' && selectedJobId) {
      setParams({ tab, job_id: selectedJobId })
      return
    }
    setParams({ tab })
  }

  return (
    <>
      <PageHeader title={t('deployments.title')} description={t('deployments.description')} />
      <nav className="mb-4 flex flex-wrap gap-2" aria-label={t('deployments.sections')}>
        {tabs.map((tab) => (
          <Button
            key={tab}
            type="button"
            className="h-10"
            variant={selectedTab === tab ? 'default' : 'outline'}
            aria-pressed={selectedTab === tab}
            onClick={() => selectTab(tab)}
          >
            {t(`deployments.tabs.${tab}`)}
          </Button>
        ))}
      </nav>
      {selectedTab === 'overview' ? <DeploymentOverview /> : null}
      {selectedTab === 'jobs' ? (
        <DeploymentJobs onOpen={(jobId) => setParams({ tab: 'detail', job_id: jobId })} />
      ) : null}
      {selectedTab === 'detail' ? <DeploymentJobDetail jobId={selectedJobId} /> : null}
    </>
  )
}

function DeploymentOverview() {
  const { t } = useTranslation()
  const templates = useQuery({ queryKey: ['runtime-templates'], queryFn: listRuntimeTemplates })
  const agents = useQuery({ queryKey: ['agents'], queryFn: listAgents })

  return (
    <div className="grid gap-4 xl:grid-cols-2">
      <Card>
        <CardHeader>
          <CardTitle>{t('deployments.templates')}</CardTitle>
        </CardHeader>
        <CardContent>
          {templates.isError ? (
            <RetryState
              message={t('deployments.templatesError')}
              onRetry={() => void templates.refetch()}
            />
          ) : templates.isPending ? (
            <EmptyState title={t('deployments.loadingTemplates')} />
          ) : templates.data.length ? (
            <ul className="divide-y divide-border rounded-md border border-border">
              {templates.data.map((template) => (
                <li key={template.kind} className="p-3">
                  <div className="flex flex-wrap items-center gap-2">
                    <KindBadge kind={template.kind} />
                    <p className="font-medium text-text-primary">{template.display_name}</p>
                    <StatusBadge value={template.implemented ? 'implemented' : 'planned'} />
                    <StatusBadge value={template.enabled ? 'enabled' : 'disabled'} />
                  </div>
                  <p className="mt-2 text-sm text-text-muted">{template.description}</p>
                  <details className="mt-2">
                    <summary className="flex min-h-10 cursor-pointer items-center text-sm font-medium text-accent">
                      {t('deployments.capabilities')}
                    </summary>
                    <JsonBlock value={template.capabilities} />
                  </details>
                </li>
              ))}
            </ul>
          ) : (
            <EmptyState title={t('deployments.noTemplates')} />
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t('deployments.installedAgents')}</CardTitle>
        </CardHeader>
        <CardContent>
          {agents.isError ? (
            <RetryState
              message={t('deployments.agentsError')}
              onRetry={() => void agents.refetch()}
            />
          ) : agents.isPending ? (
            <EmptyState title={t('deployments.loadingAgents')} />
          ) : agents.data.length ? (
            <ul className="divide-y divide-border rounded-md border border-border">
              {agents.data.map((agent) => (
                <li key={agent.id} className="p-3">
                  <div className="flex flex-wrap items-center gap-2">
                    <p className="font-medium text-text-primary">{agent.display_name}</p>
                    <KindBadge kind={agent.kind} />
                    <StatusBadge value={agent.status} />
                  </div>
                  <p className="mt-1 text-xs text-text-muted">{agent.name}</p>
                  <p className="mt-1 break-all font-mono text-xs text-text-muted">
                    {agent.paths.runtime}
                  </p>
                </li>
              ))}
            </ul>
          ) : (
            <EmptyState title={t('deployments.noInstalledAgents')} />
          )}
        </CardContent>
      </Card>
    </div>
  )
}

function DeploymentJobs({ onOpen }: { onOpen: (jobId: string) => void }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const jobs = useQuery({ queryKey: ['deployment-jobs'], queryFn: () => listDeploymentJobs(100) })
  const agents = useQuery({ queryKey: ['agents'], queryFn: listAgents })
  const [title, setTitle] = useState(() => t('deployments.provisionDefaultTitle'))
  const [jobKind, setJobKind] = useState<DeploymentJobKind>('provision')
  const [runtimeKind, setRuntimeKind] = useState<AgentKind>('hermes')
  const [agentId, setAgentId] = useState('')
  const [bulkSelected, setBulkSelected] = useState<string[]>([])
  const [bulkRollback, setBulkRollback] = useState(false)
  const [bulkTitle, setBulkTitle] = useState(() => t('deployments.bulkDefaultTitle'))
  const [cancelTarget, setCancelTarget] = useState<DeploymentJob | null>(null)
  const bulkAgents = (agents.data ?? []).filter((agent) => agent.status !== 'archived')

  const createMutation = useMutation({
    mutationFn: () =>
      createDeploymentJob({
        title: title.trim(),
        job_kind: jobKind,
        runtime_kind: runtimeKind,
        agent_id: agentId || null,
        detail: { requested_from: 'deployments_page' },
      }),
    onSuccess: async (created) => {
      await queryClient.invalidateQueries({ queryKey: ['deployment-jobs'] })
      toast.success(t('deployments.jobCreated', { title: created.title }))
    },
  })
  const cancelMutation = useMutation({
    mutationFn: cancelDeploymentJob,
    onSuccess: async (cancelled) => {
      await queryClient.invalidateQueries({ queryKey: ['deployment-jobs'] })
      setCancelTarget(null)
      toast.success(t('deployments.jobCancelled', { title: cancelled.title }))
    },
  })
  const bulkMutation = useMutation({
    mutationFn: () =>
      bulkCreateDeploymentJobs({
        job_kind: 'runtime_update',
        agent_ids: bulkSelected,
        title: bulkTitle.trim(),
        rollback: bulkRollback,
        detail: { requested_from: 'deployments_page_bulk' },
      }),
    onSuccess: async (result) => {
      await queryClient.invalidateQueries({ queryKey: ['deployment-jobs'] })
      setBulkSelected([])
      toast.success(t('deployments.bulkCreated', { count: result.created }))
    },
  })

  function changeJobKind(value: string) {
    const nextKind = value as DeploymentJobKind
    const knownDefaults = [
      t('deployments.provisionDefaultTitle'),
      t('deployments.updateDefaultTitle'),
    ]
    if (knownDefaults.includes(title)) {
      setTitle(
        nextKind === 'provision'
          ? t('deployments.provisionDefaultTitle')
          : t('deployments.updateDefaultTitle'),
      )
    }
    setJobKind(nextKind)
    createMutation.reset()
  }

  function toggleBulkAgent(selectedAgentId: string) {
    setBulkSelected((current) =>
      current.includes(selectedAgentId)
        ? current.filter((id) => id !== selectedAgentId)
        : [...current, selectedAgentId],
    )
    bulkMutation.reset()
  }

  return (
    <div className="space-y-4">
      <div className="grid gap-4 xl:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>{t('deployments.bulkActions')}</CardTitle>
          </CardHeader>
          <CardContent>
            <form
              className="grid gap-3"
              aria-busy={bulkMutation.isPending}
              onSubmit={(event) => {
                event.preventDefault()
                if (bulkSelected.length && bulkTitle.trim()) bulkMutation.mutate()
              }}
            >
              <p className="text-sm text-text-muted">
                {t('deployments.bulkDescription', {
                  selected: bulkSelected.length,
                  total: bulkAgents.length,
                })}
              </p>
              <div className="grid gap-2">
                <Label htmlFor="bulk-deployment-title">{t('deployments.jobTitle')}</Label>
                <Input
                  id="bulk-deployment-title"
                  className="h-10"
                  value={bulkTitle}
                  required
                  disabled={bulkMutation.isPending}
                  onChange={(event) => {
                    setBulkTitle(event.target.value)
                    bulkMutation.reset()
                  }}
                />
              </div>
              {agents.isError ? (
                <RetryState
                  message={t('deployments.bulkAgentsError')}
                  onRetry={() => void agents.refetch()}
                />
              ) : agents.isPending ? (
                <EmptyState title={t('deployments.loadingAgents')} />
              ) : bulkAgents.length ? (
                <fieldset
                  className="grid max-h-56 gap-1 overflow-y-auto"
                  disabled={bulkMutation.isPending}
                >
                  <legend className="sr-only">{t('deployments.selectAgents')}</legend>
                  {bulkAgents.map((agent) => (
                    <label
                      key={agent.id}
                      className="flex min-h-10 cursor-pointer items-center gap-2 rounded-md border border-border px-3 py-2 text-sm hover:bg-surface-raised"
                    >
                      <input
                        type="checkbox"
                        className="h-4 w-4 shrink-0"
                        checked={bulkSelected.includes(agent.id)}
                        onChange={() => toggleBulkAgent(agent.id)}
                      />
                      <span className="min-w-0 truncate">
                        {agent.display_name} ({agent.name})
                      </span>
                    </label>
                  ))}
                </fieldset>
              ) : (
                <EmptyState title={t('deployments.noBulkAgents')} />
              )}
              <label className="flex min-h-10 cursor-pointer items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  className="h-4 w-4 shrink-0"
                  checked={bulkRollback}
                  disabled={bulkMutation.isPending}
                  onChange={(event) => {
                    setBulkRollback(event.target.checked)
                    bulkMutation.reset()
                  }}
                />
                {t('deployments.rollback')}
              </label>
              <Button
                type="submit"
                className="h-10"
                disabled={
                  bulkMutation.isPending ||
                  agents.isPending ||
                  agents.isError ||
                  bulkSelected.length === 0 ||
                  !bulkTitle.trim()
                }
              >
                <PackagePlus className="h-4 w-4" />
                {bulkMutation.isPending ? t('deployments.creating') : t('deployments.createBulk')}
              </Button>
              {bulkMutation.data ? (
                <p role="status" className="text-sm text-text-muted" data-testid="bulk-result">
                  {t('deployments.bulkResult', {
                    created: bulkMutation.data.created,
                    skipped: bulkMutation.data.skipped,
                  })}
                </p>
              ) : null}
              {bulkMutation.isError ? (
                <ErrorState message={t('deployments.bulkCreateError')} />
              ) : null}
            </form>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>{t('deployments.createJob')}</CardTitle>
          </CardHeader>
          <CardContent>
            <form
              className="grid gap-3"
              aria-busy={createMutation.isPending}
              onSubmit={(event) => {
                event.preventDefault()
                if (title.trim() && !agents.isPending) createMutation.mutate()
              }}
            >
              <LabeledSelect
                id="deployment-kind"
                label={t('deployments.kind')}
                value={jobKind}
                disabled={createMutation.isPending}
                onChange={changeJobKind}
                options={[
                  ['provision', t('deployments.provision')],
                  ['runtime_update', t('deployments.runtimeUpdate')],
                ]}
              />
              <LabeledSelect
                id="deployment-runtime"
                label={t('deployments.runtime')}
                value={runtimeKind}
                disabled={createMutation.isPending}
                onChange={(value) => {
                  setRuntimeKind(value as AgentKind)
                  createMutation.reset()
                }}
                options={[
                  ['hermes', 'Hermes'],
                  ['java_agent', 'Java Agent'],
                ]}
              />
              <LabeledSelect
                id="deployment-agent"
                label={t('deployments.agent')}
                value={agentId}
                disabled={agents.isPending || agents.isError || createMutation.isPending}
                onChange={(value) => {
                  setAgentId(value)
                  createMutation.reset()
                }}
                options={[
                  [
                    '',
                    agents.isPending
                      ? t('deployments.loadingAgents')
                      : t('deployments.fleetLevelJob'),
                  ],
                  ...(agents.data ?? []).map(
                    (agent) =>
                      [agent.id, `${agent.display_name} (${agent.name})`] as [string, string],
                  ),
                ]}
              />
              {agents.isError ? (
                <RetryState
                  message={t('deployments.singleAgentsError')}
                  onRetry={() => void agents.refetch()}
                />
              ) : null}
              <div className="grid gap-2">
                <Label htmlFor="deployment-title">{t('deployments.jobTitle')}</Label>
                <Input
                  id="deployment-title"
                  className="h-10"
                  value={title}
                  required
                  disabled={createMutation.isPending}
                  onChange={(event) => {
                    setTitle(event.target.value)
                    createMutation.reset()
                  }}
                />
              </div>
              <Button
                type="submit"
                className="h-10"
                disabled={createMutation.isPending || agents.isPending || !title.trim()}
              >
                <PackagePlus className="h-4 w-4" />
                {createMutation.isPending ? t('deployments.creating') : t('deployments.create')}
              </Button>
              {createMutation.isError ? (
                <ErrorState message={t('deployments.createError')} />
              ) : null}
            </form>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>
            {t('deployments.jobs')}
            {jobs.data ? (
              <span className="ml-2 text-sm font-normal text-text-muted">{jobs.data.length}</span>
            ) : null}
          </CardTitle>
        </CardHeader>
        <CardContent>
          {jobs.isError ? (
            <RetryState message={t('deployments.jobsError')} onRetry={() => void jobs.refetch()} />
          ) : jobs.isPending ? (
            <EmptyState title={t('deployments.loadingJobs')} />
          ) : jobs.data.length ? (
            <ul className="divide-y divide-border rounded-md border border-border">
              {jobs.data.map((job) => {
                const canCancel = job.state === 'queued' || job.state === 'running'
                return (
                  <li key={job.id} className="p-3">
                    <div className="flex flex-wrap items-center gap-2">
                      <button
                        type="button"
                        onClick={() => onOpen(job.id)}
                        className="flex min-h-10 min-w-0 items-center break-words text-left font-medium text-accent hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
                      >
                        {job.title}
                      </button>
                      <StatusBadge value={job.job_kind} />
                      <StatusBadge value={job.state} />
                      {job.runtime_kind ? <KindBadge kind={job.runtime_kind} /> : null}
                    </div>
                    <p className="mt-1 text-xs text-text-muted">
                      {t('deployments.updated', { date: formatDate(job.updated_at) })}
                    </p>
                    {job.last_error ? (
                      <p className="mt-2 text-sm text-danger">{job.last_error}</p>
                    ) : null}
                    {canCancel ? (
                      <Button
                        className="mt-2 h-10"
                        variant="outline"
                        aria-label={t('deployments.cancelJob', { title: job.title })}
                        onClick={() => {
                          cancelMutation.reset()
                          setCancelTarget(job)
                        }}
                        disabled={cancelMutation.isPending}
                      >
                        <Ban className="h-4 w-4" />
                        {t('deployments.cancel')}
                      </Button>
                    ) : null}
                  </li>
                )
              })}
            </ul>
          ) : (
            <EmptyState title={t('deployments.noJobs')} />
          )}
        </CardContent>
      </Card>

      <AlertDialog
        open={Boolean(cancelTarget)}
        onOpenChange={(open) => {
          if (!open && !cancelMutation.isPending) {
            setCancelTarget(null)
            cancelMutation.reset()
          }
        }}
      >
        <AlertDialogContent aria-busy={cancelMutation.isPending}>
          <AlertDialogHeader>
            <AlertDialogTitle>{t('deployments.cancelConfirmTitle')}</AlertDialogTitle>
            <AlertDialogDescription>
              {t('deployments.cancelConfirmDescription', { title: cancelTarget?.title ?? '' })}
            </AlertDialogDescription>
          </AlertDialogHeader>
          {cancelMutation.isError ? <ErrorState message={t('deployments.cancelError')} /> : null}
          <AlertDialogFooter>
            <AlertDialogCancel disabled={cancelMutation.isPending}>
              {t('deployments.keepJob')}
            </AlertDialogCancel>
            <AlertDialogAction
              disabled={cancelMutation.isPending || !cancelTarget}
              style={{ color: 'var(--color-accent-foreground)' }}
              onClick={(event) => {
                event.preventDefault()
                if (cancelTarget) cancelMutation.mutate(cancelTarget.id)
              }}
            >
              {cancelMutation.isPending
                ? t('deployments.cancelling')
                : cancelMutation.isError
                  ? t('deployments.retryCancel')
                  : t('deployments.confirmCancel')}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  )
}

function DeploymentJobDetail({ jobId }: { jobId: string | null }) {
  const { t } = useTranslation()
  const jobs = useQuery({
    queryKey: ['deployment-jobs'],
    queryFn: () => listDeploymentJobs(100),
    enabled: !jobId,
  })
  const fallbackJobId = useMemo(() => jobs.data?.[0]?.id ?? null, [jobs.data])
  const effectiveJobId = jobId ?? fallbackJobId
  const job = useQuery({
    queryKey: ['deployment-job', effectiveJobId],
    queryFn: () => getDeploymentJob(effectiveJobId!),
    enabled: Boolean(effectiveJobId),
  })

  if (!jobId && jobs.isError) {
    return <RetryState message={t('deployments.jobsError')} onRetry={() => void jobs.refetch()} />
  }
  if (!jobId && jobs.isPending) return <EmptyState title={t('deployments.loadingJobs')} />
  if (!effectiveJobId) return <EmptyState title={t('deployments.noJobSelected')} />
  if (job.isError) {
    return <RetryState message={t('deployments.jobError')} onRetry={() => void job.refetch()} />
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('deployments.jobDetail')}</CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        {job.data ? (
          <>
            <div className="flex flex-wrap gap-2">
              <StatusBadge value={job.data.job_kind} />
              <StatusBadge value={job.data.state} />
              {job.data.runtime_kind ? <KindBadge kind={job.data.runtime_kind} /> : null}
            </div>
            <h2 className="break-words text-base font-semibold text-text-primary">
              {job.data.title}
            </h2>
            <p className="text-xs text-text-muted">
              {t('deployments.updated', { date: formatDate(job.data.updated_at) })}
            </p>
            {job.data.last_error ? <ErrorState message={job.data.last_error} /> : null}
            <div>
              <h3 className="mb-2 text-sm font-medium text-text-primary">
                {t('deployments.technicalDetail')}
              </h3>
              <JsonBlock value={job.data.detail} />
            </div>
          </>
        ) : (
          <EmptyState title={t('deployments.loadingJob')} />
        )}
      </CardContent>
    </Card>
  )
}

function LabeledSelect({
  id,
  label,
  value,
  disabled,
  onChange,
  options,
}: {
  id: string
  label: string
  value: string
  disabled?: boolean
  onChange: (value: string) => void
  options: [string, string][]
}) {
  return (
    <div className="grid gap-2">
      <Label htmlFor={id}>{label}</Label>
      <select
        id={id}
        value={value}
        disabled={disabled}
        onChange={(event) => onChange(event.target.value)}
        className="h-10 rounded-md border border-border bg-background px-3 text-sm disabled:cursor-not-allowed disabled:opacity-60"
      >
        {options.map(([optionValue, optionLabel]) => (
          <option key={optionValue || optionLabel} value={optionValue}>
            {optionLabel}
          </option>
        ))}
      </select>
    </div>
  )
}

function RetryState({ message, onRetry }: { message: string; onRetry: () => void }) {
  const { t } = useTranslation()
  return (
    <div className="space-y-2">
      <ErrorState message={message} />
      <Button type="button" className="h-10" variant="outline" onClick={onRetry}>
        {t('deployments.retry')}
      </Button>
    </div>
  )
}
