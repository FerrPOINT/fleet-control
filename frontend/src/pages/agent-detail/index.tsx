import { FormEvent, useEffect, useState } from 'react'
import { Link, NavLink, useLocation, useParams } from 'react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  FileCode2,
  Folder,
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
  SkillState,
  UpdateAgentConfigRequest,
} from '@/api/types'
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
  formatDate,
} from '../common'
import { cn } from '@/shared/lib/utils'

const tabs = [
  ['overview', 'Overview'],
  ['runtime', 'Runtime'],
  ['skills', 'Skills'],
  ['config', 'Config'],
  ['workspace', 'Workspace'],
  ['sessions', 'Sessions'],
] as const

export function AgentDetailPage({ tab }: { tab: (typeof tabs)[number][0] }) {
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

  if (!agentId) return <ErrorState message="Agent id is missing" />
  if (agent.isError) return <ErrorState message={agent.error.message} />
  if (!agent.data) return <EmptyState title="Loading agent..." />

  return (
    <>
      <PageHeader
        title={agent.data.display_name}
        description={`${agent.data.name} controls an isolated ${agent.data.kind} runtime.`}
        actions={
          <Button asChild variant="outline">
            <Link to={`${basePath}/edit`}>
              <Pencil className="h-4 w-4" />
              Edit agent
            </Link>
          </Button>
        }
      />
      <div className="mb-4 flex gap-1 overflow-x-auto">
        {tabs.map(([value, label]) => (
          <NavLink
            key={value}
            to={value === 'overview' ? basePath : `${basePath}/${value}`}
            end={value === 'overview'}
            className={({ isActive }) =>
              cn(
                'h-9 shrink-0 rounded-md px-3 py-2 text-sm text-text-muted hover:bg-surface-raised hover:text-text-primary',
                isActive && 'bg-surface-raised text-text-primary',
              )
            }
          >
            {label}
          </NavLink>
        ))}
      </div>
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
  return (
    <div className="grid gap-4 xl:grid-cols-[1fr_1fr]">
      <Card>
        <CardHeader>
          <CardTitle>Identity</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <AgentIdentity agent={agent} />
          <p className="text-sm text-text-secondary">{agent.description ?? 'No description'}</p>
          <dl className="grid gap-3 text-sm sm:grid-cols-2">
            <Field label="Namespace" value={agent.namespace_id ?? 'unbound'} />
            <Field label="Workflow" value={agent.workflow_id ?? 'unbound'} />
            <Field label="API port" value={agent.api_port ?? 'n/a'} />
            <Field label="Dashboard port" value={agent.dashboard_port ?? 'n/a'} />
            <Field label="Updated" value={formatDate(agent.updated_at)} />
            <Field label="Version" value={agent.runtime_version ?? 'unknown'} />
          </dl>
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardTitle>Runtime snapshot</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <StatusBadge value={agent.status} />
          <JsonBlock value={agent.runtime} />
        </CardContent>
      </Card>
    </div>
  )
}

function RuntimeTab({ agent }: { agent: Agent }) {
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
    <div className="grid gap-4 xl:grid-cols-[1fr_1fr]">
      <Card>
        <CardHeader>
          <CardTitle>Controls</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex flex-wrap gap-2">
            <Button onClick={() => operation.mutate('start')} disabled={operation.isPending}>
              <Play className="h-4 w-4" />
              Start
            </Button>
            <Button
              variant="outline"
              onClick={() => operation.mutate('stop')}
              disabled={operation.isPending}
            >
              <Square className="h-4 w-4" />
              Stop
            </Button>
            <Button
              variant="outline"
              onClick={() => operation.mutate('restart')}
              disabled={operation.isPending}
            >
              <RotateCcw className="h-4 w-4" />
              Restart
            </Button>
            <Button
              variant="outline"
              onClick={() => operation.mutate('health')}
              disabled={operation.isPending}
            >
              <HeartPulse className="h-4 w-4" />
              Health
            </Button>
          </div>
          <dl className="grid gap-3 text-sm sm:grid-cols-3">
            <div>
              <dt className="text-xs text-text-muted">Status</dt>
              <dd className="mt-1">
                <StatusBadge value={agent.status} />
              </dd>
            </div>
            <Field label="PID" value={agent.runtime.pid ?? 'not tracked'} />
            <Field label="Health" value={agent.runtime.health_status ?? 'unknown'} />
            <Field label="API port" value={agent.api_port ?? 'n/a'} />
            <Field label="Dashboard port" value={agent.dashboard_port ?? 'n/a'} />
            <Field label="Last health" value={formatDate(agent.runtime.last_health_at)} />
          </dl>
          {agent.runtime.health_detail ? (
            <p className="rounded-md border border-border bg-background p-3 text-sm text-text-secondary">
              {agent.runtime.health_detail}
            </p>
          ) : null}
          {operation.isError ? <ErrorState message={operation.error.message} /> : null}
          <div>
            <p className="mb-2 text-xs font-medium uppercase text-text-muted">Command preview</p>
            <pre className="overflow-auto rounded-md border border-border bg-background p-3 text-xs text-text-secondary">
              {agent.runtime.startup_command_redacted ?? agent.runtime.command_preview}
            </pre>
          </div>
          <JsonBlock value={agent.runtime.env_preview} />
          <div>
            <p className="mb-2 text-xs font-medium uppercase text-text-muted">Capabilities</p>
            <JsonBlock value={agent.runtime.last_capabilities_json} />
          </div>
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardTitle>Recent logs</CardTitle>
        </CardHeader>
        <CardContent className="space-y-2">
          {logs.data?.length ? (
            logs.data.map((entry) => (
              <div key={entry.id} className="rounded-md border border-border p-2 text-xs">
                <span className="text-text-muted">{formatDate(entry.created_at)}</span>
                <span className="ml-2 font-medium text-text-primary">{entry.stream}</span>
                <p className="mt-1 text-text-secondary">{entry.message}</p>
              </div>
            ))
          ) : (
            <EmptyState title={logs.isLoading ? 'Loading logs...' : 'No logs for this agent'} />
          )}
        </CardContent>
      </Card>
    </div>
  )
}

function SkillsTab({ agent }: { agent: Agent }) {
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
    <div className="grid gap-4 xl:grid-cols-[1fr_1fr]">
      <Card>
        <CardHeader>
          <CardTitle>Skills</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          {skills.data?.length ? (
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
                  className="min-w-0 text-left"
                  onClick={() => {
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
                    {skill.name} from {skill.source}
                  </p>
                </button>
                <div className="flex items-start justify-end">
                  <Button
                    variant="outline"
                    size="sm"
                    disabled={mutation.isPending}
                    onClick={() => toggleSkill(skill)}
                  >
                    {skill.state === 'enabled' ? 'Disable' : 'Enable'}
                  </Button>
                </div>
              </div>
            ))
          ) : (
            <EmptyState title={skills.isLoading ? 'Loading skills...' : 'No skills selected'} />
          )}
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardTitle>Per-agent editor</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          {selectedSkill ? (
            <>
              <div className="flex flex-wrap items-center gap-2">
                <p className="font-medium text-text-primary">{selectedSkill.title}</p>
                <StatusBadge value={selectedSkill.state} />
              </div>
              <p className="break-all text-xs text-text-muted">{selectedSkill.source}</p>
              <Textarea
                className="min-h-72 font-mono text-xs"
                value={skillDraft}
                onChange={(event) => setSkillDraft(event.target.value)}
              />
              {selectedSkill.state === 'dirty' ? (
                <p className="text-xs text-warning">This skill has a local per-agent override.</p>
              ) : null}
              {mutation.isError ? <ErrorState message={mutation.error.message} /> : null}
              <Button onClick={saveSelectedSkill} disabled={mutation.isPending}>
                <FileCode2 className="h-4 w-4" />
                Save skill
              </Button>
            </>
          ) : (
            <EmptyState title="Select a skill to edit" />
          )}
        </CardContent>
      </Card>
    </div>
  )
}

function ConfigTab({ agent }: { agent: Agent }) {
  const queryClient = useQueryClient()
  const config = useQuery({
    queryKey: ['config', agent.id],
    queryFn: () => getAgentConfig(agent.id),
  })
  const [draft, setDraft] = useState<AgentConfig | null>(null)

  useEffect(() => {
    if (config.data) setDraft(config.data)
  }, [config.data])

  const mutation = useMutation({
    mutationFn: (payload: UpdateAgentConfigRequest) => updateAgentConfig(agent.id, payload),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['config', agent.id] }),
  })

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!draft) return
    mutation.mutate({
      config_json: draft.config_json,
      soul_md: draft.soul_md,
      env_json: draft.env_json,
    })
  }

  if (!draft)
    return <EmptyState title={config.isLoading ? 'Loading config...' : 'Config not found'} />

  return (
    <form className="grid gap-4 xl:grid-cols-2" onSubmit={submit}>
      <Card>
        <CardHeader>
          <CardTitle>SOUL.md</CardTitle>
        </CardHeader>
        <CardContent>
          <Textarea
            className="min-h-72 font-mono text-xs"
            value={draft.soul_md}
            onChange={(event) => setDraft({ ...draft, soul_md: event.target.value })}
          />
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardTitle>Config and env</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <JsonEditor
            label="config.json"
            value={draft.config_json}
            onChange={(config_json) => setDraft({ ...draft, config_json })}
          />
          <JsonEditor
            label="env.json"
            value={draft.env_json}
            onChange={(env_json) => setDraft({ ...draft, env_json })}
          />
          {mutation.isError ? <ErrorState message={mutation.error.message} /> : null}
          <Button type="submit" disabled={mutation.isPending}>
            <FileCode2 className="h-4 w-4" />
            Save config
          </Button>
        </CardContent>
      </Card>
    </form>
  )
}

function WorkspaceTab({ agent }: { agent: Agent }) {
  const queryClient = useQueryClient()
  const [confirmation, setConfirmation] = useState('')
  const purge = useMutation({
    mutationFn: () => purgeAgentFiles(agent.id, { confirmation }),
    onSuccess: async () => {
      setConfirmation('')
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['agent', agent.id] }),
        queryClient.invalidateQueries({ queryKey: ['agents'] }),
        queryClient.invalidateQueries({ queryKey: ['logs'] }),
        queryClient.invalidateQueries({ queryKey: ['events'] }),
      ])
    },
  })
  const canPurge = agent.status === 'archived' && confirmation === agent.name

  return (
    <div className="grid gap-4 xl:grid-cols-[1fr_0.8fr]">
      <Card>
        <CardHeader>
          <CardTitle>Workspace guard</CardTitle>
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
      <Card>
        <CardHeader>
          <CardTitle>File purge</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <p className="text-sm text-text-secondary">
            Physical purge removes the managed agent folder after archive. The database agent,
            sessions, logs and audit records stay available.
          </p>
          <div className="rounded-md border border-border bg-background p-3 text-xs text-text-muted">
            Purge target: <span className="font-medium text-text-primary">{agent.name}</span>
          </div>
          <div className="grid gap-2">
            <Label htmlFor="purge-confirmation">Type agent name to confirm</Label>
            <Input
              id="purge-confirmation"
              value={confirmation}
              onChange={(event) => setConfirmation(event.target.value)}
              placeholder={agent.name}
              disabled={purge.isPending}
            />
          </div>
          {agent.status !== 'archived' ? (
            <p className="text-xs text-text-muted">Archive the agent before purging files.</p>
          ) : null}
          {purge.isError ? <ErrorState message={purge.error.message} /> : null}
          {purge.data ? (
            <p className="rounded-md border border-border bg-background p-3 text-sm text-text-secondary">
              {purge.data.message}: {purge.data.purged_path}
            </p>
          ) : null}
          <Button
            variant="destructive"
            onClick={() => purge.mutate()}
            disabled={!canPurge || purge.isPending}
          >
            <Trash2 className="h-4 w-4" />
            Purge files
          </Button>
        </CardContent>
      </Card>
    </div>
  )
}

function SessionsTab({ agent }: { agent: Agent }) {
  const userFilter = useSessionUserFilter()
  const sessions = useQuery({
    queryKey: ['sessions', agent.id, userFilter.selectedUserIds],
    queryFn: () => listSessions(agent.id, userFilter.selectedUserIds),
  })
  return (
    <Card>
      <CardHeader>
        <CardTitle>Agent sessions</CardTitle>
      </CardHeader>
      <CardContent className="space-y-2">
        <SessionUserFilter filter={userFilter} className="mb-3" />
        {sessions.data?.length ? (
          sessions.data.map((session) => <AgentSessionLink key={session.id} session={session} />)
        ) : (
          <EmptyState
            title={
              sessions.isLoading
                ? 'Loading sessions...'
                : userFilter.selectedUserIds.length
                  ? 'No sessions for this agent and selected users'
                  : 'No sessions for this agent'
            }
          />
        )}
      </CardContent>
    </Card>
  )
}

function AgentSessionLink({ session }: { session: AgentSession }) {
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
            {session.user_display_name} - leader {session.leader_agent_name ?? 'private'} -{' '}
            {session.last_message_preview ?? 'No messages yet'}
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
}: {
  label: string
  value: Record<string, unknown>
  onChange: (value: Record<string, unknown>) => void
}) {
  const [text, setText] = useState(JSON.stringify(value, null, 2))
  const [error, setError] = useState<string | null>(null)

  useEffect(() => setText(JSON.stringify(value, null, 2)), [value])

  function handleChange(next: string) {
    setText(next)
    try {
      const parsed = JSON.parse(next) as Record<string, unknown>
      setError(null)
      onChange(parsed)
    } catch {
      setError('Invalid JSON')
    }
  }

  return (
    <div className="grid gap-2">
      <label className="text-xs font-medium uppercase text-text-muted">{label}</label>
      <Textarea
        className="min-h-44 font-mono text-xs"
        value={text}
        onChange={(event) => handleChange(event.target.value)}
      />
      {error ? <p className="text-xs text-danger">{error}</p> : null}
    </div>
  )
}
