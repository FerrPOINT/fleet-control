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
import { Link, NavLink, Outlet, useLocation } from 'react-router'
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
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  PageFrame,
  PlatformHeader,
  PlatformMark,
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
  const location = useLocation()
  const displayName = useAuthStore((state) => state.displayName)
  const email = useAuthStore((state) => state.email)
  const permissions = useAuthStore((state) => state.permissions)
  const setUser = useAuthStore((state) => state.setUser)
  const visibleNavItems = navItems.filter(
    (item) => !item.permission || permissions.includes(item.permission),
  )
  const permissionsQuery = useQuery({
    queryKey: ['me', 'permissions'],
    queryFn: getCurrentUserPermissions,
    staleTime: 60_000,
  })
  const operatorName = displayName?.trim() || email || t('app.operator')
  const pageLayout =
    location.pathname === '/settings' ||
    /\/(agents|leaders|executors)\/(new|[^/]+\/edit)$/.test(location.pathname)
      ? 'reading'
      : /^\/(agents|leaders|executors|sessions)\/[^/]+(?:\/.*)?$/.test(location.pathname)
        ? 'detail-with-aside'
        : 'wide'

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
    endSso(ssoConfig)
  }

  return (
    <div className="min-h-screen bg-background text-text-primary">
      <PlatformHeader
        currentServiceKey="fleet-control"
        leading={
          <>
            <MobileNavigation items={visibleNavItems} />
            <Link
              to="/"
              aria-label={t('app.name')}
              className="hidden h-11 min-w-11 items-center justify-center rounded-md focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus min-[360px]:flex md:h-10 md:min-w-10"
            >
              <PlatformMark size="sm" withName={false} />
            </Link>
          </>
        }
        actions={
          <>
            <ThemeToggle />
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-11 w-11 md:h-10 md:w-10"
                  aria-label={t('shell.account')}
                >
                  <UserRound className="h-5 w-5" aria-hidden />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-64 max-w-[calc(100vw-2rem)]">
                <div className="break-words px-2 py-1.5 text-sm font-medium text-text-primary">
                  {operatorName}
                </div>
                {email && email !== operatorName && (
                  <div className="break-words px-2 pb-2 text-xs text-text-muted">{email}</div>
                )}
                <DropdownMenuItem
                  onSelect={handleLogout}
                  className="min-h-11 gap-2 text-text-secondary md:min-h-10"
                >
                  <LogOut className="h-4 w-4" aria-hidden />
                  <span>{t('app.signOut')}</span>
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </>
        }
      />
      <aside className="fixed bottom-0 left-0 top-[var(--shell-header-height)] z-20 hidden w-[var(--shell-sidebar-compact)] flex-col border-r border-border bg-surface md:flex xl:w-[var(--shell-sidebar-expanded)]">
        <div className="min-h-0 flex-1 overflow-y-auto p-2 xl:p-3">
          <ShellNavigation items={visibleNavItems} compact />
        </div>
      </aside>
      <div className="min-w-0 md:pl-[var(--shell-sidebar-compact)] xl:pl-[var(--shell-sidebar-expanded)]">
        <main className="shell-main min-h-[calc(100dvh-var(--shell-header-height))]">
          <PageFrame mode={pageLayout}>
            <Outlet />
          </PageFrame>
        </main>
      </div>
    </div>
  )
}

function ShellNavigation({
  items,
  compact = false,
  onNavigate,
}: {
  items: NavigationItem[]
  compact?: boolean
  onNavigate?: () => void
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
            onClick={onNavigate}
            aria-label={compact ? label : undefined}
            title={compact ? label : undefined}
            className={({ isActive }) =>
              cn(
                'flex min-h-11 items-center gap-3 rounded-md px-3 text-sm text-text-secondary transition-colors hover:bg-surface-raised hover:text-text-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus md:min-h-10',
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

function MobileNavigation({ items }: { items: NavigationItem[] }) {
  const { t } = useTranslation()
  const location = useLocation()
  const [open, setOpen] = useState(false)

  useEffect(() => {
    setOpen(false)
  }, [location.pathname])

  useEffect(() => {
    const desktop = window.matchMedia('(min-width: 768px)')
    const closeOnDesktop = () => {
      if (desktop.matches) setOpen(false)
    }
    desktop.addEventListener('change', closeOnDesktop)
    return () => desktop.removeEventListener('change', closeOnDesktop)
  }, [])

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="h-11 w-11 md:hidden"
          aria-label={t('shell.openNavigation')}
          title={t('shell.openNavigation')}
        >
          <Menu className="h-5 w-5" aria-hidden />
        </Button>
      </DialogTrigger>
      <DialogContent
        aria-describedby={undefined}
        className="!left-0 !top-0 !flex !h-dvh !max-h-dvh !w-[min(320px,calc(100%-2rem))] !max-w-none !translate-x-0 !translate-y-0 !flex-col !gap-0 !rounded-none !border-y-0 !border-l-0 !p-0 [&>button]:h-11 [&>button]:w-11"
      >
        <DialogHeader className="flex h-[var(--shell-header-height)] flex-row items-center gap-3 border-b border-border px-4 pr-14 text-left">
          <PlatformMark withName={false} />
          <div className="min-w-0">
            <DialogTitle className="truncate text-base">{t('app.name')}</DialogTitle>
            <p className="truncate text-xs text-text-muted">{t('app.subtitle')}</p>
          </div>
        </DialogHeader>
        <div className="min-h-0 flex-1 overflow-y-auto px-3 py-4">
          <ShellNavigation items={items} onNavigate={() => setOpen(false)} />
        </div>
      </DialogContent>
    </Dialog>
  )
}
