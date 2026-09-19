import { FormEvent, useEffect, useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ExternalLink, Save, Search } from 'lucide-react'
import { listUsers } from '@/api/auth'
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
import type { AuthSettings, IntegrationSettings, PortSettings, RuntimeSettings } from '@/api/types'
import { Button, Tabs, TabsContent, TabsList, TabsTrigger, usePlatformServices } from '@sdlc/ui/ui'
import { Card, CardContent, CardHeader, CardTitle } from '@sdlc/ui/ui'
import { Input } from '@sdlc/ui/ui'
import { Label } from '@sdlc/ui/ui'
import { UserAvatar } from '@/shared/ui/user-avatar'
import { ErrorState, PageHeader } from '../common'

const tabs = ['runtime', 'ports', 'integrations', 'auth', 'users'] as const
type SettingsTab = (typeof tabs)[number]

export function SettingsPage() {
  const { t } = useTranslation()
  const [params, setParams] = useSearchParams()
  const selectedTab = tabs.includes(params.get('tab') as SettingsTab)
    ? (params.get('tab') as SettingsTab)
    : 'runtime'

  return (
    <>
      <PageHeader title={t('settings.title')} />
      <Tabs value={selectedTab} onValueChange={(tab) => setParams({ tab })}>
        <div className="mb-4 max-w-full">
          <TabsList
            aria-label={t('settings.title')}
            className="h-auto min-h-10 w-full flex-wrap justify-start sm:w-max"
          >
            {tabs.map((tab) => (
              <TabsTrigger key={tab} value={tab} className="min-h-10">
                {t(`settings.tabs.${tab}`)}
              </TabsTrigger>
            ))}
          </TabsList>
        </div>
        <TabsContent value="runtime" forceMount className="data-[state=inactive]:hidden">
          <RuntimeSettingsPanel active={selectedTab === 'runtime'} />
        </TabsContent>
        <TabsContent value="ports" forceMount className="data-[state=inactive]:hidden">
          <PortSettingsPanel active={selectedTab === 'ports'} />
        </TabsContent>
        <TabsContent value="integrations" forceMount className="data-[state=inactive]:hidden">
          <IntegrationSettingsPanel active={selectedTab === 'integrations'} />
        </TabsContent>
        <TabsContent value="auth" forceMount className="data-[state=inactive]:hidden">
          <AuthSettingsPanel active={selectedTab === 'auth'} />
        </TabsContent>
        <TabsContent value="users" forceMount className="data-[state=inactive]:hidden">
          <UsersPanel active={selectedTab === 'users'} />
        </TabsContent>
      </Tabs>
    </>
  )
}

function hasChanges<T extends object>(current: T, saved: T) {
  return Object.keys(current).some((key) => current[key as keyof T] !== saved[key as keyof T])
}

function useSettingsForm<T extends object>(data: T | undefined) {
  const [snapshot, setSnapshot] = useState<{ current: T; saved: T } | null>(null)
  useEffect(() => {
    if (!data) return
    setSnapshot((previous) =>
      previous && hasChanges(previous.current, previous.saved)
        ? { current: previous.current, saved: data }
        : { current: data, saved: data },
    )
  }, [data])

  function change<K extends keyof T>(key: K, value: T[K]) {
    setSnapshot((previous) =>
      previous ? { ...previous, current: { ...previous.current, [key]: value } } : previous,
    )
  }

  return {
    form: snapshot?.current ?? null,
    dirty: snapshot ? hasChanges(snapshot.current, snapshot.saved) : false,
    change,
    reset: () =>
      setSnapshot((previous) => (previous ? { ...previous, current: previous.saved } : previous)),
    markSaved: (saved: T) => setSnapshot({ current: saved, saved }),
  }
}

function RuntimeSettingsPanel({ active }: { active: boolean }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const query = useQuery({
    queryKey: ['settings', 'runtime'],
    queryFn: getRuntimeSettings,
    enabled: active,
  })
  const { form, dirty, change, reset, markSaved } = useSettingsForm<RuntimeSettings>(query.data)
  const mutation = useMutation({
    mutationFn: () => updateRuntimeSettings(form!),
    onSuccess: (saved) => {
      markSaved(saved)
      queryClient.setQueryData(['settings', 'runtime'], saved)
    },
  })

  if (query.isError && !form) return <PanelError onRetry={() => void query.refetch()} />
  if (!form) return <CardLoading title={t('settings.runtimeTitle')} />

  return (
    <SettingsCard
      title={t('settings.runtimeTitle')}
      onSubmit={() => mutation.mutate()}
      dirty={dirty}
      pending={mutation.isPending}
      success={mutation.isSuccess}
      error={mutation.isError}
      loadError={query.isError}
      onRetry={() => void query.refetch()}
      onReset={() => {
        reset()
        mutation.reset()
      }}
    >
      <TextField
        label={t('settings.agentsRoot')}
        value={form.agents_root}
        onChange={(value) => change('agents_root', value)}
        required
      />
      <TextField
        label={t('settings.hermesSource')}
        value={form.hermes_source}
        onChange={(value) => change('hermes_source', value)}
      />
      <TextField
        label={t('settings.hermesCommand')}
        value={form.hermes_command}
        onChange={(value) => change('hermes_command', value)}
        required
      />
      <TextField
        label={t('settings.javaAgentSource')}
        value={form.java_agent_source}
        onChange={(value) => change('java_agent_source', value)}
      />
      <TextField
        label={t('settings.javaAgentCommand')}
        value={form.java_agent_command}
        onChange={(value) => change('java_agent_command', value)}
        required
      />
    </SettingsCard>
  )
}

function PortSettingsPanel({ active }: { active: boolean }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const query = useQuery({
    queryKey: ['settings', 'ports'],
    queryFn: getPortSettings,
    enabled: active,
  })
  const { form, dirty, change, reset, markSaved } = useSettingsForm<PortSettings>(query.data)
  const mutation = useMutation({
    mutationFn: () => updatePortSettings(form!),
    onSuccess: (saved) => {
      markSaved(saved)
      queryClient.setQueryData(['settings', 'ports'], saved)
    },
  })

  if (query.isError && !form) return <PanelError onRetry={() => void query.refetch()} />
  if (!form) return <CardLoading title={t('settings.portsTitle')} />

  return (
    <SettingsCard
      title={t('settings.portsTitle')}
      onSubmit={() => mutation.mutate()}
      dirty={dirty}
      pending={mutation.isPending}
      success={mutation.isSuccess}
      error={mutation.isError}
      loadError={query.isError}
      onRetry={() => void query.refetch()}
      onReset={() => {
        reset()
        mutation.reset()
      }}
    >
      <NumberField
        label={t('settings.backendPort')}
        value={form.backend_port}
        onChange={(value) => change('backend_port', value)}
        max={65535}
      />
      <NumberField
        label={t('settings.frontendPort')}
        value={form.frontend_port}
        onChange={(value) => change('frontend_port', value)}
        max={65535}
      />
      <NumberField
        label={t('settings.agentPortBase')}
        value={form.agent_port_base}
        onChange={(value) => change('agent_port_base', value)}
        max={65535}
      />
      <NumberField
        label={t('settings.agentPortStride')}
        value={form.agent_port_stride}
        onChange={(value) => change('agent_port_stride', value)}
        min={4}
        max={65535}
      />
    </SettingsCard>
  )
}

function IntegrationSettingsPanel({ active }: { active: boolean }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const query = useQuery({
    queryKey: ['settings', 'integrations'],
    queryFn: getIntegrationSettings,
    enabled: active,
  })
  const { form, dirty, change, reset, markSaved } = useSettingsForm<IntegrationSettings>(query.data)
  const mutation = useMutation({
    mutationFn: () => updateIntegrationSettings(form!),
    onSuccess: (saved) => {
      markSaved(saved)
      queryClient.setQueryData(['settings', 'integrations'], saved)
    },
  })

  if (query.isError && !form) return <PanelError onRetry={() => void query.refetch()} />
  if (!form) return <CardLoading title={t('settings.integrationsTitle')} />

  return (
    <SettingsCard
      title={t('settings.integrationsTitle')}
      onSubmit={() => mutation.mutate()}
      dirty={dirty}
      pending={mutation.isPending}
      success={mutation.isSuccess}
      error={mutation.isError}
      loadError={query.isError}
      onRetry={() => void query.refetch()}
      onReset={() => {
        reset()
        mutation.reset()
      }}
    >
      <TextField
        label={t('settings.projectWorkflowUrl')}
        value={form.project_workflow_url ?? ''}
        onChange={(value) => change('project_workflow_url', value || null)}
      />
      <TextField
        label={t('settings.projectWorkflowStatus')}
        value={form.project_workflow_status}
        onChange={(value) => change('project_workflow_status', value)}
      />
      <TextField
        label={t('settings.githubRemote')}
        value={form.github_remote ?? ''}
        onChange={(value) => change('github_remote', value || null)}
      />
    </SettingsCard>
  )
}

function AuthSettingsPanel({ active }: { active: boolean }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const query = useQuery({
    queryKey: ['settings', 'auth'],
    queryFn: getAuthSettings,
    enabled: active,
  })
  const { form, dirty, change, reset, markSaved } = useSettingsForm<AuthSettings>(query.data)
  const mutation = useMutation({
    mutationFn: () => updateAuthSettings(form!),
    onSuccess: (saved) => {
      markSaved(saved)
      queryClient.setQueryData(['settings', 'auth'], saved)
    },
  })

  if (query.isError && !form) return <PanelError onRetry={() => void query.refetch()} />
  if (!form) return <CardLoading title={t('settings.authTitle')} />

  return (
    <SettingsCard
      title={t('settings.authTitle')}
      onSubmit={() => mutation.mutate()}
      dirty={dirty}
      pending={mutation.isPending}
      success={mutation.isSuccess}
      error={mutation.isError}
      loadError={query.isError}
      onRetry={() => void query.refetch()}
      onReset={() => {
        reset()
        mutation.reset()
      }}
    >
      <SelectField
        label={t('settings.authMode')}
        value={form.mode}
        options={[{ value: 'hmac', label: 'HMAC' }]}
        onChange={(value) => change('mode', value)}
      />
      <TextField
        label={t('settings.jwtIssuer')}
        value={form.jwt_issuer}
        onChange={(value) => change('jwt_issuer', value)}
        required
      />
      <TextField
        label={t('settings.jwtAudience')}
        value={form.jwt_audience}
        onChange={(value) => change('jwt_audience', value)}
        required
      />
      <NumberField
        label={t('settings.accessTokenTtl')}
        value={form.access_token_ttl_minutes}
        onChange={(value) => change('access_token_ttl_minutes', value)}
      />
      <NumberField
        label={t('settings.refreshTokenTtl')}
        value={form.refresh_token_ttl_days}
        onChange={(value) => change('refresh_token_ttl_days', value)}
      />
      <TextField
        label={t('settings.refreshCookieName')}
        value={form.refresh_cookie_name}
        onChange={(value) => change('refresh_cookie_name', value)}
        required
      />
      <SelectField
        label={t('settings.sameSite')}
        value={form.refresh_cookie_same_site}
        options={['Lax', 'Strict', 'None'].map((value) => ({ value, label: value }))}
        onChange={(value) => change('refresh_cookie_same_site', value)}
      />
      <TextField
        label={t('settings.cookieDomain')}
        value={form.refresh_cookie_domain ?? ''}
        onChange={(value) => change('refresh_cookie_domain', value || null)}
      />
      <TextField
        label={t('settings.cookiePath')}
        value={form.refresh_cookie_path}
        onChange={(value) => change('refresh_cookie_path', value)}
        required
      />
      <label className="flex min-h-10 items-center gap-2 text-sm text-text-secondary">
        <input
          type="checkbox"
          className="h-4 w-4"
          checked={form.refresh_cookie_secure}
          onChange={(event) => change('refresh_cookie_secure', event.target.checked)}
        />
        {t('settings.secureRefreshCookie')}
      </label>
    </SettingsCard>
  )
}

function UsersPanel({ active }: { active: boolean }) {
  const { t } = useTranslation()
  const users = useQuery({ queryKey: ['users'], queryFn: listUsers, enabled: active })
  const { services } = usePlatformServices()
  const adminUrl = services.find((service) => service.key === 'admin-panel')?.ui_url
  const [search, setSearch] = useState('')
  const [page, setPage] = useState(1)
  const normalizedSearch = search.trim().toLocaleLowerCase()
  const filteredUsers = (users.data ?? [])
    .filter((user) =>
      `${user.display_name} ${user.email}`.toLocaleLowerCase().includes(normalizedSearch),
    )
    .sort((a, b) => a.display_name.localeCompare(b.display_name))
  const totalPages = Math.max(1, Math.ceil(filteredUsers.length / 12))
  const currentPage = Math.min(page, totalPages)
  const visibleUsers = filteredUsers.slice((currentPage - 1) * 12, currentPage * 12)

  if (users.isError) return <PanelError onRetry={() => void users.refetch()} />
  if (users.isLoading) return <CardLoading title={t('settings.usersTitle')} />

  return (
    <Card className="max-w-4xl">
      <CardHeader className="flex-row flex-wrap items-center justify-between gap-3">
        <CardTitle>{t('settings.usersTitle')}</CardTitle>
        {adminUrl && (
          <Button asChild variant="secondary" size="sm" className="min-h-10 sm:min-h-10">
            <a href={`${adminUrl.replace(/\/$/, '')}/users`}>
              <ExternalLink className="h-4 w-4" aria-hidden />
              {t('settings.manageUsers')}
            </a>
          </Button>
        )}
      </CardHeader>
      <CardContent className="space-y-3">
        {!adminUrl && <p className="text-sm text-text-muted">{t('settings.adminUnavailable')}</p>}
        {(users.data?.length ?? 0) > 0 && (
          <div className="relative">
            <Search
              className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-text-muted"
              aria-hidden
            />
            <Input
              type="search"
              aria-label={t('settings.searchUsers')}
              placeholder={t('settings.searchUsers')}
              className="min-h-10 pl-9"
              value={search}
              onChange={(event) => {
                setSearch(event.target.value)
                setPage(1)
              }}
            />
          </div>
        )}
        {(users.data?.length ?? 0) > 0 && (
          <p className="text-xs text-text-muted">
            {t('settings.shown', { count: visibleUsers.length, total: filteredUsers.length })}
          </p>
        )}
        {users.data?.length === 0 && (
          <p className="text-sm text-text-muted">{t('settings.noUsers')}</p>
        )}
        {users.data && users.data.length > 0 && filteredUsers.length === 0 && (
          <p role="status" className="py-4 text-sm text-text-muted">
            {t('settings.noMatches')}
          </p>
        )}
        {visibleUsers.map((user) => (
          <div
            key={user.id}
            className="flex min-w-0 items-center gap-3 border-b border-border py-2 last:border-0"
          >
            <UserAvatar name={user.display_name} userId={user.id} size="md" />
            <div className="min-w-0 flex-1">
              <p className="break-words text-sm font-medium text-text-primary">
                {user.display_name}
              </p>
              <p className="break-all text-xs text-text-muted">{user.email}</p>
            </div>
          </div>
        ))}
        {filteredUsers.length > 12 && (
          <nav
            aria-label={t('settings.usersTitle')}
            className="flex items-center justify-end gap-2"
          >
            <Button
              type="button"
              size="sm"
              variant="outline"
              className="min-h-10 sm:min-h-10"
              disabled={currentPage === 1}
              onClick={() => setPage(currentPage - 1)}
            >
              {t('settings.previous')}
            </Button>
            <span className="text-sm text-text-muted">
              {currentPage} / {totalPages}
            </span>
            <Button
              type="button"
              size="sm"
              variant="outline"
              className="min-h-10 sm:min-h-10"
              disabled={currentPage === totalPages}
              onClick={() => setPage(currentPage + 1)}
            >
              {t('settings.next')}
            </Button>
          </nav>
        )}
      </CardContent>
    </Card>
  )
}

function SettingsCard({
  title,
  children,
  onSubmit,
  dirty,
  pending,
  success,
  error,
  loadError,
  onRetry,
  onReset,
}: {
  title: string
  children: React.ReactNode
  onSubmit: () => void
  dirty: boolean
  pending: boolean
  success: boolean
  error: boolean
  loadError: boolean
  onRetry: () => void
  onReset: () => void
}) {
  const { t } = useTranslation()
  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (dirty && !pending) onSubmit()
  }
  return (
    <Card className="max-w-4xl">
      <CardHeader>
        <CardTitle>{title}</CardTitle>
      </CardHeader>
      <CardContent>
        <form aria-label={title} className="grid gap-3 md:grid-cols-2" onSubmit={submit}>
          {loadError && (
            <div
              role="alert"
              className="flex flex-wrap items-center gap-2 text-sm text-danger md:col-span-2"
            >
              <span>{t('settings.loadError')}</span>
              <Button
                type="button"
                variant="outline"
                className="min-h-10 sm:min-h-10"
                onClick={onRetry}
              >
                {t('settings.retry')}
              </Button>
            </div>
          )}
          <fieldset disabled={pending} className="contents">
            {children}
          </fieldset>
          <div className="flex flex-wrap items-center justify-end gap-2 border-t border-border pt-3 md:col-span-2">
            {success && !dirty && (
              <p role="status" className="mr-auto text-sm text-success">
                {t('settings.saved')}
              </p>
            )}
            {error && (
              <p role="alert" className="w-full text-sm text-danger">
                {t('settings.saveError')}
              </p>
            )}
            <Button
              type="button"
              variant="outline"
              className="min-h-10 sm:min-h-10"
              onClick={onReset}
              disabled={!dirty || pending}
            >
              {t('settings.reset')}
            </Button>
            <Button type="submit" className="min-h-10 sm:min-h-10" disabled={!dirty || pending}>
              <Save className="h-4 w-4" aria-hidden />
              {pending ? t('settings.saving') : t('settings.save')}
            </Button>
          </div>
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
  const id = useId()
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
  required = false,
}: {
  label: string
  value: string
  onChange: (value: string) => void
  required?: boolean
}) {
  const id = useId()
  return (
    <div className="grid gap-2">
      <Label htmlFor={id}>{label}</Label>
      <Input
        id={id}
        className="min-h-10"
        value={value}
        onChange={(event) => onChange(event.target.value)}
        required={required}
      />
    </div>
  )
}

function NumberField({
  label,
  value,
  onChange,
  min = 1,
  max,
}: {
  label: string
  value: number
  onChange: (value: number) => void
  min?: number
  max?: number
}) {
  const id = useId()
  return (
    <div className="grid gap-2">
      <Label htmlFor={id}>{label}</Label>
      <Input
        id={id}
        type="number"
        className="min-h-10"
        min={min}
        max={max}
        step={1}
        required
        value={Number.isNaN(value) ? '' : value}
        onChange={(event) => onChange(event.target.value === '' ? NaN : Number(event.target.value))}
      />
    </div>
  )
}

function CardLoading({ title }: { title: string }) {
  const { t } = useTranslation()
  return (
    <Card className="max-w-4xl">
      <CardHeader>
        <CardTitle>{title}</CardTitle>
      </CardHeader>
      <CardContent role="status" className="text-sm text-text-muted">
        {t('settings.loading')}
      </CardContent>
    </Card>
  )
}

function PanelError({ onRetry }: { onRetry: () => void }) {
  const { t } = useTranslation()
  return (
    <div className="flex flex-wrap items-center gap-3">
      <ErrorState message={t('settings.loadError')} />
      <Button type="button" variant="outline" className="min-h-10 sm:min-h-10" onClick={onRetry}>
        {t('settings.retry')}
      </Button>
    </div>
  )
}
