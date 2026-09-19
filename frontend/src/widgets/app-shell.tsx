import {
  Activity,
  Bot,
  Boxes,
  Crown,
  Files,
  Gauge,
  GitBranch,
  LogOut,
  ScrollText,
  Settings,
  TerminalSquare,
  TriangleAlert,
  UserRoundCheck,
} from 'lucide-react'
import { useEffect } from 'react'
import { NavLink, Outlet, useLocation, useNavigate } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { getCurrentUserPermissions } from '@/api/auth'
import { endSso } from '@sdlc/ui/sso'
import { ssoConfig, useAuthStore } from '@/shared/auth/store'
import { ThemeToggle } from '@sdlc/ui/ui'
import { ServiceSwitcher } from '@sdlc/ui/ui'
import { Button, PlatformMark } from '@sdlc/ui/ui'
import { cn } from '@/shared/lib/utils'

const navItems = [
  { to: '/', label: 'Dashboard', icon: Gauge, permission: 'agents:manage' },
  { to: '/leaders', label: 'Leaders', icon: Crown, permission: 'leaders:manage' },
  { to: '/executors', label: 'Executors', icon: UserRoundCheck, permission: 'executors:manage' },
  { to: '/agents', label: 'Agents', icon: Bot, permission: 'agents:manage' },
  { to: '/sessions', label: 'Sessions', icon: Activity },
  { to: '/workflows', label: 'Workflows', icon: GitBranch, permission: 'agents:manage' },
  { to: '/deployments', label: 'Deployments', icon: Boxes, permission: 'deployments:manage' },
  { to: '/alerts', label: 'Alerts', icon: TriangleAlert, permission: 'logs:read' },
  { to: '/logs', label: 'Logs', icon: ScrollText, permission: 'logs:read' },
  { to: '/settings', label: 'Settings', icon: Settings, permission: 'settings:manage' },
]

export function AppShell() {
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
            <p className="text-xs text-text-muted">Agent fleet plane</p>
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
              {item.label}
            </NavLink>
          ))}
        </nav>
      </aside>

      <div className="min-w-0 overflow-x-hidden lg:pl-64">
        <header className="sticky top-0 z-20 border-b border-border bg-background/95 backdrop-blur">
          <div className="flex min-h-16 flex-wrap items-center justify-between gap-3 px-4 py-3 lg:px-6">
            <div className="min-w-0">
              <p className="text-sm font-semibold text-text-primary">Fleet Control</p>
              <p className="truncate text-xs text-text-muted">
                Hermes now, Java Agent contract next
              </p>
            </div>
            <div className="flex items-center gap-2">
              <ServiceSwitcher currentKey="fleet-control" />
          <ThemeToggle />
              <Button variant="outline" size="sm" onClick={handleLogout}>
                <LogOut className="h-4 w-4" />
                Sign out
              </Button>
            </div>
          </div>
          <nav className="border-t border-border px-4 py-2 lg:hidden" aria-label="Fleet sections">
            <select
              aria-label="Fleet section"
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
                  {item.label}
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
            <span>{displayName ?? email ?? 'Fleet operator'}</span>
            <span className="hidden sm:inline">Runtime root: guarded per-agent workspaces</span>
            <TerminalSquare className="h-3.5 w-3.5" />
            <Files className="h-3.5 w-3.5" />
          </div>
        </footer>
      </div>
    </div>
  )
}
