import { test as base, expect, type Page, type TestInfo } from '@playwright/test'
import { createServer } from 'node:http'
import { createHash } from 'node:crypto'
import { mkdir } from 'node:fs/promises'
import { resolve } from 'node:path'
import { installSsoMocks } from './chats-core-sso'

const userId = '00000000-0000-4000-8000-000000000001'
const agentId = '00000000-0000-4000-8000-000000000101'
const sessionId = '00000000-0000-4000-8000-000000000201'
const now = '2026-10-07T10:00:00Z'
const agent = {
  id: agentId,
  name: 'agent1',
  display_name: 'Developer Hermes',
  kind: 'hermes',
  product_role: 'executor',
  role: 'developer',
  sdlc_role: 'developer',
  status: 'running',
}
const session = {
  id: sessionId,
  agent_id: agentId,
  primary_agent_id: agentId,
  agent_name: 'agent1',
  primary_agent_name: 'agent1',
  user_id: userId,
  user_display_name: 'Owner',
  title: 'Production Chats core',
  task_key: 'CORE-1',
  visibility: 'private',
  state: 'active',
  created_at: now,
  updated_at: now,
}
function message(body: string, id = 'm1') {
  return {
    id,
    session_id: sessionId,
    author_type: 'user',
    author_user_id: userId,
    author_agent_id: null,
    author_display_name: 'Owner',
    body,
    delivery_state: 'completed',
    message_kind: 'user_prompt',
    created_at: now,
    replayed: false,
  }
}
type State = {
  actor: string
  permissions: string[]
  denied: boolean
  runs: unknown[]
  messages: ReturnType<typeof message>[]
  posts: { path: string; body: Record<string, string>; key?: string }[]
  controlLookups: { runId: string; key: string | undefined }[]
  requests: string[]
  unknown: boolean
  preflightDenied: boolean
  creates: number
  createdTitle: string | null
  rejection: number | null
  receiptBody: string | null
  receiptHash: 'correct' | 'missing' | 'wrong'
  pendingDelivery: boolean | null
  taskContextDenied: boolean
  controlRunId: string | null
  controlCapabilities: boolean
  controlUnknown: boolean
}
type Stream = { url: string; emit: (data: unknown) => void; connected: () => number }
const test = base.extend<{ stream: Stream }>({
  stream: async ({ baseURL }, provide) => {
    const clients = new Set<import('node:http').ServerResponse>()
    const server = createServer((request, response) => {
      response.setHeader('Access-Control-Allow-Origin', new URL(baseURL!).origin)
      response.setHeader('Access-Control-Allow-Headers', 'Authorization, Accept, Last-Event-ID')
      if (request.method === 'OPTIONS') {
        response.writeHead(204).end()
        return
      }
      if (request.method !== 'GET' || request.headers.authorization !== 'Bearer qa-access-token') {
        response.writeHead(403).end()
        return
      }
      response.writeHead(200, {
        'Content-Type': 'text/event-stream',
        'Cache-Control': 'no-cache',
      })
      response.write(': connected\n\n')
      clients.add(response)
      const keepAlive = setInterval(() => response.write(': fixture heartbeat\n\n'), 1000)
      response.on('close', () => {
        clearInterval(keepAlive)
        clients.delete(response)
      })
    })
    await new Promise<void>((done) => server.listen(0, '127.0.0.1', done))
    const address = server.address()
    if (!address || typeof address === 'string') throw new Error('No stream address')
    try {
      await provide({
        url: `http://127.0.0.1:${address.port}/stream`,
        connected: () => clients.size,
        emit: (data) => {
          for (const client of clients)
            client.write(`event: session\ndata: ${JSON.stringify(data)}\n\n`)
        },
      })
    } finally {
      for (const client of clients) client.end()
      server.closeAllConnections()
      await new Promise<void>((done) => server.close(() => done()))
    }
  },
})
async function install(page: Page, stream: Stream, overrides: Partial<State> = {}) {
  const state: State = {
    actor: userId,
    permissions: [
      'sessions:read_own',
      'sessions:write_own',
      'agents:read_directory',
      'sessions:read_all',
    ],
    denied: false,
    runs: [],
    messages: [
      message(
        'Discuss the current implementation. The PM integration is unavailable in this build.',
      ),
    ],
    posts: [],
    controlLookups: [],
    requests: [],
    unknown: false,
    receiptBody: null,
    receiptHash: 'correct',
    pendingDelivery: false,
    taskContextDenied: false,
    controlRunId: null,
    controlCapabilities: false,
    controlUnknown: false,
    preflightDenied: false,
    creates: 0,
    createdTitle: null,
    rejection: null,
    ...overrides,
  }
  await installSsoMocks(page, () => state.actor)
  await page.route('**/api/v1/**', async (route) => {
    const req = route.request()
    const url = new URL(req.url())
    const path = url.pathname
    const reply = (value: unknown, status = 200) =>
      route.fulfill({
        status,
        contentType: 'application/json',
        headers: { 'access-control-allow-origin': '*' },
        body: JSON.stringify(value),
      })
    if (req.method() === 'OPTIONS')
      return route.fulfill({
        status: 204,
        headers: {
          'access-control-allow-origin': '*',
          'access-control-allow-methods': 'GET, POST, OPTIONS',
          'access-control-allow-headers': 'Authorization, Content-Type, Last-Event-ID',
        },
      })
    state.requests.push(path)
    if (path.endsWith('/stream')) return route.continue({ url: stream.url })
    const user = {
      id: state.actor,
      email: 'owner@example.test',
      username: 'owner',
      display_name: 'Owner',
      system_role: 'user',
      is_system_admin: false,
      is_active: true,
    }
    if (path.endsWith('/auth/refresh'))
      return reply({ access_token: 'qa-access-token', user_id: state.actor, ...user })
    if (path.endsWith('/users/me/permissions'))
      return reply({
        user_id: state.actor,
        role: 'user',
        is_system_admin: false,
        permissions: state.permissions,
      })
    if (path.endsWith('/users/me')) return reply(user)
    if (path === '/api/v1/users') return reply({ users: [user] })
    if (path === '/api/v1/agent-directory') return reply([agent])
    if (path === '/api/v1/chats/directory' && req.method() === 'GET') {
      const query = url.searchParams
      const invalid = (message: string) =>
        reply({ error: { code: 'VALIDATION_ERROR', message } }, 422)
      if (
        [...query.keys()].some(
          (key) => !['agent_id', 'user_id', 'q', 'before', 'limit'].includes(key),
        )
      )
        throw new Error('Unsupported directory fixture query')
      const limit = query.get('limit') ?? '50'
      if (!/^\d+$/.test(limit)) throw new Error('Unsupported directory fixture limit')
      if (Number(limit) < 1 || Number(limit) > 100)
        return invalid('limit must be between 1 and 100')
      const search = query.get('q')?.trim() ?? ''
      if ([...search].length > 200) return invalid('q must contain at most 200 characters')
      if (query.has('agent_id') && query.get('agent_id') !== agentId)
        return invalid('invalid or archived agent_id')
      if (query.has('before') && !query.has('agent_id')) return invalid('before requires agent_id')
      const scope = query.get('user_id')?.trim() ?? state.actor
      const allUsers = !scope || scope.toLowerCase() === 'all'
      const users = scope
        .split(',')
        .map((id) => id.trim())
        .filter(Boolean)
      if (
        !allUsers &&
        users.some((id) => !/^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/i.test(id))
      )
        return invalid('invalid user_id')
      if (
        !state.permissions.includes('sessions:read_all') &&
        (allUsers || users.some((id) => id !== state.actor))
      )
        return reply({ error: { code: 'FORBIDDEN', message: 'forbidden' } }, 403)
      const item = { ...session, title: state.createdTitle ?? session.title }
      const matches =
        state.actor === item.user_id &&
        (allUsers || users.includes(item.user_id)) &&
        [item.title, item.task_key, item.user_display_name].some((value) =>
          value.toLowerCase().includes(search.toLowerCase()),
        )
      if (query.has('before') && (!matches || query.get('before') !== sessionId))
        return invalid('invalid cursor for the selected scope')
      return reply({
        agents: [
          {
            agent: {
              ...agent,
              ordinal: 1,
              description: null,
              namespace_id: null,
              workflow_id: null,
              runtime_version: null,
              dashboard_port: null,
              api_port: null,
            },
            matching_session_count: matches ? 1 : 0,
          },
        ],
        selected_agent_id: agentId,
        items:
          matches && !query.has('before')
            ? [
                {
                  ...item,
                  user_email: 'owner@example.test',
                  user_username: 'owner',
                  leader_agent_id: null,
                  leader_agent_name: null,
                  parent_session_id: null,
                  created_by_leader_agent_id: null,
                  namespace_id: null,
                  external_session_id: null,
                  last_message_preview: null,
                },
              ]
            : [],
        next_before: null,
      })
    }
    if (path === `/api/v1/sessions/${sessionId}/task-context` && req.method() === 'GET')
      return state.denied || state.preflightDenied || state.taskContextDenied
        ? reply({ error: { code: 'FORBIDDEN', message: 'forbidden' } }, 403)
        : reply({ binding: null, tracker: null })
    if (
      path === `/api/v1/sessions/${sessionId}/history` &&
      req.method() === 'GET' &&
      (state.denied || state.preflightDenied || state.taskContextDenied)
    )
      return reply({ error: { code: 'FORBIDDEN', message: 'forbidden' } }, 403)
    if (path === `/api/v1/sessions/${sessionId}/chat-controls` && req.method() === 'GET') {
      if (state.denied || state.preflightDenied || state.taskContextDenied)
        return reply({ error: { code: 'FORBIDDEN', message: 'forbidden' } }, 403)
      const active = [...state.runs].reverse().find((value) => {
        const run = value as Record<string, unknown>
        return (
          ['pending', 'running', 'waiting', 'stopping'].includes(String(run.state)) &&
          (run.state !== 'pending' ||
            run.runtime_session_id != null ||
            run.runtime_run_id != null ||
            run.last_event_at != null)
        )
      }) as Record<string, unknown> | undefined
      const owner = state.actor === session.user_id
      const pending = state.pendingDelivery !== false
      return reply({
        can_send: owner && !active && !pending,
        can_steer: owner && state.controlCapabilities && active?.id === state.controlRunId,
        can_stop: owner && state.controlCapabilities && active?.id === state.controlRunId,
        active_run_id: active?.id ?? null,
        blocked_reason: !owner
          ? 'read_only'
          : pending || (active && active.runtime_run_id == null)
            ? 'dispatch_pending_or_uncertain'
            : null,
      })
    }
    if (path === '/api/v1/sessions' && req.method() === 'GET') return reply([session])
    if (path === `/api/v1/sessions/${sessionId}`)
      return reply(
        state.denied || state.preflightDenied
          ? { error: { message: 'Denied' } }
          : {
              ...session,
              title: state.createdTitle ?? session.title,
              pending_delivery: state.pendingDelivery,
              task_bound: false,
            },
        state.denied || state.preflightDenied ? 403 : 200,
      )
    if (path === `/api/v1/sessions/${sessionId}/messages` && req.method() === 'GET')
      return reply(state.messages)
    const controlRead = path.match(
      new RegExp(`^/api/v1/sessions/${sessionId}/runs/([^/]+)/controls(/lookup)?$`),
    )
    if (controlRead && req.method() === 'GET') {
      const runId = controlRead[1]
      const key = req.headers()['idempotency-key']
      const originals = state.posts.filter(
        (command) =>
          command.path === `/api/v1/sessions/${sessionId}/runs/${runId}/steer` ||
          command.path === `/api/v1/sessions/${sessionId}/runs/${runId}/stop`,
      )
      const receipts = originals.map((command) => ({
        id: '00000000-0000-4000-8000-000000000301',
        session_id: sessionId,
        session_run_id: runId,
        agent_id: agentId,
        actor_user_id: userId,
        operation: command.path.endsWith('/steer') ? 'steer' : 'stop',
        state: 'uncertain',
        acknowledgement: null,
        observed_run_state: null,
        created_at: now,
        updated_at: now,
      }))
      if (!controlRead[2]) return reply(receipts)
      state.controlLookups.push({ runId, key })
      const index = originals.findIndex((command) => command.key === key)
      return index < 0
        ? reply({ error: { code: 'NOT_FOUND', message: 'Original command not found' } }, 404)
        : reply(receipts[index])
    }
    if (path.endsWith('/runs')) return reply(state.runs)
    if (path.endsWith('/history')) return reply({ items: state.messages, next_before: null })
    if (req.method() === 'POST') {
      const body = req.postDataJSON() as Record<string, string>
      state.posts.push({ path, body, key: req.headers()['idempotency-key'] })
      if (
        state.controlRunId &&
        (path === `/api/v1/sessions/${sessionId}/runs/${state.controlRunId}/steer` ||
          path === `/api/v1/sessions/${sessionId}/runs/${state.controlRunId}/stop`)
      ) {
        if (state.controlUnknown) return route.abort('failed')
        return reply({
          session_id: sessionId,
          run_id: state.controlRunId,
          runtime_run_id: 'controlled-runtime-run',
          accepted: true,
          state: 'running',
          message: 'Accepted',
        })
      }
      if (state.rejection)
        return reply({ error: { message: 'Conflicting original key' } }, state.rejection)
      if (state.unknown) return route.abort('failed')
      if (path === '/api/v1/sessions') {
        state.creates++
        state.createdTitle = body.title
        return reply({ ...session, title: body.title })
      }
      if (path.endsWith('/messages')) {
        const requestHash = createHash('sha256')
          .update(
            JSON.stringify({
              author_agent_id: body.author_agent_id ?? null,
              body: body.body,
              idempotency_key: body.idempotency_key ?? null,
              message_kind: body.message_kind ?? null,
              runtime_message_id: body.runtime_message_id ?? null,
            }),
          )
          .digest('hex')
        const saved = {
          ...message(state.receiptBody ?? body.body, 'saved'),
          request_payload_hash:
            state.receiptHash === 'missing'
              ? undefined
              : state.receiptHash === 'wrong'
                ? '0'.repeat(64)
                : requestHash,
        }
        state.messages.push(saved)
        return reply(saved)
      }
    }
    return reply({ error: { message: `Unsupported ${path}` } }, 404)
  })
  return state
}
const sizes = [
  { width: 375, height: 812 },
  { width: 1920, height: 1080 },
  { width: 2560, height: 1440 },
]

test('unknown steer stays held after reload and never becomes an ordinary prompt', async ({
  page,
  stream,
}) => {
  const state = await install(page, stream, {
    controlRunId: 'controlled-run',
    controlCapabilities: true,
    controlUnknown: true,
    runs: [
      {
        id: 'controlled-run',
        session_id: sessionId,
        agent_id: agentId,
        state: 'running',
        runtime_session_id: 'controlled-session',
        runtime_run_id: 'controlled-runtime-run',
      },
    ],
  })
  await page.goto(`/chats/${sessionId}`)
  await page.getByLabel('Уточнение активному запуску').fill('Original private steer')
  await page.getByRole('button', { name: 'Передать уточнение запуску' }).click()
  await expect(page.getByText(/Нельзя повторить его как новый prompt/)).toBeVisible()
  expect(state.posts).toHaveLength(1)
  expect(state.posts[0].body).toEqual({ input: 'Original private steer' })
  const originalKey = state.posts[0].key
  expect(originalKey).toBeTruthy()
  const metadata = await page.evaluate(() => JSON.stringify(sessionStorage))
  expect(metadata).not.toContain('Original private steer')
  expect(metadata).not.toContain('qa-access-token')
  state.runs = []
  state.controlRunId = null
  state.controlCapabilities = false
  await page.reload()
  await expect(page.getByLabel('Сообщение агенту', { exact: true })).toBeDisabled()
  await expect(
    page.getByRole('button', { name: 'Отправить сообщение', exact: true }),
  ).toBeDisabled()
  expect(state.posts).toHaveLength(1)
  expect(state.posts[0].path).toMatch(/\/runs\/controlled-run\/steer$/)
  await expect.poll(() => state.controlLookups.length).toBeGreaterThan(0)
  expect(
    state.controlLookups.every(
      (lookup) => lookup.runId === 'controlled-run' && lookup.key === originalKey,
    ),
  ).toBe(true)
})
async function capture(page: Page, info: TestInfo, name: string) {
  if (info.project.name !== 'chromium') return
  const viewport = page.viewportSize()!
  const dir = resolve('../docs/assets/screens/chats-core-main-20261007')
  await mkdir(dir, { recursive: true })
  await page.screenshot({
    path: resolve(dir, `${name}-${viewport.width}x${viewport.height}.png`),
    fullPage: true,
    animations: 'disabled',
  })
}

test('dialogue and unavailable PM tabs fit all sizes and keep context accessible', async ({
  page,
  stream,
}, info) => {
  const state = await install(page, stream)
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto(`/chats/${sessionId}`)
  await expect(page.getByLabel('Сообщение', { exact: true })).toBeVisible()
  for (const size of sizes) {
    await page.setViewportSize(size)
    await page.getByRole('tab', { name: 'Диалог', exact: true }).click()
    await expect(page.getByText(/Discuss the current implementation/)).toBeVisible()
    await capture(page, info, 'dialogue')
    for (const name of ['Уточнения', 'Требования']) {
      await page.getByRole('tab', { name, exact: true }).click()
      await expect(page.getByRole('tab', { name, exact: true })).toHaveAttribute(
        'aria-selected',
        'true',
      )
      await expect(page.getByRole('tabpanel').getByText(/PM не подключён/)).toBeVisible()
      await expect(page.getByRole('tabpanel').getByRole('button')).toBeDisabled()
      await capture(
        page,
        info,
        name === 'Уточнения' ? 'clarification-unavailable' : 'requirements-unavailable',
      )
    }
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
  }
  await page.setViewportSize(sizes[0])
  await page.getByRole('button', { name: 'Контекст', exact: true }).click()
  await expect(page.getByRole('dialog').getByText('Owner', { exact: true })).toBeVisible()
  expect(state.posts).toEqual([])
  expect(state.requests).toContain(`/api/v1/sessions/${sessionId}/task-context`)
  expect(state.requests).toContain(`/api/v1/sessions/${sessionId}/chat-controls`)
  expect(
    state.requests.some((path) => /clarifications|requirements|controls\/lookup/.test(path)),
  ).toBe(false)
  expect(errors).toEqual([])
})

test('unknown prompt freezes payload, retries original key and releases only a matching receipt', async ({
  page,
  stream,
}) => {
  const state = await install(page, stream, { unknown: true })
  await page.goto(`/chats/${sessionId}`)
  await page.getByLabel('Сообщение', { exact: true }).fill('Private original prompt')
  await page.getByRole('button', { name: 'Отправить', exact: true }).click()
  await expect(page.getByText(/Ответ неизвестен/)).toBeVisible()
  await expect(page.getByLabel('Сообщение', { exact: true })).toBeDisabled()
  expect(state.posts).toHaveLength(1)
  const metadata = await page.evaluate(() => JSON.stringify(sessionStorage))
  expect(metadata).not.toContain('Private original prompt')
  state.unknown = false
  await page.getByRole('button', { name: 'Повторить исходный запрос' }).click()
  await expect(page.getByLabel('Сообщение', { exact: true })).toHaveValue('')
  expect(state.posts).toHaveLength(2)
  expect(state.posts[1].body).toEqual(state.posts[0].body)
})

test('a matching redacted prompt receipt releases the original command', async ({
  page,
  stream,
}) => {
  const state = await install(page, stream, { receiptBody: 'Discuss password=redacted' })
  await page.goto(`/chats/${sessionId}`)
  await page.getByLabel('Сообщение', { exact: true }).fill('Discuss password=fixture-value')
  await page.getByRole('button', { name: 'Отправить', exact: true }).click()
  await expect(page.getByLabel('Сообщение', { exact: true })).toHaveValue('')
  await expect(page.getByLabel('Сообщение', { exact: true })).toBeEnabled()
  await expect(page.getByText('Discuss password=redacted', { exact: true })).toBeVisible()
  expect(state.posts).toHaveLength(1)
})

for (const receiptHash of ['missing', 'wrong'] as const) {
  test(`a ${receiptHash} request hash cannot release an original prompt`, async ({
    page,
    stream,
  }) => {
    const state = await install(page, stream, { receiptHash })
    await page.goto(`/chats/${sessionId}`)
    await page.getByLabel('Сообщение', { exact: true }).fill('Original prompt')
    await page.getByRole('button', { name: 'Отправить', exact: true }).click()
    await expect(page.getByLabel('Сообщение', { exact: true })).toBeDisabled()
    await expect(page.getByLabel('Сообщение', { exact: true })).toHaveValue('Original prompt')
    await expect(page.getByText(/Исходная команда требует сверки/)).toBeVisible()
    expect(state.posts).toHaveLength(1)
  })
}

test('reload and another actor retain unknown prompt hold without redispatch', async ({
  page,
  stream,
}) => {
  const state = await install(page, stream, { unknown: true })
  page.on('dialog', (dialog) => void dialog.accept())
  await page.goto(`/chats/${sessionId}`)
  await page.getByLabel('Сообщение', { exact: true }).fill('Sensitive reload draft')
  await page.getByRole('button', { name: 'Отправить', exact: true }).click()
  await expect(page.getByText(/Ответ неизвестен/)).toBeVisible()
  state.actor = '00000000-0000-4000-8000-000000000002'
  await page.reload()
  await expect(page.getByText(/Исходная команда требует сверки/)).toBeVisible()
  await expect(page.getByLabel('Сообщение', { exact: true })).toHaveValue('')
  await expect(page.getByRole('button', { name: 'Отправить', exact: true })).toBeDisabled()
  expect(state.posts).toHaveLength(1)
})

test('preflight ACL denial sends no POST', async ({ page, stream }) => {
  const state = await install(page, stream)
  await page.goto(`/chats/${sessionId}`)
  await page.getByLabel('Сообщение', { exact: true }).fill('Draft before revoked access')
  await expect(page.getByRole('button', { name: 'Отправить', exact: true })).toBeEnabled()
  state.preflightDenied = true
  await page.getByRole('button', { name: 'Отправить', exact: true }).click()
  await expect(page.getByText(/Сообщение не подтверждено/)).toBeVisible()
  expect(state.posts).toEqual([])
})

test('denied session hides dialogue and controls at all sizes', async ({ page, stream }, info) => {
  const state = await install(page, stream, { denied: true })
  await page.goto(`/chats/${sessionId}`)
  for (const size of sizes) {
    await page.setViewportSize(size)
    await expect(page.getByText(/Нет доступа к чату/)).toBeVisible()
    await expect(page.getByLabel('Сообщение', { exact: true })).toHaveCount(0)
    await capture(page, info, 'denied')
  }
  expect(state.posts).toEqual([])
})

test('owner without write permission is read-only at all sizes', async ({ page, stream }, info) => {
  const state = await install(page, stream, {
    permissions: ['sessions:read_own', 'agents:read_directory'],
  })
  await page.goto(`/chats/${sessionId}`)
  for (const size of sizes) {
    await page.setViewportSize(size)
    await expect(page.getByLabel('Сообщение', { exact: true })).toBeDisabled()
    await expect(page.getByRole('button', { name: 'Отправить', exact: true })).toBeDisabled()
    await capture(page, info, 'read-only')
  }
  expect(state.posts).toEqual([])
})

test('an unbound initial pending slot permits the first prompt', async ({ page, stream }) => {
  const state = await install(page, stream, {
    runs: [
      {
        id: 'initial',
        session_id: sessionId,
        agent_id: agentId,
        state: 'pending',
        runtime_session_id: null,
        runtime_run_id: null,
      },
    ],
  })
  await page.goto(`/chats/${sessionId}`)
  await page.getByLabel('Сообщение', { exact: true }).fill('First prompt')
  await expect(page.getByRole('button', { name: 'Отправить', exact: true })).toBeEnabled()
  await page.getByRole('button', { name: 'Отправить', exact: true }).click()
  await expect(page.getByLabel('Сообщение', { exact: true })).toHaveValue('')
  expect(state.posts).toHaveLength(1)
})

test('full-session pending delivery holds a prompt outside the visible history', async ({
  page,
  stream,
}) => {
  const state = await install(page, stream, {
    pendingDelivery: true,
    runs: [
      {
        id: 'initial',
        session_id: sessionId,
        agent_id: agentId,
        state: 'pending',
        runtime_session_id: null,
        runtime_run_id: null,
      },
    ],
  })
  await page.goto(`/chats/${sessionId}`)
  await page.getByLabel('Сообщение', { exact: true }).fill('Another prompt')
  await expect(page.getByRole('button', { name: 'Отправить', exact: true })).toBeDisabled()
  await expect(page.getByText(/Запуск или доставка ожидают завершения/)).toBeVisible()
  expect(state.posts).toEqual([])
})

test('missing full-session delivery projection holds sending', async ({ page, stream }) => {
  const state = await install(page, stream, { pendingDelivery: null })
  await page.goto(`/chats/${sessionId}`)
  await page.getByLabel('Сообщение', { exact: true }).fill('A guarded prompt')
  await expect(page.getByRole('button', { name: 'Отправить', exact: true })).toBeDisabled()
  expect(state.posts).toEqual([])
})

test('active primary run blocks prompts without pretending stop or steer capability', async ({
  page,
  stream,
}) => {
  const state = await install(page, stream, {
    runs: [
      {
        id: 'run1',
        session_id: sessionId,
        agent_id: agentId,
        state: 'running',
        provider: 'fixture',
        model: 'fixture',
      },
    ],
  })
  await page.goto(`/chats/${sessionId}`)
  await page.getByLabel('Сообщение', { exact: true }).fill('A new prompt')
  await expect(page.getByText(/Запуск или доставка ожидают завершения/)).toBeVisible()
  await expect(page.getByRole('button', { name: 'Отправить', exact: true })).toBeDisabled()
  expect(state.posts).toEqual([])
})

test('legacy unknown stop marker is held without unsupported lookup', async ({ page, stream }) => {
  const state = await install(page, stream)
  await page.addInitScript(
    (id) =>
      sessionStorage.setItem(`fleet-control.control-recovery.v1:${id}`, '{"operation":"stop"}'),
    sessionId,
  )
  await page.goto(`/chats/${sessionId}`)
  await expect(page.getByText(/Исходная команда требует сверки/)).toBeVisible()
  await expect(page.getByRole('button', { name: 'Отправить', exact: true })).toBeDisabled()
  expect(state.posts).toEqual([])
  expect(state.requests.some((path) => path.includes('/lookup'))).toBe(false)
})

test('list search and agent filters survive dialogue and browser return', async ({
  page,
  stream,
}) => {
  await install(page, stream)
  await page.goto(`/chats?agent=${agentId}&q=Production&users=${userId}`)
  await expect(page.getByLabel('Количество сессий', { exact: true })).toHaveText('1')
  await expect(page.getByRole('button', { name: /Developer Hermes/ })).toHaveAttribute(
    'aria-current',
    'true',
  )
  await page.getByRole('link', { name: /Production Chats core/ }).click()
  await page.getByRole('link', { name: 'Назад к чатам' }).click()
  await expect(page.getByRole('textbox', { name: /Поиск/ })).toHaveValue('Production')
  expect(new URL(page.url()).searchParams.get('users')).toBe(userId)
  expect(new URL(page.url()).searchParams.get('agent')).toBe(agentId)
})

test('reading position and draft survive tabs; inactive deltas do not signal new messages', async ({
  page,
  stream,
}) => {
  const state = await install(page, stream, {
    messages: Array.from({ length: 35 }, (_, index) =>
      message(`Entry ${index}: ${'Retained history. '.repeat(35)}`, `m${index}`),
    ),
  })
  await page.goto(`/chats/${sessionId}`)
  const scroll = page.locator('.fc-chat-scroll').first()
  await expect(scroll.getByText(/Entry 34/)).toBeVisible()
  await scroll.evaluate((node) => {
    node.scrollTop = 180
    node.dispatchEvent(new Event('scroll'))
  })
  await page.getByLabel('Сообщение', { exact: true }).fill('Retained draft')
  await page.getByRole('tab', { name: 'Требования', exact: true }).click()
  await expect.poll(() => stream.connected()).toBeGreaterThan(0)
  stream.emit({ type: 'session_run_delta', run_id: 'another-agent-run', delta: 'Foreign delta' })
  await page.getByRole('tab', { name: 'Диалог', exact: true }).click()
  await expect.poll(() => scroll.evaluate((node) => node.scrollTop)).toBe(180)
  await expect(page.getByLabel('Сообщение', { exact: true })).toHaveValue('Retained draft')
  await expect(page.getByRole('button', { name: 'Новые сообщения' })).toHaveCount(0)
  state.messages.push(message('Appended message', 'appended'))
  stream.emit({ type: 'session_message_created' })
  await expect(page.getByRole('button', { name: 'Новые сообщения' })).toBeVisible()
  await page.getByRole('button', { name: 'Новые сообщения' }).click()
  await expect(page.getByRole('button', { name: 'Новые сообщения' })).toHaveCount(0)
})

test('unknown private-chat creation freezes title and replays only the original request', async ({
  page,
  stream,
}) => {
  const state = await install(page, stream, { unknown: true })
  await page.goto('/chats')
  await page.getByRole('button', { name: 'Новый чат', exact: true }).click()
  await page.getByLabel('Название', { exact: true }).fill('Original private work')
  await page.getByRole('button', { name: 'Создать сессию', exact: true }).click()
  await expect(page.getByText(/Создание не подтверждено/)).toBeVisible()
  await expect(page.getByLabel('Название', { exact: true })).toBeDisabled()
  expect(state.posts).toHaveLength(1)
  expect(await page.evaluate(() => JSON.stringify(sessionStorage))).not.toContain(
    'Original private work',
  )
  state.unknown = false
  await page.getByRole('button', { name: 'Создать сессию', exact: true }).click()
  await expect(
    page.getByRole('heading', { name: 'Original private work', exact: true }),
  ).toBeVisible()
  expect(state.posts).toHaveLength(2)
  expect(state.posts[1].body).toEqual(state.posts[0].body)
})

test('reload of unknown creation cannot silently issue a new key', async ({ page, stream }) => {
  const state = await install(page, stream, { unknown: true })
  await page.goto('/chats')
  await page.getByRole('button', { name: 'Новый чат', exact: true }).click()
  await page.getByLabel('Название', { exact: true }).fill('Private creation draft')
  await page.getByRole('button', { name: 'Создать сессию', exact: true }).click()
  await expect(page.getByText(/Создание не подтверждено/)).toBeVisible()
  await page.reload()
  await page.getByRole('button', { name: 'Новый чат', exact: true }).click()
  await expect(page.getByText(/Создание не подтверждено/)).toBeVisible()
  await expect(page.getByRole('button', { name: 'Создать сессию', exact: true })).toBeDisabled()
  expect(state.posts).toHaveLength(1)
})

test('a rejected retry cannot release an earlier unknown prompt', async ({ page, stream }) => {
  const state = await install(page, stream, { unknown: true })
  await page.goto(`/chats/${sessionId}`)
  await page.getByLabel('Сообщение', { exact: true }).fill('Uncertain original')
  await page.getByRole('button', { name: 'Отправить', exact: true }).click()
  await expect(page.getByText(/Ответ неизвестен/)).toBeVisible()
  state.unknown = false
  state.rejection = 409
  await page.getByRole('button', { name: 'Повторить исходный запрос' }).click()
  await expect(page.getByText(/Сообщение не подтверждено/)).toBeVisible()
  await expect(page.getByLabel('Сообщение', { exact: true })).toBeDisabled()
  expect(
    await page.evaluate(
      (id) => sessionStorage.getItem(`fleet-control.chat-dispatch.v1:${id}`),
      sessionId,
    ),
  ).not.toBeNull()
  expect(state.posts[1].body).toEqual(state.posts[0].body)
})

test('denied task context cannot become an unbound standalone dispatch', async ({
  page,
  stream,
}) => {
  const state = await install(page, stream, { taskContextDenied: true })
  await page.goto(`/chats/${sessionId}`)
  await expect(page.getByRole('alert')).toBeVisible()
  await expect(page.getByLabel('Сообщение агенту', { exact: true })).toHaveCount(0)
  await expect(page.getByLabel('Сообщение', { exact: true })).toHaveCount(0)
  await expect(
    page.getByRole('button', { name: 'Отправить сообщение', exact: true }),
  ).toHaveCount(0)
  expect(state.requests).toContain(`/api/v1/sessions/${sessionId}/task-context`)
  expect(state.requests).not.toContain(`/api/v1/sessions/${sessionId}/messages`)
  expect(state.posts).toEqual([])
  expect(
    await page.evaluate(
      (id) => sessionStorage.getItem(`fleet-control.chat-dispatch.v1:${id}`),
      sessionId,
    ),
  ).toBeNull()
})
