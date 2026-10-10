import { expect, test } from '@playwright/test'
import { fileURLToPath } from 'node:url'
import { resolve } from 'node:path'
import { build } from 'vite'
import { createHash } from 'node:crypto'
import type { AnswerInput, Question } from '../src/api/task-chats'
import {
  assertAnswerCommand,
  sameAnswerRequest,
  type ClarificationCommand,
} from '../src/api/clarification-custody'

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
  const lookups: { runId: string; key: string | undefined }[] = []
  let activeRun = 'original-run'
  const receipt = (runId: string, operation: string) => ({
    id:
      operation === 'stop'
        ? '00000000-0000-4000-8000-000000000001'
        : '00000000-0000-4000-8000-000000000002',
    session_id: 'session1',
    session_run_id: runId,
    agent_id: 'agent1',
    actor_user_id: 'owner',
    operation,
    state: 'uncertain',
    acknowledgement: null,
    observed_run_state: null,
    created_at: '2026-10-09T10:00:00Z',
    updated_at: '2026-10-09T10:00:01Z',
  })
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
    if (path.endsWith('/controls/lookup')) {
      const runId = path.split('/')[6]
      const key = request.headers()['idempotency-key']
      lookups.push({ runId, key })
      const original = commands.find(
        (command) => command.key === key && command.path.split('/')[6] === runId,
      )
      return original
        ? route.fulfill({
            json: receipt(runId, original.path.endsWith('/stop') ? 'stop' : 'steer'),
          })
        : route.fulfill({ status: 404, json: { error: 'Command not found' } })
    }
    if (path.endsWith('/controls')) {
      const runId = path.split('/')[6]
      const operations = new Set(
        commands
          .filter((command) => command.path.split('/')[6] === runId)
          .map((command) => (command.path.endsWith('/stop') ? 'stop' : 'steer')),
      )
      return route.fulfill({
        json: [...operations].map((operation) => receipt(runId, operation)),
      })
    }
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
  const input = page.getByLabel('Уточнение активному запуску')
  await input.fill('я'.repeat(32769))
  await expect(page.getByText('Уточнение не должно превышать 64 КиБ в UTF-8.')).toBeVisible()
  await expect(page.getByRole('button', { name: 'Передать уточнение запуску' })).toBeDisabled()
  expect(commands).toHaveLength(0)
  await input.fill('')
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
  await expect(
    page.getByText('Исход команды неизвестен. Повторная отправка не разрешена.'),
  ).toBeVisible()

  await input.fill('Проверь миграцию без изменения чужих данных')
  await page.getByRole('button', { name: 'Передать уточнение запуску' }).click()
  await expect(
    page.getByText('Принятие команды не подтверждено. Текст и ключ команды сохранены.'),
  ).toBeVisible()
  await expect(input).toHaveValue('Проверь миграцию без изменения чужих данных')
  await expect(input).toBeDisabled()
  await expect(page.getByRole('button', { name: 'Передать уточнение запуску' })).toBeDisabled()
  expect(commands).toHaveLength(3)
  await page.reload({ waitUntil: 'domcontentloaded' })
  await expect(input).toBeDisabled()
  await expect(input).toHaveValue('')
  await expect
    .poll(() =>
      lookups.some((lookup) => lookup.runId === 'new-run' && lookup.key === commands[2].key),
    )
    .toBe(true)
  expect(commands).toHaveLength(3)
  await stop.click()
  await expect.poll(() => commands.length).toBe(4)
  expect(commands[3]).toEqual(commands[0])
  for (const viewport of [
    { width: 375, height: 812 },
    { width: 1920, height: 1080 },
    { width: 2560, height: 1440 },
  ]) {
    await page.setViewportSize(viewport)
    await expect(input).toBeVisible()
    const readback = page
      .getByRole('region', { name: 'Последние команды запуска original-run' })
      .getByText('Исход команды неизвестен. Повторная отправка не разрешена.')
    await readback.scrollIntoViewIfNeeded()
    await expect(readback).toBeInViewport({ ratio: 1 })
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

test('fixture: uncertain clarification retains its original command across questions and versions', async ({
  page,
}, testInfo) => {
  const commands: { path: string; payload: AnswerInput }[] = []
  const stored: ClarificationCommand[] = []
  const deliveries: { path: string; body: string | null }[] = []
  const journalReads: { status: number; commands?: ClarificationCommand[] }[] = []
  let journalAvailable = true
  const errors: string[] = []
  let version = 1
  page.on('pageerror', (error) => errors.push(error.message))
  const now = '2026-10-09T12:00:00Z'
  const question: Question = {
    id: '00000000-0000-4000-8000-000000000801',
    request_id: '00000000-0000-4000-8000-000000000803',
    task_id: '00000000-0000-4000-8000-000000000804',
    root_task_id: '00000000-0000-4000-8000-000000000804',
    assignment_id: '00000000-0000-4000-8000-000000000805',
    execution_id: '00000000-0000-4000-8000-000000000806',
    agent_id: '00000000-0000-4000-8000-000000000807',
    assignment_version: 1,
    checkpoint_id: '00000000-0000-4000-8000-000000000808',
    author_subject: 'pm',
    created_at: now,
    version: 1,
    requirement_revision: 3,
    text: 'Кто видит задачи?',
    rationale: 'Определяет границы доступа',
    required: true,
    mode: 'single',
    options: [
      {
        id: '00000000-0000-4000-8000-000000000809',
        label: 'Участники проекта',
        consequences: 'Только проект',
        is_custom: false,
      },
    ],
    recommended_option_id: '00000000-0000-4000-8000-000000000809',
    requirement_reference: 'REQ-04',
    state: 'open',
    answer: null,
  }
  const secondQuestion: Question = {
    ...question,
    id: '00000000-0000-4000-8000-000000000802',
    text: 'Второй вопрос',
  }
  const storePath = `/api/v1/sessions/session1/clarifications/${question.id}/answer-commands`
  const journalPath = '/api/v1/sessions/session1/clarification-answer-commands'
  const commandId = '00000000-0000-4000-8000-000000000810'
  const deliveryPath = `${journalPath}/${commandId}/delivery`
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
      if (path === storePath) {
        const payload = request.postDataJSON() as AnswerInput
        expect(payload).toEqual({
          expected_question_version: 1,
          requirement_revision: 3,
          selected_option_ids: question.options.map((option) => option.id),
          text: null,
          comment: 'Исходный ответ владельца',
          idempotency_key: expect.any(String),
        })
        expect(payload.idempotency_key).toBeTruthy()
        commands.push({ path, payload: structuredClone(payload) })
        if (!stored.length) {
          const canonical = JSON.stringify({
            expected_question_version: payload.expected_question_version,
            requirement_revision: payload.requirement_revision,
            selected_option_ids: [...payload.selected_option_ids].sort(),
            text: payload.text,
            comment: payload.comment,
            idempotency_key: payload.idempotency_key,
          })
          stored.push({
            id: commandId,
            session_id: 'session1',
            question_id: question.id,
            request: structuredClone(payload),
            payload_sha256: createHash('sha256').update(canonical).digest('hex'),
            state: 'stored',
            answer: null,
            rejection_status: null,
            created_at: now,
            updated_at: now,
          })
        }
        expect(stored).toHaveLength(1)
        const command = stored[0]
        if (!command) throw new Error('Stored fixture command is missing')
        expect(sameAnswerRequest(command.request, payload)).toBe(true)
        expect(command.request).toEqual(payload)
        assertAnswerCommand(command, 'session1')
        return route.fulfill({ headers: { 'Cache-Control': 'no-store' }, json: command })
      }
      if (path === deliveryPath) {
        expect(stored).toHaveLength(1)
        expect(request.postData()).toBeNull()
        deliveries.push({ path, body: request.postData() })
        const command = stored[0]
        if (!command) throw new Error('Stored fixture command is missing')
        command.state = 'uncertain'
        command.updated_at = '2026-10-09T12:00:01Z'
        version = 2
        // Keep the lost-ACK and journal-outage phases distinct from successful delivery.
        if (deliveries.length === 1) journalAvailable = false
        assertAnswerCommand(command, 'session1')
        return route.fulfill({ headers: { 'Cache-Control': 'no-store' }, json: command })
      }
      throw new Error(`Unexpected clarification command: ${path}`)
    }
    if (path === journalPath) {
      if (!journalAvailable) {
        journalReads.push({ status: 503 })
        return route.fulfill({
          status: 503,
          json: { error: { code: 'UNAVAILABLE', message: 'Fixture journal unavailable' } },
        })
      }
      const pending = structuredClone(stored)
      for (const command of pending) assertAnswerCommand(command, 'session1')
      journalReads.push({ status: 200, commands: pending })
      return route.fulfill({ headers: { 'Cache-Control': 'no-store' }, json: pending })
    }
    if (path === '/api/v1/agent-directory') return route.fulfill({ json: [] })
    if (path === '/api/v1/sessions/session1')
      return route.fulfill({
        json: {
          id: 'session1',
          user_id: 'owner',
          user_display_name: 'Владелец задачи',
          primary_agent_id: 'agent1',
          primary_agent_name: 'Project Manager',
          title: 'Уточнение требований',
          visibility: 'private',
          task_key: 'FIXTURE-2',
        },
      })
    if (path.endsWith('/task-context'))
      return route.fulfill({
        json: {
          binding: {
            tracker_instance_id: 'tracker',
            project_id: '00000000-0000-4000-8000-000000000811',
            task_id: question.task_id,
            root_task_id: question.root_task_id,
            agent_id: question.agent_id,
            owner_subject: 'subject-owner',
          },
          tracker: {
            contract_version: 1,
            tracker_instance_id: 'tracker',
            project_id: '00000000-0000-4000-8000-000000000811',
            task_id: question.task_id,
            root_task_id: question.root_task_id,
            owner_subject: 'subject-owner',
            stage: 'Draft',
            requirement_revision: 3,
            waiting_reason: deliveries.length < 2 ? 'Требуется ответ' : 'Ответы сохранены',
            permissions: { can_answer: deliveries.length < 2, can_confirm: false },
            assignment: null,
          },
        },
      })
    if (path.endsWith('/chat-controls'))
      return route.fulfill({
        json: {
          can_send: false,
          can_steer: false,
          can_stop: false,
          active_run_id: null,
          blocked_reason: 'workflow_assignment_required',
        },
      })
    if (path.endsWith('/clarifications')) {
      // Current Tracker readback can advance without proving this command's delivery ACK.
      const answered = deliveries.length >= 2 ? stored[0] : undefined
      return route.fulfill({
        json: {
          questions: [question, secondQuestion].map((item, index): Question => ({
            ...item,
            version: index === 0 ? version : item.version,
            state: answered ? 'answered' : 'open',
            answer: answered
              ? {
                  id: `00000000-0000-4000-8000-${String(812 + index).padStart(12, '0')}`,
                  question_id: item.id,
                  question_version: 1,
                  requirement_revision: item.requirement_revision,
                  selected_option_ids: [...answered.request.selected_option_ids],
                  text: answered.request.text,
                  comment: index === 0 ? answered.request.comment : 'Независимый fixture-ответ',
                  author_subject: 'subject-owner',
                  created_at: now,
                }
              : null,
          })),
        },
      })
    }
    if (path.endsWith('/requirements')) return route.fulfill({ json: { revisions: [] } })
    if (path.endsWith('/history')) return route.fulfill({ json: { items: [], next_before: null } })
    if (path.endsWith('/runs') || path.endsWith('/approvals')) return route.fulfill({ json: [] })
    throw new Error(`Unexpected clarification read: ${path}`)
  })
  await page.goto(fixturePath, { waitUntil: 'domcontentloaded' })
  await page.getByRole('tab', { name: /Уточнения/ }).click()
  await expect.poll(() => journalReads.some((read) => read.status === 200)).toBe(true)
  const choice = page.getByRole('radio', { name: /Участники проекта/ })
  await expect(choice).not.toBeChecked()
  await choice.check()
  await page.getByLabel('Комментарий').fill('Исходный ответ владельца')
  await page.getByRole('button', { name: 'Сохранить ответ' }).click()
  await expect(page.getByText(/Неизвестен исход сохранения/)).toBeVisible()
  await expect(
    page.getByRole('button', { name: 'Перенести черновик и проверить новый вопрос' }),
  ).toBeDisabled()
  await page.getByRole('button', { name: /2\. Второй вопрос/ }).click()
  await expect(page.getByText('Непроверенный ответ: Кто видит задачи?')).toBeVisible()
  await expect(page.getByRole('button', { name: 'Сохранить ответ' })).toBeDisabled()
  expect(commands).toHaveLength(1)
  const retry = page.getByRole('button', { name: 'Повторить исходный ответ' })
  for (const viewport of [
    { width: 375, height: 812 },
    { width: 1920, height: 1080 },
    { width: 2560, height: 1440 },
  ]) {
    await page.setViewportSize(viewport)
    await retry.scrollIntoViewIfNeeded()
    await expect(retry).toBeInViewport({ ratio: 1 })
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= document.documentElement.clientWidth,
      ),
    ).toBe(true)
    await page.screenshot({
      path: testInfo.outputPath(`clarification-uncertain-${viewport.width}.png`),
      fullPage: true,
      scale: 'css',
    })
  }
  await retry.click()
  await expect.poll(() => commands.length).toBe(2)
  expect(commands[1]).toEqual(commands[0])
  expect(commands[0].payload).toMatchObject({
    expected_question_version: 1,
    comment: 'Исходный ответ владельца',
  })
  await expect(page.getByText('Ответы сохранены', { exact: true })).toBeVisible()
  await expect(page.getByRole('button', { name: /2\. Второй вопрос/ })).toContainText('answered')
  await expect(choice).toBeDisabled()
  await expect(page.getByRole('button', { name: 'Сохранить ответ' })).toBeDisabled()
  await expect(retry).toBeEnabled()
  await retry.click()
  await expect.poll(() => commands.length).toBe(3)
  expect(commands[2]).toEqual(commands[0])
  await expect.poll(() => deliveries.length).toBe(3)
  expect(stored).toHaveLength(1)
  const original = structuredClone(stored[0])
  if (!original) throw new Error('Original fixture command is missing')
  expect(original.state).toBe('uncertain')
  expect(original.answer).toBeNull()
  expect(original.rejection_status).toBeNull()
  expect(original.request).toEqual(commands[0].payload)
  expect(deliveries).toEqual(Array(3).fill({ path: deliveryPath, body: null }))
  expect(journalReads.some((read) => read.status === 503)).toBe(true)

  journalAvailable = true
  const readsBeforeReload = journalReads.length
  page.once('dialog', (dialog) => dialog.accept())
  await page.reload({ waitUntil: 'domcontentloaded' })
  await page.getByRole('tab', { name: /Уточнения/ }).click()
  await expect.poll(() => journalReads.length).toBeGreaterThan(readsBeforeReload)
  await expect
    .poll(() => journalReads.slice(readsBeforeReload).some((read) => read.status === 200))
    .toBe(true)
  expect(journalReads.at(-1)?.commands).toEqual([original])
  await page.getByRole('button', { name: /2\. Второй вопрос/ }).click()
  const recover = page.getByRole('button', { name: 'Продолжить исходную команду' })
  const retainedAnswer = page.getByRole('status').filter({ has: recover })
  await expect(retainedAnswer).toHaveCount(1)
  await expect(retainedAnswer).toContainText(original.question_id)
  await expect(retainedAnswer.getByText('Исходный ответ владельца', { exact: true })).toBeVisible()
  await expect(retry).toHaveCount(0)
  await expect(choice).toBeDisabled()
  await expect(page.getByRole('button', { name: 'Сохранить ответ' })).toBeDisabled()
  await expect(recover).toBeEnabled()
  await recover.click()
  await expect.poll(() => deliveries.length).toBe(4)
  await expect(page.getByRole('status', { name: 'Статус команды' })).toContainText(
    'Доставка исходного ответа ещё не подтверждена.',
  )
  await expect(recover).toBeEnabled()
  await expect(choice).toBeDisabled()
  await expect(page.getByRole('button', { name: 'Сохранить ответ' })).toBeDisabled()
  expect(commands).toHaveLength(3)
  expect(stored).toEqual([original])
  expect(deliveries).toEqual(Array(4).fill({ path: deliveryPath, body: null }))
  expect(errors).toEqual([])
})
