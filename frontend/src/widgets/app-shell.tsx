import {
  Activity,
  Bot,
  Boxes,
  Crown,
  Gauge,
  GitBranch,
  LogOut,
  ScrollText,
  Settings,
  TriangleAlert,
  UserRoundCheck,
} from 'lucide-react'
import { useEffect } from 'react'
import { NavLink, Outlet, useLocation, useNavigate } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { getCurrentUserPermissions } from '@/api/auth'
import { endSso } from '@sdlc/ui/sso'
import { ssoConfig, useAuthStore } from '@/shared/auth/store'
import { ThemeToggle } from '@sdlc/ui/ui'
import { ServiceSwitcher } from '@sdlc/ui/ui'
import { Button, PlatformMark } from '@sdlc/ui/ui'
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

export function AppShell() {
  const { t } = useTranslation()
  const location = useLocation()
  const navigate = useNavigate()
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
      <aside className="fixed inset-y-0 left-0 hidden w-64 border-r border-border bg-surface px-3 py-4 lg:block">
        <div className="mb-6 flex items-center gap-3 px-2">
          <PlatformMark />
          <div>
            <p className="text-sm font-semibold text-text-primary">Fleet Control</p>
            <p className="text-xs text-text-muted">{t('app.subtitle')}</p>
          </div>
        </div>
        <nav className="space-y-1">
          {visibleNavItems.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              end={item.to === '/'}
              className={({ isActive }) =>
                cn(
                  'flex h-10 items-center gap-3 rounded-md px-3 text-sm text-text-secondary transition-colors hover:bg-surface-raised hover:text-text-primary',
                  isActive && 'bg-surface-raised text-text-primary',
                )
              }
            >
              <item.icon className="h-4 w-4" />
              {t(item.labelKey)}
            </NavLink>
          ))}
        </nav>
      </aside>

      <div className="min-w-0 overflow-x-hidden lg:pl-64">
        <header className="sticky top-0 z-20 border-b border-border bg-background/95 backdrop-blur">
          <div className="flex min-h-16 flex-wrap items-center justify-between gap-3 px-4 py-3 lg:justify-end lg:px-6">
            <div className="min-w-0 lg:hidden">
              <p className="text-sm font-semibold text-text-primary">Fleet Control</p>
              <p className="truncate text-xs text-text-muted">{t('app.subtitle')}</p>
            </div>
            <div className="flex items-center gap-2">
              <ServiceSwitcher currentKey="fleet-control" />
              <ThemeToggle />
              <Button variant="outline" size="sm" onClick={handleLogout}>
                <LogOut className="h-4 w-4" />
                {t('app.signOut')}
              </Button>
            </div>
          </div>
          <nav
            className="border-t border-border px-4 py-2 lg:hidden"
            aria-label={t('navigation.sections')}
          >
            <select
              aria-label={t('navigation.section')}
              className="h-10 w-full rounded-md border border-border-strong bg-surface px-3 text-sm text-text-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
              value={
                visibleNavItems.find(
                  (item) =>
                    item.to !== '/' &&
                    (location.pathname === item.to || location.pathname.startsWith(`${item.to}/`)),
                )?.to ?? visibleNavItems[0]?.to
              }
              onChange={(event) => navigate(event.target.value)}
            >
              {visibleNavItems.map((item) => (
                <option key={item.to} value={item.to}>
                  {t(item.labelKey)}
                </option>
              ))}
            </select>
          </nav>
        </header>

        <main className="min-w-0 px-4 py-5 lg:px-6">
          <Outlet />
        </main>

        <footer className="border-t border-border px-4 py-3 text-xs text-text-muted lg:px-6">
          <div className="flex flex-wrap items-center gap-3">
            <span>{displayName ?? email ?? t('app.operator')}</span>
          </div>
        </footer>
      </div>
    </div>
  )
}
