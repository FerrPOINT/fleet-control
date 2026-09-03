import {
  Activity,
  Bot,
  Boxes,
  Cable,
  Crown,
  Files,
  Gauge,
  GitBranch,
  LogOut,
  ScrollText,
  Settings,
  TerminalSquare,
  UserRoundCheck,
} from 'lucide-react'
import { useEffect } from 'react'
import { NavLink, Outlet, useNavigate } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { getCurrentUserPermissions, logout } from '@/api/auth'
import { useAuthStore } from '@/shared/auth/store'
import { ThemeToggle } from '@sdlc/ui/ui'
import { Button } from '@sdlc/ui/ui'
import { cn } from '@/shared/lib/utils'

const navItems = [
  { to: '/', label: 'Dashboard', icon: Gauge, permission: 'agents:manage' },
  { to: '/leaders', label: 'Leaders', icon: Crown, permission: 'leaders:manage' },
  { to: '/executors', label: 'Executors', icon: UserRoundCheck, permission: 'executors:manage' },
  { to: '/agents', label: 'Agents', icon: Bot, permission: 'agents:manage' },
  { to: '/sessions', label: 'Sessions', icon: Activity },
  { to: '/workflows', label: 'Workflows', icon: GitBranch, permission: 'agents:manage' },
  { to: '/deployments', label: 'Deployments', icon: Boxes, permission: 'deployments:manage' },
  { to: '/logs', label: 'Logs', icon: ScrollText, permission: 'logs:read' },
  { to: '/settings', label: 'Settings', icon: Settings, permission: 'settings:manage' },
]

export function AppShell() {
  const navigate = useNavigate()
  const displayName = useAuthStore((state) => state.displayName)
  const email = useAuthStore((state) => state.email)
  const systemRole = useAuthStore((state) => state.systemRole)
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

  async function handleLogout() {
    await logout().catch(() => undefined)
    clearAuth()
    navigate('/login')
  }

  return (
    <div className="min-h-screen bg-background text-text-primary">
      <aside className="fixed inset-y-0 left-0 hidden w-64 border-r border-border bg-surface px-3 py-4 lg:block">
        <div className="mb-6 flex items-center gap-3 px-2">
          <div className="flex h-10 w-10 items-center justify-center rounded-md bg-accent text-white">
            <Cable className="h-5 w-5" />
          </div>
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
              <ThemeToggle />
              <Button variant="outline" size="sm" onClick={handleLogout}>
                <LogOut className="h-4 w-4" />
                Sign out
              </Button>
            </div>
          </div>
          <nav className="flex max-w-full gap-1 overflow-x-auto border-t border-border px-2 py-2 lg:hidden">
            {visibleNavItems.map((item) => (
              <NavLink
                key={item.to}
                to={item.to}
                end={item.to === '/'}
                className={({ isActive }) =>
                  cn(
                    'flex h-9 shrink-0 items-center gap-2 rounded-md px-3 text-xs text-text-secondary',
                    isActive && 'bg-surface-raised text-text-primary',
                  )
                }
              >
                <item.icon className="h-4 w-4" />
                {item.label}
              </NavLink>
            ))}
          </nav>
        </header>

        <main className="min-w-0 px-4 py-5 lg:px-6">
          <Outlet />
        </main>

        <footer className="border-t border-border px-4 py-3 text-xs text-text-muted lg:px-6">
          <div className="flex flex-wrap items-center gap-3">
            <span>{displayName ?? email ?? 'Fleet operator'}</span>
            <span className="capitalize">{systemRole}</span>
            <span className="hidden sm:inline">Runtime root: guarded per-agent workspaces</span>
            <TerminalSquare className="h-3.5 w-3.5" />
            <Files className="h-3.5 w-3.5" />
          </div>
        </footer>
      </div>
    </div>
  )
}
