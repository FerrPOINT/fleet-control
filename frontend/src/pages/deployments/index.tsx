import { useMemo, useState } from 'react'
import { useSearchParams } from 'react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Ban, PackagePlus } from 'lucide-react'
import {
  cancelDeploymentJob,
  createDeploymentJob,
  getDeploymentJob,
  listAgents,
  listDeploymentJobs,
  listRuntimeTemplates,
} from '@/api/fleet'
import type { AgentKind, DeploymentJobKind } from '@/api/types'
import { Button } from '@/shared/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/shared/ui/card'
import { Input } from '@/shared/ui/input'
import { Label } from '@/shared/ui/label'
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
  const [params, setParams] = useSearchParams()
  const selectedTab = tabs.includes(params.get('tab') as DeploymentTab)
    ? (params.get('tab') as DeploymentTab)
    : 'overview'
  const selectedJobId = params.get('job_id')

  return (
    <>
      <PageHeader
        title="Deployments"
        description="Runtime templates, installed versions and provision/update jobs."
      />
      <div className="mb-4 flex flex-wrap gap-2">
        {tabs.map((tab) => (
          <Button
            key={tab}
            type="button"
            variant={selectedTab === tab ? 'default' : 'outline'}
            onClick={() => setParams({ tab })}
          >
            {tab}
          </Button>
        ))}
      </div>
      {selectedTab === 'overview' ? <DeploymentOverview /> : null}
      {selectedTab === 'jobs' ? (
        <DeploymentJobs onOpen={(jobId) => setParams({ tab: 'detail', job_id: jobId })} />
      ) : null}
      {selectedTab === 'detail' ? <DeploymentJobDetail jobId={selectedJobId} /> : null}
    </>
  )
}

function DeploymentOverview() {
  const templates = useQuery({ queryKey: ['runtime-templates'], queryFn: listRuntimeTemplates })
  const agents = useQuery({ queryKey: ['agents'], queryFn: listAgents })

  if (templates.isError) return <ErrorState message={templates.error.message} />

  return (
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
              title={templates.isLoading ? 'Loading runtime templates...' : 'No runtime templates'}
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
  )
}

function DeploymentJobs({ onOpen }: { onOpen: (jobId: string) => void }) {
  const queryClient = useQueryClient()
  const jobs = useQuery({ queryKey: ['deployment-jobs'], queryFn: () => listDeploymentJobs(100) })
  const agents = useQuery({ queryKey: ['agents'], queryFn: listAgents })
  const [title, setTitle] = useState('Provision Hermes runtime')
  const [jobKind, setJobKind] = useState<DeploymentJobKind>('provision')
  const [runtimeKind, setRuntimeKind] = useState<AgentKind>('hermes')
  const [agentId, setAgentId] = useState('')
  const createMutation = useMutation({
    mutationFn: () =>
      createDeploymentJob({
        title,
        job_kind: jobKind,
        runtime_kind: runtimeKind,
        agent_id: agentId || null,
        detail: { requested_from: 'deployments_page' },
      }),
    onSuccess: async () => queryClient.invalidateQueries({ queryKey: ['deployment-jobs'] }),
  })
  const cancelMutation = useMutation({
    mutationFn: cancelDeploymentJob,
    onSuccess: async () => queryClient.invalidateQueries({ queryKey: ['deployment-jobs'] }),
  })

  return (
    <div className="grid gap-4 xl:grid-cols-[380px_1fr]">
      <Card>
        <CardHeader>
          <CardTitle>Create job</CardTitle>
        </CardHeader>
        <CardContent>
          <form
            className="grid gap-3"
            onSubmit={(event) => {
              event.preventDefault()
              createMutation.mutate()
            }}
          >
            <LabeledSelect
              label="Kind"
              value={jobKind}
              onChange={(value) => setJobKind(value as DeploymentJobKind)}
              options={[
                ['provision', 'provision'],
                ['runtime_update', 'runtime update'],
              ]}
            />
            <LabeledSelect
              label="Runtime"
              value={runtimeKind}
              onChange={(value) => setRuntimeKind(value as AgentKind)}
              options={[
                ['hermes', 'Hermes'],
                ['java_agent', 'Java Agent'],
              ]}
            />
            <LabeledSelect
              label="Agent"
              value={agentId}
              onChange={setAgentId}
              options={[
                ['', 'Fleet-level job'],
                ...(agents.data ?? []).map(
                  (agent) =>
                    [agent.id, `${agent.name} - ${agent.display_name}`] as [string, string],
                ),
              ]}
            />
            <div className="grid gap-2">
              <Label htmlFor="deployment-title">Title</Label>
              <Input
                id="deployment-title"
                value={title}
                onChange={(event) => setTitle(event.target.value)}
              />
            </div>
            <Button type="submit" disabled={createMutation.isPending || !title.trim()}>
              <PackagePlus className="h-4 w-4" />
              Create job
            </Button>
            {createMutation.isError ? <ErrorState message={createMutation.error.message} /> : null}
          </form>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Provision and update jobs</CardTitle>
        </CardHeader>
        <CardContent className="space-y-2">
          {jobs.data?.length ? (
            jobs.data.map((job) => (
              <div key={job.id} className="rounded-md border border-border p-3">
                <div className="flex flex-wrap items-center gap-2">
                  <button
                    type="button"
                    onClick={() => onOpen(job.id)}
                    className="font-medium text-accent hover:underline"
                  >
                    {job.title}
                  </button>
                  <StatusBadge value={job.job_kind} />
                  <StatusBadge value={job.state} />
                  {job.runtime_kind ? <KindBadge kind={job.runtime_kind} /> : null}
                </div>
                <p className="mt-1 text-xs text-text-muted">{formatDate(job.updated_at)}</p>
                {job.state === 'queued' || job.state === 'running' ? (
                  <Button
                    className="mt-3"
                    size="sm"
                    variant="outline"
                    onClick={() => cancelMutation.mutate(job.id)}
                    disabled={cancelMutation.isPending}
                  >
                    <Ban className="h-4 w-4" />
                    Cancel
                  </Button>
                ) : null}
              </div>
            ))
          ) : (
            <EmptyState title={jobs.isLoading ? 'Loading jobs...' : 'No deployment jobs'} />
          )}
        </CardContent>
      </Card>
    </div>
  )
}

function DeploymentJobDetail({ jobId }: { jobId: string | null }) {
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

  if (!effectiveJobId) return <EmptyState title="No job selected" />
  if (job.isError) return <ErrorState message={job.error.message} />

  return (
    <Card>
      <CardHeader>
        <CardTitle>Job detail</CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        {job.data ? (
          <>
            <div className="flex flex-wrap gap-2">
              <StatusBadge value={job.data.job_kind} />
              <StatusBadge value={job.data.state} />
              {job.data.runtime_kind ? <KindBadge kind={job.data.runtime_kind} /> : null}
            </div>
            <p className="text-sm font-medium text-text-primary">{job.data.title}</p>
            <p className="text-xs text-text-muted">Updated {formatDate(job.data.updated_at)}</p>
            <JsonBlock value={job.data.detail} />
          </>
        ) : (
          <EmptyState title="Loading job..." />
        )}
      </CardContent>
    </Card>
  )
}

function LabeledSelect({
  label,
  value,
  onChange,
  options,
}: {
  label: string
  value: string
  onChange: (value: string) => void
  options: [string, string][]
}) {
  const id = label.toLowerCase().replaceAll(' ', '-')
  return (
    <div className="grid gap-2">
      <Label htmlFor={id}>{label}</Label>
      <select
        id={id}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="h-9 rounded-md border border-border bg-background px-3 text-sm"
      >
        {options.map(([optionValue, label]) => (
          <option key={optionValue || label} value={optionValue}>
            {label}
          </option>
        ))}
      </select>
    </div>
  )
}
