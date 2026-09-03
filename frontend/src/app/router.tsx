import { createBrowserRouter, Navigate } from 'react-router'
import { RequireAuth } from '@/shared/auth/require-auth'
import { AppShell } from '@/widgets/app-shell'
import { DashboardPage } from '@/pages/dashboard'
import { AgentsPage } from '@/pages/agents'
import { AgentDetailPage } from '@/pages/agent-detail'
import { AgentEditPage } from '@/pages/agent-edit'
import { ExecutorsPage } from '@/pages/executors'
import { LeaderDetailPage, LeadersPage } from '@/pages/leaders'
import { SessionsPage } from '@/pages/sessions'
import { SessionDetailPage } from '@/pages/session-detail'
import { WorkflowsPage } from '@/pages/workflows'
import { DeploymentsPage } from '@/pages/deployments'
import { LogsPage } from '@/pages/logs'
import { SettingsPage } from '@/pages/settings'
import { LoginPage } from '@/pages/login'
import { RegisterPage } from '@/pages/register'
import { AccessDeniedState, NotFoundState } from '@/pages/common'
import { useAuthStore } from '@/shared/auth/store'

function PermissionGate({
  permission,
  children,
}: {
  permission: string
  children: React.ReactNode
}) {
  const permissions = useAuthStore((state) => state.permissions)
  if (!permissions.includes(permission)) return <AccessDeniedState />
  return children
}

function NotFoundPage() {
  return <NotFoundState />
}

function HomePage() {
  const permissions = useAuthStore((state) => state.permissions)
  if (!permissions.includes('agents:manage')) return <Navigate to="/sessions" replace />
  return <DashboardPage />
}

export const router = createBrowserRouter([
  {
    element: <RequireAuth />,
    children: [
      { path: '/login', element: <LoginPage /> },
      { path: '/register', element: <RegisterPage /> },
      {
        element: <AppShell />,
        children: [
          { index: true, element: <HomePage /> },
          { path: '/dashboard', element: <Navigate to="/" replace /> },
          {
            path: '/agents',
            element: (
              <PermissionGate permission="agents:manage">
                <AgentsPage />
              </PermissionGate>
            ),
          },
          {
            path: '/agents/new',
            element: (
              <PermissionGate permission="agents:manage">
                <AgentsPage createMode />
              </PermissionGate>
            ),
          },
          {
            path: '/agents/:agentId/edit',
            element: (
              <PermissionGate permission="agents:manage">
                <AgentEditPage />
              </PermissionGate>
            ),
          },
          {
            path: '/agents/:agentId',
            element: (
              <PermissionGate permission="agents:manage">
                <AgentDetailPage tab="overview" />
              </PermissionGate>
            ),
          },
          {
            path: '/agents/:agentId/runtime',
            element: (
              <PermissionGate permission="agents:manage">
                <AgentDetailPage tab="runtime" />
              </PermissionGate>
            ),
          },
          {
            path: '/agents/:agentId/skills',
            element: (
              <PermissionGate permission="agents:manage">
                <AgentDetailPage tab="skills" />
              </PermissionGate>
            ),
          },
          {
            path: '/agents/:agentId/config',
            element: (
              <PermissionGate permission="agents:manage">
                <AgentDetailPage tab="config" />
              </PermissionGate>
            ),
          },
          {
            path: '/agents/:agentId/workspace',
            element: (
              <PermissionGate permission="agents:manage">
                <AgentDetailPage tab="workspace" />
              </PermissionGate>
            ),
          },
          {
            path: '/agents/:agentId/sessions',
            element: (
              <PermissionGate permission="agents:manage">
                <AgentDetailPage tab="sessions" />
              </PermissionGate>
            ),
          },
          {
            path: '/leaders',
            element: (
              <PermissionGate permission="leaders:manage">
                <LeadersPage />
              </PermissionGate>
            ),
          },
          {
            path: '/leaders/new',
            element: (
              <PermissionGate permission="leaders:manage">
                <AgentsPage createMode defaultProductRole="leader" />
              </PermissionGate>
            ),
          },
          {
            path: '/leaders/:leaderId/edit',
            element: (
              <PermissionGate permission="leaders:manage">
                <AgentEditPage defaultProductRole="leader" />
              </PermissionGate>
            ),
          },
          {
            path: '/leaders/:leaderId',
            element: (
              <PermissionGate permission="leaders:manage">
                <LeaderDetailPage />
              </PermissionGate>
            ),
          },
          {
            path: '/executors',
            element: (
              <PermissionGate permission="executors:manage">
                <ExecutorsPage />
              </PermissionGate>
            ),
          },
          {
            path: '/executors/new',
            element: (
              <PermissionGate permission="executors:manage">
                <AgentsPage createMode defaultProductRole="executor" />
              </PermissionGate>
            ),
          },
          {
            path: '/executors/:agentId/edit',
            element: (
              <PermissionGate permission="executors:manage">
                <AgentEditPage defaultProductRole="executor" />
              </PermissionGate>
            ),
          },
          {
            path: '/executors/:agentId',
            element: (
              <PermissionGate permission="executors:manage">
                <AgentDetailPage tab="overview" />
              </PermissionGate>
            ),
          },
          {
            path: '/executors/:agentId/runtime',
            element: (
              <PermissionGate permission="executors:manage">
                <AgentDetailPage tab="runtime" />
              </PermissionGate>
            ),
          },
          {
            path: '/executors/:agentId/skills',
            element: (
              <PermissionGate permission="executors:manage">
                <AgentDetailPage tab="skills" />
              </PermissionGate>
            ),
          },
          {
            path: '/executors/:agentId/config',
            element: (
              <PermissionGate permission="executors:manage">
                <AgentDetailPage tab="config" />
              </PermissionGate>
            ),
          },
          {
            path: '/executors/:agentId/workspace',
            element: (
              <PermissionGate permission="executors:manage">
                <AgentDetailPage tab="workspace" />
              </PermissionGate>
            ),
          },
          {
            path: '/executors/:agentId/sessions',
            element: (
              <PermissionGate permission="executors:manage">
                <AgentDetailPage tab="sessions" />
              </PermissionGate>
            ),
          },
          { path: '/sessions', element: <SessionsPage /> },
          { path: '/sessions/:sessionId', element: <SessionDetailPage /> },
          {
            path: '/workflows',
            element: (
              <PermissionGate permission="agents:manage">
                <WorkflowsPage />
              </PermissionGate>
            ),
          },
          {
            path: '/deployments',
            element: (
              <PermissionGate permission="deployments:manage">
                <DeploymentsPage />
              </PermissionGate>
            ),
          },
          {
            path: '/logs',
            element: (
              <PermissionGate permission="logs:read">
                <LogsPage />
              </PermissionGate>
            ),
          },
          {
            path: '/settings',
            element: (
              <PermissionGate permission="settings:manage">
                <SettingsPage />
              </PermissionGate>
            ),
          },
          { path: '/access-denied', element: <AccessDeniedState /> },
          { path: '*', element: <NotFoundPage /> },
        ],
      },
    ],
  },
])
