import { chromium } from '@playwright/test'
import { mkdirSync, readdirSync, rmSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const repoRoot = path.resolve(__dirname, '../..')
const outputRoot = path.join(repoRoot, 'docs/assets/screens')
const baseUrl = process.env.SCREENSHOT_BASE_URL ?? 'http://127.0.0.1:4173'
const now = '2026-09-01T10:00:00+03:00'

const ids = {
  admin: '00000000-0000-4000-8000-000000000001',
  dev: '00000000-0000-4000-8000-000000000101',
  tester: '00000000-0000-4000-8000-000000000102',
  lead: '00000000-0000-4000-8000-000000000103',
  sessionDev: '00000000-0000-4000-8000-000000000201',
  sessionQa: '00000000-0000-4000-8000-000000000202',
  skillDev: '00000000-0000-4000-8000-000000000301',
  skillWorkflow: '00000000-0000-4000-8000-000000000302',
  skillGh: '00000000-0000-4000-8000-000000000303',
  bindingDev: '00000000-0000-4000-8000-000000000401',
  bindingQa: '00000000-0000-4000-8000-000000000402',
}

function agent({
  id,
  ordinal,
  role,
  status,
  displayName,
  namespaceId,
  workflowId,
  dashboardPort,
  apiPort,
  healthStatus,
  productRole = 'executor',
}) {
  const name = `agent${ordinal}`
  const base = `C:\\fleet-control\\agents\\${name}`
  return {
    id,
    ordinal,
    name,
    kind: 'hermes',
    product_role: productRole,
    role,
    status,
    display_name: displayName,
    description:
      productRole === 'leader'
        ? 'Team lead Hermes coordinating managed executor sessions.'
        : role === 'tester'
          ? 'Isolated Hermes tester with QA namespace and test workflow.'
          : 'Isolated Hermes developer with personal workflow and skills.',
    namespace_id: namespaceId,
    workflow_id: workflowId,
    runtime_version: 'hermes-main@local',
    dashboard_port: dashboardPort,
    api_port: apiPort,
    paths: {
      runtime: `${base}\\runtime`,
      config: `${base}\\config`,
      workspace: `${base}\\workspace`,
      logs: `${base}\\logs`,
    },
    runtime: {
      desired_state: status === 'running' ? 'running' : 'stopped',
      pid: status === 'running' ? 23801 + ordinal : null,
      health_status: healthStatus,
      health_detail:
        status === 'running'
          ? 'Hermes serve process is tracked by Fleet Control.'
          : 'Process is provisioned and ready to start.',
      command_preview: `hermes serve --host 127.0.0.1 --port ${apiPort}`,
      env_preview: {
        HERMES_HOME: `${base}\\config`,
        HERMES_SERVE_HEADLESS: '1',
        cwd: `${base}\\workspace`,
        secrets: 'redacted',
      },
      started_at: status === 'running' ? now : null,
      stopped_at: status === 'running' ? null : now,
      last_health_at: now,
    },
    created_at: now,
    updated_at: now,
  }
}

function agentDirectoryItem(item) {
  return {
    id: item.id,
    ordinal: item.ordinal,
    name: item.name,
    kind: item.kind,
    product_role: item.product_role,
    role: item.role,
    status: item.status,
    display_name: item.display_name,
    description: item.description,
    namespace_id: item.namespace_id,
    workflow_id: item.workflow_id,
    runtime_version: item.runtime_version,
    dashboard_port: item.dashboard_port,
    api_port: item.api_port,
  }
}

const agents = [
  agent({
    id: ids.dev,
    ordinal: 1,
    role: 'developer',
    status: 'running',
    displayName: 'Developer Hermes',
    namespaceId: 'dev',
    workflowId: 'workflow-dev',
    dashboardPort: 29002,
    apiPort: 29001,
    healthStatus: 'running',
  }),
  agent({
    id: ids.tester,
    ordinal: 2,
    role: 'tester',
    status: 'stopped',
    displayName: 'Tester Hermes',
    namespaceId: 'qa',
    workflowId: 'workflow-qa',
    dashboardPort: 29012,
    apiPort: 29011,
    healthStatus: 'ready',
  }),
  agent({
    id: ids.lead,
    ordinal: 3,
    role: 'it_lead',
    status: 'running',
    displayName: 'IT Lead Hermes',
    namespaceId: 'lead',
    workflowId: 'workflow-lead',
    dashboardPort: 29022,
    apiPort: 29021,
    healthStatus: 'running',
    productRole: 'leader',
  }),
]

const leaderExecutors = {
  [ids.lead]: [ids.dev, ids.tester],
}

const runtimeTemplates = [
  {
    kind: 'hermes',
    display_name: 'Hermes',
    implemented: true,
    enabled: true,
    description: 'Local Hermes runtime with HERMES_HOME isolation and workspace cwd.',
    capabilities: {
      provision: true,
      start: true,
      stop: true,
      restart: true,
      sessions: true,
      chat: 'prompt.submit',
      skills: true,
      config: true,
    },
  },
  {
    kind: 'java_agent',
    display_name: 'Java Agent',
    implemented: false,
    enabled: true,
    description: 'Spring Boot adapter contract reserved for AGENT_SERVER_PORT and actuator health.',
    capabilities: {
      provision: false,
      start: false,
      contract: [
        'AGENT_SERVER_PORT',
        'SPRING_CONFIG_ADDITIONAL_LOCATION',
        '/actuator/health',
        '/api/v1/agent/chat/stream',
        '/api/v2/sessions',
        '/v1/capabilities',
      ],
    },
  },
]

const skills = [
  {
    id: ids.skillDev,
    agent_id: ids.dev,
    name: 'development',
    title: 'Development',
    state: 'enabled',
    source: 'C:\\Users\\ferru\\.codex\\skills\\development',
    content: '# Development\n\nPrimary workflow rules for coding tasks.',
    updated_at: now,
  },
  {
    id: ids.skillWorkflow,
    agent_id: ids.dev,
    name: 'project-workflow',
    title: 'Project Workflow',
    state: 'enabled',
    source: 'project-workflow',
    content: '# Project Workflow\n\nNamespace-aware task orchestration.',
    updated_at: now,
  },
  {
    id: ids.skillGh,
    agent_id: ids.dev,
    name: 'gh-commit-pr',
    title: 'GitHub Commit and PR',
    state: 'dirty',
    source: 'C:\\Users\\ferru\\.codex\\skills\\gh-commit-pr',
    content: '# GitHub Commit and PR\n\nPer-agent local adjustment draft.',
    updated_at: now,
  },
]

const testerSkills = [
  {
    ...skills[0],
    id: '00000000-0000-4000-8000-000000000304',
    agent_id: ids.tester,
    name: 'audit-web-system',
    title: 'Web System Audit',
  },
  {
    ...skills[1],
    id: '00000000-0000-4000-8000-000000000305',
    agent_id: ids.tester,
  },
]

const leaderSkills = [
  {
    ...skills[1],
    id: '00000000-0000-4000-8000-000000000306',
    agent_id: ids.lead,
  },
  {
    ...skills[0],
    id: '00000000-0000-4000-8000-000000000307',
    agent_id: ids.lead,
  },
]

const user = {
  id: ids.admin,
  email: 'admin@fleet-control.local',
  username: 'admin',
  display_name: 'Fleet Admin',
  system_role: 'admin',
  is_system_admin: true,
  is_active: true,
}

const qaUser = {
  id: '00000000-0000-4000-8000-000000000002',
  email: 'qa@fleet-control.local',
  username: 'qa',
  display_name: 'QA Reviewer',
  system_role: 'user',
  is_system_admin: false,
  is_active: true,
}

const sessions = [
  {
    id: ids.sessionDev,
    agent_id: ids.dev,
    primary_agent_id: ids.dev,
    agent_name: 'agent1',
    primary_agent_name: 'agent1',
    user_id: user.id,
    user_email: user.email,
    user_username: user.username,
    user_display_name: user.display_name,
    leader_agent_id: null,
    leader_agent_name: null,
    parent_session_id: null,
    created_by_leader_agent_id: null,
    visibility: 'private',
    title: 'Implement Fleet Control runtime isolation',
    task_key: 'FC-001',
    state: 'active',
    namespace_id: 'dev',
    external_session_id: 'hermes-dev-42',
    last_message_preview: 'Provisioning completed, runtime status is healthy.',
    created_at: now,
    updated_at: now,
  },
  {
    id: ids.sessionQa,
    agent_id: ids.tester,
    primary_agent_id: ids.tester,
    agent_name: 'agent2',
    primary_agent_name: 'agent2',
    user_id: qaUser.id,
    user_email: qaUser.email,
    user_username: qaUser.username,
    user_display_name: qaUser.display_name,
    leader_agent_id: ids.lead,
    leader_agent_name: 'agent3',
    parent_session_id: null,
    created_by_leader_agent_id: null,
    visibility: 'leader_scoped',
    title: 'Verify Hermes handoff workflow',
    task_key: 'FC-QA-007',
    state: 'handoff_requested',
    namespace_id: 'qa',
    external_session_id: 'hermes-qa-11',
    last_message_preview: 'Waiting for Tester Hermes to accept handoff.',
    created_at: now,
    updated_at: now,
  },
]

const sessionMessages = {
  [ids.sessionDev]: [
    {
      id: '00000000-0000-4000-8000-000000000701',
      session_id: ids.sessionDev,
      author_type: 'system',
      author_user_id: null,
      author_agent_id: null,
      author_display_name: 'Fleet Control',
      body: 'Session created in Fleet Control',
      message_kind: 'system_event',
      runtime_message_id: null,
      replayed: false,
      created_at: now,
    },
  ],
  [ids.sessionQa]: [
    {
      id: '00000000-0000-4000-8000-000000000702',
      session_id: ids.sessionQa,
      author_type: 'agent',
      author_user_id: null,
      author_agent_id: ids.lead,
      author_display_name: 'IT Lead Hermes',
      body: 'Please verify the Hermes handoff workflow and report blockers.',
      message_kind: 'assistant_message',
      runtime_message_id: 'hermes-lead-message-1',
      replayed: false,
      created_at: now,
    },
  ],
}

const deploymentJobs = [
  {
    id: '00000000-0000-4000-8000-000000000901',
    job_kind: 'provision',
    state: 'queued',
    agent_id: ids.dev,
    runtime_kind: 'hermes',
    requested_by_user_id: user.id,
    title: 'Provision Developer Hermes',
    detail: { requested_from: 'screenshot', secret: 'redacted' },
    last_error: null,
    created_at: now,
    updated_at: now,
  },
]

const auditLog = [
  {
    id: '00000000-0000-4000-8000-000000000951',
    actor_user_id: user.id,
    action: 'session.create',
    entity_type: 'session',
    entity_id: ids.sessionDev,
    payload: { title: 'Implement Fleet Control runtime isolation', token: 'redacted' },
    created_at: now,
  },
]

const permissions = [
  'sessions:read_own',
  'sessions:write_own',
  'agents:read_directory',
  'agents:manage',
  'leaders:manage',
  'executors:manage',
  'runtime:manage',
  'config:manage',
  'skills:manage',
  'deployments:manage',
  'logs:read',
  'audit_log:read',
  'settings:manage',
  'sessions:read_all',
  'users:manage',
  'rbac:manage',
]

function participantsForSession(session) {
  const rows = [
    {
      id: `${session.id}-owner`,
      session_id: session.id,
      participant_type: 'user',
      user_id: session.user_id,
      agent_id: null,
      session_role: 'owner',
      display_name: session.user_display_name,
      created_at: now,
    },
    {
      id: `${session.id}-primary`,
      session_id: session.id,
      participant_type: 'agent',
      user_id: null,
      agent_id: session.primary_agent_id,
      session_role: 'primary',
      display_name: session.primary_agent_name,
      created_at: now,
    },
  ]
  if (session.leader_agent_id) {
    rows.push({
      id: `${session.id}-leader`,
      session_id: session.id,
      participant_type: 'agent',
      user_id: null,
      agent_id: session.leader_agent_id,
      session_role: 'leader',
      display_name: session.leader_agent_name,
      created_at: now,
    })
  }
  return rows
}

const sessionRuns = {
  [ids.sessionDev]: [
    {
      id: '00000000-0000-4000-8000-000000000801',
      session_id: ids.sessionDev,
      agent_id: ids.dev,
      agent_name: 'agent1',
      runtime_session_id: 'hermes-dev-42',
      run_role: 'primary',
      state: 'running',
      last_error: null,
      created_at: now,
      updated_at: now,
    },
  ],
  [ids.sessionQa]: [
    {
      id: '00000000-0000-4000-8000-000000000802',
      session_id: ids.sessionQa,
      agent_id: ids.tester,
      agent_name: 'agent2',
      runtime_session_id: 'hermes-qa-11',
      run_role: 'primary',
      state: 'pending',
      last_error: null,
      created_at: now,
      updated_at: now,
    },
    {
      id: '00000000-0000-4000-8000-000000000803',
      session_id: ids.sessionQa,
      agent_id: ids.lead,
      agent_name: 'agent3',
      runtime_session_id: 'hermes-lead-4',
      run_role: 'leader',
      state: 'running',
      last_error: null,
      created_at: now,
      updated_at: now,
    },
  ],
}

const workflowBindings = [
  {
    id: ids.bindingDev,
    agent_id: ids.dev,
    namespace_id: 'dev',
    namespace_name: 'Developer',
    workflow_id: 'workflow-dev',
    workflow_name: 'Developer Workflow',
    binding_status: 'connected',
    created_at: now,
    updated_at: now,
  },
  {
    id: ids.bindingQa,
    agent_id: ids.tester,
    namespace_id: 'qa',
    namespace_name: 'Tester',
    workflow_id: 'workflow-qa',
    workflow_name: 'Tester Workflow',
    binding_status: 'connected',
    created_at: now,
    updated_at: now,
  },
]

const events = [
  {
    id: '00000000-0000-4000-8000-000000000501',
    agent_id: ids.dev,
    event_type: 'runtime_started',
    message: 'agent1 Hermes runtime started',
    payload: { pid: 23802 },
    created_at: now,
  },
  {
    id: '00000000-0000-4000-8000-000000000502',
    agent_id: ids.tester,
    event_type: 'agent_created',
    message: 'agent2 Tester Hermes provisioned',
    payload: { namespace_id: 'qa' },
    created_at: now,
  },
]

const logs = [
  {
    id: '00000000-0000-4000-8000-000000000601',
    agent_id: ids.dev,
    stream: 'system',
    message: 'Hermes start requested',
    created_at: now,
  },
  {
    id: '00000000-0000-4000-8000-000000000602',
    agent_id: ids.dev,
    stream: 'stdout',
    message: 'Hermes serve listening on 127.0.0.1:29001',
    created_at: now,
  },
  {
    id: '00000000-0000-4000-8000-000000000603',
    agent_id: ids.tester,
    stream: 'system',
    message: 'Tester Hermes provisioned and waiting for start command',
    created_at: now,
  },
]

const agentConfig = {
  agent_id: ids.dev,
  config_json: {
    namespace_id: 'dev',
    workflow_id: 'workflow-dev',
    model: { provider: 'openai', name: 'gpt-5.6-sol' },
    paths: { workspace: agents[0].paths.workspace },
  },
  soul_md: '# Developer Hermes\n\nFocus on implementation, review and release flow.',
  env_json: {
    HERMES_HOME: agents[0].paths.config,
    OPENAI_API_KEY: '[REDACTED]',
    GITHUB_TOKEN: '[REDACTED]',
  },
  updated_at: now,
}

function json(route, value, status = 200) {
  return route.fulfill({
    status,
    contentType: 'application/json',
    body: JSON.stringify(value),
  })
}

async function mockApi(context) {
  await context.route('**/api/v1/**', async (route) => {
    const request = route.request()
    const url = new URL(request.url())
    const pathName = url.pathname
    const method = request.method()

    if (pathName === '/api/v1/auth/refresh' || pathName === '/api/v1/auth/login') {
      return json(route, {
        access_token: 'screenshot-token',
        user_id: user.id,
        email: user.email,
        username: user.username,
        display_name: user.display_name,
        system_role: 'admin',
        is_system_admin: true,
      })
    }
    if (pathName === '/api/v1/auth/register') {
      return json(route, {
        access_token: 'screenshot-token',
        user_id: user.id,
        email: user.email,
        username: user.username,
        display_name: user.display_name,
        system_role: 'admin',
        is_system_admin: true,
      })
    }
    if (pathName === '/api/v1/users/me') return json(route, user)
    if (pathName === '/api/v1/users/me/permissions') {
      return json(route, {
        user_id: user.id,
        role: 'admin',
        is_system_admin: true,
        permissions,
      })
    }
    if (pathName === '/api/v1/users') return json(route, { users: [user, qaUser] })
    const userRoleMatch = pathName.match(/^\/api\/v1\/users\/([^/]+)\/role$/)
    if (userRoleMatch) return json(route, userRoleMatch[1] === qaUser.id ? qaUser : user)
    if (pathName === '/api/v1/runtime-templates') return json(route, runtimeTemplates)
    if (pathName === '/api/v1/agent-directory') {
      return json(route, agents.map(agentDirectoryItem))
    }
    if (pathName === '/api/v1/leaders') {
      return json(
        route,
        agents.filter((item) => item.product_role === 'leader'),
      )
    }
    if (pathName === '/api/v1/executors') {
      return json(
        route,
        agents.filter((item) => item.product_role === 'executor'),
      )
    }
    const leaderExecutorsMatch = pathName.match(/^\/api\/v1\/leaders\/([^/]+)\/executors$/)
    if (leaderExecutorsMatch) {
      const leaderId = leaderExecutorsMatch[1]
      return json(
        route,
        (leaderExecutors[leaderId] ?? []).map((executorId) => {
          const executor = agents.find((item) => item.id === executorId)
          return {
            leader_agent_id: leaderId,
            executor_agent_id: executor.id,
            executor_name: executor.name,
            executor_display_name: executor.display_name,
            executor_profile: executor.role,
            namespace_id: executor.namespace_id,
            workflow_id: executor.workflow_id,
            created_by_user_id: user.id,
            created_at: now,
          }
        }),
      )
    }
    if (pathName === '/api/v1/dashboard') {
      return json(route, {
        total_agents: agents.length,
        leader_agents: agents.filter((item) => item.product_role === 'leader').length,
        executor_agents: agents.filter((item) => item.product_role === 'executor').length,
        running_agents: 2,
        failed_agents: 0,
        active_sessions: 2,
        private_sessions: 1,
        leader_scoped_sessions: 1,
        agents,
        recent_events: events,
      })
    }
    if (pathName === '/api/v1/settings/runtime') {
      return json(route, {
        agents_root: 'C:\\fleet-control\\agents',
        hermes_source: '..\\hermes',
        hermes_command: 'hermes',
        java_agent_source: '..\\java-agent',
        java_agent_command: 'java',
      })
    }
    if (pathName === '/api/v1/settings/ports') {
      return json(route, {
        backend_port: 23801,
        frontend_port: 23802,
        agent_port_base: 29000,
        agent_port_stride: 10,
      })
    }
    if (pathName === '/api/v1/settings/integrations') {
      return json(route, {
        project_workflow_url: 'http://localhost:23811',
        project_workflow_status: 'connected',
        github_remote: 'https://github.com/FerrPOINT/fleet-control',
      })
    }
    if (pathName === '/api/v1/settings/auth') {
      return json(route, {
        mode: 'hmac',
        jwt_issuer: 'fleet-control',
        jwt_audience: 'sdlc',
        access_token_ttl_minutes: 15,
        refresh_token_ttl_days: 7,
        refresh_cookie_name: 'refresh_token',
        refresh_cookie_secure: true,
        refresh_cookie_same_site: 'Lax',
        refresh_cookie_domain: null,
        refresh_cookie_path: '/api/v1/auth',
      })
    }
    if (pathName === '/api/v1/deployments/jobs' && method === 'POST')
      return json(route, deploymentJobs[0])
    if (pathName === '/api/v1/deployments/jobs') return json(route, deploymentJobs)
    const deploymentJobMatch = pathName.match(
      /^\/api\/v1\/deployments\/jobs\/([^/]+)(?:\/cancel)?$/,
    )
    if (deploymentJobMatch) return json(route, deploymentJobs[0])
    if (pathName === '/api/v1/agents' && method === 'GET') return json(route, agents)
    if (pathName === '/api/v1/agents' && method === 'POST') {
      return json(route, {
        ...agents[0],
        id: '00000000-0000-4000-8000-000000000104',
        ordinal: 4,
        name: 'agent4',
        display_name: 'Custom Hermes',
      })
    }

    const agentMatch = pathName.match(/^\/api\/v1\/agents\/([^/]+)(?:\/([^/]+))?(?:\/(.+))?$/)
    if (agentMatch) {
      const [, agentId, section, rest] = agentMatch
      const found = agents.find((item) => item.id === agentId) ?? agents[0]
      if (!section) return json(route, found)
      if (section === 'config') return json(route, { ...agentConfig, agent_id: agentId })
      if (section === 'skills') {
        if (rest)
          return json(
            route,
            [...skills, ...testerSkills].find((item) => item.name === rest),
          )
        if (agentId === ids.tester) return json(route, testerSkills)
        if (agentId === ids.lead) return json(route, leaderSkills)
        return json(route, skills)
      }
      if (['provision', 'start', 'stop', 'restart', 'health'].includes(section)) {
        return json(route, {
          agent_id: agentId,
          status: section === 'stop' ? 'stopped' : 'running',
          message: `${section} accepted by screenshot mock`,
        })
      }
    }

    if (pathName === '/api/v1/sessions') {
      const agentId = url.searchParams.get('agent_id')
      const userFilter = url.searchParams.get('user_id')
      const userIds =
        userFilter && userFilter !== 'all'
          ? userFilter
              .split(',')
              .map((item) => item.trim())
              .filter(Boolean)
          : []
      const byAgent = agentId
        ? sessions.filter((session) => session.primary_agent_id === agentId)
        : sessions
      const byUser = userIds.length
        ? byAgent.filter((session) => userIds.includes(session.user_id))
        : byAgent
      return json(route, byUser)
    }
    const sessionMessagesMatch = pathName.match(/^\/api\/v1\/sessions\/([^/]+)\/messages$/)
    if (sessionMessagesMatch) {
      return json(route, sessionMessages[sessionMessagesMatch[1]] ?? [])
    }
    const sessionParticipantsMatch = pathName.match(/^\/api\/v1\/sessions\/([^/]+)\/participants$/)
    if (sessionParticipantsMatch) {
      const session = sessions.find((item) => item.id === sessionParticipantsMatch[1])
      return json(route, session ? participantsForSession(session) : [])
    }
    const sessionDelegationsMatch = pathName.match(/^\/api\/v1\/sessions\/([^/]+)\/delegations$/)
    if (sessionDelegationsMatch) return json(route, sessions[1])
    const sessionRunsMatch = pathName.match(/^\/api\/v1\/sessions\/([^/]+)\/runs$/)
    if (sessionRunsMatch) {
      return json(route, sessionRuns[sessionRunsMatch[1]] ?? [])
    }
    const sessionLeaderMatch = pathName.match(/^\/api\/v1\/sessions\/([^/]+)\/leader$/)
    if (sessionLeaderMatch) {
      const session = sessions.find((item) => item.id === sessionLeaderMatch[1]) ?? sessions[0]
      return json(route, session)
    }
    const sessionMatch = pathName.match(/^\/api\/v1\/sessions\/([^/]+)(?:\/handoff)?$/)
    if (sessionMatch) {
      const session = sessions.find((item) => item.id === sessionMatch[1]) ?? sessions[0]
      return json(
        route,
        pathName.endsWith('/handoff') ? { ...session, state: 'handoff_requested' } : session,
      )
    }

    if (pathName === '/api/v1/workflow-bindings') return json(route, workflowBindings)
    if (pathName === '/api/v1/logs') {
      const agentId = url.searchParams.get('agent_id')
      return json(route, agentId ? logs.filter((log) => log.agent_id === agentId) : logs)
    }
    if (pathName === '/api/v1/events/recent') return json(route, events)
    if (pathName === '/api/v1/audit-log') return json(route, auditLog)
    if (pathName === '/api/v1/health') return json(route, { status: 'ok' })

    return json(route, { error: `Unhandled screenshot mock route: ${pathName}` }, 404)
  })
}

const coreScreens = [
  ['01-login.png', '/login'],
  ['02-register.png', '/register'],
  ['03-dashboard.png', '/'],
  ['04-leaders.png', '/leaders'],
  ['05-leader-new.png', '/leaders/new'],
  ['06-leader-detail.png', `/leaders/${ids.lead}`],
  ['07-leader-edit.png', `/leaders/${ids.lead}/edit`],
  ['08-executors.png', '/executors'],
  ['09-executor-new.png', '/executors/new'],
  ['10-executor-detail.png', `/executors/${ids.dev}`],
  ['11-executor-edit.png', `/executors/${ids.dev}/edit`],
  ['12-agents.png', '/agents'],
  ['13-agent-create.png', '/agents/new'],
  ['14-agent-overview.png', `/agents/${ids.dev}`],
  ['15-agent-edit.png', `/agents/${ids.dev}/edit`],
  ['16-agent-runtime.png', `/agents/${ids.dev}/runtime`],
  ['17-agent-skills.png', `/agents/${ids.dev}/skills`],
  ['18-agent-config.png', `/agents/${ids.dev}/config`],
  ['19-agent-workspace.png', `/agents/${ids.dev}/workspace`],
  ['20-agent-sessions.png', `/agents/${ids.dev}/sessions`],
  ['21-executor-runtime.png', `/executors/${ids.dev}/runtime`],
  ['22-executor-skills.png', `/executors/${ids.dev}/skills`],
  ['23-executor-config.png', `/executors/${ids.dev}/config`],
  ['24-executor-workspace.png', `/executors/${ids.dev}/workspace`],
  ['25-executor-sessions.png', `/executors/${ids.dev}/sessions`],
  ['26-sessions.png', '/sessions'],
  ['27-session-private-detail.png', `/sessions/${ids.sessionDev}`],
  ['28-session-leader-detail.png', `/sessions/${ids.sessionQa}`],
  ['29-workflows.png', '/workflows'],
  ['30-deployments.png', '/deployments'],
  ['31-deployments-jobs.png', '/deployments?tab=jobs'],
  ['32-deployments-job-detail.png', '/deployments?tab=detail'],
  ['33-logs.png', '/logs'],
  ['34-logs-events.png', '/logs?tab=events'],
  ['35-logs-audit.png', '/logs?tab=audit'],
  ['36-settings.png', '/settings'],
  ['37-settings-ports.png', '/settings?tab=ports'],
  ['38-settings-integrations.png', '/settings?tab=integrations'],
  ['39-settings-auth.png', '/settings?tab=auth'],
  ['40-settings-users.png', '/settings?tab=users'],
  ['41-access-denied.png', '/access-denied'],
  ['42-not-found.png', '/not-a-fleet-route'],
]

const mobileOnlyScreens = [
  ['43-mobile-dashboard.png', '/'],
  ['44-mobile-leader-detail.png', `/leaders/${ids.lead}`],
]

const viewports = [
  { name: '375x812', width: 375, height: 812, screens: [...coreScreens, ...mobileOnlyScreens] },
  { name: '1920x1080', width: 1920, height: 1080, screens: coreScreens },
  { name: '2560x1440', width: 2560, height: 1440, screens: coreScreens },
]

const browser = await chromium.launch()
const captured = []

try {
  for (const viewport of viewports) {
    const context = await browser.newContext({
      viewport: { width: viewport.width, height: viewport.height },
      deviceScaleFactor: 1,
    })
    await mockApi(context)

    const page = await context.newPage()
    const outputDir = path.join(outputRoot, viewport.name)
    mkdirSync(outputDir, { recursive: true })
    const expectedFiles = new Set(viewport.screens.map(([fileName]) => fileName))
    for (const entry of readdirSync(outputDir)) {
      if (entry.endsWith('.png') && !expectedFiles.has(entry)) {
        rmSync(path.join(outputDir, entry))
      }
    }

    for (const [fileName, urlPath] of viewport.screens) {
      await page.goto(`${baseUrl}${urlPath}`, { waitUntil: 'networkidle' })
      await page.waitForTimeout(1000)
      await page.screenshot({
        path: path.join(outputDir, fileName),
        fullPage: true,
        animations: 'disabled',
      })
      captured.push({ viewport: viewport.name, fileName, route: urlPath })
      console.log(`${viewport.name}/${fileName}`)
    }

    await context.close()
  }
} finally {
  await browser.close()
}

const manifest = [
  '# Screenshot Manifest',
  '',
  `Generated by \`frontend/scripts/capture-screenshots.mjs\` on ${new Date().toISOString()}.`,
  '',
  `Total screenshots: ${captured.length}.`,
  '',
  '| Viewport | File | Route |',
  '| --- | --- | --- |',
  ...captured.map(
    (item) =>
      `| ${item.viewport} | \`docs/assets/screens/${item.viewport}/${item.fileName}\` | \`${item.route}\` |`,
  ),
  '',
].join('\n')

writeFileSync(path.join(outputRoot, 'manifest.md'), manifest)
