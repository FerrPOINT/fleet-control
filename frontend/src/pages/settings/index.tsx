import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { ExternalLink, Search } from 'lucide-react'
import { listUsers } from '@/api/auth'
import {
  getAuthSettings,
  getIntegrationSettings,
  getPortSettings,
  getRuntimeSettings,
} from '@/api/fleet'
import { Button, Tabs, TabsContent, TabsList, TabsTrigger, usePlatformServices } from '@sdlc/ui/ui'
import { Card, CardContent, CardHeader, CardTitle } from '@sdlc/ui/ui'
import { Input } from '@sdlc/ui/ui'
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
        {selectedTab !== 'users' && (
          <p
            role="note"
            className="mb-4 max-w-4xl border-l-2 border-warning pl-3 text-sm text-text-secondary"
          >
            {t('settings.managedNotice')}
          </p>
        )}
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

function RuntimeSettingsPanel({ active }: { active: boolean }) {
  const { t } = useTranslation()
  const query = useQuery({
    queryKey: ['settings', 'runtime'],
    queryFn: getRuntimeSettings,
    enabled: active,
  })

  if (query.isError && !query.data) return <PanelError onRetry={() => void query.refetch()} />
  if (!query.data) return <CardLoading title={t('settings.runtimeTitle')} />

  return (
    <SettingsSnapshotCard
      title={t('settings.runtimeTitle')}
      owner={t('settings.runtimeOwner')}
      rows={[
        { label: t('settings.agentsRoot'), value: query.data.agents_root, technical: true },
        { label: t('settings.hermesSource'), value: query.data.hermes_source, technical: true },
        { label: t('settings.hermesCommand'), value: query.data.hermes_command, technical: true },
        {
          label: t('settings.javaAgentSource'),
          value: query.data.java_agent_source,
          technical: true,
        },
        {
          label: t('settings.javaAgentCommand'),
          value: query.data.java_agent_command,
          technical: true,
        },
      ]}
      refreshError={query.isError}
      onRetry={() => void query.refetch()}
    />
  )
}

function PortSettingsPanel({ active }: { active: boolean }) {
  const { t } = useTranslation()
  const query = useQuery({
    queryKey: ['settings', 'ports'],
    queryFn: getPortSettings,
    enabled: active,
  })

  if (query.isError && !query.data) return <PanelError onRetry={() => void query.refetch()} />
  if (!query.data) return <CardLoading title={t('settings.portsTitle')} />

  return (
    <SettingsSnapshotCard
      title={t('settings.portsTitle')}
      owner={t('settings.portsOwner')}
      rows={[
        { label: t('settings.backendPort'), value: String(query.data.backend_port) },
        { label: t('settings.frontendPort'), value: String(query.data.frontend_port) },
        { label: t('settings.agentPortBase'), value: String(query.data.agent_port_base) },
        { label: t('settings.agentPortStride'), value: String(query.data.agent_port_stride) },
      ]}
      refreshError={query.isError}
      onRetry={() => void query.refetch()}
    />
  )
}

function IntegrationSettingsPanel({ active }: { active: boolean }) {
  const { t } = useTranslation()
  const query = useQuery({
    queryKey: ['settings', 'integrations'],
    queryFn: getIntegrationSettings,
    enabled: active,
  })

  if (query.isError && !query.data) return <PanelError onRetry={() => void query.refetch()} />
  if (!query.data) return <CardLoading title={t('settings.integrationsTitle')} />

  return (
    <SettingsSnapshotCard
      title={t('settings.integrationsTitle')}
      owner={t('settings.integrationsOwner')}
      rows={[
        {
          label: t('settings.projectWorkflowUrl'),
          value: query.data.project_workflow_url ?? t('settings.notConfigured'),
          technical: Boolean(query.data.project_workflow_url),
        },
        {
          label: t('settings.projectWorkflowStatus'),
          value: t(`settings.integrationStatus.${query.data.project_workflow_status}`, {
            defaultValue: query.data.project_workflow_status,
          }),
        },
      ]}
      refreshError={query.isError}
      onRetry={() => void query.refetch()}
    />
  )
}

function AuthSettingsPanel({ active }: { active: boolean }) {
  const { t } = useTranslation()
  const query = useQuery({
    queryKey: ['settings', 'auth'],
    queryFn: getAuthSettings,
    enabled: active,
  })

  if (query.isError && !query.data) return <PanelError onRetry={() => void query.refetch()} />
  if (!query.data) return <CardLoading title={t('settings.authTitle')} />

  return (
    <SettingsSnapshotCard
      title={t('settings.authTitle')}
      owner={t('settings.authOwner')}
      rows={[
        { label: t('settings.authMode'), value: query.data.mode.toUpperCase() },
        { label: t('settings.jwtIssuer'), value: query.data.jwt_issuer, technical: true },
        { label: t('settings.jwtAudience'), value: query.data.jwt_audience, technical: true },
        {
          label: t('settings.accessTokenTtl'),
          value: String(query.data.access_token_ttl_minutes),
        },
        { label: t('settings.refreshTokenTtl'), value: String(query.data.refresh_token_ttl_days) },
        {
          label: t('settings.refreshCookieName'),
          value: query.data.refresh_cookie_name,
          technical: true,
        },
        { label: t('settings.sameSite'), value: query.data.refresh_cookie_same_site },
        {
          label: t('settings.cookieDomain'),
          value: query.data.refresh_cookie_domain ?? t('settings.notConfigured'),
          technical: Boolean(query.data.refresh_cookie_domain),
        },
        { label: t('settings.cookiePath'), value: query.data.refresh_cookie_path, technical: true },
        {
          label: t('settings.secureRefreshCookie'),
          value: query.data.refresh_cookie_secure ? t('settings.yes') : t('settings.no'),
        },
      ]}
      refreshError={query.isError}
      onRetry={() => void query.refetch()}
    />
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

function SettingsSnapshotCard({
  title,
  owner,
  rows,
  refreshError,
  onRetry,
}: {
  title: string
  owner: string
  rows: Array<{ label: string; value: string; technical?: boolean }>
  refreshError: boolean
  onRetry: () => void
}) {
  const { t } = useTranslation()
  return (
    <section aria-label={title} className="max-w-4xl">
      <Card>
        <CardHeader className="flex-row flex-wrap items-center justify-between gap-2">
          <CardTitle>{title}</CardTitle>
          <span className="rounded bg-surface-raised px-2 py-1 text-xs font-medium text-text-secondary">
            {t('settings.readOnly')}
          </span>
        </CardHeader>
        <CardContent className="space-y-4">
          <p className="text-sm text-text-secondary">{owner}</p>
          {refreshError && (
            <div role="alert" className="flex flex-wrap items-center gap-2 text-sm text-danger">
              <span>{t('settings.refreshError')}</span>
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
          <dl className="divide-y divide-border border-y border-border">
            {rows.map((row) => (
              <div
                key={row.label}
                className="grid min-w-0 gap-1 py-3 sm:grid-cols-[13rem_minmax(0,1fr)] sm:gap-4"
              >
                <dt className="text-sm text-text-muted">{row.label}</dt>
                <dd
                  className={`min-w-0 break-words text-sm text-text-primary ${row.technical ? 'font-mono' : ''}`}
                >
                  {row.value}
                </dd>
              </div>
            ))}
          </dl>
        </CardContent>
      </Card>
    </section>
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
