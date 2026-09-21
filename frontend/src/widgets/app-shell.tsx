import {
  Activity,
  Bot,
  Boxes,
  Crown,
  Gauge,
  GitBranch,
  LogOut,
  Menu,
  ScrollText,
  Settings,
  TriangleAlert,
  UserRound,
  UserRoundCheck,
} from 'lucide-react'
import { useEffect, useState } from 'react'
import { NavLink, Outlet } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { getCurrentUserPermissions } from '@/api/auth'
import { endSso } from '@sdlc/ui/sso'
import { ssoConfig, useAuthStore } from '@/shared/auth/store'
import {
  Button,
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
  PlatformMark,
  ServiceSwitcher,
  ThemeToggle,
} from '@sdlc/ui/ui'
import { cn } from '@/shared/lib/utils'

const navItems = [
  { to: '/', labelKey: 'navigation.dashboard', icon: Gauge, permission: 'agents:manage' },
  { to: '/leaders', labelKey: 'navigation.leaders', icon: Crown, permission: 'leaders:manage' },
  {
    to: '/executors',
    labelKey: 'navigation.executors',
    icon: UserRoundCheck,
    permission: 'executors:manage',
  },
  { to: '/agents', labelKey: 'navigation.agents', icon: Bot, permission: 'agents:manage' },
  { to: '/sessions', labelKey: 'navigation.sessions', icon: Activity },
  {
    to: '/workflows',
    labelKey: 'navigation.workflows',
    icon: GitBranch,
    permission: 'agents:manage',
  },
  {
    to: '/deployments',
    labelKey: 'navigation.deployments',
    icon: Boxes,
    permission: 'deployments:manage',
  },
  { to: '/alerts', labelKey: 'navigation.alerts', icon: TriangleAlert, permission: 'logs:read' },
  { to: '/logs', labelKey: 'navigation.logs', icon: ScrollText, permission: 'logs:read' },
  {
    to: '/settings',
    labelKey: 'navigation.settings',
    icon: Settings,
    permission: 'settings:manage',
  },
]

type NavigationItem = (typeof navItems)[number]

export function AppShell() {
  const { t } = useTranslation()
  const displayName = useAuthStore((state) => state.displayName)
  const email = useAuthStore((state) => state.email)
  const permissions = useAuthStore((state) => state.permissions)
  const setUser = useAuthStore((state) => state.setUser)
  const clearAuth = useAuthStore((state) => state.logout)
  const visibleNavItems = navItems.filter(
    (item) => !item.permission || permissions.includes(item.permission),
  )
  const permissionsQuery = useQuery({
    queryKey: ['me', 'permissions'],
    queryFn: getCurrentUserPermissions,
    staleTime: 60_000,
  })
  const operatorName = displayName ?? email ?? t('app.operator')

  useEffect(() => {
    if (!permissionsQuery.data) return
    setUser({
      userId: permissionsQuery.data.user_id,
      systemRole: permissionsQuery.data.role,
      isSystemAdmin: permissionsQuery.data.is_system_admin,
      permissions: permissionsQuery.data.permissions,
    })
  }, [permissionsQuery.data, setUser])

  function handleLogout() {
    clearAuth()
    endSso(ssoConfig)
  }

  return (
    <div className="min-h-screen bg-background text-text-primary">
      <aside className="fixed inset-y-0 left-0 z-30 hidden w-[72px] flex-col border-r border-border bg-surface px-2 py-3 md:flex xl:w-[264px] xl:px-3">
        <div className="mb-4 flex h-10 items-center justify-center gap-3 px-1 xl:justify-start xl:px-2">
          <PlatformMark withName={false} />
          <div className="hidden min-w-0 xl:block">
            <p className="truncate text-sm font-semibold text-text-primary">{t('app.name')}</p>
            <p className="truncate text-xs text-text-muted">{t('app.subtitle')}</p>
          </div>
        </div>
        <ShellNavigation items={visibleNavItems} compact />
        <div className="mt-auto border-t border-border px-1 pt-3 xl:px-2">
          <div
            className="flex min-h-10 items-center justify-center gap-3 text-text-secondary xl:justify-start"
            title={operatorName}
          >
            <UserRound className="h-4 w-4 shrink-0" aria-hidden />
            <div className="hidden min-w-0 xl:block">
              <p className="truncate text-sm font-medium text-text-primary">{operatorName}</p>
              {displayName && email ? (
                <p className="truncate text-xs text-text-muted">{email}</p>
              ) : null}
            </div>
            <span className="sr-only xl:hidden">{operatorName}</span>
          </div>
        </div>
      </aside>

      <div className="min-w-0 overflow-x-hidden md:pl-[72px] xl:pl-[264px]">
        <header className="sticky top-0 z-20 h-[60px] border-b border-border bg-background/95 backdrop-blur">
          <div className="flex h-full items-center gap-2 px-4 md:px-5 xl:px-6">
            <MobileNavigation items={visibleNavItems} operatorName={operatorName} email={email} />
            <div className="min-w-0 md:hidden">
              <p className="truncate text-sm font-semibold text-text-primary">{t('app.name')}</p>
              <p className="truncate text-xs text-text-muted">{t('app.subtitle')}</p>
            </div>

            <div className="ml-auto hidden min-w-0 text-right lg:block">
              <p className="truncate text-xs font-medium text-text-primary">{operatorName}</p>
              {displayName && email ? (
                <p className="truncate text-xs text-text-muted">{email}</p>
              ) : null}
            </div>
            <div className="flex shrink-0 items-center gap-1 sm:gap-2">
              <ServiceSwitcher currentKey="fleet-control" />
              <div className="[&>button]:h-10 [&>button]:min-h-10 [&>button]:w-10 [&>button]:min-w-10">
                <ThemeToggle />
              </div>
              <Button
                type="button"
                variant="outline"
                size="icon"
                className="h-10 min-h-10 w-10 min-w-10 sm:w-auto sm:px-3"
                aria-label={t('app.signOut')}
                title={t('app.signOut')}
                onClick={handleLogout}
              >
                <LogOut className="h-4 w-4" aria-hidden />
                <span className="hidden sm:inline">{t('app.signOut')}</span>
              </Button>
            </div>
          </div>
        </header>

        <main className="min-h-[calc(100dvh-60px)] min-w-0 px-4 py-5 md:px-5 xl:px-6">
          <Outlet />
        </main>
      </div>
    </div>
  )
}

function ShellNavigation({
  items,
  compact = false,
}: {
  items: NavigationItem[]
  compact?: boolean
}) {
  const { t } = useTranslation()
  return (
    <nav className="space-y-1" aria-label={t('navigation.sections')}>
      {items.map((item) => {
        const label = t(item.labelKey)
        return (
          <NavLink
            key={item.to}
            to={item.to}
            end={item.to === '/'}
            aria-label={compact ? label : undefined}
            title={compact ? label : undefined}
            className={({ isActive }) =>
              cn(
                'flex h-10 items-center gap-3 rounded-md px-3 text-sm text-text-secondary transition-colors hover:bg-surface-raised hover:text-text-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus',
                compact && 'justify-center xl:justify-start',
                isActive && 'bg-surface-raised text-text-primary',
              )
            }
          >
            <item.icon className="h-4 w-4 shrink-0" aria-hidden />
            <span className={compact ? 'hidden xl:inline' : undefined}>{label}</span>
          </NavLink>
        )
      })}
    </nav>
  )
}

function MobileNavigation({
  items,
  operatorName,
  email,
}: {
  items: NavigationItem[]
  operatorName: string
  email: string | null
}) {
  const { t } = useTranslation()
  const [open, setOpen] = useState(false)
  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="h-10 min-h-10 w-10 min-w-10 md:hidden"
          aria-label={t('shell.openNavigation')}
          title={t('shell.openNavigation')}
        >
          <Menu className="h-5 w-5" aria-hidden />
        </Button>
      </DialogTrigger>
      <DialogContent className="!left-0 !top-0 !flex !h-dvh !max-h-dvh !w-[min(320px,calc(100%-2rem))] !max-w-none !translate-x-0 !translate-y-0 !flex-col !gap-0 !rounded-none !border-y-0 !border-l-0 !p-0 [&>button]:h-10 [&>button]:min-h-10 [&>button]:w-10 [&>button]:min-w-10">
        <DialogHeader className="flex h-[60px] flex-row items-center gap-3 border-b border-border px-4 pr-14 text-left">
          <PlatformMark withName={false} />
          <div className="min-w-0">
            <DialogTitle className="truncate text-base">{t('app.name')}</DialogTitle>
            <p className="truncate text-xs text-text-muted">{t('app.subtitle')}</p>
          </div>
        </DialogHeader>
        <div className="flex min-h-0 flex-1 flex-col overflow-y-auto px-3 py-4">
          <nav className="space-y-1" aria-label={t('navigation.sections')}>
            {items.map((item) => (
              <NavLink
                key={item.to}
                to={item.to}
                end={item.to === '/'}
                onClick={() => setOpen(false)}
                className={({ isActive }) =>
                  cn(
                    'flex h-10 items-center gap-3 rounded-md px-3 text-sm text-text-secondary transition-colors hover:bg-surface-raised hover:text-text-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus',
                    isActive && 'bg-surface-raised text-text-primary',
                  )
                }
              >
                <item.icon className="h-4 w-4 shrink-0" aria-hidden />
                <span>{t(item.labelKey)}</span>
              </NavLink>
            ))}
          </nav>
          <div className="mt-auto border-t border-border px-3 pt-4">
            <p className="truncate text-sm font-medium text-text-primary">{operatorName}</p>
            {email ? <p className="truncate text-xs text-text-muted">{email}</p> : null}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}
