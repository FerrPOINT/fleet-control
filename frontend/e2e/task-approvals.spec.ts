import { expect, test, type Page } from '@playwright/test'
import { fileURLToPath } from 'node:url'
import { resolve } from 'node:path'
import { build } from 'vite'
import type { ApprovalDecision, RuntimeApprovalRequest } from '../src/api/task-approvals'

// Standalone source-component fixtures, not live runtime or human-session authorization evidence.
test.describe.configure({ mode: 'default' })
const frontendRoot = fileURLToPath(new URL('../', import.meta.url))
const fixturePath = '/__approval-fixture/'
const sessionId = '00000000-0000-4000-8000-000000000001'
const approvalId = '00000000-0000-4000-8000-000000000002'
const runId = '00000000-0000-4000-8000-000000000003'
const approval: RuntimeApprovalRequest = {
  id: approvalId,
  session_id: sessionId,
  session_run_id: runId,
  agent_id: '00000000-0000-4000-8000-000000000004',
  runtime_run_id: 'hermes-run-exact-approval-target',
  runtime_approval_id: 'hermes-approval-exact-target',
  prompt: `Разрешить чтение /workspace/${'requirements_'.repeat(30)}.md?`,
  detail: {
    action: 'read_file',
    token: 'redacted',
    arguments: { path: '/workspace/requirements.md' },
  },
  state: 'pending',
  resolved_by_user_id: null,
  resolved_at: null,
  created_at: '2026-10-01T12:00:00Z',
}
const assets = new Map<string, { body: string | Buffer; contentType: string }>()
let fixtureHtml = ''

test.beforeAll(async () => {
  const virtualId = 'virtual:approval-fixture'
  const bundles = await build({
    root: frontendRoot,
    configFile: resolve(frontendRoot, 'vite.config.ts'),
    logLevel: 'error',
    define: { 'import.meta.env.VITE_API_BASE_URL': JSON.stringify('') },
    plugins: [
      {
        name: 'approval-fixture',
        resolveId: (id) => (id === virtualId ? `\0${virtualId}` : undefined),
        load: (id) =>
          id === `\0${virtualId}`
            ? `
        import { createElement } from 'react'
        import { createRoot } from 'react-dom/client'
        import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
        import { ThemeProvider } from '@sdlc/ui/lib'
        import { TaskApprovalsPanel } from '@/pages/chat-detail/approvals'
        import '@/index.css'
        const params = new URLSearchParams(location.search)
        localStorage.setItem('theme', params.get('theme') || 'dark')
        const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
        createRoot(document.getElementById('root')).render(
          createElement(QueryClientProvider, { client },
            createElement(ThemeProvider, null,
              createElement(TaskApprovalsPanel, {
                sessionId: ${JSON.stringify(sessionId)}, canResolve: params.get('resolve') !== 'false'
              })
            )
          )
        )
      `
            : undefined,
      },
    ],
    build: {
      write: false,
      emptyOutDir: false,
      outDir: resolve(frontendRoot, '../../../pm-clarification/approvals-fixture'),
      rollupOptions: { input: virtualId },
    },
  })
  let entry = ''
  const styles: string[] = []
  for (const bundle of Array.isArray(bundles) ? bundles : [bundles]) {
    if (!('output' in bundle)) throw new Error('Fixture build must finish without a watcher')
    for (const output of bundle.output) {
      if (output.type === 'chunk') {
        assets.set(`${fixturePath}${output.fileName}`, {
          body: output.code,
          contentType: 'text/javascript',
        })
        if (output.isEntry) entry = `${fixturePath}${output.fileName}`
      } else {
        assets.set(`${fixturePath}${output.fileName}`, {
          body: typeof output.source === 'string' ? output.source : Buffer.from(output.source),
          contentType: output.fileName.endsWith('.css') ? 'text/css' : 'application/octet-stream',
        })
        if (output.fileName.endsWith('.css')) styles.push(`${fixturePath}${output.fileName}`)
      }
    }
  }
  if (!entry || !styles.length)
    throw new Error('Fixture must include the real component and theme CSS')
  fixtureHtml = `<!doctype html><html lang="ru"><head><meta charset="utf-8">
    <meta name="viewport" content="width=device-width,initial-scale=1">
    ${styles.map((path) => `<link rel="stylesheet" href="${path}">`).join('')}
    </head><body><main id="root" class="mx-auto w-full min-w-0 max-w-3xl p-4"></main>
    <script type="module" src="${entry}"></script></body></html>`
})

async function installFixture(
  page: Page,
  runtimeApprovalId: string | null = approval.runtime_approval_id,
) {
  await page.route(`**${fixturePath}**`, (route) => {
    const path = new URL(route.request().url()).pathname
    if (path === fixturePath) return route.fulfill({ contentType: 'text/html', body: fixtureHtml })
    const asset = assets.get(path)
    return asset ? route.fulfill(asset) : route.fulfill({ status: 404 })
  })
  const commands: { path: string; body: { choice: 'once' | 'deny'; idempotency_key: string } }[] =
    []
  let saved: ApprovalDecision | null = null
  let approvalState: RuntimeApprovalRequest['state'] = 'pending'
  let lookups = 0
  await page.route('**/api/v1/**', async (route) => {
    const path = new URL(route.request().url()).pathname
    if (path === `/api/v1/sessions/${sessionId}/approvals`)
      return route.fulfill({
        json: [{ ...approval, state: approvalState, runtime_approval_id: runtimeApprovalId }],
      })
    if (path !== `/api/v1/sessions/${sessionId}/approvals/${approvalId}/decision`)
      throw new Error(`Unexpected fixture request: ${path}`)
    if (route.request().method() === 'GET') {
      lookups++
      return saved
        ? route.fulfill({ json: saved })
        : route.fulfill({ status: 404, json: { error: { message: 'no command' } } })
    }
    const body = route.request().postDataJSON() as {
      choice: 'once' | 'deny'
      idempotency_key: string
    }
    commands.push({ path, body })
    saved = {
      id: '00000000-0000-4000-8000-000000000005',
      session_id: sessionId,
      approval_id: approvalId,
      session_run_id: runId,
      actor_user_id: '00000000-0000-4000-8000-000000000006',
      choice: body.choice,
      state: 'uncertain',
      created_at: '2026-10-01T12:01:00Z',
    }
    return route.abort('failed')
  })
  return {
    commands,
    lookups: () => lookups,
    confirm: () => {
      if (saved) saved = { ...saved, state: 'delivered' }
      approvalState = 'approved'
    },
  }
}

test('fixture: explicit targeted choice, lost response and historical command survive reload', async ({
  page,
}, testInfo) => {
  test.setTimeout(90000)
  const fixture = await installFixture(page)
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  for (const theme of ['dark', 'light']) {
    await page.goto(`${fixturePath}?theme=${theme}`, { waitUntil: 'domcontentloaded' })
    const approve = page.getByRole('button', { name: 'Разрешить один раз' })
    await expect(approve).toBeEnabled()
    await expect(page.locator('html')).toHaveAttribute('data-theme', theme)
    await expect(page.getByText('Параметры', { exact: true })).toBeVisible()
    await expect(page.getByText('Путь', { exact: true })).toBeVisible()
    await expect(page.getByRole('article', { name: 'read_file' })).toContainText('redacted')
    expect(fixture.commands).toHaveLength(0)
    for (const viewport of [
      { width: 375, height: 812 },
      { width: 1440, height: 900 },
      { width: 2560, height: 1440 },
    ]) {
      await page.setViewportSize(viewport)
      await expect(approve).toBeVisible()
      await expect(page.getByRole('button', { name: 'Запретить' })).toBeVisible()
      expect(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= document.documentElement.clientWidth,
        ),
      ).toBe(true)
      await page.waitForTimeout(1000)
      await page.screenshot({
        path: testInfo.outputPath(`approvals-${theme}-${viewport.width}.png`),
        fullPage: true,
        animations: 'disabled',
      })
    }
  }
  const approve = page.getByRole('button', { name: 'Разрешить один раз' })
  await approve.focus()
  expect(fixture.commands).toHaveLength(0)
  await page.keyboard.press('Enter')
  await expect(
    page.getByText('Доставка решения не подтверждена. Требуется сверка состояния.'),
  ).toBeVisible()
  await expect(approve).toBeDisabled()
  await expect(page.getByRole('button', { name: 'Запретить' })).toBeDisabled()
  expect(fixture.commands).toHaveLength(1)
  expect(fixture.commands[0]).toEqual({
    path: `/api/v1/sessions/${sessionId}/approvals/${approvalId}/decision`,
    body: { choice: 'once', idempotency_key: expect.stringMatching(/^[0-9a-f-]{36}$/) },
  })
  expect(fixture.lookups()).toBeGreaterThan(2)
  await expect(page.getByRole('status', { name: 'Результат решения' })).toBeFocused()
  await page.reload({ waitUntil: 'domcontentloaded' })
  await expect(
    page.getByText('Доставка решения не подтверждена. Требуется сверка состояния.'),
  ).toBeVisible()
  await expect(approve).toBeDisabled()
  expect(fixture.commands).toHaveLength(1)
  fixture.confirm()
  await page.getByRole('button', { name: 'Обновить разрешения' }).click()
  await expect(page.getByText('Разрешено', { exact: true })).toBeVisible()
  await expect(approve).toHaveCount(0)
  expect(fixture.commands).toHaveLength(1)
  expect(errors).toEqual([])
})

test('fixture: passed read-only permission never offers an enabled decision', async ({ page }) => {
  const fixture = await installFixture(page)
  await page.goto(`${fixturePath}?resolve=false`, { waitUntil: 'domcontentloaded' })
  await expect(page.getByText('Только просмотр')).toBeVisible()
  await expect(page.getByRole('button', { name: 'Разрешить один раз' })).toBeDisabled()
  await expect(page.getByRole('button', { name: 'Запретить' })).toBeDisabled()
  await expect(page.getByRole('article', { name: 'read_file' })).toContainText(runId)
  expect(fixture.commands).toHaveLength(0)
})

test('fixture: missing exact runtime approval ID cannot produce a decision command', async ({
  page,
}) => {
  const fixture = await installFixture(page, null)
  await page.goto(fixturePath, { waitUntil: 'domcontentloaded' })
  await expect(
    page.getByText('Идентификатор точного запроса агента отсутствует. Решение недоступно.'),
  ).toBeVisible()
  await expect(page.getByRole('button', { name: 'Разрешить один раз' })).toBeDisabled()
  await expect(page.getByRole('button', { name: 'Запретить' })).toBeDisabled()
  expect(fixture.commands).toHaveLength(0)
})
