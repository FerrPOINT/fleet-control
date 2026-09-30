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
  new URL('../../../.local/screenshots/fleet-detail-layout/', import.meta.url),
)

test('detail rails measure 320 px at 1024 and stack below primary content at 1023', async ({
  page,
  request,
}) => {
  test.setTimeout(900_000)
  mkdirSync(screenshots, { recursive: true })
  const login = await request.post('http://localhost:7701/auth/login', {
    data: { email: account.email, password: account.password },
  })
  expect(login.status()).toBe(200)
  const { access_token } = (await login.json()) as { access_token: string }
  const headers = { Authorization: `Bearer ${access_token}` }
  const agents: string[] = []
  const errors: string[] = []
  const name = `QA rails final-20261001 ${Date.now()}`
  try {
    for (const role of ['executor', 'leader'] as const) {
      const created = await request.post(`${base}/api/v1/agents`, {
        headers,
        data: {
          kind: 'hermes',
          product_role: role,
          role: role === 'leader' ? 'it_lead' : 'developer',
          display_name: `${name} ${role}`,
          description: 'QA layout acceptance; never started',
        },
      })
      if (created.ok()) agents.push(((await created.json()) as { id: string }).id)
      expect(created.ok()).toBeTruthy()
    }
    const [executor, leader] = agents
    const createdSession = await request.post(`${base}/api/v1/sessions`, {
      headers,
      data: {
        agent_id: executor,
        title: `${name} session`,
      },
    })
    expect(createdSession.ok()).toBeTruthy()
    const session = ((await createdSession.json()) as { id: string }).id
    await page.goto(`${base}/agents/${executor}`)
    await page.getByLabel('Email').fill(account.email)
    await page.getByLabel('Пароль').fill(account.password)
    await page.getByRole('button', { name: 'Войти', exact: true }).click()
    await expect(page.getByRole('navigation', { name: 'Разделы агента' })).toBeVisible()
    page.on('pageerror', (error) => errors.push(`page: ${error.message}`))
    page.on('console', (message) => {
      if (message.type() === 'error') errors.push(`console: ${message.text()}`)
    })
    page.on('response', (response) => {
      if (response.url().includes('/api/v1/') && response.status() >= 400)
        errors.push(`${response.status()} ${new URL(response.url()).pathname}`)
    })
    const routes = [
      ['overview', `/agents/${executor}`, 'Состояние среды'],
      ['workspace', `/agents/${executor}/workspace`, 'Использование диска'],
      ['leader', `/leaders/${leader}`, 'Исполнители команды'],
      ['session', `/sessions/${session}`, 'Управление сессией'],
    ] as const
    for (const theme of ['light', 'gray', 'dark']) {
      await page.evaluate((value) => localStorage.setItem('theme', value), theme)
      for (const [route, path, label] of routes) {
        await page.goto(`${base}${path}`)
        const layout = page.locator('.page-split')
        const rail = layout.getByRole('complementary', { name: label })
        await expect(rail).toBeVisible()
        await expect(page.locator('html')).toHaveAttribute('data-theme', theme)
        for (const [width, height] of [
          [375, 812],
          [768, 1024],
          [1023, 800],
          [1024, 800],
          [1279, 800],
          [1280, 800],
          [1920, 1080],
        ]) {
          await page.setViewportSize({ width, height })
          const dimensions = await layout.evaluate((element) => {
            const primary = element.firstElementChild!.getBoundingClientRect()
            const aside = element.querySelector('aside')!.getBoundingClientRect()
            return {
              primary: {
                x: primary.x,
                y: primary.y,
                right: primary.right,
                bottom: primary.bottom,
                width: primary.width,
              },
              aside: { x: aside.x, y: aside.y, width: aside.width },
              gap: parseFloat(getComputedStyle(element).columnGap),
            }
          })
          if (width >= 1024) {
            expect(dimensions.aside.width, `${route} ${theme} ${width} rail width`).toBeCloseTo(
              320,
              0,
            )
            expect(dimensions.aside.x - dimensions.primary.right).toBeCloseTo(dimensions.gap, 0)
            expect(dimensions.aside.y).toBeCloseTo(dimensions.primary.y, 0)
          } else {
            expect(dimensions.aside.width).toBeCloseTo(dimensions.primary.width, 0)
            expect(dimensions.aside.x).toBeCloseTo(dimensions.primary.x, 0)
            expect(dimensions.aside.y).toBeGreaterThanOrEqual(dimensions.primary.bottom)
          }
          expect(
            await page.evaluate(
              () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
            ),
            `${route} ${width} overflow`,
          ).toBeLessThanOrEqual(1)
          const audit = await new AxeBuilder({ page }).analyze()
          expect(
            audit.violations
              .filter((issue) => issue.impact === 'serious' || issue.impact === 'critical')
              .map((issue) => ({ id: issue.id, targets: issue.nodes.map((node) => node.target) })),
            `${route} ${theme} ${width} axe`,
          ).toEqual([])
          await page.screenshot({
            path: `${screenshots}/${route}-${theme}-${width}.png`,
            fullPage: true,
            animations: 'disabled',
          })
        }
      }
    }
    expect(errors).toEqual([])
  } finally {
    for (const id of agents.reverse()) {
      const archived = await request.delete(`${base}/api/v1/agents/${id}`, { headers })
      expect(archived.ok()).toBeTruthy()
    }
  }
})
