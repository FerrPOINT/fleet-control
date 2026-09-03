import { expect, test, type Page, type Route } from '@playwright/test'

const now = '2026-09-01T10:00:00+03:00'
const ids = {
  user: '00000000-0000-4000-8000-000000000001',
  reviewer: '00000000-0000-4000-8000-000000000002',
  dev: '00000000-0000-4000-8000-000000000101',
  tester: '00000000-0000-4000-8000-000000000102',
  lead: '00000000-0000-4000-8000-000000000103',
  created: '00000000-0000-4000-8000-000000000104',
  session: '00000000-0000-4000-8000-000000000201',
  createdSession: '00000000-0000-4000-8000-000000000203',
}

type AgentStatus = 'running' | 'stopped' | 'ready'
type ProductRole = 'leader' | 'executor'
type AgentProfile = 'developer' | 'tester' | 'it_lead' | 'custom'
type Agent = ReturnType<typeof makeAgent>
type Skill = ReturnType<typeof makeSkill>
type Session = ReturnType<typeof makeSession>
type Message = ReturnType<typeof makeMessage>
type Run = ReturnType<typeof makeRun>
type DeploymentJob = ReturnType<typeof makeDeploymentJob>
type ApiState = {
  agents: Agent[]
  skillsByAgent: Record<string, Skill[]>
  sessions: Session[]
  leaderExecutors: Record<string, string[]>
  messagesBySession: Record<string, Message[]>
  runsBySession: Record<string, Run[]>
  deploymentJobs: DeploymentJob[]
}

function makeAgent(
  id: string,
  ordinal: number,
  displayName: string,
  status: AgentStatus,
  productRole: ProductRole = 'executor',
  profile?: AgentProfile,
) {
  const name = `agent${ordinal}`
  const base = `C:\\fleet-control\\agents\\${name}`
  const role =
    profile ?? (productRole === 'leader' ? 'it_lead' : ordinal === 2 ? 'tester' : 'developer')
  return {
    id,
    ordinal,
    name,
    kind: 'hermes',
    product_role: productRole,
    role,
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
      command_preview: `hermes serve --host 127.0.0.1 --port ${29001 + (ordinal - 1) * 10}`,
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

function agentDirectoryItem(agent: Agent) {
  return {
    id: agent.id,
    ordinal: agent.ordinal,
    name: agent.name,
    kind: agent.kind,
    product_role: agent.product_role,
    role: agent.role,
    status: agent.status,
    display_name: agent.display_name,
    description: agent.description,
    namespace_id: agent.namespace_id,
    workflow_id: agent.workflow_id,
    runtime_version: agent.runtime_version,
    dashboard_port: agent.dashboard_port,
    api_port: agent.api_port,
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

function makeSession(
  id: string,
  agent: Agent,
  title: string,
  taskKey: string,
  user = userResponse(),
  leader?: Agent | null,
  parentSessionId: string | null = null,
) {
  return {
    id,
    agent_id: agent.id,
    primary_agent_id: agent.id,
    agent_name: agent.name,
    primary_agent_name: agent.name,
    user_id: user.id,
    user_email: user.email,
    user_username: user.username,
    user_display_name: user.display_name,
    leader_agent_id: leader?.id ?? null,
    leader_agent_name: leader?.name ?? null,
    parent_session_id: parentSessionId,
    created_by_leader_agent_id: parentSessionId ? (leader?.id ?? null) : null,
    visibility: leader ? 'leader_scoped' : 'private',
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

function makeMessage(
  sessionId: string,
  body: string,
  authorType: 'system' | 'user' | 'agent' = 'system',
) {
  return {
    id: `${sessionId}-${authorType}-${body.length}`,
    session_id: sessionId,
    author_type: authorType,
    author_user_id: authorType === 'user' ? ids.user : null,
    author_agent_id: authorType === 'agent' ? ids.lead : null,
    author_display_name:
      authorType === 'user'
        ? 'Fleet Admin'
        : authorType === 'agent'
          ? 'IT Lead Hermes'
          : 'Fleet Control',
    body,
    message_kind:
      authorType === 'user'
        ? 'user_prompt'
        : authorType === 'agent'
          ? 'assistant_message'
          : 'system_event',
    runtime_message_id: null,
    replayed: false,
    created_at: now,
  }
}

function makeRun(sessionId: string, agent: Agent, runRole: 'primary' | 'leader' = 'primary') {
  return {
    id: `${sessionId}-${agent.id}-${runRole}`,
    session_id: sessionId,
    agent_id: agent.id,
    agent_name: agent.name,
    runtime_session_id: `hermes-${agent.name}`,
    run_role: runRole,
    state: 'pending',
    last_error: null,
    created_at: now,
    updated_at: now,
  }
}

function makeDeploymentJob() {
  return {
    id: '00000000-0000-4000-8000-000000000701',
    job_kind: 'provision',
    state: 'queued',
    agent_id: ids.dev,
    runtime_kind: 'hermes',
    requested_by_user_id: ids.user,
    title: 'Provision Developer Hermes',
    detail: { requested_from: 'e2e', secret: 'redacted' },
    last_error: null,
    created_at: now,
    updated_at: now,
  }
}

function permissions() {
  return [
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
}

function createState(): ApiState {
  const dev = makeAgent(ids.dev, 1, 'Developer Hermes', 'running')
  const tester = makeAgent(ids.tester, 2, 'Tester Hermes', 'stopped')
  const lead = makeAgent(ids.lead, 3, 'IT Lead Hermes', 'running', 'leader', 'it_lead')
  const devSession = makeSession(ids.session, dev, 'Initial developer task', 'FC-001')
  const testerSession = makeSession(
    '00000000-0000-4000-8000-000000000202',
    tester,
    'Tester review sweep',
    'FC-002',
    reviewerResponse(),
    lead,
  )
  return {
    agents: [dev, tester, lead],
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
      [lead.id]: [
        makeSkill(lead.id, 'project-workflow', 'Project Workflow'),
        makeSkill(lead.id, 'development', 'Development'),
      ],
    },
    sessions: [devSession, testerSession],
    leaderExecutors: { [lead.id]: [dev.id, tester.id] },
    messagesBySession: {
      [devSession.id]: [makeMessage(devSession.id, 'Session created in Fleet Control')],
      [testerSession.id]: [makeMessage(testerSession.id, 'Session created in Fleet Control')],
    },
    runsBySession: {
      [devSession.id]: [makeRun(devSession.id, dev)],
      [testerSession.id]: [
        makeRun(testerSession.id, tester),
        makeRun(testerSession.id, lead, 'leader'),
      ],
    },
    deploymentJobs: [makeDeploymentJob()],
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
    if (pathName === '/api/v1/users/me/permissions') {
      return fulfill(route, {
        user_id: ids.user,
        role: 'admin',
        is_system_admin: true,
        permissions: permissions(),
      })
    }
    if (pathName === '/api/v1/users') {
      return fulfill(route, { users: [userResponse(), reviewerResponse()] })
    }
    const userRoleMatch = pathName.match(/^\/api\/v1\/users\/([^/]+)\/role$/)
    if (userRoleMatch && method === 'PATCH') {
      const body = (await request.postDataJSON()) as { role: 'admin' | 'operator' | 'user' }
      const user = userRoleMatch[1] === ids.reviewer ? reviewerResponse() : userResponse()
      return fulfill(route, {
        ...user,
        system_role: body.role,
        is_system_admin: body.role === 'admin',
      })
    }
    if (pathName === '/api/v1/runtime-templates') return fulfill(route, runtimeTemplates())
    if (pathName === '/api/v1/agent-directory') {
      return fulfill(route, state.agents.map(agentDirectoryItem))
    }
    if (pathName === '/api/v1/leaders') {
      return fulfill(
        route,
        state.agents.filter((agent) => agent.product_role === 'leader'),
      )
    }
    if (pathName === '/api/v1/executors') {
      return fulfill(
        route,
        state.agents.filter((agent) => agent.product_role === 'executor'),
      )
    }
    const leaderExecutorsMatch = pathName.match(/^\/api\/v1\/leaders\/([^/]+)\/executors$/)
    if (leaderExecutorsMatch) {
      const leaderId = leaderExecutorsMatch[1]
      if (method === 'PUT') {
        const body = (await request.postDataJSON()) as { executor_ids: string[] }
        state.leaderExecutors[leaderId] = body.executor_ids
      }
      const executorIds = state.leaderExecutors[leaderId] ?? []
      return fulfill(
        route,
        executorIds.map((executorId) => {
          const executor = state.agents.find((agent) => agent.id === executorId) ?? state.agents[0]
          return {
            leader_agent_id: leaderId,
            executor_agent_id: executor.id,
            executor_name: executor.name,
            executor_display_name: executor.display_name,
            executor_profile: executor.role,
            namespace_id: executor.namespace_id,
            workflow_id: executor.workflow_id,
            created_by_user_id: ids.user,
            created_at: now,
          }
        }),
      )
    }
    if (pathName === '/api/v1/dashboard') {
      return fulfill(route, {
        total_agents: state.agents.length,
        leader_agents: state.agents.filter((agent) => agent.product_role === 'leader').length,
        executor_agents: state.agents.filter((agent) => agent.product_role === 'executor').length,
        running_agents: state.agents.filter((agent) => agent.status === 'running').length,
        failed_agents: 0,
        active_sessions: state.sessions.length,
        private_sessions: state.sessions.filter((session) => session.visibility === 'private')
          .length,
        leader_scoped_sessions: state.sessions.filter(
          (session) => session.visibility === 'leader_scoped',
        ).length,
        agents: state.agents,
        recent_events: events(),
      })
    }
    if (pathName === '/api/v1/settings/runtime') {
      return fulfill(route, {
        agents_root: 'C:\\fleet-control\\agents',
        hermes_source: '..\\hermes',
        hermes_command: 'hermes',
        java_agent_source: '..\\java-agent',
        java_agent_command: 'java',
      })
    }
    if (pathName === '/api/v1/settings/ports') {
      return fulfill(route, {
        backend_port: 23801,
        frontend_port: 23802,
        agent_port_base: 29000,
        agent_port_stride: 10,
      })
    }
    if (pathName === '/api/v1/settings/integrations') {
      return fulfill(route, {
        project_workflow_url: 'http://localhost:23811',
        project_workflow_status: 'connected',
        github_remote: 'https://github.com/FerrPOINT/fleet-control',
      })
    }
    if (pathName === '/api/v1/settings/auth') {
      return fulfill(route, {
        access_token_ttl_minutes: 15,
        refresh_token_ttl_days: 7,
        refresh_cookie_name: 'refresh_token',
        refresh_cookie_secure: true,
        refresh_cookie_same_site: 'Lax',
        refresh_cookie_domain: null,
        refresh_cookie_path: '/api/v1/auth',
      })
    }
    if (pathName === '/api/v1/deployments/jobs' && method === 'POST') {
      const body = (await request.postDataJSON()) as { title: string; job_kind: string }
      const job = {
        ...makeDeploymentJob(),
        id: `${ids.created}-job`,
        title: body.title,
        job_kind: body.job_kind,
      }
      state.deploymentJobs.unshift(job)
      return fulfill(route, job)
    }
    if (pathName === '/api/v1/deployments/jobs') {
      return fulfill(route, state.deploymentJobs)
    }
    const deploymentJobMatch = pathName.match(
      /^\/api\/v1\/deployments\/jobs\/([^/]+)(?:\/cancel)?$/,
    )
    if (deploymentJobMatch) {
      const job =
        state.deploymentJobs.find((item) => item.id === deploymentJobMatch[1]) ??
        state.deploymentJobs[0]
      if (pathName.endsWith('/cancel')) job.state = 'cancelled'
      return fulfill(route, job)
    }
    if (pathName === '/api/v1/agents' && method === 'GET') return fulfill(route, state.agents)
    if (pathName === '/api/v1/agents' && method === 'POST') {
      const body = (await request.postDataJSON()) as {
        display_name?: string
        product_role?: ProductRole
        role?: AgentProfile
        executor_ids?: string[]
      }
      const next = makeAgent(
        ids.created,
        4,
        body.display_name ?? 'Custom Hermes',
        'ready',
        body.product_role ?? 'executor',
        body.role,
      )
      state.agents.push(next)
      state.skillsByAgent[next.id] = [makeSkill(next.id, 'project-workflow', 'Project Workflow')]
      if (next.product_role === 'leader') state.leaderExecutors[next.id] = body.executor_ids ?? []
      return fulfill(route, next)
    }

    const agentMatch = pathName.match(/^\/api\/v1\/agents\/([^/]+)(?:\/([^/]+))?(?:\/(.+))?$/)
    if (agentMatch) {
      const [, agentId, section, rest] = agentMatch
      const agent = state.agents.find((item) => item.id === agentId) ?? state.agents[0]
      if (!section) {
        if (method === 'PATCH') {
          const body = (await request.postDataJSON()) as {
            product_role?: ProductRole
            role?: AgentProfile
            display_name?: string
            description?: string
            namespace_id?: string
            workflow_id?: string
            executor_ids?: string[]
          }
          agent.product_role = body.product_role ?? agent.product_role
          agent.role = body.role ?? agent.role
          agent.display_name = body.display_name ?? agent.display_name
          agent.description = body.description ?? agent.description
          agent.namespace_id = body.namespace_id ?? agent.namespace_id
          agent.workflow_id = body.workflow_id ?? agent.workflow_id
          if (body.executor_ids) state.leaderExecutors[agent.id] = body.executor_ids
        }
        return fulfill(route, agent)
      }
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
        primary_agent_id?: string
        agent_id?: string
        title: string
        task_key?: string | null
        leader_agent_id?: string | null
        parent_session_id?: string | null
      }
      const agentId = body.primary_agent_id ?? body.agent_id ?? state.agents[0].id
      const agent = state.agents.find((item) => item.id === agentId) ?? state.agents[0]
      const leader =
        state.agents.find((item) => item.id === (body.leader_agent_id ?? undefined)) ??
        (agent.product_role === 'leader' ? agent : null)
      const session = makeSession(
        ids.createdSession,
        agent,
        body.title,
        body.task_key ?? 'FC-NEW',
        userResponse(),
        leader,
        body.parent_session_id ?? null,
      )
      state.sessions.push(session)
      state.messagesBySession[session.id] = [
        makeMessage(session.id, 'Session created in Fleet Control'),
      ]
      state.runsBySession[session.id] = [makeRun(session.id, agent)]
      if (leader) state.runsBySession[session.id].push(makeRun(session.id, leader, 'leader'))
      return fulfill(route, session)
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
        ? state.sessions.filter((session) => session.primary_agent_id === agentId)
        : state.sessions
      const byUser = userIds.length
        ? byAgent.filter((session) => userIds.includes(session.user_id))
        : byAgent
      return fulfill(route, byUser)
    }
    const sessionMessagesMatch = pathName.match(/^\/api\/v1\/sessions\/([^/]+)\/messages$/)
    if (sessionMessagesMatch) {
      const sessionId = sessionMessagesMatch[1]
      if (method === 'POST') {
        const body = (await request.postDataJSON()) as {
          body: string
          author_agent_id?: string | null
        }
        const message = makeMessage(sessionId, body.body, body.author_agent_id ? 'agent' : 'user')
        state.messagesBySession[sessionId] = [
          ...(state.messagesBySession[sessionId] ?? []),
          message,
        ]
        const session = state.sessions.find((item) => item.id === sessionId)
        if (session) session.last_message_preview = body.body
        return fulfill(route, message)
      }
      return fulfill(route, state.messagesBySession[sessionId] ?? [])
    }
    const sessionParticipantsMatch = pathName.match(/^\/api\/v1\/sessions\/([^/]+)\/participants$/)
    if (sessionParticipantsMatch) {
      const session = state.sessions.find((item) => item.id === sessionParticipantsMatch[1])
      return fulfill(route, session ? participantsForSession(session) : [])
    }
    const sessionDelegationsMatch = pathName.match(/^\/api\/v1\/sessions\/([^/]+)\/delegations$/)
    if (sessionDelegationsMatch && method === 'POST') {
      const parent =
        state.sessions.find((item) => item.id === sessionDelegationsMatch[1]) ?? state.sessions[0]
      const body = (await request.postDataJSON()) as {
        executor_agent_id: string
        title: string
        initial_message?: string | null
      }
      const executor =
        state.agents.find((item) => item.id === body.executor_agent_id) ?? state.agents[0]
      const leader =
        state.agents.find((item) => item.id === parent.leader_agent_id) ?? state.agents[2]
      const child = makeSession(
        `${ids.createdSession}-child`,
        executor,
        body.title,
        'FC-DEL',
        userResponse(),
        leader,
        parent.id,
      )
      state.sessions.push(child)
      state.messagesBySession[child.id] = [
        makeMessage(child.id, 'Session created in Fleet Control'),
        makeMessage(child.id, body.initial_message ?? 'Delegated task', 'agent'),
      ]
      state.runsBySession[child.id] = [
        makeRun(child.id, executor),
        makeRun(child.id, leader, 'leader'),
      ]
      return fulfill(route, child)
    }
    const sessionRunsMatch = pathName.match(/^\/api\/v1\/sessions\/([^/]+)\/runs$/)
    if (sessionRunsMatch) {
      return fulfill(route, state.runsBySession[sessionRunsMatch[1]] ?? [])
    }
    const sessionLeaderMatch = pathName.match(/^\/api\/v1\/sessions\/([^/]+)\/leader$/)
    if (sessionLeaderMatch && method === 'PUT') {
      const body = (await request.postDataJSON()) as { leader_agent_id: string | null }
      const session =
        state.sessions.find((item) => item.id === sessionLeaderMatch[1]) ?? state.sessions[0]
      const leader = state.agents.find((item) => item.id === body.leader_agent_id)
      session.leader_agent_id = leader?.id ?? null
      session.leader_agent_name = leader?.name ?? null
      session.visibility = leader ? 'leader_scoped' : 'private'
      return fulfill(route, session)
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
        session.primary_agent_id = target.id
        session.agent_name = target.name
        session.primary_agent_name = target.name
        session.namespace_id = target.namespace_id
        session.state = 'handoff_requested'
        state.runsBySession[session.id] = [
          ...(state.runsBySession[session.id] ?? []),
          makeRun(session.id, target),
        ]
      }
      return fulfill(route, session)
    }

    if (pathName === '/api/v1/workflow-bindings') return fulfill(route, workflowBindings())
    if (pathName === '/api/v1/logs') return fulfill(route, logs())
    if (pathName === '/api/v1/events/recent') return fulfill(route, events())
    if (pathName === '/api/v1/audit-log') return fulfill(route, auditLog())
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
    system_role: 'admin',
    is_system_admin: true,
    is_active: true,
  }
}

function reviewerResponse() {
  return {
    id: ids.reviewer,
    email: 'qa@fleet-control.local',
    username: 'qa',
    display_name: 'QA Reviewer',
    system_role: 'user',
    is_system_admin: false,
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
    system_role: 'admin',
    is_system_admin: true,
  }
}

function participantsForSession(session: Session) {
  const participants = [
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
    participants.push({
      id: `${session.id}-leader`,
      session_id: session.id,
      participant_type: 'agent',
      user_id: null,
      agent_id: session.leader_agent_id,
      session_role: 'leader',
      display_name: session.leader_agent_name ?? 'Leader',
      created_at: now,
    })
  }
  return participants
}

function auditLog() {
  return [
    {
      id: '00000000-0000-4000-8000-000000000801',
      actor_user_id: ids.user,
      action: 'session.create',
      entity_type: 'session',
      entity_id: ids.session,
      payload: { title: 'Initial developer task', token: 'redacted' },
      created_at: now,
    },
  ]
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
  await expect(page.getByText('agent1 - profile developer - namespace dev')).toBeVisible()
  await expect(page.getByText('agent2 - profile tester - namespace qa')).toBeVisible()
  await expect(page.getByText('agent3 - profile it lead - namespace dev')).toBeVisible()

  await page.getByRole('link', { name: 'Leaders' }).click()
  await expect(page.getByRole('heading', { name: 'Leaders' })).toBeVisible()
  await expect(page.getByRole('link', { name: /Tester review sweep/ })).not.toBeVisible()
  await page.getByRole('button', { name: 'Remove Fleet Admin filter' }).click()
  await expect(page.getByRole('link', { name: /Tester review sweep/ })).toBeVisible()
  await page.goto(`/leaders/${ids.lead}/edit`)
  await expect(page.getByRole('heading', { name: 'Edit IT Lead Hermes' })).toBeVisible()
  await expect(page.getByText('Managed executors')).toBeVisible()
  await page.getByRole('button', { name: 'Save agent' }).click()
  await expect(page).toHaveURL(new RegExp(`/leaders/${ids.lead}$`))

  await page.getByRole('link', { name: 'Executors' }).click()
  await expect(page.getByRole('heading', { name: 'Executors' })).toBeVisible()
  await expect(page.getByText('Developer Hermes')).toBeVisible()

  await page.getByRole('link', { name: 'Agents' }).click()
  await expect(page.getByRole('link', { name: /Initial developer task/ })).toBeVisible()
  await expect(page.getByRole('link', { name: /Tester review sweep/ })).not.toBeVisible()
  await page.getByRole('link', { name: 'New agent' }).click()
  await expect(page.getByRole('heading', { name: 'Create agent' })).toBeVisible()
  await expect(page.getByRole('button', { name: /Hermes implemented/ })).toBeVisible()
  await expect(page.getByRole('button', { name: /Java Agent planned/ })).toBeVisible()
  await page.getByLabel('Profile').selectOption('tester')
  await expect(page.getByLabel('Display name')).toHaveValue('Tester Hermes')
  await page.getByRole('button', { name: 'Create agent' }).click()
  await expect(page).toHaveURL(new RegExp(`/executors/${ids.created}$`))
  await expect(page.getByRole('heading', { level: 1, name: 'Tester Hermes' })).toBeVisible()

  await page.goto(`/agents/${ids.dev}/runtime`)
  await page.getByRole('button', { exact: true, name: 'Stop' }).click()
  await expect(page.getByText('stopped').first()).toBeVisible()
  await page.getByRole('button', { exact: true, name: 'Start' }).click()
  await expect(page.getByText('running').first()).toBeVisible()

  await page.goto(`/agents/${ids.dev}/workspace`)
  await expect(page.getByRole('heading', { name: 'File purge' })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Purge files' })).toBeDisabled()

  await page.goto(`/agents/${ids.dev}/skills`)
  await page.getByRole('button', { name: /GitHub Commit and PR/ }).click()
  await page.getByRole('textbox').fill('# GitHub Commit and PR\n\nCustom agent override.')
  await page.getByRole('button', { name: 'Save skill' }).click()
  await expect(page.getByText('This skill has a local per-agent override.')).toBeVisible()

  await page.goto('/sessions')
  await expect(page.getByRole('link', { name: /Initial developer task/ })).toBeVisible()
  await expect(page.getByRole('link', { name: /Tester review sweep/ })).not.toBeVisible()
  await page.getByRole('button', { name: 'Remove Fleet Admin filter' }).click()
  await expect(page.getByRole('link', { name: /Tester review sweep/ })).toBeVisible()
  await page.getByLabel('Add session user').selectOption(ids.reviewer)
  await expect(page.getByRole('link', { name: /Initial developer task/ })).not.toBeVisible()
  await expect(page.getByRole('link', { name: /Tester review sweep/ })).toBeVisible()
  await page.getByLabel('Add session user').selectOption(ids.user)
  await expect(page.getByRole('link', { name: /Initial developer task/ })).toBeVisible()
  await expect(page.getByRole('link', { name: /Tester review sweep/ })).toBeVisible()
  await page.getByLabel('Agent').selectOption(ids.dev)
  await expect(page.getByLabel('Leader')).toHaveValue('')
  await page.getByLabel('Title').fill('Create checkout smoke')
  await page.getByLabel('Task key').fill('FC-777')
  await page.getByRole('button', { name: 'Create session' }).click()
  await expect(page.getByRole('link', { name: /Create checkout smoke/ })).toBeVisible()

  await page.goto(`/sessions/${ids.createdSession}`)
  await page.getByLabel('Session leader').selectOption(ids.lead)
  await page.getByRole('button', { name: 'Save leader' }).click()
  await expect(page.getByText('leader scoped')).toBeVisible()
  await page.getByLabel('Message author').selectOption('leader')
  await page.getByPlaceholder('Write a session message').fill('Please coordinate the smoke test.')
  await page.getByRole('button', { name: 'Send message' }).click()
  await expect(
    page.locator('p').filter({ hasText: 'Please coordinate the smoke test.' }).first(),
  ).toBeVisible()
  await page.getByLabel('Executor').selectOption(ids.tester)
  await page.getByLabel('Title').fill('Delegated QA smoke')
  await page.getByPlaceholder('Initial task for the executor').fill('Run the delegated QA sweep.')
  await page.getByRole('button', { name: 'Delegate task' }).click()
  await page.goto('/sessions')
  await page.getByRole('button', { name: 'Remove Fleet Admin filter' }).click()
  await expect(page.getByRole('link', { name: /Delegated QA smoke/ })).toBeVisible()
  await page.goto(`/sessions/${ids.createdSession}`)
  await page.getByLabel('Handoff target agent').selectOption(ids.tester)
  await page.getByRole('button', { name: 'Handoff session' }).click()
  await expect(page.getByText('handoff requested')).toBeVisible()
  await expect(page.getByText('agent2', { exact: true })).toBeVisible()

  await page.goto('/deployments?tab=jobs')
  await expect(page.getByRole('heading', { name: 'Provision and update jobs' })).toBeVisible()
  await page.getByLabel('Title').fill('Runtime update dry run')
  await page.getByRole('button', { name: 'Create job' }).click()
  await expect(page.getByText('Runtime update dry run')).toBeVisible()
  await page.getByRole('button', { name: 'Cancel' }).first().click()
  await expect(page.getByText('cancelled')).toBeVisible()

  await page.goto('/logs?tab=events')
  await expect(page.getByRole('heading', { name: 'Control-plane events' })).toBeVisible()
  await expect(page.getByText('runtime_started')).toBeVisible()
  await page.goto('/logs?tab=audit')
  await expect(page.getByRole('heading', { name: 'Audit trail' })).toBeVisible()
  await expect(page.getByText('session.create')).toBeVisible()

  await page.goto('/settings?tab=users')
  await expect(page.getByRole('heading', { name: 'Users and roles' })).toBeVisible()
  await page.getByRole('combobox').nth(1).selectOption('operator')
  await expect(page.getByText('QA Reviewer')).toBeVisible()

  await page.goto('/access-denied')
  await expect(page.getByRole('heading', { name: 'Access denied' })).toBeVisible()
  await page.goto('/not-a-fleet-route')
  await expect(page.getByRole('heading', { name: 'Page not found' })).toBeVisible()
})
