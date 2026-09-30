import { mkdirSync, readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import AxeBuilder from '@axe-core/playwright'
import { expect, test } from '@playwright/test'

test.skip(process.env.SDLC_LIVE_QA !== '1', 'Requires live Fleet and Central Auth')
test.skip(({ browserName }) => browserName !== 'chromium', 'Single browser live acceptance')
test.use({ trace: 'off' })

const account =
  process.env.SDLC_LIVE_QA === '1'
    ? (JSON.parse(
        readFileSync(
          process.env.SDLC_QA_SESSION_FILE ??
            fileURLToPath(
              new URL('../../../services-base/deploy/.local/qa-session.json', import.meta.url),
            ),
          'utf8',
        ),
      ) as { email: string; password: string })
    : { email: '', password: '' }
const base = process.env.PLAYWRIGHT_BASE_URL ?? 'http://localhost:7742'
const screenshots = fileURLToPath(
  new URL('../../../.local/screenshots/fleet-agent-detail/', import.meta.url),
)

test('all agent detail tabs are localized, accessible and responsive against live APIs', async ({
  page,
  request,
}) => {
  test.setTimeout(900_000)
  mkdirSync(screenshots, { recursive: true })
  const login = await request.post('http://localhost:7701/auth/login', { data: account })
  expect(login.status()).toBe(200)
  const { access_token } = (await login.json()) as { access_token: string }
  const headers = { Authorization: `Bearer ${access_token}` }
  let agentId = ''
  const errors: string[] = []
  try {
    const created = await request.post(`${base}/api/v1/agents`, {
      headers,
      data: {
        kind: 'hermes',
        product_role: 'executor',
        role: 'developer',
        display_name: `QA agent-detail final-20261001 ${Date.now()}`,
        description: 'QA detail acceptance; never started',
      },
    })
    if (created.ok()) agentId = ((await created.json()) as { id: string }).id
    expect(created.ok()).toBeTruthy()
    expect(agentId).not.toBe('')
    await page.goto(`${base}/agents/${agentId}`)
    await page.getByLabel('Email').fill(account.email)
    await page.getByLabel('Пароль').fill(account.password)
    await page.getByRole('button', { name: 'Войти', exact: true }).click()
    await expect(page.getByRole('navigation', { name: 'Разделы агента' })).toBeVisible()
    page.on('pageerror', (error) => errors.push(`page: ${error.message}`))
    page.on('console', (message) => {
      if (message.type() === 'error') errors.push(`console: ${message.text()}`)
    })
    page.on('requestfailed', (failed) => {
      const reason = failed.failure()?.errorText ?? 'unknown'
      if (!reason.includes('ERR_ABORTED'))
        errors.push(`${new URL(failed.url()).pathname}: ${reason}`)
    })
    page.on('response', (response) => {
      if (response.url().includes('/api/v1/') && response.status() >= 400)
        errors.push(`${response.status()} ${new URL(response.url()).pathname}`)
    })
    const tabs = [
      ['', 'Обзор'],
      ['runtime', 'Среда'],
      ['skills', 'Навыки'],
      ['config', 'Конфигурация'],
      ['workspace', 'Файлы'],
      ['sessions', 'Сессии'],
    ] as const
    for (const theme of ['light', 'gray', 'dark']) {
      await page.evaluate((value) => localStorage.setItem('theme', value), theme)
      for (const [tab, label] of tabs) {
        await page.goto(`${base}/agents/${agentId}${tab ? `/${tab}` : ''}`)
        await expect(page.locator('html')).toHaveAttribute('data-theme', theme)
        const nav = page.getByRole('navigation', { name: 'Разделы агента' })
        await expect(nav.getByRole('link', { name: label, exact: true })).toHaveAttribute(
          'aria-current',
          'page',
        )
        if (tab === 'config')
          await expect(page.getByRole('textbox', { name: 'config.json' })).toBeVisible()
        if (tab === 'workspace') await expect(page.getByText('Общий размер').first()).toBeVisible()
        if (tab === 'runtime') await expect(page.getByText('Загрузка журнала...')).toHaveCount(0)
        if (tab === 'skills') await expect(page.getByRole('textbox').first()).toBeVisible()
        if (tab === 'sessions')
          await expect(page.getByText(/Сессий этого агента.*нет/)).toBeVisible()
        for (const [width, height] of [
          [375, 812],
          [768, 1024],
          [1280, 800],
          [1920, 1080],
          [2560, 1440],
        ]) {
          await page.setViewportSize({ width, height })
          expect(
            await page.evaluate(
              () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
            ),
            `${tab} ${theme} ${width} page overflow`,
          ).toBeLessThanOrEqual(1)
          expect(
            await nav.evaluate((element) => element.scrollWidth - element.clientWidth),
            'Tabs must wrap rather than scroll',
          ).toBeLessThanOrEqual(1)
          expect(
            await nav
              .locator('a')
              .evaluateAll((links) =>
                links.every((link) => link.getBoundingClientRect().height >= 40),
              ),
          ).toBeTruthy()
          const audit = await new AxeBuilder({ page }).analyze()
          expect(
            audit.violations
              .filter((issue) => issue.impact === 'serious' || issue.impact === 'critical')
              .map((issue) => ({ id: issue.id, targets: issue.nodes.map((node) => node.target) })),
            `${tab} ${theme} ${width} axe`,
          ).toEqual([])
          await page.screenshot({
            path: `${screenshots}/${tab || 'overview'}-${theme}-${width}.png`,
            fullPage: true,
            animations: 'disabled',
          })
        }
      }
    }
    await page.setViewportSize({ width: 375, height: 812 })
    const configLink = page
      .getByRole('navigation', { name: 'Разделы агента' })
      .getByRole('link', { name: 'Конфигурация', exact: true })
    await configLink.focus()
    await page.keyboard.press('Enter')
    await expect(page.getByRole('textbox', { name: 'config.json' })).toBeVisible()
    await page.getByRole('textbox', { name: 'config.json' }).fill('{')
    await expect(page.getByRole('button', { name: 'Сохранить конфигурацию' })).toBeDisabled()
    await expect(page.getByText('Введите корректный JSON-объект')).toBeVisible()
    expect(errors).toEqual([])
  } finally {
    if (agentId) {
      const removed = await request.delete(`${base}/api/v1/agents/${agentId}`, { headers })
      expect(removed.ok()).toBeTruthy()
    }
  }
})
