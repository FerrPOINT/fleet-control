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

test('live executor directory supports search, sessions and responsive themes', async ({
  page,
}) => {
  test.setTimeout(180_000)
  await page.goto('http://localhost:7742/executors')
  await page.getByLabel('Email').fill(account.email)
  await page.getByLabel('Пароль').fill(account.password)
  await page.getByRole('button', { name: 'Войти', exact: true }).click()

  if (process.env.SDLC_LIVE_QA_URL) await page.goto(process.env.SDLC_LIVE_QA_URL)
  await expect(page.getByRole('heading', { name: 'Исполнители' })).toBeVisible()

  const badResponses: string[] = []
  const writes: string[] = []
  const consoleErrors: string[] = []
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

  await expect(page.getByText(/Показано \d+ из \d+/)).toBeVisible()
  const firstExecutor = page.locator('ul.divide-y > li').first()
  await expect(firstExecutor).toBeVisible()
  const firstName = await firstExecutor.locator('h2').innerText()
  await page.getByLabel('Найти исполнителя').fill(firstName)
  await expect(page.locator('ul.divide-y > li')).toHaveCount(1)
  await page.getByLabel('Найти исполнителя').fill('')

  const firstSessions = page.locator('ul.divide-y > li details').first()
  if (await firstSessions.count()) {
    await firstSessions.locator('summary').focus()
    await page.keyboard.press('Enter')
    await expect(firstSessions).toHaveAttribute('open', '')
    await page.keyboard.press('Enter')
    await expect(firstSessions).not.toHaveAttribute('open', '')
  }

  for (const theme of ['light', 'gray', 'dark']) {
    await page.evaluate((value) => localStorage.setItem('theme', value), theme)
    await page.reload()
    await expect(page.locator('html')).toHaveAttribute('data-theme', theme)
    await expect(page.getByText(/Показано \d+ из \d+/)).toBeVisible()
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
      const evidence = (theme === 'light' && width === 375) || (theme === 'gray' && width === 1280)
      const screenshot = fileURLToPath(
        new URL(
          evidence
            ? `../../docs/assets/screens/2026-09-19-executors/${theme}-${width}.png`
            : `../../../.local/screenshots/fleet-executors-${theme}-${width}.png`,
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
})
