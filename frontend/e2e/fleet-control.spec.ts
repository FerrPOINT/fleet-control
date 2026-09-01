import { expect, test, type Page, type Route } from '@playwright/test'

const now = '2026-09-01T10:00:00+03:00'
const ids = {
  user: '00000000-0000-4000-8000-000000000001',
  dev: '00000000-0000-4000-8000-000000000101',
  tester: '00000000-0000-4000-8000-000000000102',
  created: '00000000-0000-4000-8000-000000000103',
  session: '00000000-0000-4000-8000-000000000201',
  createdSession: '00000000-0000-4000-8000-000000000203',
}

type AgentStatus = 'running' | 'stopped' | 'ready'
type Agent = ReturnType<typeof makeAgent>
type Skill = ReturnType<typeof makeSkill>
type Session = ReturnType<typeof makeSession>
type ApiState = {
  agents: Agent[]
  skillsByAgent: Record<string, Skill[]>
  sessions: Session[]
}

function makeAgent(id: string, ordinal: number, displayName: string, status: AgentStatus) {
  const name = `agent${ordinal}`
  const base = `C:\\fleet-control\\agents\\${name}`
  return {
    id,
    ordinal,
    name,
    kind: 'hermes',
    role: ordinal === 2 ? 'tester' : 'developer',
    status,
    display_name: displayName,
    description: `${displayName} isolated Hermes runtime.`,
    namespace_id: ordinal === 2 ? 'qa' : 'dev',
    workflow_id: ordinal === 2 ? 'workflow-qa' : 'workflow-dev',
    runtime_version: 'hermes-main@local',
    dashboard_port: 29002 + (ordinal - 1) * 10,
    api_port: 29001 + (ordinal - 1) * 10,
    paths: {
      runtime: `${base}\\runtime`,
      config: `${base}\\config`,
      workspace: `${base}\\workspace`,
      logs: `${base}\\logs`,
    },
    runtime: {
      desired_state: status === 'running' ? 'running' : 'stopped',
      pid: status === 'running' ? 32000 + ordinal : null,
      health_status: status,
      health_detail: `${displayName} ${status}`,
      command_preview: `hermes dashboard --host 127.0.0.1 --port ${29002 + (ordinal - 1) * 10}`,
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

function makeSkill(agentId: string, name: string, title: string, state = 'enabled') {
  return {
    id: `${agentId}-${name}`,
    agent_id: agentId,
    name,
    title,
    state,
    source: `skills/${name}`,
    content: `# ${title}\n\nDefault per-agent content.`,
    updated_at: now,
  }
}

function makeSession(id: string, agent: Agent, title: string, taskKey: string) {
  return {
    id,
    agent_id: agent.id,
    agent_name: agent.name,
    title,
    task_key: taskKey,
    state: 'active',
    namespace_id: agent.namespace_id,
    external_session_id: `hermes-${agent.name}`,
    last_message_preview: 'Session is active.',
    created_at: now,
    updated_at: now,
  }
}

function createState(): ApiState {
  const dev = makeAgent(ids.dev, 1, 'Developer Hermes', 'running')
  const tester = makeAgent(ids.tester, 2, 'Tester Hermes', 'stopped')
  return {
    agents: [dev, tester],
    skillsByAgent: {
      [dev.id]: [
        makeSkill(dev.id, 'development', 'Development'),
        makeSkill(dev.id, 'project-workflow', 'Project Workflow'),
        makeSkill(dev.id, 'gh-commit-pr', 'GitHub Commit and PR', 'dirty'),
      ],
      [tester.id]: [
        makeSkill(tester.id, 'audit-web-system', 'Web System Audit'),
        makeSkill(tester.id, 'project-workflow', 'Project Workflow'),
      ],
    },
    sessions: [makeSession(ids.session, dev, 'Initial developer task', 'FC-001')],
  }
}

async function installMocks(page: Page, state: ApiState) {
  await page.route('**/api/v1/**', async (route) => {
    const request = route.request()
    const url = new URL(request.url())
    const pathName = url.pathname
    const method = request.method()

    if (pathName === '/api/v1/auth/refresh') {
      return fulfill(route, authResponse())
    }
    if (pathName === '/api/v1/users/me') return fulfill(route, userResponse())
    if (pathName === '/api/v1/users') return fulfill(route, { users: [userResponse()] })
    if (pathName === '/api/v1/runtime-templates') return fulfill(route, runtimeTemplates())
    if (pathName === '/api/v1/dashboard') {
      return fulfill(route, {
        total_agents: state.agents.length,
        running_agents: state.agents.filter((agent) => agent.status === 'running').length,
        failed_agents: 0,
        active_sessions: state.sessions.length,
        agents: state.agents,
        recent_events: events(),
      })
    }
    if (pathName === '/api/v1/agents' && method === 'GET') return fulfill(route, state.agents)
    if (pathName === '/api/v1/agents' && method === 'POST') {
      const body = (await request.postDataJSON()) as { display_name?: string }
      const next = makeAgent(ids.created, 3, body.display_name ?? 'Custom Hermes', 'ready')
      state.agents.push(next)
      state.skillsByAgent[next.id] = [makeSkill(next.id, 'project-workflow', 'Project Workflow')]
      return fulfill(route, next)
    }

    const agentMatch = pathName.match(/^\/api\/v1\/agents\/([^/]+)(?:\/([^/]+))?(?:\/(.+))?$/)
    if (agentMatch) {
      const [, agentId, section, rest] = agentMatch
      const agent = state.agents.find((item) => item.id === agentId) ?? state.agents[0]
      if (!section) return fulfill(route, agent)
      if (section === 'config') return fulfill(route, agentConfig(agent))
      if (section === 'skills') {
        const skills = state.skillsByAgent[agentId] ?? []
        if (rest && method === 'PUT') {
          const body = (await request.postDataJSON()) as { state: string; content?: string | null }
          const skill = skills.find((item) => item.name === rest)
          if (skill) {
            skill.state = body.content !== skill.content ? 'dirty' : body.state
            skill.content = body.content ?? null
            return fulfill(route, skill)
          }
        }
        return fulfill(route, skills)
      }
      if (['start', 'stop', 'restart', 'health', 'provision'].includes(section)) {
        agent.status = section === 'stop' ? 'stopped' : 'running'
        agent.runtime.health_status = agent.status
        return fulfill(route, {
          agent_id: agentId,
          status: agent.status,
          message: `${section} accepted`,
        })
      }
    }

    if (pathName === '/api/v1/sessions' && method === 'POST') {
      const body = (await request.postDataJSON()) as {
        agent_id: string
        title: string
        task_key?: string | null
      }
      const agent = state.agents.find((item) => item.id === body.agent_id) ?? state.agents[0]
      const session = makeSession(ids.createdSession, agent, body.title, body.task_key ?? 'FC-NEW')
      state.sessions.push(session)
      return fulfill(route, session)
    }
    if (pathName === '/api/v1/sessions') {
      const agentId = url.searchParams.get('agent_id')
      return fulfill(
        route,
        agentId ? state.sessions.filter((session) => session.agent_id === agentId) : state.sessions,
      )
    }
    const sessionMatch = pathName.match(/^\/api\/v1\/sessions\/([^/]+)(?:\/handoff)?$/)
    if (sessionMatch) {
      const session =
        state.sessions.find((item) => item.id === sessionMatch[1]) ?? state.sessions[0]
      if (pathName.endsWith('/handoff')) {
        const body = (await request.postDataJSON()) as { target_agent_id: string }
        const target =
          state.agents.find((item) => item.id === body.target_agent_id) ?? state.agents[1]
        session.agent_id = target.id
        session.agent_name = target.name
        session.namespace_id = target.namespace_id
        session.state = 'handoff_requested'
      }
      return fulfill(route, session)
    }

    if (pathName === '/api/v1/workflow-bindings') return fulfill(route, workflowBindings())
    if (pathName === '/api/v1/logs') return fulfill(route, logs())
    return fulfill(route, { error: `Unhandled route ${pathName}` }, 404)
  })
}

function fulfill(route: Route, value: unknown, status = 200) {
  return route.fulfill({
    status,
    contentType: 'application/json',
    body: JSON.stringify(value),
  })
}

function userResponse() {
  return {
    id: ids.user,
    email: 'admin@fleet-control.local',
    username: 'admin',
    display_name: 'Fleet Admin',
    is_system_admin: true,
    is_active: true,
  }
}

function authResponse() {
  return {
    access_token: 'test-token',
    user_id: ids.user,
    email: 'admin@fleet-control.local',
    username: 'admin',
    display_name: 'Fleet Admin',
    is_system_admin: true,
  }
}

function runtimeTemplates() {
  return [
    {
      kind: 'hermes',
      display_name: 'Hermes',
      implemented: true,
      enabled: true,
      description: 'Local Hermes runtime with HERMES_HOME isolation.',
      capabilities: { provision: true, start: true, stop: true, restart: true },
    },
    {
      kind: 'java_agent',
      display_name: 'Java Agent',
      implemented: false,
      enabled: true,
      description: 'Spring Boot adapter contract reserved for phase 2.',
      capabilities: {
        provision: false,
        endpoints: ['/actuator/health', '/api/v1/agent/chat/stream', '/api/v2/sessions'],
      },
    },
  ]
}

function agentConfig(agent: Agent) {
  return {
    agent_id: agent.id,
    config_json: { namespace_id: agent.namespace_id, workflow_id: agent.workflow_id },
    soul_md: `# ${agent.display_name}\n\nAgent-local operating notes.`,
    env_json: { HERMES_HOME: agent.paths.config, OPENAI_API_KEY: '[REDACTED]' },
    updated_at: now,
  }
}

function workflowBindings() {
  return [
    {
      id: '00000000-0000-4000-8000-000000000401',
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
      id: '00000000-0000-4000-8000-000000000402',
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
}

function events() {
  return [
    {
      id: '00000000-0000-4000-8000-000000000501',
      agent_id: ids.dev,
      event_type: 'runtime_started',
      message: 'agent1 Hermes runtime started',
      payload: {},
      created_at: now,
    },
  ]
}

function logs() {
  return [
    {
      id: '00000000-0000-4000-8000-000000000601',
      agent_id: ids.dev,
      stream: 'system',
      message: 'Hermes start requested',
      created_at: now,
    },
  ]
}

test('Hermes fleet control flow covers agents, runtime, skills, sessions and handoff', async ({
  page,
}) => {
  const state = createState()
  await installMocks(page, state)

  await page.goto('/')
  await expect(page.getByRole('heading', { name: 'Fleet dashboard' })).toBeVisible()
  await expect(page.getByText('agent1 - developer - namespace dev')).toBeVisible()
  await expect(page.getByText('agent2 - tester - namespace qa')).toBeVisible()

  await page.getByRole('link', { name: 'Agents' }).click()
  await page.getByRole('link', { name: 'New agent' }).click()
  await expect(page.getByRole('heading', { name: 'Create agent' })).toBeVisible()
  await expect(page.getByRole('button', { name: /Hermes implemented/ })).toBeVisible()
  await expect(page.getByRole('button', { name: /Java Agent planned/ })).toBeVisible()
  await page.getByLabel('Role').selectOption('tester')
  await expect(page.getByLabel('Display name')).toHaveValue('Tester Hermes')
  await page.getByRole('button', { name: 'Create agent' }).click()
  await expect(page).toHaveURL(new RegExp(`/agents/${ids.created}$`))
  await expect(page.getByRole('heading', { level: 1, name: 'Tester Hermes' })).toBeVisible()

  await page.goto(`/agents/${ids.dev}/runtime`)
  await page.getByRole('button', { exact: true, name: 'Stop' }).click()
  await expect(page.getByText('stopped')).toBeVisible()
  await page.getByRole('button', { exact: true, name: 'Start' }).click()
  await expect(page.getByText('running')).toBeVisible()

  await page.goto(`/agents/${ids.dev}/skills`)
  await page.getByRole('button', { name: /GitHub Commit and PR/ }).click()
  await page.getByRole('textbox').fill('# GitHub Commit and PR\n\nCustom agent override.')
  await page.getByRole('button', { name: 'Save skill' }).click()
  await expect(page.getByText('This skill has a local per-agent override.')).toBeVisible()

  await page.goto('/sessions')
  await page.getByLabel('Agent').selectOption(ids.dev)
  await page.getByLabel('Title').fill('Create checkout smoke')
  await page.getByLabel('Task key').fill('FC-777')
  await page.getByRole('button', { name: 'Create session' }).click()
  await expect(page.getByRole('link', { name: /Create checkout smoke/ })).toBeVisible()

  await page.goto(`/sessions/${ids.createdSession}`)
  await page.getByRole('combobox').selectOption(ids.tester)
  await page.getByRole('button', { name: 'Handoff session' }).click()
  await expect(page.getByText('handoff requested')).toBeVisible()
  await expect(page.getByText('agent2', { exact: true })).toBeVisible()
})
