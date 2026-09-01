import { chromium } from '@playwright/test'
import { mkdirSync } from 'node:fs'
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
}) {
  const name = `agent${ordinal}`
  const base = `C:\\fleet-control\\agents\\${name}`
  return {
    id,
    ordinal,
    name,
    kind: 'hermes',
    role,
    status,
    display_name: displayName,
    description:
      role === 'tester'
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
          ? 'Hermes dashboard process is tracked by Fleet Control.'
          : 'Process is provisioned and ready to start.',
      command_preview: `hermes dashboard --host 127.0.0.1 --port ${dashboardPort}`,
      env_preview: {
        HERMES_HOME: `${base}\\config`,
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
]

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

const sessions = [
  {
    id: ids.sessionDev,
    agent_id: ids.dev,
    agent_name: 'agent1',
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
    agent_name: 'agent2',
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
    message: 'Hermes dashboard listening on 127.0.0.1:29002',
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

const user = {
  id: ids.admin,
  email: 'admin@fleet-control.local',
  username: 'admin',
  display_name: 'Fleet Admin',
  is_system_admin: true,
  is_active: true,
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
        is_system_admin: true,
      })
    }
    if (pathName === '/api/v1/users/me') return json(route, user)
    if (pathName === '/api/v1/users') return json(route, { users: [user] })
    if (pathName === '/api/v1/runtime-templates') return json(route, runtimeTemplates)
    if (pathName === '/api/v1/dashboard') {
      return json(route, {
        total_agents: agents.length,
        running_agents: 1,
        failed_agents: 0,
        active_sessions: 2,
        agents,
        recent_events: events,
      })
    }
    if (pathName === '/api/v1/agents' && method === 'GET') return json(route, agents)
    if (pathName === '/api/v1/agents' && method === 'POST') {
      return json(route, {
        ...agents[0],
        id: '00000000-0000-4000-8000-000000000103',
        ordinal: 3,
        name: 'agent3',
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
        return json(route, agentId === ids.tester ? testerSkills : skills)
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
      return json(
        route,
        agentId ? sessions.filter((session) => session.agent_id === agentId) : sessions,
      )
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
    if (pathName === '/api/v1/events') return json(route, events)
    if (pathName === '/api/v1/health') return json(route, { status: 'ok' })

    return json(route, { error: `Unhandled screenshot mock route: ${pathName}` }, 404)
  })
}

const coreScreens = [
  ['01-login.png', '/login'],
  ['02-register.png', '/register'],
  ['03-dashboard.png', '/'],
  ['04-agents.png', '/agents'],
  ['05-agent-create.png', '/agents/new'],
  ['06-agent-overview.png', `/agents/${ids.dev}`],
  ['07-agent-runtime.png', `/agents/${ids.dev}/runtime`],
  ['08-agent-skills.png', `/agents/${ids.dev}/skills`],
  ['09-agent-config.png', `/agents/${ids.dev}/config`],
  ['10-agent-workspace.png', `/agents/${ids.dev}/workspace`],
  ['11-agent-sessions.png', `/agents/${ids.dev}/sessions`],
  ['12-sessions.png', '/sessions'],
  ['13-session-detail.png', `/sessions/${ids.sessionDev}`],
  ['14-workflows.png', '/workflows'],
  ['15-deployments.png', '/deployments'],
  ['16-logs.png', '/logs'],
  ['17-settings.png', '/settings'],
]

const mobileOnlyScreens = [
  ['18-mobile-dashboard.png', '/'],
  ['19-mobile-agent-detail.png', `/agents/${ids.dev}`],
]

const viewports = [
  { name: '375x812', width: 375, height: 812, screens: [...coreScreens, ...mobileOnlyScreens] },
  { name: '1920x1080', width: 1920, height: 1080, screens: coreScreens },
  { name: '2560x1440', width: 2560, height: 1440, screens: coreScreens },
]

const browser = await chromium.launch()

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

    for (const [fileName, urlPath] of viewport.screens) {
      await page.goto(`${baseUrl}${urlPath}`, { waitUntil: 'networkidle' })
      await page.waitForTimeout(250)
      await page.screenshot({
        path: path.join(outputDir, fileName),
        fullPage: true,
        animations: 'disabled',
      })
      console.log(`${viewport.name}/${fileName}`)
    }

    await context.close()
  }
} finally {
  await browser.close()
}
