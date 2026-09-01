import { createBrowserRouter, Navigate } from 'react-router'
import { RequireAuth } from '@/shared/auth/require-auth'
import { AppShell } from '@/widgets/app-shell'
import { DashboardPage } from '@/pages/dashboard'
import { AgentsPage } from '@/pages/agents'
import { AgentDetailPage } from '@/pages/agent-detail'
import { SessionsPage } from '@/pages/sessions'
import { SessionDetailPage } from '@/pages/session-detail'
import { WorkflowsPage } from '@/pages/workflows'
import { DeploymentsPage } from '@/pages/deployments'
import { LogsPage } from '@/pages/logs'
import { SettingsPage } from '@/pages/settings'
import { LoginPage } from '@/pages/login'
import { RegisterPage } from '@/pages/register'

export const router = createBrowserRouter([
  {
    element: <RequireAuth />,
    children: [
      { path: '/login', element: <LoginPage /> },
      { path: '/register', element: <RegisterPage /> },
      {
        element: <AppShell />,
        children: [
          { index: true, element: <DashboardPage /> },
          { path: '/dashboard', element: <Navigate to="/" replace /> },
          { path: '/agents', element: <AgentsPage /> },
          { path: '/agents/new', element: <AgentsPage createMode /> },
          { path: '/agents/:agentId', element: <AgentDetailPage tab="overview" /> },
          { path: '/agents/:agentId/runtime', element: <AgentDetailPage tab="runtime" /> },
          { path: '/agents/:agentId/skills', element: <AgentDetailPage tab="skills" /> },
          { path: '/agents/:agentId/config', element: <AgentDetailPage tab="config" /> },
          { path: '/agents/:agentId/workspace', element: <AgentDetailPage tab="workspace" /> },
          { path: '/agents/:agentId/sessions', element: <AgentDetailPage tab="sessions" /> },
          { path: '/sessions', element: <SessionsPage /> },
          { path: '/sessions/:sessionId', element: <SessionDetailPage /> },
          { path: '/workflows', element: <WorkflowsPage /> },
          { path: '/deployments', element: <DeploymentsPage /> },
          { path: '/logs', element: <LogsPage /> },
          { path: '/settings', element: <SettingsPage /> },
        ],
      },
    ],
  },
])
