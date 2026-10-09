import {
  Activity,
  Bot,
  Boxes,
  Gauge,
  GitBranch,
  ScrollText,
  Settings,
  TriangleAlert,
} from 'lucide-react'
import { useEffect } from 'react'
import { toast } from 'sonner'
import { Outlet, useLocation } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { getCurrentUserPermissions } from '@/api/auth'
import { endSso } from '@sdlc/ui/sso'
import { isCurrentAuth, ssoConfig, useAuthStore } from '@/shared/auth/store'
import { AuthContextChangedError } from '@/api/client'
import { AppShell as BaseAppShell } from '@sdlc/ui/ui'

const navItems = [
  { to: '/', labelKey: 'navigation.dashboard', icon: Gauge, permission: 'agents:manage' },
  { to: '/agents', labelKey: 'navigation.agents', icon: Bot, permission: 'agents:read_directory' },
  { to: '/chats', labelKey: 'chats.title', icon: Activity },
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
  const { pathname } = useLocation()
  const displayName = useAuthStore((state) => state.displayName)
  const email = useAuthStore((state) => state.email)
  const permissions = useAuthStore((state) => state.permissions)
  const authVersion = useAuthStore((state) => state.authVersion)
  const token = useAuthStore((state) => state.token)
  const signingOut = useAuthStore((state) => state.signingOut)
  const setUser = useAuthStore((state) => state.setUser)
  const accountName = displayName?.trim()
  const accountEmail = email?.trim()
  const permissionsQuery = useQuery({
    queryKey: ['me', 'permissions', authVersion],
    queryFn: async () => {
      const scope = useAuthStore.getState()
      const result = await getCurrentUserPermissions()
      if (!isCurrentAuth(scope) || result.user_id !== scope.userId)
        throw new AuthContextChangedError()
      return result
    },
    enabled: Boolean(token) && !signingOut,
    staleTime: 60_000,
  })
  useEffect(() => {
    if (
      !permissionsQuery.data ||
      signingOut ||
      !token ||
      permissionsQuery.data.user_id !== useAuthStore.getState().userId
    )
      return
    setUser({
      userId: permissionsQuery.data.user_id,
      systemRole: permissionsQuery.data.role,
      isSystemAdmin: permissionsQuery.data.is_system_admin,
      permissions: permissionsQuery.data.permissions,
    })
  }, [permissionsQuery.data, setUser, signingOut, token])
  const navigation = navItems
    .filter((item) => !item.permission || permissions.includes(item.permission))
    .map((item) => ({
      to: item.to,
      icon: item.icon,
      label: t(item.labelKey),
      active: item.to === '/chats' && pathname.startsWith('/sessions') ? true : undefined,
    }))
  const layout =
    pathname === '/settings' ||
    /^\/(agents|executors|leaders)\/new$/.test(pathname) ||
    /^\/(agents|executors|leaders)\/[^/]+\/edit$/.test(pathname)
      ? 'reading'
      : /^\/(agents|executors|leaders)\/[^/]+(\/[^/]+)?$/.test(pathname) ||
          /^\/(sessions|chats)\/[^/]+$/.test(pathname)
        ? 'detail-with-aside'
        : 'wide'
  function logout() {
    const original = useAuthStore.getState()
    if (original.signingOut) return
    useAuthStore.getState().startSignOut()
    const complete = () => {
      const current = useAuthStore.getState()
      if (
        current.signingOut &&
        current.authVersion === original.authVersion &&
        current.token === original.token &&
        current.userId === original.userId
      )
        current.logout()
    }
    window.addEventListener('pagehide', complete, { once: true })
    try {
      endSso(ssoConfig)
    } catch {
      window.removeEventListener('pagehide', complete)
      const current = useAuthStore.getState()
      if (
        current.authVersion === original.authVersion &&
        current.token === original.token &&
        current.userId === original.userId
      )
        useAuthStore.setState({ signingOut: false })
      toast.error('Не удалось начать выход. Повторите попытку.')
    }
  }
  return (
    <BaseAppShell
      currentServiceKey="fleet-control"
      title={t('app.name')}
      description={t('app.subtitle')}
      navigation={navigation}
      layout={layout}
      account={{
        label: accountName || accountEmail || t('app.operator'),
        secondary:
          accountName && accountEmail && accountName !== accountEmail ? accountEmail : undefined,
        onLogout: logout,
      }}
    >
      <Outlet />
    </BaseAppShell>
  )
}
