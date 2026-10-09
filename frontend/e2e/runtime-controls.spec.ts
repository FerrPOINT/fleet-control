import { expect, test } from '@playwright/test'
import { fileURLToPath } from 'node:url'
import { resolve } from 'node:path'
import { build } from 'vite'

// Real production component with mocked HTTP: not native runtime or authorization evidence.
const root = fileURLToPath(new URL('../', import.meta.url))
const fixturePath = '/__runtime-controls-fixture/'
const assets = new Map<string, { body: string | Buffer; contentType: string }>()
let html = ''

test.beforeAll(async () => {
  test.setTimeout(120000)
  const virtualId = 'virtual:runtime-controls-fixture'
  const bundles = await build({
    root,
    configFile: resolve(root, 'vite.config.ts'),
    logLevel: 'error',
    define: { 'import.meta.env.VITE_API_BASE_URL': JSON.stringify('') },
    plugins: [
      {
        name: 'runtime-controls-fixture',
        resolveId: (id) => (id === virtualId ? `\0${virtualId}` : undefined),
        load: (id) =>
          id === `\0${virtualId}`
            ? `
          import { createElement } from 'react'
          import { createRoot } from 'react-dom/client'
          import { createMemoryRouter, RouterProvider } from 'react-router'
          import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
          import { ThemeProvider } from '@sdlc/ui/lib'
          import { ChatDetailPage } from '@/pages/chat-detail'
          import { useAuthStore } from '@/shared/auth/store'
          import '@/index.css'
          localStorage.setItem('theme', 'dark')
          useAuthStore.setState({ userId: 'owner', token: null, permissions: [] })
          const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
          const router = createMemoryRouter([
            { path: '/chats/:sessionId', element: createElement(ChatDetailPage) },
            { path: '/chats', element: createElement('p', null, 'Список') }
          ], { initialEntries: ['/chats/session1'] })
          createRoot(document.getElementById('root')).render(
            createElement(QueryClientProvider, { client },
              createElement(ThemeProvider, null, createElement(RouterProvider, { router }))
            )
          )
        `
            : undefined,
      },
    ],
    build: {
      write: false,
      emptyOutDir: false,
      rollupOptions: { input: virtualId },
    },
  })
  let entry = ''
  const styles: string[] = []
  for (const bundle of Array.isArray(bundles) ? bundles : [bundles]) {
    if (!('output' in bundle)) throw new Error('Fixture build must terminate')
    for (const output of bundle.output) {
      const path = `${fixturePath}${output.fileName}`
      if (output.type === 'chunk') {
        assets.set(path, { body: output.code, contentType: 'text/javascript' })
        if (output.isEntry) entry = path
      } else {
        assets.set(path, {
          body: typeof output.source === 'string' ? output.source : Buffer.from(output.source),
          contentType: output.fileName.endsWith('.css') ? 'text/css' : 'application/octet-stream',
        })
        if (output.fileName.endsWith('.css')) styles.push(path)
      }
    }
  }
  if (!entry || !styles.length) throw new Error('Production component/CSS bundle missing')
  html = `<!doctype html><html lang="ru"><head><meta charset="utf-8">
    <meta name="viewport" content="width=device-width,initial-scale=1">
    ${styles.map((path) => `<link rel="stylesheet" href="${path}">`).join('')}
    </head><body><main id="root" class="min-w-0 p-4"></main>
    <script type="module" src="${entry}"></script></body></html>`
})

test('fixture: HTTP-success uncertainty retains steer and the original stop target', async ({
  page,
}, testInfo) => {
  test.setTimeout(90000)
  const errors: string[] = []
  const commands: { path: string; key: string | undefined; body: unknown }[] = []
  let activeRun = 'original-run'
  page.on('pageerror', (error) => errors.push(error.message))
  await page.route(`**${fixturePath}**`, (route) => {
    const path = new URL(route.request().url()).pathname
    if (path === fixturePath) return route.fulfill({ contentType: 'text/html', body: html })
    const asset = assets.get(path)
    return asset ? route.fulfill(asset) : route.fulfill({ status: 404 })
  })
  await page.route('**/api/v1/**', (route) => {
    const request = route.request()
    const path = new URL(request.url()).pathname
    if (request.method() === 'POST') {
      if (!/^\/api\/v1\/sessions\/session1\/runs\/[^/]+\/(stop|steer)$/.test(path))
        throw new Error(`Unexpected fixture command: ${path}`)
      commands.push({
        path,
        key: request.headers()['idempotency-key'],
        body: request.postDataJSON(),
      })
      activeRun = 'new-run'
      return route.fulfill({
        json: {
          session_id: 'session1',
          run_id: path.split('/')[6],
          runtime_run_id: 'native-run',
          accepted: false,
          state: 'running',
          message: 'Acceptance unknown',
        },
      })
    }
    if (path === '/api/v1/agent-directory') return route.fulfill({ json: [] })
    if (path === '/api/v1/sessions/session1')
      return route.fulfill({
        json: {
          id: 'session1',
          user_id: 'owner',
          user_display_name: 'Владелец задачи',
          primary_agent_id: 'agent1',
          primary_agent_name: 'Developer',
          title: 'Проверка управляющих команд',
          visibility: 'private',
          task_key: 'FIXTURE-1',
        },
      })
    if (path.endsWith('/task-context'))
      return route.fulfill({ json: { binding: null, tracker: null } })
    if (path.endsWith('/chat-controls'))
      return route.fulfill({
        json: {
          can_send: false,
          can_steer: true,
          can_stop: true,
          active_run_id: activeRun,
          blocked_reason: null,
        },
      })
    if (path.endsWith('/history')) return route.fulfill({ json: { items: [], next_before: null } })
    if (path.endsWith('/runs') || path.endsWith('/approvals')) return route.fulfill({ json: [] })
    throw new Error(`Unexpected fixture read: ${path}`)
  })
  const ready = Promise.all(
    ['/api/v1/sessions/session1', '/api/v1/sessions/session1/chat-controls'].map((path) =>
      page.waitForResponse(
        (response) => new URL(response.url()).pathname === path && response.ok(),
      ),
    ),
  )
  await page.goto(fixturePath, { waitUntil: 'domcontentloaded' })
  await ready
  const stop = page.getByRole('button', { name: 'Остановить запуск' })
  await expect(stop).toBeEnabled()
  await stop.click()
  await expect(page.getByText(/Принятие остановки не подтверждено/)).toBeVisible()
  expect(commands).toHaveLength(1)
  expect(commands[0].key).toBeTruthy()
  await stop.click()
  await expect.poll(() => commands.length).toBe(2)
  expect(commands[1]).toEqual(commands[0])
  expect(commands[1].path).toContain('/original-run/stop')

  const input = page.getByLabel('Уточнение активному запуску')
  await input.fill('Проверь миграцию без изменения чужих данных')
  await page.getByRole('button', { name: 'Передать уточнение запуску' }).click()
  await expect(
    page.getByText('Принятие команды не подтверждено. Текст и ключ команды сохранены.'),
  ).toBeVisible()
  await expect(input).toHaveValue('Проверь миграцию без изменения чужих данных')
  await expect(input).toBeDisabled()
  await expect(page.getByRole('button', { name: 'Передать уточнение запуску' })).toBeDisabled()
  expect(commands).toHaveLength(3)
  for (const viewport of [
    { width: 375, height: 812 },
    { width: 1920, height: 1080 },
    { width: 2560, height: 1440 },
  ]) {
    await page.setViewportSize(viewport)
    await expect(input).toBeVisible()
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= document.documentElement.clientWidth,
      ),
    ).toBe(true)
    await page.screenshot({
      path: testInfo.outputPath(`control-uncertain-${viewport.width}.png`),
      fullPage: true,
      scale: 'css',
    })
  }
  expect(errors).toEqual([])
})
