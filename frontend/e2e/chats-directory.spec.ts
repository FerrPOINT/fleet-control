import { expect, test, type Page } from '@playwright/test'
import { generateKeyPairSync, sign } from 'node:crypto'
import type { ChatsDirectoryPage } from '../src/api/chats-directory'
import type { AgentDirectoryItem, AgentSession } from '../src/api/types'

const owner = '00000000-0000-4000-8000-000000000001'
const dev = '00000000-0000-4000-8000-000000000101'
const qa = '00000000-0000-4000-8000-000000000102'
const first = '00000000-0000-4000-8000-000000000201'
const last = '00000000-0000-4000-8000-000000000202'
const now = '2026-10-01T12:00:00Z'

function agent(id: string, ordinal: number, displayName: string): AgentDirectoryItem {
  return {
    id,
    ordinal,
    name: `agent${ordinal}`,
    kind: 'hermes',
    product_role: 'executor',
    role: 'developer',
    sdlc_role: 'developer',
    status: 'running',
    display_name: displayName,
    description: null,
    namespace_id: null,
    workflow_id: null,
    runtime_version: null,
    dashboard_port: 29002 + ordinal * 10,
    api_port: 29001 + ordinal * 10,
  }
}
function session(id: string, agentId: string, title: string): AgentSession {
  return {
    id,
    agent_id: agentId,
    primary_agent_id: agentId,
    agent_name: 'agent1',
    primary_agent_name: 'agent1',
    user_id: owner,
    user_email: 'owner@example.test',
    user_username: 'owner',
    user_display_name: 'Directory owner',
    leader_agent_id: null,
    leader_agent_name: null,
    parent_session_id: null,
    created_by_leader_agent_id: null,
    visibility: 'private',
    title,
    task_key: 'DISPLAY-ONLY',
    state: 'active',
    namespace_id: null,
    external_session_id: null,
    last_message_preview: 'Concrete session preview',
    created_at: now,
    updated_at: now,
  }
}

// Exercise the existing OIDC entry point instead of persisting an access token.
async function mockLogin(page: Page) {
  const { privateKey, publicKey } = generateKeyPairSync('ec', { namedCurve: 'P-256' })
  let issuer = ''
  let nonce = ''
  await page.route('**/oidc/authorize**', async (route) => {
    const url = new URL(route.request().url())
    issuer = url.origin
    nonce = url.searchParams.get('nonce') ?? ''
    const callback = new URL(url.searchParams.get('redirect_uri')!)
    callback.searchParams.set('code', 'directory-code')
    callback.searchParams.set('state', url.searchParams.get('state')!)
    await route.fulfill({
      contentType: 'text/html',
      body: `<script>location.replace(${JSON.stringify(callback.toString())})</script>`,
    })
  })
  await page.route('**/oidc/token', async (route) => {
    const header = Buffer.from(
      JSON.stringify({ alg: 'ES256', typ: 'JWT', kid: 'directory' }),
    ).toString('base64url')
    const payload = Buffer.from(
      JSON.stringify({
        iss: issuer,
        aud: 'fleet-control',
        sub: owner,
        email: 'owner@example.test',
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
      json: {
        access_token: 'directory-access',
        id_token: `${content}.${signature}`,
        expires_in: 3600,
      },
    })
  })
  await page.route('**/oidc/jwks', (route) =>
    route.fulfill({
      json: {
        keys: [
          { ...publicKey.export({ format: 'jwk' }), kid: 'directory', alg: 'ES256', use: 'sig' },
        ],
      },
    }),
  )
}

test('directory uses server counts, concrete cursors and scoped returnTo across viewports', async ({
  page,
}, testInfo) => {
  await mockLogin(page)
  const requests: URL[] = []
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.route('**/api/v1/**', async (route) => {
    const url = new URL(route.request().url())
    if (url.pathname === '/api/v1/users/me')
      return route.fulfill({
        json: {
          id: owner,
          email: 'owner@example.test',
          username: 'owner',
          display_name: 'Directory owner',
        },
      })
    if (url.pathname === '/api/v1/users/me/permissions')
      return route.fulfill({
        json: {
          user_id: owner,
          role: 'user',
          is_system_admin: false,
          permissions: ['sessions:read_own', 'sessions:write_own', 'agents:read_directory'],
        },
      })
    if (url.pathname === '/api/v1/agent-directory')
      return route.fulfill({
        json: [agent(dev, 1, 'Directory developer'), agent(qa, 2, 'Directory reviewer')],
      })
    if (url.pathname === `/api/v1/sessions/${last}`)
      return route.fulfill({ json: session(last, dev, 'Second page chat') })
    if (url.pathname.endsWith('/task-context'))
      return route.fulfill({ json: { binding: null, tracker: null } })
    if (url.pathname.endsWith('/chat-controls'))
      return route.fulfill({
        json: { active_run_id: null, can_send: true, can_steer: false, can_stop: false },
      })
    if (url.pathname.endsWith('/runs')) return route.fulfill({ json: [] })
    if (url.pathname.endsWith('/approvals')) return route.fulfill({ json: [] })
    if (url.pathname.endsWith('/history'))
      return route.fulfill({ json: { items: [], next_before: null } })
    if (url.pathname.endsWith('/stream'))
      return route.fulfill({ contentType: 'text/event-stream', body: ': directory test\n\n' })
    if (url.pathname !== '/api/v1/chats/directory') return route.fulfill({ json: {} })
    requests.push(url)
    const selected = url.searchParams.get('agent_id') ?? dev
    const searching = Boolean(url.searchParams.get('q'))
    const next = Boolean(url.searchParams.get('before'))
    const data: ChatsDirectoryPage = {
      agents: [
        { agent: agent(dev, 1, 'Directory developer'), matching_session_count: searching ? 7 : 51 },
        { agent: agent(qa, 2, 'Directory reviewer'), matching_session_count: searching ? 2 : 3 },
      ],
      selected_agent_id: selected,
      items: [
        session(
          next ? last : first,
          selected,
          next ? 'Second page chat' : searching ? 'Server search result' : 'First page chat',
        ),
      ],
      next_before: next ? null : first,
    }
    return route.fulfill({ json: data })
  })
  await page.goto('/chats')
  await expect(page.getByRole('link', { name: /First page chat/ })).toBeVisible()
  expect(requests[0].searchParams.get('user_id')).toBe(owner)
  expect(requests[0].searchParams.get('limit')).toBe('50')
  const agents = page.getByRole('navigation', { name: 'Агенты', exact: true })
  await expect(agents.getByRole('button', { name: /Directory developer/ })).toContainText('51')
  await page.getByRole('textbox', { name: 'Поиск по задачам' }).fill('literal%_')
  await expect(page.getByRole('link', { name: /Server search result/ })).toBeVisible()
  await expect(agents.getByRole('button', { name: /Directory developer/ })).toContainText('7')
  expect(requests.at(-1)!.searchParams.get('q')).toBe('literal%_')
  await page.getByRole('button', { name: 'Следующая страница' }).click()
  const link = page.getByRole('link', { name: /Second page chat/ })
  await expect(link).toBeVisible()
  expect(requests.at(-1)!.searchParams.get('before')).toBe(first)
  const href = new URL((await link.getAttribute('href'))!, 'http://localhost')
  expect(href.pathname).toBe(`/chats/${last}`)
  const returnTo = new URL(href.searchParams.get('returnTo')!, 'http://localhost')
  expect(returnTo.searchParams.get('q')).toBe('literal%_')
  expect(returnTo.searchParams.get('before')).toBe(first)
  expect(returnTo.searchParams.get('users')).toBe(owner)
  expect(returnTo.searchParams.get('agent')).toBe(dev)
  await expect(page.getByRole('button', { name: 'Следующая страница' })).toBeDisabled()
  for (const theme of ['dark', 'light']) {
    if (theme === 'light') {
      await page.getByRole('button', { name: /^Тема:/ }).click()
      await page.getByRole('button', { name: /^Тема:/ }).click()
    }
    await expect(page.locator('html')).toHaveAttribute('data-theme', theme)
    for (const viewport of [
      { width: 375, height: 812 },
      { width: 1440, height: 900 },
      { width: 2560, height: 1440 },
    ]) {
      await page.setViewportSize(viewport)
      await expect(link).toBeVisible()
      expect(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= document.documentElement.clientWidth,
        ),
      ).toBe(true)
      await expect(page.locator('img').first()).toBeVisible()
      expect(
        await page
          .locator('img')
          .evaluateAll((images) =>
            images.every((image) => (image as HTMLImageElement).naturalWidth > 0),
          ),
      ).toBe(true)
      await page.waitForTimeout(1000)
      await page.screenshot({
        path: testInfo.outputPath(`directory-${theme}-${viewport.width}.png`),
        fullPage: true,
        animations: 'disabled',
      })
    }
  }
  await link.click()
  await expect(page.getByRole('heading', { name: 'Second page chat' })).toBeVisible()
  await page.getByRole('link', { name: 'Вернуться к чатам' }).click()
  await expect(link).toBeVisible()
  const restored = new URL(page.url())
  expect(Object.fromEntries(restored.searchParams)).toEqual(
    Object.fromEntries(returnTo.searchParams),
  )
  await agents.getByRole('button', { name: /Directory reviewer/ }).click()
  await expect(page.getByRole('link', { name: /Server search result/ })).toBeVisible()
  expect(requests.at(-1)!.searchParams.get('agent_id')).toBe(qa)
  expect(requests.at(-1)!.searchParams.has('before')).toBe(false)
  expect(errors).toEqual([])
})
