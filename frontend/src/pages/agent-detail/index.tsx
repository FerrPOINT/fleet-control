import { FormEvent, useEffect, useId, useState } from 'react'
import { Link, NavLink, useLocation, useParams } from 'react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import {
  FileCode2,
  Folder,
  HardDrive,
  HeartPulse,
  Pencil,
  Play,
  RotateCcw,
  Square,
  Trash2,
  Wrench,
} from 'lucide-react'
import {
  getAgent,
  getAgentConfig,
  getAgentStorage,
  listAgentSkills,
  listLogs,
  listSessions,
  purgeAgentFiles,
  runAgentOperation,
  updateAgentConfig,
  updateAgentSkill,
} from '@/api/fleet'
import type {
  Agent,
  AgentConfig,
  AgentSession,
  AgentSkill,
  AgentStorageReport,
  SkillState,
  UpdateAgentConfigRequest,
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
import { cn } from '@/shared/lib/utils'
import { ConfigRevisions } from './config-revisions'

const tabs = ['overview', 'runtime', 'skills', 'config', 'workspace', 'sessions'] as const

export function AgentDetailPage({ tab }: { tab: (typeof tabs)[number] }) {
  const { t } = useTranslation()
  const { agentId } = useParams()
  const location = useLocation()
  const basePath = location.pathname.startsWith('/executors')
    ? `/executors/${agentId}`
    : `/agents/${agentId}`
  const agent = useQuery({
    queryKey: ['agent', agentId],
    queryFn: () => getAgent(agentId!),
    enabled: Boolean(agentId),
  })

  if (!agentId) return <ErrorState message={t('agentDetail.missingId')} />
  if (agent.isError)
    return <RetryState message={t('agentDetail.loadError')} onRetry={() => void agent.refetch()} />
  if (!agent.data) return <EmptyState title={t('agentDetail.loading')} />

  return (
    <>
      <PageHeader
        title={agent.data.display_name}
        description={t('agentDetail.description', {
          name: agent.data.name,
          kind: agent.data.kind === 'java_agent' ? 'Java Agent' : 'Hermes',
        })}
        actions={
          <Button asChild variant="outline" className="min-h-10 sm:min-h-10">
            <Link to={`${basePath}/edit`}>
              <Pencil className="h-4 w-4" />
              {t('agentDetail.edit')}
            </Link>
          </Button>
        }
      />
      <nav aria-label={t('agentDetail.sections')} className="mb-4 flex flex-wrap gap-1">
        {tabs.map((value) => (
          <NavLink
            key={value}
            to={value === 'overview' ? basePath : `${basePath}/${value}`}
            end={value === 'overview'}
            className={({ isActive }) =>
              cn(
                'inline-flex min-h-10 items-center rounded-md px-3 py-2 text-sm text-text-muted hover:bg-surface-raised hover:text-text-primary focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent',
                isActive && 'bg-surface-raised text-text-primary',
              )
            }
          >
            {t(`agentDetail.tabs.${value}`)}
          </NavLink>
        ))}
      </nav>
      {tab === 'overview' ? <OverviewTab agent={agent.data} /> : null}
      {tab === 'runtime' ? <RuntimeTab agent={agent.data} /> : null}
      {tab === 'skills' ? <SkillsTab agent={agent.data} /> : null}
      {tab === 'config' ? <ConfigTab agent={agent.data} /> : null}
      {tab === 'workspace' ? <WorkspaceTab agent={agent.data} /> : null}
      {tab === 'sessions' ? <SessionsTab agent={agent.data} /> : null}
    </>
  )
}

function OverviewTab({ agent }: { agent: Agent }) {
  const { t, i18n } = useTranslation()
  return (
    <div className="page-split items-start" data-page-layout="detail-with-aside">
      <Card className="min-w-0">
        <CardHeader>
          <CardTitle>{t('agentDetail.identity')}</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <AgentIdentity agent={agent} />
          <p className="text-sm text-text-secondary">
            {agent.description || t('agentDetail.noDescription')}
          </p>
          <dl className="grid gap-3 text-sm sm:grid-cols-2">
            <Field
              label={t('agentDetail.namespace')}
              value={agent.namespace_id ?? t('agentDetail.unbound')}
            />
            <Field
              label={t('agentDetail.workflow')}
              value={agent.workflow_id ?? t('agentDetail.unbound')}
            />
            <Field
              label={t('agentDetail.apiPort')}
              value={agent.api_port ?? t('agentDetail.notSet')}
            />
            <Field
              label={t('agentDetail.dashboardPort')}
              value={agent.dashboard_port ?? t('agentDetail.notSet')}
            />
            <Field
              label={t('agentDetail.updated')}
              value={
                agent.updated_at
                  ? formatDate(agent.updated_at, i18n.language)
                  : t('agentDetail.never')
              }
            />
            <Field
              label={t('agentDetail.version')}
              value={agent.runtime_version ?? t('statuses.unknown')}
            />
          </dl>
        </CardContent>
      </Card>
      <aside className="min-w-0" aria-label={t('agentDetail.snapshot')}>
        <Card>
          <CardHeader>
            <CardTitle>{t('agentDetail.snapshot')}</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            <StatusBadge value={agent.status} />
            <JsonBlock value={agent.runtime} />
          </CardContent>
        </Card>
      </aside>
    </div>
  )
}

function RuntimeTab({ agent }: { agent: Agent }) {
  const { t, i18n } = useTranslation()
  const queryClient = useQueryClient()
  const logs = useQuery({ queryKey: ['logs', agent.id], queryFn: () => listLogs(agent.id, 40) })
  const operation = useMutation({
    mutationFn: (action: 'provision' | 'start' | 'stop' | 'restart' | 'health') =>
      runAgentOperation(agent.id, action),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['agent', agent.id] }),
        queryClient.invalidateQueries({ queryKey: ['logs', agent.id] }),
        queryClient.invalidateQueries({ queryKey: ['agents'] }),
      ])
    },
  })

  return (
    <div
      className="grid min-w-0 gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]"
      data-page-layout="wide"
    >
      <Card>
        <CardHeader>
          <CardTitle>{t('agentDetail.controls')}</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex flex-wrap gap-2">
            <Button
              className="min-h-10 sm:min-h-10"
              onClick={() => operation.mutate('start')}
              disabled={operation.isPending}
            >
              <Play className="h-4 w-4" />
              {t('agentDetail.start')}
            </Button>
            <Button
              className="min-h-10 sm:min-h-10"
              variant="outline"
              onClick={() => operation.mutate('stop')}
              disabled={operation.isPending}
            >
              <Square className="h-4 w-4" />
              {t('agentDetail.stop')}
            </Button>
            <Button
              className="min-h-10 sm:min-h-10"
              variant="outline"
              onClick={() => operation.mutate('restart')}
              disabled={operation.isPending}
            >
              <RotateCcw className="h-4 w-4" />
              {t('agentDetail.restart')}
            </Button>
            <Button
              className="min-h-10 sm:min-h-10"
              variant="outline"
              onClick={() => operation.mutate('health')}
              disabled={operation.isPending}
            >
              <HeartPulse className="h-4 w-4" />
              {t('agentDetail.checkHealth')}
            </Button>
          </div>
          <dl className="grid gap-3 text-sm sm:grid-cols-3">
            <div>
              <dt className="text-xs text-text-muted">{t('agentDetail.status')}</dt>
              <dd className="mt-1">
                <StatusBadge value={agent.status} />
              </dd>
            </div>
            <Field label="PID" value={agent.runtime.pid ?? t('agentDetail.notTracked')} />
            <div>
              <dt className="text-xs text-text-muted">{t('agentDetail.health')}</dt>
              <dd className="mt-1">
                <StatusBadge value={agent.runtime.health_status} />
              </dd>
            </div>
            <Field
              label={t('agentDetail.apiPort')}
              value={agent.api_port ?? t('agentDetail.notSet')}
            />
            <Field
              label={t('agentDetail.dashboardPort')}
              value={agent.dashboard_port ?? t('agentDetail.notSet')}
            />
            <Field
              label={t('agentDetail.lastHealth')}
              value={
                agent.runtime.last_health_at
                  ? formatDate(agent.runtime.last_health_at, i18n.language)
                  : t('agentDetail.never')
              }
            />
          </dl>
          {agent.runtime.health_detail ? (
            <p className="rounded-md border border-border bg-background p-3 text-sm text-text-secondary">
              {agent.runtime.health_detail}
            </p>
          ) : null}
          {operation.isError ? <ErrorState message={t('agentDetail.operationError')} /> : null}
          {operation.isPending ? <p role="status">{t('agentDetail.operationPending')}</p> : null}
          {operation.isSuccess ? <p role="status">{t('agentDetail.operationSuccess')}</p> : null}
          <div>
            <p className="mb-2 text-xs font-medium uppercase text-text-muted">
              {t('agentDetail.command')}
            </p>
            <pre className="whitespace-pre-wrap break-all rounded-md border border-border bg-background p-3 text-xs text-text-secondary">
              {agent.runtime.startup_command_redacted ?? agent.runtime.command_preview}
            </pre>
          </div>
          <JsonBlock value={agent.runtime.env_preview} />
          <div>
            <p className="mb-2 text-xs font-medium uppercase text-text-muted">
              {t('agentDetail.capabilities')}
            </p>
            <JsonBlock value={agent.runtime.last_capabilities_json} />
          </div>
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardTitle>{t('agentDetail.recentLogs')}</CardTitle>
        </CardHeader>
        <CardContent className="space-y-2">
          {logs.isError ? (
            <RetryState message={t('agentDetail.logsError')} onRetry={() => void logs.refetch()} />
          ) : logs.data?.length ? (
            logs.data.map((entry) => (
              <div key={entry.id} className="rounded-md border border-border p-2 text-xs">
                <span className="text-text-muted">
                  {formatDate(entry.created_at, i18n.language)}
                </span>
                <span className="ml-2 font-medium text-text-primary">{entry.stream}</span>
                <p className="mt-1 whitespace-pre-wrap break-all text-text-secondary">
                  {entry.message}
                </p>
              </div>
            ))
          ) : (
            <EmptyState
              title={t(logs.isLoading ? 'agentDetail.logsLoading' : 'agentDetail.logsEmpty')}
            />
          )}
        </CardContent>
      </Card>
    </div>
  )
}

function SkillsTab({ agent }: { agent: Agent }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const [selectedSkillId, setSelectedSkillId] = useState<string | null>(null)
  const [skillDraft, setSkillDraft] = useState('')
  const skills = useQuery({
    queryKey: ['skills', agent.id],
    queryFn: () => listAgentSkills(agent.id),
  })
  const mutation = useMutation({
    mutationFn: ({
      skill,
      state,
      content,
    }: {
      skill: AgentSkill
      state: SkillState
      content: string | null
    }) => updateAgentSkill(agent.id, skill.name, { state, content }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['skills', agent.id] }),
  })
  const selectedSkill =
    skills.data?.find((skill) => skill.id === selectedSkillId) ?? skills.data?.[0] ?? null

  useEffect(() => {
    if (!skills.data?.length) return
    const firstSkill = skills.data[0]
    if (!firstSkill) return
    const next = selectedSkill ?? firstSkill
    setSelectedSkillId(next.id)
    setSkillDraft(next.content ?? '')
  }, [selectedSkill, skills.data])

  function toggleSkill(skill: AgentSkill) {
    mutation.mutate({
      skill,
      state: skill.state === 'enabled' ? 'disabled' : 'enabled',
      content: skill.content,
    })
  }

  function saveSelectedSkill() {
    if (!selectedSkill) return
    mutation.mutate({
      skill: selectedSkill,
      state: selectedSkill.state,
      content: skillDraft,
    })
  }

  return (
    <div
      className="grid min-w-0 gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]"
      data-page-layout="wide"
    >
      <Card>
        <CardHeader>
          <CardTitle>{t('agentDetail.tabs.skills')}</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          {skills.isError ? (
            <RetryState
              message={t('agentDetail.skillsError')}
              onRetry={() => void skills.refetch()}
            />
          ) : skills.data?.length ? (
            skills.data.map((skill) => (
              <div
                key={skill.id}
                className={cn(
                  'grid gap-3 rounded-md border border-border p-3 md:grid-cols-[1fr_auto]',
                  selectedSkill?.id === skill.id && 'border-accent/70 bg-accent/10',
                )}
              >
                <button
                  type="button"
                  className="min-h-10 min-w-0 text-left"
                  disabled={mutation.isPending}
                  onClick={() => {
                    mutation.reset()
                    setSelectedSkillId(skill.id)
                    setSkillDraft(skill.content ?? '')
                  }}
                >
                  <div className="flex flex-wrap items-center gap-2">
                    <Wrench className="h-4 w-4 text-text-muted" />
                    <p className="font-medium text-text-primary">{skill.title}</p>
                    <StatusBadge value={skill.state} />
                  </div>
                  <p className="mt-1 break-all text-xs text-text-muted">
                    {t('agentDetail.skillSource', { name: skill.name, source: skill.source })}
                  </p>
                </button>
                <div className="flex items-start justify-end">
                  <Button
                    variant="outline"
                    size="sm"
                    className="min-h-10 sm:min-h-10"
                    disabled={mutation.isPending}
                    onClick={() => toggleSkill(skill)}
                  >
                    {t(skill.state === 'enabled' ? 'agentDetail.disable' : 'agentDetail.enable')}
                  </Button>
                </div>
              </div>
            ))
          ) : (
            <EmptyState
              title={t(skills.isLoading ? 'agentDetail.skillsLoading' : 'agentDetail.skillsEmpty')}
            />
          )}
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardTitle>{t('agentDetail.skillEditor')}</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          {!skills.isError && selectedSkill ? (
            <>
              <div className="flex flex-wrap items-center gap-2">
                <p className="font-medium text-text-primary">{selectedSkill.title}</p>
                <StatusBadge value={selectedSkill.state} />
              </div>
              <p className="break-all text-xs text-text-muted">{selectedSkill.source}</p>
              <Textarea
                aria-label={t('agentDetail.editSkill', { title: selectedSkill.title })}
                className="min-h-72 font-mono text-xs"
                style={{ minHeight: 288 }}
                value={skillDraft}
                disabled={mutation.isPending}
                onChange={(event) => {
                  mutation.reset()
                  setSkillDraft(event.target.value)
                }}
              />
              {selectedSkill.state === 'dirty' ? (
                <p className="text-xs text-warning">{t('agentDetail.skillOverride')}</p>
              ) : null}
              {mutation.isError ? <ErrorState message={t('agentDetail.skillSaveError')} /> : null}
              {mutation.isSuccess ? <p role="status">{t('agentDetail.skillSaved')}</p> : null}
              <Button
                className="min-h-10 sm:min-h-10"
                onClick={saveSelectedSkill}
                disabled={mutation.isPending}
              >
                <FileCode2 className="h-4 w-4" />
                {t(mutation.isPending ? 'agentDetail.saving' : 'agentDetail.saveSkill')}
              </Button>
            </>
          ) : (
            <EmptyState title={t('agentDetail.selectSkill')} />
          )}
        </CardContent>
      </Card>
    </div>
  )
}

function ConfigTab({ agent }: { agent: Agent }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const config = useQuery({
    queryKey: ['config', agent.id],
    queryFn: () => getAgentConfig(agent.id),
  })
  const [draft, setDraft] = useState<AgentConfig | null>(null)
  const [configValid, setConfigValid] = useState(true)
  const [envValid, setEnvValid] = useState(true)

  useEffect(() => {
    if (config.data) setDraft(config.data)
  }, [config.data])

  const mutation = useMutation({
    mutationFn: (payload: UpdateAgentConfigRequest) => updateAgentConfig(agent.id, payload),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ['config', agent.id] })
      await queryClient.invalidateQueries({ queryKey: ['agent-config-revisions', agent.id] })
      await queryClient.invalidateQueries({ queryKey: ['agent-readiness', agent.id] })
    },
  })

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!draft || !configValid || !envValid || mutation.isPending || config.isError) return
    mutation.mutate({
      config_json: draft.config_json,
      soul_md: draft.soul_md,
      env_json: draft.env_json,
    })
  }

  if (config.isError && !draft)
    return (
      <RetryState message={t('agentDetail.configError')} onRetry={() => void config.refetch()} />
    )
  if (!draft)
    return (
      <EmptyState
        title={t(config.isLoading ? 'agentDetail.configLoading' : 'agentDetail.configEmpty')}
      />
    )

  return (
    <>
      <form onSubmit={submit} data-page-layout="wide">
        {config.isError ? (
          <RetryState
            message={t('agentDetail.configError')}
            onRetry={() => void config.refetch()}
          />
        ) : null}
        <fieldset
          disabled={mutation.isPending}
          className="grid min-w-0 gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]"
        >
          <Card>
            <CardHeader>
              <CardTitle>SOUL.md</CardTitle>
            </CardHeader>
            <CardContent>
              <Textarea
                aria-label="SOUL.md"
                className="min-h-72 font-mono text-xs"
                value={draft.soul_md}
                onChange={(event) => {
                  mutation.reset()
                  setDraft({ ...draft, soul_md: event.target.value })
                }}
              />
            </CardContent>
          </Card>
          <Card>
            <CardHeader>
              <CardTitle>{t('agentDetail.configAndEnv')}</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <JsonEditor
                label="config.json"
                value={draft.config_json}
                onValidityChange={(valid) => {
                  mutation.reset()
                  setConfigValid(valid)
                }}
                onChange={(config_json) => setDraft({ ...draft, config_json })}
              />
              <JsonEditor
                label="env.json"
                value={draft.env_json}
                onValidityChange={(valid) => {
                  mutation.reset()
                  setEnvValid(valid)
                }}
                onChange={(env_json) => setDraft({ ...draft, env_json })}
              />
              {mutation.isError ? <ErrorState message={t('agentDetail.configSaveError')} /> : null}
              {mutation.isSuccess ? <p role="status">{t('agentDetail.configSaved')}</p> : null}
              <Button
                className="min-h-10 sm:min-h-10"
                type="submit"
                disabled={mutation.isPending || !configValid || !envValid || config.isError}
              >
                <FileCode2 className="h-4 w-4" />
                {t(mutation.isPending ? 'agentDetail.saving' : 'agentDetail.saveConfig')}
              </Button>
            </CardContent>
          </Card>
        </fieldset>
      </form>
      <ConfigRevisions agentId={agent.id} />
    </>
  )
}

function WorkspaceTab({ agent }: { agent: Agent }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const [confirmation, setConfirmation] = useState('')
  const storage = useQuery({
    queryKey: ['agent-storage', agent.id],
    queryFn: () => getAgentStorage(agent.id),
  })
  const purge = useMutation({
    mutationFn: () => purgeAgentFiles(agent.id, { confirmation }),
    onSuccess: async () => {
      setConfirmation('')
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['agent', agent.id] }),
        queryClient.invalidateQueries({ queryKey: ['agent-storage', agent.id] }),
        queryClient.invalidateQueries({ queryKey: ['agents'] }),
        queryClient.invalidateQueries({ queryKey: ['logs'] }),
        queryClient.invalidateQueries({ queryKey: ['events'] }),
      ])
    },
  })
  const canPurge = agent.status === 'archived' && confirmation === agent.name

  return (
    <div className="page-split items-start" data-page-layout="detail-with-aside">
      <Card className="min-w-0">
        <CardHeader>
          <CardTitle>{t('agentDetail.workspaceGuard')}</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          {Object.entries(agent.paths).map(([key, value]) => (
            <div key={key} className="rounded-md border border-border p-3">
              <div className="flex items-center gap-2 text-sm font-medium text-text-primary">
                <Folder className="h-4 w-4 text-text-muted" />
                {key}
              </div>
              <p className="mt-1 break-all text-xs text-text-muted">{value}</p>
            </div>
          ))}
        </CardContent>
      </Card>
      <aside className="min-w-0 space-y-4" aria-label={t('agentDetail.storageReport')}>
        <StorageReportCard
          report={storage.data}
          isLoading={storage.isLoading}
          error={storage.isError ? storage.error.message : null}
          onRetry={() => void storage.refetch()}
        />
        <Card>
          <CardHeader>
            <CardTitle>{t('agentDetail.filePurge')}</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            <p className="text-sm text-text-secondary">{t('agentDetail.purgeDescription')}</p>
            <div className="rounded-md border border-border bg-background p-3 text-xs text-text-muted">
              {t('agentDetail.purgeTarget')}:{' '}
              <span className="font-medium text-text-primary">{agent.name}</span>
            </div>
            {!storage.isError && storage.data ? (
              <div className="grid gap-2 rounded-md border border-border bg-background p-3 text-xs text-text-secondary">
                <span>
                  {t('agentDetail.totalSize')}: {formatBytes(storage.data.total_bytes)}
                </span>
                <span>
                  {t('agentDetail.marker')}:{' '}
                  {t(
                    storage.data.marker_verified
                      ? 'agentDetail.verified'
                      : 'agentDetail.notVerified',
                  )}
                </span>
                <span>
                  {t(
                    !storage.data.root_exists
                      ? 'agentDetail.rootAbsent'
                      : !storage.data.marker_verified
                        ? 'agentDetail.markerRequired'
                        : storage.data.retention.archived
                          ? 'agentDetail.purgeAllowed'
                          : 'agentDetail.archiveFirst',
                  )}
                </span>
              </div>
            ) : null}
            <div className="grid gap-2">
              <Label htmlFor="purge-confirmation">{t('agentDetail.confirmName')}</Label>
              <Input
                id="purge-confirmation"
                value={confirmation}
                onChange={(event) => setConfirmation(event.target.value)}
                placeholder={agent.name}
                disabled={purge.isPending}
              />
            </div>
            {purge.isError ? <ErrorState message={t('agentDetail.purgeError')} /> : null}
            {purge.data ? (
              <p
                role="status"
                className="break-all rounded-md border border-border bg-background p-3 text-sm text-text-secondary"
              >
                {t('agentDetail.purgeSuccess')}: {purge.data.purged_path}
              </p>
            ) : null}
            <Button
              className="min-h-10 sm:min-h-10"
              variant="destructive"
              onClick={() => purge.mutate()}
              disabled={
                !canPurge || purge.isPending || storage.isError || !storage.data?.marker_verified
              }
            >
              <Trash2 className="h-4 w-4" />
              {t(purge.isPending ? 'agentDetail.purging' : 'agentDetail.purgeFiles')}
            </Button>
          </CardContent>
        </Card>
      </aside>
    </div>
  )
}

function StorageReportCard({
  report,
  isLoading,
  error,
  onRetry,
}: {
  report?: AgentStorageReport
  isLoading: boolean
  error: string | null
  onRetry: () => void
}) {
  const { t } = useTranslation()
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <HardDrive className="h-4 w-4" />
          {t('agentDetail.storageReport')}
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        {error ? <RetryState message={t('agentDetail.storageError')} onRetry={onRetry} /> : null}
        {isLoading ? (
          <p className="text-sm text-text-muted">{t('agentDetail.storageLoading')}</p>
        ) : null}
        {!error && report ? (
          <>
            <div className="grid grid-cols-2 gap-2 text-sm">
              <Metric label={t('agentDetail.totalSize')} value={formatBytes(report.total_bytes)} />
              <Metric label={t('agentDetail.files')} value={String(report.total_files)} />
              <Metric
                label={t('agentDetail.directories')}
                value={String(report.total_directories)}
              />
              <Metric label={t('agentDetail.symlinks')} value={String(report.total_symlinks)} />
            </div>
            <div className="rounded-md border border-border bg-background p-3 text-xs text-text-muted">
              <p className="break-all">{report.root_path}</p>
              <div className="mt-2 flex flex-wrap gap-2">
                <StatusBadge value={report.root_exists ? 'root present' : 'root missing'} />
                <StatusBadge value={report.marker_present ? 'marker present' : 'marker missing'} />
                <StatusBadge
                  value={report.marker_verified ? 'marker verified' : 'marker invalid'}
                />
                <StatusBadge value={report.retention.purge_eligible ? 'purge eligible' : 'kept'} />
              </div>
            </div>
            <div className="grid gap-2">
              {report.areas.map((area) => (
                <div
                  key={area.name}
                  className="grid gap-2 rounded-md border border-border p-3 text-xs sm:grid-cols-[1fr_auto]"
                >
                  <div className="min-w-0">
                    <p className="font-medium text-text-primary">{area.name}</p>
                    <p className="break-all text-text-muted">{area.path}</p>
                  </div>
                  <div className="text-left text-text-secondary sm:text-right">
                    <p>{area.exists ? formatBytes(area.bytes) : t('statuses.missing')}</p>
                    <p>
                      {t('agentDetail.areaCounts', {
                        files: area.files,
                        directories: area.directories,
                      })}
                    </p>
                  </div>
                </div>
              ))}
            </div>
          </>
        ) : null}
      </CardContent>
    </Card>
  )
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border border-border bg-background p-3">
      <p className="text-xs text-text-muted">{label}</p>
      <p className="mt-1 font-medium text-text-primary">{value}</p>
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

function SessionsTab({ agent }: { agent: Agent }) {
  const { t } = useTranslation()
  const userFilter = useSessionUserFilter()
  const sessions = useQuery({
    queryKey: ['sessions', agent.id, userFilter.selectedUserIds],
    queryFn: () => listSessions(agent.id, userFilter.selectedUserIds),
  })
  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('agentDetail.sessions')}</CardTitle>
      </CardHeader>
      <CardContent className="space-y-2">
        <SessionUserFilter filter={userFilter} className="mb-3" />
        {sessions.isError ? (
          <RetryState
            message={t('agentDetail.sessionsError')}
            onRetry={() => void sessions.refetch()}
          />
        ) : sessions.data?.length ? (
          sessions.data.map((session) => <AgentSessionLink key={session.id} session={session} />)
        ) : (
          <EmptyState
            title={t(
              sessions.isLoading
                ? 'agentDetail.sessionsLoading'
                : userFilter.selectedUserIds.length
                  ? 'agentDetail.sessionsFilteredEmpty'
                  : 'agentDetail.sessionsEmpty',
            )}
          />
        )}
      </CardContent>
    </Card>
  )
}

function AgentSessionLink({ session }: { session: AgentSession }) {
  const { t } = useTranslation()
  return (
    <Link
      to={`/sessions/${session.id}`}
      className="block rounded-md border border-border p-3 hover:bg-surface-raised"
    >
      <div className="flex min-w-0 items-start gap-3">
        <UserAvatar name={session.user_display_name} userId={session.user_id} size="md" />
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <p className="font-medium text-text-primary">{session.title}</p>
            <StatusBadge value={session.state} />
            <StatusBadge value={session.visibility} />
          </div>
          <p className="mt-1 text-xs text-text-muted">
            {session.user_display_name} · {t('agentDetail.leader')}{' '}
            {session.leader_agent_name ?? t('statuses.private')} ·{' '}
            {session.last_message_preview ?? t('agentDetail.noMessages')}
          </p>
        </div>
      </div>
    </Link>
  )
}

function Field({ label, value }: { label: string; value: unknown }) {
  return (
    <div>
      <dt className="text-xs text-text-muted">{label}</dt>
      <dd className="break-words font-medium text-text-primary">{String(value)}</dd>
    </div>
  )
}

function JsonEditor({
  label,
  value,
  onChange,
  onValidityChange,
}: {
  label: string
  value: Record<string, unknown>
  onChange: (value: Record<string, unknown>) => void
  onValidityChange: (valid: boolean) => void
}) {
  const { t } = useTranslation()
  const inputId = useId()
  const [text, setText] = useState(JSON.stringify(value, null, 2))
  const [error, setError] = useState<string | null>(null)

  useEffect(() => setText(JSON.stringify(value, null, 2)), [value])

  function handleChange(next: string) {
    setText(next)
    try {
      const parsed = JSON.parse(next) as Record<string, unknown>
      if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed))
        throw new Error('object required')
      setError(null)
      onValidityChange(true)
      onChange(parsed)
    } catch {
      setError(t('agentDetail.invalidJson'))
      onValidityChange(false)
    }
  }

  return (
    <div className="grid gap-2">
      <label htmlFor={inputId} className="text-xs font-medium uppercase text-text-muted">
        {label}
      </label>
      <Textarea
        id={inputId}
        className="min-h-44 font-mono text-xs"
        style={{ minHeight: 176 }}
        value={text}
        aria-invalid={Boolean(error)}
        aria-describedby={error ? `${inputId}-error` : undefined}
        onChange={(event) => handleChange(event.target.value)}
      />
      {error ? (
        <p id={`${inputId}-error`} role="alert" className="text-xs text-danger">
          {error}
        </p>
      ) : null}
    </div>
  )
}

function RetryState({ message, onRetry }: { message: string; onRetry: () => void }) {
  const { t } = useTranslation()
  return (
    <div className="space-y-2">
      <ErrorState message={message} />
      <Button className="min-h-10 sm:min-h-10" variant="outline" onClick={onRetry}>
        {t('agentDetail.retry')}
      </Button>
    </div>
  )
}
