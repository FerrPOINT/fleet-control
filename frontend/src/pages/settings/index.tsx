import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { ExternalLink, Search } from 'lucide-react'
import { listUsers } from '@/api/auth'
import {
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Input,
  Tabs,
  TabsList,
  TabsTrigger,
  usePlatformServices,
} from '@sdlc/ui/ui'
import { UserAvatar } from '@/shared/ui/user-avatar'
import { ErrorState, PageHeader } from '../common'
import { ManagedSettingsWorkspace, type ManagedSettingsTab } from './managed-settings'

const tabs = ['runtime', 'ports', 'integrations', 'auth', 'retention', 'history', 'users'] as const
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
      </Tabs>
      {selectedTab === 'users' ? (
        <UsersPanel />
      ) : (
        <ManagedSettingsWorkspace tab={selectedTab as ManagedSettingsTab} />
      )}
    </>
  )
}

function UsersPanel() {
  const { t } = useTranslation()
  const users = useQuery({ queryKey: ['users'], queryFn: listUsers })
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
          <Button asChild variant="secondary" size="sm" className="h-10">
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
              className="h-10"
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
              className="h-10"
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
      <Button type="button" variant="outline" className="h-10" onClick={onRetry}>
        {t('settings.retry')}
      </Button>
    </div>
  )
}
