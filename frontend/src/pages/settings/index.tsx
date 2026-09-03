import { FormEvent, useEffect, useState } from 'react'
import { useSearchParams } from 'react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Save, ShieldCheck } from 'lucide-react'
import { listUsers, updateUserRole } from '@/api/auth'
import {
  getAuthSettings,
  getIntegrationSettings,
  getPortSettings,
  getRuntimeSettings,
  updateAuthSettings,
  updateIntegrationSettings,
  updatePortSettings,
  updateRuntimeSettings,
} from '@/api/fleet'
import type {
  AuthSettings,
  IntegrationSettings,
  PortSettings,
  RuntimeSettings,
  SystemRole,
} from '@/api/types'
import { Button } from '@sdlc/ui/ui'
import { Card, CardContent, CardHeader, CardTitle } from '@sdlc/ui/ui'
import { Input } from '@sdlc/ui/ui'
import { Label } from '@sdlc/ui/ui'
import { UserAvatar } from '@/shared/ui/user-avatar'
import { ErrorState, PageHeader, StatusBadge } from '../common'

const tabs = ['runtime', 'ports', 'integrations', 'auth', 'users'] as const
type SettingsTab = (typeof tabs)[number]

export function SettingsPage() {
  const [params, setParams] = useSearchParams()
  const selectedTab = tabs.includes(params.get('tab') as SettingsTab)
    ? (params.get('tab') as SettingsTab)
    : 'runtime'

  return (
    <>
      <PageHeader title="Settings" description="Runtime roots, ports, integrations and RBAC." />
      <div className="mb-4 flex flex-wrap gap-2">
        {tabs.map((tab) => (
          <Button
            key={tab}
            type="button"
            variant={selectedTab === tab ? 'default' : 'outline'}
            onClick={() => setParams({ tab })}
          >
            {tab.replaceAll('_', ' ')}
          </Button>
        ))}
      </div>
      {selectedTab === 'runtime' ? <RuntimeSettingsPanel /> : null}
      {selectedTab === 'ports' ? <PortSettingsPanel /> : null}
      {selectedTab === 'integrations' ? <IntegrationSettingsPanel /> : null}
      {selectedTab === 'auth' ? <AuthSettingsPanel /> : null}
      {selectedTab === 'users' ? <UsersPanel /> : null}
    </>
  )
}

function RuntimeSettingsPanel() {
  const queryClient = useQueryClient()
  const query = useQuery({ queryKey: ['settings', 'runtime'], queryFn: getRuntimeSettings })
  const [form, setForm] = useState<RuntimeSettings | null>(null)
  useEffect(() => {
    if (query.data) setForm(query.data)
  }, [query.data])
  const mutation = useMutation({
    mutationFn: () => updateRuntimeSettings(form!),
    onSuccess: async () => queryClient.invalidateQueries({ queryKey: ['settings', 'runtime'] }),
  })

  if (query.isError) return <ErrorState message={query.error.message} />
  if (!form) return <CardLoading title="Runtime" />

  return (
    <SettingsCard title="Runtime source policy" onSubmit={() => mutation.mutate()}>
      <TextField
        label="Agents root"
        value={form.agents_root}
        onChange={(agents_root) => setForm({ ...form, agents_root })}
      />
      <TextField
        label="Hermes source"
        value={form.hermes_source}
        onChange={(hermes_source) => setForm({ ...form, hermes_source })}
      />
      <TextField
        label="Hermes command"
        value={form.hermes_command}
        onChange={(hermes_command) => setForm({ ...form, hermes_command })}
      />
      <TextField
        label="Java Agent source"
        value={form.java_agent_source}
        onChange={(java_agent_source) => setForm({ ...form, java_agent_source })}
      />
      <TextField
        label="Java Agent command"
        value={form.java_agent_command}
        onChange={(java_agent_command) => setForm({ ...form, java_agent_command })}
      />
      <SaveButton pending={mutation.isPending} />
      {mutation.isError ? <ErrorState message={mutation.error.message} /> : null}
    </SettingsCard>
  )
}

function PortSettingsPanel() {
  const queryClient = useQueryClient()
  const query = useQuery({ queryKey: ['settings', 'ports'], queryFn: getPortSettings })
  const [form, setForm] = useState<PortSettings | null>(null)
  useEffect(() => {
    if (query.data) setForm(query.data)
  }, [query.data])
  const mutation = useMutation({
    mutationFn: () => updatePortSettings(form!),
    onSuccess: async () => queryClient.invalidateQueries({ queryKey: ['settings', 'ports'] }),
  })

  if (query.isError) return <ErrorState message={query.error.message} />
  if (!form) return <CardLoading title="Ports" />

  return (
    <SettingsCard title="Ports" onSubmit={() => mutation.mutate()}>
      <NumberField
        label="Backend port"
        value={form.backend_port}
        onChange={(backend_port) => setForm({ ...form, backend_port })}
      />
      <NumberField
        label="Frontend port"
        value={form.frontend_port}
        onChange={(frontend_port) => setForm({ ...form, frontend_port })}
      />
      <NumberField
        label="Agent port base"
        value={form.agent_port_base}
        onChange={(agent_port_base) => setForm({ ...form, agent_port_base })}
      />
      <NumberField
        label="Agent port stride"
        value={form.agent_port_stride}
        onChange={(agent_port_stride) => setForm({ ...form, agent_port_stride })}
      />
      <SaveButton pending={mutation.isPending} />
      {mutation.isError ? <ErrorState message={mutation.error.message} /> : null}
    </SettingsCard>
  )
}

function IntegrationSettingsPanel() {
  const queryClient = useQueryClient()
  const query = useQuery({
    queryKey: ['settings', 'integrations'],
    queryFn: getIntegrationSettings,
  })
  const [form, setForm] = useState<IntegrationSettings | null>(null)
  useEffect(() => {
    if (query.data) setForm(query.data)
  }, [query.data])
  const mutation = useMutation({
    mutationFn: () => updateIntegrationSettings(form!),
    onSuccess: async () =>
      queryClient.invalidateQueries({ queryKey: ['settings', 'integrations'] }),
  })

  if (query.isError) return <ErrorState message={query.error.message} />
  if (!form) return <CardLoading title="Integrations" />

  return (
    <SettingsCard title="Integrations" onSubmit={() => mutation.mutate()}>
      <TextField
        label="Project Workflow URL"
        value={form.project_workflow_url ?? ''}
        onChange={(project_workflow_url) =>
          setForm({ ...form, project_workflow_url: project_workflow_url || null })
        }
      />
      <TextField
        label="Project Workflow status"
        value={form.project_workflow_status}
        onChange={(project_workflow_status) => setForm({ ...form, project_workflow_status })}
      />
      <TextField
        label="GitHub remote"
        value={form.github_remote ?? ''}
        onChange={(github_remote) => setForm({ ...form, github_remote: github_remote || null })}
      />
      <SaveButton pending={mutation.isPending} />
      {mutation.isError ? <ErrorState message={mutation.error.message} /> : null}
    </SettingsCard>
  )
}

function AuthSettingsPanel() {
  const queryClient = useQueryClient()
  const query = useQuery({ queryKey: ['settings', 'auth'], queryFn: getAuthSettings })
  const [form, setForm] = useState<AuthSettings | null>(null)
  useEffect(() => {
    if (query.data) setForm(query.data)
  }, [query.data])
  const mutation = useMutation({
    mutationFn: () => updateAuthSettings(form!),
    onSuccess: async () => queryClient.invalidateQueries({ queryKey: ['settings', 'auth'] }),
  })

  if (query.isError) return <ErrorState message={query.error.message} />
  if (!form) return <CardLoading title="Auth" />

  return (
    <SettingsCard title="Auth policy" onSubmit={() => mutation.mutate()}>
      <SelectField
        label="Auth mode"
        value={form.mode}
        options={[{ value: 'hmac', label: 'hmac' }]}
        onChange={(mode) => setForm({ ...form, mode })}
      />
      <TextField
        label="JWT issuer"
        value={form.jwt_issuer}
        onChange={(jwt_issuer) => setForm({ ...form, jwt_issuer })}
      />
      <TextField
        label="JWT audience"
        value={form.jwt_audience}
        onChange={(jwt_audience) => setForm({ ...form, jwt_audience })}
      />
      <NumberField
        label="Access token TTL minutes"
        value={form.access_token_ttl_minutes}
        onChange={(access_token_ttl_minutes) => setForm({ ...form, access_token_ttl_minutes })}
      />
      <NumberField
        label="Refresh token TTL days"
        value={form.refresh_token_ttl_days}
        onChange={(refresh_token_ttl_days) => setForm({ ...form, refresh_token_ttl_days })}
      />
      <TextField
        label="Refresh cookie name"
        value={form.refresh_cookie_name}
        onChange={(refresh_cookie_name) => setForm({ ...form, refresh_cookie_name })}
      />
      <TextField
        label="SameSite"
        value={form.refresh_cookie_same_site}
        onChange={(refresh_cookie_same_site) => setForm({ ...form, refresh_cookie_same_site })}
      />
      <TextField
        label="Cookie domain"
        value={form.refresh_cookie_domain ?? ''}
        onChange={(refresh_cookie_domain) =>
          setForm({ ...form, refresh_cookie_domain: refresh_cookie_domain || null })
        }
      />
      <TextField
        label="Cookie path"
        value={form.refresh_cookie_path}
        onChange={(refresh_cookie_path) => setForm({ ...form, refresh_cookie_path })}
      />
      <label className="flex items-center gap-2 text-sm text-text-secondary">
        <input
          type="checkbox"
          checked={form.refresh_cookie_secure}
          onChange={(event) => setForm({ ...form, refresh_cookie_secure: event.target.checked })}
        />
        Secure refresh cookie
      </label>
      <SaveButton pending={mutation.isPending} />
      {mutation.isError ? <ErrorState message={mutation.error.message} /> : null}
    </SettingsCard>
  )
}

function UsersPanel() {
  const queryClient = useQueryClient()
  const users = useQuery({ queryKey: ['users'], queryFn: listUsers })
  const mutation = useMutation({
    mutationFn: ({ id, role }: { id: string; role: SystemRole }) => updateUserRole(id, { role }),
    onSuccess: async () => queryClient.invalidateQueries({ queryKey: ['users'] }),
  })

  if (users.isError) return <ErrorState message={users.error.message} />

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <ShieldCheck className="h-4 w-4" />
          Users and roles
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-2">
        {users.data?.map((user) => (
          <div
            key={user.id}
            className="flex flex-wrap items-center gap-3 rounded-md border border-border p-3"
          >
            <UserAvatar name={user.display_name} userId={user.id} size="md" />
            <div className="min-w-0 flex-1">
              <p className="truncate text-sm font-medium text-text-primary">{user.display_name}</p>
              <p className="truncate text-xs text-text-muted">{user.email}</p>
            </div>
            <StatusBadge value={user.system_role} />
            <select
              aria-label={`Role for ${user.display_name}`}
              value={user.system_role}
              onChange={(event) =>
                mutation.mutate({ id: user.id, role: event.target.value as SystemRole })
              }
              disabled={mutation.isPending}
              className="h-9 rounded-md border border-border bg-background px-3 text-sm"
            >
              <option value="admin">admin</option>
              <option value="operator">operator</option>
              <option value="user">user</option>
            </select>
          </div>
        ))}
        {mutation.isError ? <ErrorState message={mutation.error.message} /> : null}
      </CardContent>
    </Card>
  )
}

function SettingsCard({
  title,
  children,
  onSubmit,
}: {
  title: string
  children: React.ReactNode
  onSubmit: () => void
}) {
  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    onSubmit()
  }
  return (
    <Card>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
      </CardHeader>
      <CardContent>
        <form className="grid gap-3 md:grid-cols-2" onSubmit={submit}>
          {children}
        </form>
      </CardContent>
    </Card>
  )
}

function SelectField({
  label,
  value,
  options,
  onChange,
}: {
  label: string
  value: string
  options: Array<{ value: string; label: string }>
  onChange: (value: string) => void
}) {
  const id = label.toLowerCase().replaceAll(' ', '-')
  return (
    <div className="grid gap-2">
      <Label htmlFor={id}>{label}</Label>
      <select
        id={id}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="h-10 rounded-md border border-border bg-background px-3 text-sm text-text-primary"
      >
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </div>
  )
}

function TextField({
  label,
  value,
  onChange,
}: {
  label: string
  value: string
  onChange: (value: string) => void
}) {
  const id = label.toLowerCase().replaceAll(' ', '-')
  return (
    <div className="grid gap-2">
      <Label htmlFor={id}>{label}</Label>
      <Input id={id} value={value} onChange={(event) => onChange(event.target.value)} />
    </div>
  )
}

function NumberField({
  label,
  value,
  onChange,
}: {
  label: string
  value: number
  onChange: (value: number) => void
}) {
  const id = label.toLowerCase().replaceAll(' ', '-')
  return (
    <div className="grid gap-2">
      <Label htmlFor={id}>{label}</Label>
      <Input
        id={id}
        type="number"
        value={value}
        onChange={(event) => onChange(Number(event.target.value))}
      />
    </div>
  )
}

function SaveButton({ pending }: { pending: boolean }) {
  return (
    <Button type="submit" disabled={pending} className="md:col-span-2">
      <Save className="h-4 w-4" />
      Save settings
    </Button>
  )
}

function CardLoading({ title }: { title: string }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
      </CardHeader>
      <CardContent className="text-sm text-text-muted">Loading settings...</CardContent>
    </Card>
  )
}
