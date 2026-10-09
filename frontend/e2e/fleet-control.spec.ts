import { expect, test, type Page, type Route } from '@playwright/test'
import { generateKeyPairSync, sign } from 'node:crypto'

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

const managedSettingsSnapshot = {
  runtime: {
    agents_root: 'C:\\fleet-control\\agents',
    hermes_source: '..\\hermes',
    hermes_command: 'hermes',
    java_agent_source: '..\\java-agent',
    java_agent_command: 'java',
  },
  ports: { agent_port_base: 29000, agent_port_stride: 10 },
  integrations: {
    forge_api_url: 'http://ci-cd:22801',
    forge_project: 'fleet-control',
    project_workflow_url: 'http://project-workflow:8000',
  },
  auth: {
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
  },
  retention: { stale_archived_days: 30, review_interval_secs: 3600 },
}

type AgentStatus = 'running' | 'stopped' | 'ready' | 'archived'
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
  workflowBindings: ReturnType<typeof workflowBindings>
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

function storageReport(agent: Agent) {
  const baseSize = agent.ordinal * 1024
  const areas = Object.entries(agent.paths).map(([name, path], index) => ({
    name,
    path,
    exists: true,
    is_directory: true,
    bytes: baseSize * (index + 1),
    files: 2 + index,
    directories: index,
    symlinks: 0,
    last_modified_at: now,
  }))
  return {
    agent_id: agent.id,
    agent_name: agent.name,
    root_path: `C:\\fleet-control\\agents\\${agent.name}`,
    root_exists: true,
    marker_present: true,
    marker_verified: true,
    total_bytes: areas.reduce((sum, area) => sum + area.bytes, 0),
    total_files: areas.reduce((sum, area) => sum + area.files, 0),
    total_directories: areas.reduce((sum, area) => sum + area.directories, 0),
    total_symlinks: 0,
    areas,
    retention: {
      archived: agent.status === 'archived',
      archived_since: agent.status === 'archived' ? now : null,
      purge_eligible: agent.status === 'archived',
      retention_hint:
        agent.status === 'archived'
          ? 'archived agent files can be purged explicitly by an operator'
          : 'archive the agent before physical purge',
    },
  }
}

function storageReview(agents: Agent[]) {
  const items = agents.map((agent) => {
    const report = storageReport(agent)
    return {
      agent_id: agent.id,
      agent_name: agent.name,
      display_name: agent.display_name,
      kind: agent.kind,
      product_role: agent.product_role,
      status: agent.status,
      total_bytes: report.total_bytes,
      total_files: report.total_files,
      root_exists: report.root_exists,
      marker_verified: report.marker_verified,
      purge_eligible: report.retention.purge_eligible,
      retention_hint: report.retention.retention_hint,
    }
  })
  return {
    reviewed_at: now,
    total_agents: items.length,
    total_bytes: items.reduce((sum, item) => sum + item.total_bytes, 0),
    archived_agents: items.filter((item) => item.status === 'archived').length,
    archived_bytes: items
      .filter((item) => item.status === 'archived')
      .reduce((sum, item) => sum + item.total_bytes, 0),
    purge_eligible_agents: items.filter((item) => item.purge_eligible).length,
    missing_root_agents: items.filter((item) => !item.root_exists).length,
    marker_issue_agents: items.filter((item) => item.root_exists && !item.marker_verified).length,
    items,
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
    workflowBindings: workflowBindings(),
  }
}

async function installSsoMocks(page: Page) {
  const { privateKey, publicKey } = generateKeyPairSync('ec', { namedCurve: 'P-256' })
  const jwk = { ...publicKey.export({ format: 'jwk' }), kid: 'qa', alg: 'ES256', use: 'sig' }
  let issuer = 'http://localhost:7701'
  let nonce = ''
  await page.route('**/oidc/authorize**', async (route) => {
    const url = new URL(route.request().url())
    issuer = url.origin
    nonce = url.searchParams.get('nonce') ?? ''
    const callback = new URL(url.searchParams.get('redirect_uri') ?? '/')
    callback.searchParams.set('code', 'qa-code')
    callback.searchParams.set('state', url.searchParams.get('state') ?? '')
    await route.fulfill({
      status: 200,
      contentType: 'text/html',
      body: `<!doctype html><script>location.replace(${JSON.stringify(callback.toString())})</script>`,
    })
  })
  await page.route('**/oidc/token', async (route) => {
    const header = Buffer.from(JSON.stringify({ alg: 'ES256', typ: 'JWT', kid: 'qa' })).toString(
      'base64url',
    )
    const payload = Buffer.from(
      JSON.stringify({
        iss: issuer,
        aud: 'fleet-control',
        sub: ids.user,
        email: 'admin@fleet-control.local',
        nonce,
        iat: Math.floor(Date.now() / 1000),
        exp: Math.floor(Date.now() / 1000) + 3600,
      }),
    ).toString('base64url')
    const content = `${header}.${payload}`
    const signature = sign('sha256', Buffer.from(content), {
      key: privateKey,
      dsaEncoding: 'ieee-p1363',
    }).toString('base64url')
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      headers: { 'access-control-allow-origin': '*' },
      body: JSON.stringify({
        access_token: 'qa-access-token',
        id_token: `${content}.${signature}`,
        expires_in: 3600,
      }),
    })
  })
  await page.route('**/oidc/jwks', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      headers: { 'access-control-allow-origin': '*' },
      body: JSON.stringify({ keys: [jwk] }),
    }),
  )
}

async function installMocks(page: Page, state: ApiState) {
  await installSsoMocks(page)
  await page.route('**/api/v1/**', async (route) => {
    const request = route.request()
    const url = new URL(request.url())
    const pathName = url.pathname
    const method = request.method()

    if (method === 'OPTIONS') {
      return route.fulfill({
        status: 204,
        headers: {
          'access-control-allow-origin': '*',
          'access-control-allow-methods': 'GET, POST, PUT, PATCH, DELETE, OPTIONS',
          'access-control-allow-headers': 'Authorization, Content-Type, Last-Event-ID',
        },
      })
    }

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
    if (pathName === '/api/v1/settings/managed' && method === 'GET') {
      return fulfill(route, { active_version: 2, snapshot: managedSettingsSnapshot })
    }
    if (pathName === '/api/v1/settings/managed/versions' && method === 'GET') {
      return fulfill(route, [
        {
          id: 'settings-v2',
          version: 2,
          snapshot: managedSettingsSnapshot,
          created_by_user_id: ids.user,
          rollback_of_version: null,
          created_at: now,
          is_active: true,
        },
        {
          id: 'settings-v1',
          version: 1,
          snapshot: {
            ...managedSettingsSnapshot,
            runtime: { ...managedSettingsSnapshot.runtime, hermes_command: 'hermes-old' },
          },
          created_by_user_id: ids.user,
          rollback_of_version: null,
          created_at: '2026-08-31T10:00:00+03:00',
          is_active: false,
        },
      ])
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
    if (pathName === '/api/v1/agents/storage-review') {
      return fulfill(route, storageReview(state.agents))
    }
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
      if (section === 'storage') return fulfill(route, storageReport(agent))
      if (section === 'config' && rest === 'revisions') return fulfill(route, [])
      if (section === 'readiness')
        return fulfill(route, {
          agent_id: agentId,
          runtime_healthy: true,
          ready_for_sdlc: false,
          effective_revision: null,
          blockers: ['workflow_assignment_protocol_not_verified'],
        })
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

    if (pathName === '/api/v1/workflow-catalog') {
      return fulfill(route, {
        namespaces: [{ id: '1', name: 'Основной', workflow_id: '1' }],
        workflows: [{ id: '1', name: 'sdlc-business-tech-v1' }],
      })
    }
    const bindingMatch = pathName.match(/^\/api\/v1\/workflow-bindings\/([^/]+)$/)
    if (bindingMatch && method === 'PUT') {
      const binding = state.workflowBindings.find((item) => item.agent_id === bindingMatch[1])
      if (!binding) return fulfill(route, { error: 'binding not found' }, 404)
      const body = (await request.postDataJSON()) as { namespace_id: string; workflow_id: string }
      binding.namespace_id = body.namespace_id
      binding.namespace_name = 'Основной'
      binding.workflow_id = body.workflow_id
      binding.workflow_name = 'sdlc-business-tech-v1'
      binding.binding_status = 'connected'
      return fulfill(route, binding)
    }
    if (pathName === '/api/v1/workflow-bindings') return fulfill(route, state.workflowBindings)
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
    headers: { 'access-control-allow-origin': '*' },
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
      binding_status: 'stale',
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

test('uses the shared work-area geometry across semantic page modes', async ({
  page,
}, testInfo) => {
  const state = createState()
  await installSsoMocks(page)
  await installMocks(page, state)

  for (const viewport of [
    { width: 375, height: 812 },
    { width: 1440, height: 900 },
    { width: 2560, height: 1440 },
  ]) {
    await page.setViewportSize(viewport)

    for (const [path, mode, heading] of [
      ['/', 'wide', 'Обзор агентов'],
      ['/settings', 'reading', 'Настройки'],
      [`/sessions/${ids.session}`, 'detail-with-aside', 'Initial developer task'],
    ] as const) {
      await page.goto(path)
      await expect(page.getByRole('heading', { name: heading, exact: true })).toBeVisible({
        timeout: 15_000,
      })
      const layout = page.locator('.shell-main > .page-frame > [data-page-layout]')
      await expect(layout).toHaveAttribute('data-page-layout', mode)

      const geometry = await layout.evaluate((element) => {
        const frame = element.parentElement
        const frameStyle = frame ? getComputedStyle(frame) : null
        return {
          documentFits:
            document.documentElement.scrollWidth <= document.documentElement.clientWidth,
          layoutWidth: element.getBoundingClientRect().width,
          availableWidth:
            (frame?.getBoundingClientRect().width ?? 0) -
            Number.parseFloat(frameStyle?.paddingLeft ?? '0') -
            Number.parseFloat(frameStyle?.paddingRight ?? '0'),
        }
      })

      expect(geometry.documentFits).toBe(true)
      if (mode === 'reading') {
        expect(geometry.layoutWidth).toBeLessThanOrEqual(761)
      } else {
        expect(Math.abs(geometry.layoutWidth - geometry.availableWidth)).toBeLessThanOrEqual(1)
      }
      await page.screenshot({
        path: testInfo.outputPath(`shell-${viewport.width}-${mode}.png`),
        fullPage: true,
      })
    }
  }
})

test('workflow bindings rebind only to a workflow in the selected namespace', async ({ page }) => {
  const state = createState()
  await installMocks(page, state)
  await page.goto('/workflows')

  await expect(page.getByText('Developer Hermes', { exact: true })).toBeVisible()
  await page.getByRole('button', { name: 'Перепривязать', exact: true }).click()
  await expect(page.getByText('sdlc-business-tech-v1').first()).toBeVisible()
})

test('managed settings require preview, preserve failed changes and expose rollback history', async ({
  page,
}, testInfo) => {
  const state = createState()
  const pageErrors: string[] = []
  page.on('pageerror', (error) => pageErrors.push(error.message))
  await installSsoMocks(page)
  await installMocks(page, state)
  const historicalSnapshot = {
    ...managedSettingsSnapshot,
    runtime: { ...managedSettingsSnapshot.runtime, hermes_command: 'hermes-old' },
  }
  const writes: unknown[] = []
  let releasePreview: (() => void) | null = null
  let holdPreview = false
  let failApply = true
  await page.route('**/api/v1/settings/managed**', async (route) => {
    const request = route.request()
    const pathName = new URL(request.url()).pathname
    const method = request.method()
    if (pathName === '/api/v1/settings/managed' && method === 'GET') {
      return fulfill(route, { active_version: 2, snapshot: managedSettingsSnapshot })
    }
    if (pathName === '/api/v1/settings/managed/versions' && method === 'GET') {
      return fulfill(route, [
        {
          id: 'settings-v2',
          version: 2,
          snapshot: managedSettingsSnapshot,
          created_by_user_id: ids.user,
          rollback_of_version: null,
          created_at: now,
          is_active: true,
        },
        {
          id: 'settings-v1',
          version: 1,
          snapshot: historicalSnapshot,
          created_by_user_id: ids.user,
          rollback_of_version: null,
          created_at: '2026-08-31T10:00:00+03:00',
          is_active: false,
        },
      ])
    }
    if (pathName === '/api/v1/settings/managed/preview' && method === 'POST') {
      const body = request.postDataJSON() as { snapshot: typeof managedSettingsSnapshot }
      if (holdPreview) {
        await new Promise<void>((resolve) => {
          releasePreview = resolve
        })
        holdPreview = false
      }
      return fulfill(route, {
        active_version: 2,
        restart_required: true,
        changes: [
          {
            path: 'runtime.hermes_command',
            before: managedSettingsSnapshot.runtime.hermes_command,
            after: body.snapshot.runtime.hermes_command,
            requires_restart: true,
          },
        ],
      })
    }
    if (pathName === '/api/v1/settings/managed/apply' && method === 'POST') {
      const body = request.postDataJSON()
      writes.push(body)
      if (failApply) {
        failApply = false
        return fulfill(route, { error: 'QA apply failure' }, 503)
      }
      return fulfill(route, {
        restart_scheduled: true,
        version: {
          id: 'settings-v3',
          version: 3,
          snapshot: (body as { snapshot: typeof managedSettingsSnapshot }).snapshot,
          created_by_user_id: ids.user,
          rollback_of_version: null,
          created_at: now,
          is_active: true,
        },
      })
    }
    if (pathName === '/api/v1/settings/managed/versions/1/rollback' && method === 'POST') {
      return fulfill(route, {
        restart_scheduled: true,
        version: {
          id: 'settings-v3',
          version: 3,
          snapshot: historicalSnapshot,
          created_by_user_id: ids.user,
          rollback_of_version: 1,
          created_at: now,
          is_active: true,
        },
      })
    }
    return fulfill(route, { error: `Unhandled managed settings route: ${method} ${pathName}` }, 404)
  })

  await page.setViewportSize({ width: 375, height: 812 })
  await page.goto('/settings')
  const command = page.getByRole('textbox', { name: 'Команда Hermes' })
  await expect(command).toHaveValue('hermes')
  await command.fill('hermes --qa')
  holdPreview = true
  await page.getByRole('button', { name: 'Проверить изменения' }).click()
  await expect(command).toBeDisabled()
  await expect(page.getByRole('button', { name: 'Проверяем...' })).toBeDisabled()
  await page.waitForTimeout(1000)
  await page.screenshot({
    path: testInfo.outputPath('settings-preview-pending-375.png'),
    fullPage: true,
  })
  releasePreview?.()
  const dialog = page.getByRole('alertdialog')
  await expect(dialog).toContainText('runtime.hermes_command')
  await page.waitForTimeout(1000)
  await page.screenshot({ path: testInfo.outputPath('settings-preview-375.png'), fullPage: true })

  await dialog.getByRole('button', { name: 'Применить и перезапустить' }).click()
  await expect(dialog.getByRole('alert')).toContainText('QA apply failure')
  await expect(dialog).toBeVisible()
  await expect(page.locator('#hermes-command')).toHaveValue('hermes --qa')
  await page.waitForTimeout(1000)
  await page.screenshot({
    path: testInfo.outputPath('settings-apply-error-375.png'),
    fullPage: true,
  })
  await dialog.getByRole('button', { name: 'Применить и перезапустить' }).click()
  await expect(dialog).toBeHidden()
  await expect(page.getByRole('status')).toContainText('перезапускается с новой версией')

  const tabs = [
    { name: 'Среда', text: 'Источники и команды' },
    { name: 'Порты', text: 'Сетевые порты' },
    { name: 'Интеграции', text: 'Интеграции' },
    { name: 'Доступ', text: 'Политика аутентификации' },
    { name: 'Хранение', text: 'Политика хранения' },
    { name: 'История', text: 'История версий' },
    { name: 'Пользователи', text: 'Пользователи' },
  ]
  for (const width of [375, 768, 1280, 1920]) {
    await page.setViewportSize({ width, height: width <= 768 ? 812 : 1080 })
    for (const tab of tabs) {
      await page.getByRole('tab', { name: tab.name }).click()
      await expect(page.getByText(tab.text, { exact: true }).first()).toBeVisible()
      const layout = await page.evaluate(() => {
        const visible = (element: Element) => element.getClientRects().length > 0
        const controls = [
          ...document.querySelectorAll('main button, main input, main select'),
        ].filter(visible)
        return {
          scrollWidth: document.documentElement.scrollWidth,
          smallTargets: controls
            .filter((element) => {
              if (element instanceof HTMLInputElement && element.type === 'checkbox') return false
              const rect = element.getBoundingClientRect()
              return rect.width < 40 || rect.height < 40
            })
            .map((element) => {
              const rect = element.getBoundingClientRect()
              return {
                label: element.getAttribute('aria-label') ?? element.textContent?.trim(),
                width: rect.width,
                height: rect.height,
              }
            }),
          unnamedFields: controls
            .filter(
              (element) =>
                (element instanceof HTMLInputElement || element instanceof HTMLSelectElement) &&
                !element.labels?.length &&
                !element.getAttribute('aria-label'),
            )
            .map((element) => element.outerHTML.slice(0, 100)),
        }
      })
      expect(layout.scrollWidth).toBe(width)
      expect(layout.smallTargets).toEqual([])
      expect(layout.unnamedFields).toEqual([])
      if ((width === 375 || width === 1920) && tab.name === 'История') {
        await page.waitForTimeout(1000)
        await page.screenshot({
          path: testInfo.outputPath(`settings-history-${width}.png`),
          fullPage: true,
        })
      }
    }
  }
  await page.getByRole('tab', { name: 'Среда' }).click()
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark')
  await page.getByRole('button', { name: 'Аккаунт', exact: true }).click()
  await page.getByRole('menuitemradio', { name: 'Серая', exact: true }).click()
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'gray')
  await page.getByRole('button', { name: 'Аккаунт', exact: true }).click()
  await expect(page.getByRole('menuitemradio', { name: 'Серая', exact: true })).toHaveAttribute(
    'aria-checked',
    'true',
  )
  await page.getByRole('menuitemradio', { name: 'Светлая', exact: true }).click()
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light')
  await page.waitForTimeout(1000)
  await page.screenshot({ path: testInfo.outputPath('settings-light-1280.png'), fullPage: true })
  expect(writes).toHaveLength(2)
  expect(writes[0]).toEqual({
    snapshot: {
      ...managedSettingsSnapshot,
      runtime: { ...managedSettingsSnapshot.runtime, hermes_command: 'hermes --qa' },
    },
    expected_active_version: 2,
    confirm_restart: true,
  })
  expect(writes[1]).toEqual(writes[0])
  expect(pageErrors).toEqual([])
})

test('storage review links purge candidates to their workspace', async ({ page }) => {
  const state = createState()
  const tester = state.agents.find((agent) => agent.id === ids.tester)
  if (!tester) throw new Error('tester fixture is missing')
  tester.status = 'archived'
  await installMocks(page, state)

  await page.goto('/agents')

  await expect(page.locator(`a[href="/agents/${ids.tester}/workspace"]`)).toBeVisible()
})

test('canonical heartbeat alert is localized and acknowledged at all required viewports', async ({
  page,
}, testInfo) => {
  await installMocks(page, createState())
  let state = 'open'
  await page.route('**/api/v1/fleet-alerts**', async (route) => {
    const acknowledgement = route.request().url().endsWith('/acknowledge')
    if (acknowledgement) state = 'acknowledged'
    const alert = {
      id: '00000000-0000-4000-8000-000000000a01',
      agent_id: ids.dev,
      kind: 'heartbeat_stale',
      severity: 'warning',
      detail: { last_health_at: now },
      state,
      opened_at: now,
      resolved_at: null,
      acknowledged_at: state === 'acknowledged' ? now : null,
      acknowledged_by_user_id: state === 'acknowledged' ? ids.user : null,
    }
    return fulfill(route, acknowledgement ? alert : [alert])
  })

  for (const viewport of [
    { width: 375, height: 812 },
    { width: 1920, height: 1080 },
    { width: 2560, height: 1440 },
  ]) {
    state = 'open'
    await page.setViewportSize(viewport)
    await page.goto('/alerts')
    await expect(page.getByRole('heading', { name: 'Нет свежего сигнала агента' })).toBeVisible()
    await expect(page.getByText('Developer Hermes · agent1')).toBeVisible()
    expect(
      await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth + 1),
    ).toBe(true)
    await page.screenshot({
      path: testInfo.outputPath(`heartbeat-fixture-${viewport.width}x${viewport.height}.png`),
      fullPage: true,
    })
    await page
      .getByRole('button', {
        name: 'Подтвердить получение оповещения «Нет свежего сигнала агента»',
      })
      .click()
    await expect(
      page.locator('li[aria-busy]').getByText('Подтверждено', { exact: true }).first(),
    ).toBeVisible()
    await expect(
      page.getByRole('button', { name: /Подтвердить получение оповещения/ }),
    ).toHaveCount(0)
  }
})

test('Chats groups private sessions by agent and keeps leader controls out of the new route', async ({
  page,
}) => {
  const state = createState()
  await installMocks(page, state)
  await page.goto('/chats')
  await expect(page.getByRole('heading', { name: 'Чаты', exact: true })).toBeVisible()
  await expect(page.getByRole('link', { name: /Initial developer task/ })).toBeVisible()
  await page.getByRole('button', { name: 'Убрать фильтр по Fleet Admin' }).click()
  await page.getByRole('button', { name: /Tester Hermes/ }).click()
  await expect(page.getByRole('link', { name: /Tester review sweep/ })).toBeVisible()
  await expect(page.getByRole('link', { name: /Initial developer task/ })).not.toBeVisible()
  await page.getByRole('button', { name: /Developer Hermes/ }).click()
  await page.getByRole('link', { name: /Initial developer task/ }).click()
  await expect(page.getByLabel('Лидер сессии')).not.toBeVisible()
  await expect(page.getByLabel('Новый основной агент')).not.toBeVisible()
  await expect(page.getByPlaceholder('Напишите сообщение для этой сессии')).toBeVisible()
})

test('Hermes fleet control flow covers agents, runtime, skills, sessions and handoff', async ({
  page,
}) => {
  test.setTimeout(90_000)
  const state = createState()
  await installMocks(page, state)

  await page.goto('/')
  await expect(page.getByRole('heading', { name: 'Обзор агентов' })).toBeVisible()
  await expect(page.getByText(/agent1.*разработчик.*dev/).first()).toBeVisible()
  await expect(page.getByText(/agent2.*тестировщик.*qa/).first()).toBeVisible()
  await expect(page.getByText(/agent3.*технический руководитель.*dev/).first()).toBeVisible()

  await expect(page.getByRole('link', { name: 'Лидеры' })).not.toBeVisible()
  await page.goto('/leaders')
  await expect(page.getByRole('heading', { name: 'Лидеры' })).toBeVisible()
  await expect(page.getByRole('link', { name: /Tester review sweep/ })).not.toBeVisible()
  await page.getByRole('button', { name: 'Убрать фильтр по Fleet Admin' }).click()
  await page.locator('main summary').first().click()
  await expect(page.getByRole('link', { name: /Tester review sweep/ })).toBeVisible()
  await page.goto(`/leaders/${ids.lead}/edit`)
  await expect(page.getByRole('heading', { name: 'Изменить IT Lead Hermes' })).toBeVisible()
  await expect(page.getByText('Исполнители команды')).toBeVisible()
  await page.getByRole('button', { name: 'Сохранить агента' }).click()
  await expect(page).toHaveURL(new RegExp(`/leaders/${ids.lead}$`))

  await page.goto('/executors')
  await expect(page.getByRole('heading', { name: 'Исполнители' })).toBeVisible()
  await expect(page.getByText('Developer Hermes')).toBeVisible()

  await page.getByRole('link', { name: 'Агенты' }).click()
  await expect(page.getByRole('heading', { name: 'Хранилище агентов' })).toBeVisible()
  await expect(page.getByText('Занято')).toBeVisible()
  await expect(page.getByText('Все управляемые каталоги имеют корректные маркеры')).toBeVisible()
  await expect(page.getByRole('link', { name: /Initial developer task/ })).toBeVisible()
  await expect(page.getByRole('link', { name: /Tester review sweep/ })).not.toBeVisible()
  await page.getByRole('link', { name: 'Создать агента' }).click()
  await expect(page.getByRole('heading', { name: 'Создание агента' })).toBeVisible()
  await expect(page.getByRole('button', { name: /Hermes Реализовано/ })).toBeVisible()
  await expect(page.getByRole('button', { name: /Java Agent Запланировано/ })).toBeVisible()
  await page.getByLabel('Специализация', { exact: true }).selectOption('tester')
  await expect(page.getByLabel('Отображаемое имя')).toHaveValue('Tester Hermes')
  await page.getByRole('button', { name: 'Создать агента' }).click()
  await expect(page).toHaveURL(new RegExp(`/executors/${ids.created}$`))
  await expect(page.getByRole('heading', { level: 1, name: 'Tester Hermes' })).toBeVisible()

  await page.goto(`/agents/${ids.dev}/runtime`)
  await page.getByRole('button', { exact: true, name: 'Остановить' }).click()
  await expect(page.getByText('Остановлен').first()).toBeVisible()
  await page.getByRole('button', { exact: true, name: 'Запустить' }).click()
  await expect(page.getByText('Работает').first()).toBeVisible()

  await page.goto(`/agents/${ids.dev}/workspace`)
  await expect(page.getByRole('heading', { name: 'Использование диска' })).toBeVisible()
  await expect(page.getByText('Общий размер', { exact: true }).first()).toBeVisible()
  await expect(page.getByText('Маркер проверен', { exact: true })).toBeVisible()
  await expect(page.getByRole('heading', { name: 'Удаление файлов' })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Удалить файлы' })).toBeDisabled()

  await page.goto(`/agents/${ids.dev}/skills`)
  await page.getByRole('button', { name: /GitHub Commit and PR/ }).click()
  await page.getByRole('textbox').fill('# GitHub Commit and PR\n\nCustom agent override.')
  await page.getByRole('button', { name: 'Сохранить навык' }).click()
  await expect(page.getByText('Для этого агента задана локальная версия навыка.')).toBeVisible()

  await page.goto('/sessions')
  await expect(page.getByRole('link', { name: /Initial developer task/ })).toBeVisible()
  await expect(page.getByRole('link', { name: /Tester review sweep/ })).not.toBeVisible()
  await page.getByRole('button', { name: 'Убрать фильтр по Fleet Admin' }).click()
  await expect(page.getByRole('link', { name: /Tester review sweep/ })).toBeVisible()
  await page.getByLabel('Добавить пользователя').selectOption(ids.reviewer)
  await expect(page.getByRole('link', { name: /Initial developer task/ })).not.toBeVisible()
  await expect(page.getByRole('link', { name: /Tester review sweep/ })).toBeVisible()
  await page.getByLabel('Добавить пользователя').selectOption(ids.user)
  await expect(page.getByRole('link', { name: /Initial developer task/ })).toBeVisible()
  await expect(page.getByRole('link', { name: /Tester review sweep/ })).toBeVisible()
  await page.getByLabel('Агент', { exact: true }).selectOption(ids.dev)
  await expect(page.getByLabel('Лидер')).toHaveValue('')
  await page.getByLabel('Название').fill('Create checkout smoke')
  await page.getByLabel('Ключ задачи').fill('FC-777')
  await page.getByRole('button', { name: 'Создать сессию' }).click()
  await expect(page.getByRole('link', { name: /Create checkout smoke/ })).toBeVisible()

  await page.goto(`/sessions/${ids.createdSession}`)
  await page.getByLabel('Лидер сессии').selectOption(ids.lead)
  await page.getByRole('button', { name: 'Сохранить лидера' }).click()
  await expect(page.getByText('Для лидера')).toBeVisible()
  await expect(page.getByLabel('Отправитель').locator('option[value="leader"]')).toBeDisabled()
  await page.getByLabel('Отправитель').selectOption('user')
  await page
    .getByPlaceholder('Напишите сообщение для этой сессии')
    .fill('Please coordinate the smoke test.')
  await page.getByRole('button', { name: 'Отправить', exact: true }).click()
  await expect(
    page.locator('p').filter({ hasText: 'Please coordinate the smoke test.' }).first(),
  ).toBeVisible()
  await page.getByLabel('Исполнитель').selectOption(ids.tester)
  await page.getByLabel('Название задачи').fill('Delegated QA smoke')
  await page
    .getByPlaceholder('Опишите результат, который должен подготовить исполнитель')
    .fill('Run the delegated QA sweep.')
  await page.getByRole('button', { name: 'Делегировать', exact: true }).click()
  await expect(page.getByText('Создана связанная сессия «Delegated QA smoke»')).toBeVisible()
  await page.goto('/sessions')
  await page.getByRole('button', { name: 'Убрать фильтр по Fleet Admin' }).click()
  await expect(page.getByRole('link', { name: /Delegated QA smoke/ })).toBeVisible()
  await page.goto(`/sessions/${ids.createdSession}`)
  await page.getByLabel('Новый основной агент').selectOption(ids.tester)
  await page.getByRole('button', { name: 'Передать сессию' }).click()
  await expect(page.getByText('Ожидает передачи')).toBeVisible()
  await expect(page.getByText('agent2', { exact: true })).toBeVisible()

  await page.goto('/deployments?tab=jobs')
  await expect(page.getByRole('heading', { name: 'Развёртывания', exact: true })).toBeVisible()
  await page.locator('#deployment-title').fill('Runtime update dry run')
  await page.getByRole('button', { name: 'Создать задание' }).click()
  await expect(
    page.getByRole('button', { name: 'Runtime update dry run', exact: true }),
  ).toBeVisible()
  await page.getByRole('button', { name: 'Отменить задание «Runtime update dry run»' }).click()
  const cancelDialog = page.getByRole('alertdialog')
  await expect(cancelDialog.getByRole('heading', { name: 'Отменить задание?' })).toBeVisible()
  await cancelDialog.getByRole('button', { name: 'Отменить задание' }).click()
  await expect(page.getByText('Отменено').first()).toBeVisible()

  await page.goto('/logs?tab=events')
  await expect(page.getByRole('region', { name: 'События', exact: true })).toBeVisible()
  await expect(page.getByText('runtime_started')).toBeVisible()
  await page.goto('/logs?tab=audit')
  await expect(page.getByRole('region', { name: 'Аудит', exact: true })).toBeVisible()
  await expect(page.getByText('session.create')).toBeVisible()

  await page.goto('/settings?tab=users')
  await expect(page.getByRole('heading', { name: 'Пользователи' })).toBeVisible()
  await expect(page.getByRole('link', { name: 'Открыть в Admin Panel' })).toBeVisible()
  await expect(page.getByText('QA Reviewer')).toBeVisible()

  await page.goto('/access-denied')
  await expect(page.getByRole('heading', { name: 'Нет доступа' })).toBeVisible()
  await page.goto('/not-a-fleet-route')
  await expect(page.getByRole('heading', { name: 'Объект не найден' })).toBeVisible()
})
