import { mkdirSync, readFileSync } from 'node:fs'
import { dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import { expect, test } from '@playwright/test'

test.skip(process.env.SDLC_LIVE_QA !== '1', 'Requires the running local SDLC fleet')
test.skip(({ browserName }) => browserName !== 'chromium', 'Single browser live smoke')

const account =
  process.env.SDLC_LIVE_QA === '1'
    ? (JSON.parse(
        readFileSync(
          fileURLToPath(new URL('../../../.local/qa-session.json', import.meta.url)),
          'utf8',
        ),
      ) as { email: string; password: string })
    : { email: '', password: '' }

test('live leader team is read-only until changed and fits responsive themes', async ({
  page,
  request,
}) => {
  test.setTimeout(180_000)
  const login = await request.post('http://localhost:7701/auth/login', {
    data: { email: account.email, password: account.password },
  })
  expect(login.ok(), await login.text()).toBeTruthy()
  const { access_token } = (await login.json()) as { access_token: string }
  const headers = { Authorization: `Bearer ${access_token}` }
  const created = await request.post('http://localhost:7742/api/v1/agents', {
    headers,
    data: {
      kind: 'hermes',
      product_role: 'leader',
      role: 'it_lead',
      display_name: `qa-leader-team-${Date.now()}`,
      description: 'Disposable UI QA leader',
      namespace_id: 'qa-leader',
      namespace_name: 'QA Leader',
      workflow_id: 'qa-leader-workflow',
      workflow_name: 'QA Leader Workflow',
      executor_ids: [],
    },
  })
  expect(created.ok(), await created.text()).toBeTruthy()
  const {
    id: leaderId,
    name: leaderName,
    display_name: displayName,
  } = (await created.json()) as {
    id: string
    name: string
    display_name: string
  }

  try {
    await page.goto('http://localhost:7742/leaders')
    await page.getByLabel('Email').fill(account.email)
    await page.getByLabel('Пароль').fill(account.password)
    await page.getByRole('button', { name: 'Войти', exact: true }).click()

    const badResponses: string[] = []
    const writes: string[] = []
    const consoleErrors: string[] = []
    const listTeamRequests: string[] = []
    page.on('response', (response) => {
      if (response.url().includes('/api/v1/') && response.status() >= 400) {
        badResponses.push(`${response.status()} ${response.url()}`)
      }
      if (
        response.url().includes('/api/v1/') &&
        !['GET', 'HEAD'].includes(response.request().method())
      ) {
        writes.push(`${response.request().method()} ${response.url()}`)
      }
    })
    page.on('console', (message) => {
      if (message.type() === 'error') consoleErrors.push(message.text())
    })
    page.on('request', (request) => {
      if (request.url().includes(`/api/v1/leaders/${leaderId}/executors`)) {
        listTeamRequests.push(request.url())
      }
    })

    await page.reload()
    await expect(page.getByRole('heading', { name: 'Лидеры' })).toBeVisible()
    await page.getByLabel('Найти лидера').fill(displayName)
    await expect(page.getByText('Показано 1 из 1')).toBeVisible()
    expect(listTeamRequests).toEqual([])
    for (const theme of ['light', 'gray', 'dark']) {
      await page.evaluate((value) => localStorage.setItem('theme', value), theme)
      await page.reload()
      await expect(page.locator('html')).toHaveAttribute('data-theme', theme)
      await page.getByLabel('Найти лидера').fill(displayName)
      await expect(page.getByText('Показано 1 из 1')).toBeVisible()
      for (const [width, height] of [
        [375, 812],
        [768, 1024],
        [1280, 800],
        [1920, 1080],
      ]) {
        await page.setViewportSize({ width, height })
        expect(
          await page.evaluate(
            () => document.documentElement.scrollWidth <= document.documentElement.clientWidth,
          ),
        ).toBeTruthy()
        const evidence =
          process.env.SDLC_CAPTURE_LEADERS_EVIDENCE === '1' &&
          ((theme === 'light' && width === 375) || (theme === 'gray' && width === 1280))
        const screenshot = fileURLToPath(
          new URL(
            evidence
              ? `../../docs/assets/screens/2026-09-19-leaders/${theme}-${width}.png`
              : `../../../.local/screenshots/fleet-leaders-${theme}-${width}.png`,
            import.meta.url,
          ),
        )
        mkdirSync(dirname(screenshot), { recursive: true })
        await page.screenshot({ path: screenshot, fullPage: true })
      }
    }
    expect(listTeamRequests).toEqual([])
    await page.getByRole('link', { name: 'Открыть', exact: true }).click()

    await page.goto(`http://localhost:7742/leaders/${leaderId}/edit`)
    await expect(page.getByRole('heading', { name: `Изменить ${displayName}` })).toBeVisible()
    await expect(page.getByRole('button', { name: 'Сохранить агента' })).toBeEnabled()
    await page.setViewportSize({ width: 375, height: 812 })
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= document.documentElement.clientWidth,
      ),
    ).toBeTruthy()
    await page.goto(`http://localhost:7742/leaders/${leaderId}`)

    await expect(page.getByText('Исполнители команды')).toBeVisible()
    await expect(page.getByRole('button', { name: 'Сохранить команду' })).toBeDisabled()
    await expect(page.getByRole('checkbox').first()).toBeVisible()

    for (const theme of ['light', 'gray', 'dark']) {
      await page.evaluate((value) => localStorage.setItem('theme', value), theme)
      await page.reload()
      await expect(page.locator('html')).toHaveAttribute('data-theme', theme)
      await expect(page.getByRole('button', { name: 'Сохранить команду' })).toBeDisabled()
      for (const [width, height] of [
        [375, 812],
        [768, 1024],
        [1280, 800],
        [1920, 1080],
      ]) {
        await page.setViewportSize({ width, height })
        expect(
          await page.evaluate(
            () => document.documentElement.scrollWidth <= document.documentElement.clientWidth,
          ),
        ).toBeTruthy()
        const evidence =
          process.env.SDLC_CAPTURE_EVIDENCE === '1' &&
          ((theme === 'light' && width === 375) || (theme === 'gray' && width === 1280))
        const screenshot = fileURLToPath(
          new URL(
            evidence
              ? `../../docs/assets/screens/2026-09-19-leader-team/${theme}-${width}.png`
              : `../../../.local/screenshots/fleet-leader-team-${theme}-${width}.png`,
            import.meta.url,
          ),
        )
        mkdirSync(dirname(screenshot), { recursive: true })
        await page.screenshot({ path: screenshot, fullPage: true })
      }
    }

    expect(badResponses).toEqual([])
    expect(writes).toEqual([])
    expect(consoleErrors).toEqual([])
  } finally {
    const archived = await request.delete(`http://localhost:7742/api/v1/agents/${leaderId}`, {
      headers,
    })
    expect(archived.ok(), await archived.text()).toBeTruthy()
    const purged = await request.post(
      `http://localhost:7742/api/v1/agents/${leaderId}/purge-files`,
      { headers, data: { confirmation: leaderName } },
    )
    expect(purged.ok(), await purged.text()).toBeTruthy()
    const afterPurge = await request.get(`http://localhost:7742/api/v1/agents/${leaderId}`, {
      headers,
    })
    expect(afterPurge.ok(), await afterPurge.text()).toBeTruthy()
    expect(((await afterPurge.json()) as { status: string }).status).toBe('archived')
  }
})
