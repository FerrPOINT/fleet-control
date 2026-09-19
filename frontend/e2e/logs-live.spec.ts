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

test('live logs stay compact and readable across themes and widths', async ({ page }) => {
  test.setTimeout(180_000)
  await page.goto('http://localhost:7742/logs?tab=audit')
  await page.getByLabel('Email').fill(account.email)
  await page.getByLabel('Пароль').fill(account.password)
  await page.getByRole('button', { name: 'Войти', exact: true }).click()
  await expect(page.getByRole('heading', { name: 'Журналы' })).toBeVisible()

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

  await expect(page.getByText(/Показано 25 из \d+/)).toBeVisible()
  await expect(page.locator('details')).toHaveCount(25)
  const firstEntry = page.locator('details').first()
  await firstEntry.locator('summary').focus()
  await page.keyboard.press('Enter')
  await expect(firstEntry).toHaveAttribute('open', '')
  await page.keyboard.press('Enter')
  await expect(firstEntry).not.toHaveAttribute('open', '')
  await page.getByRole('button', { name: 'Показать ещё' }).click()
  await expect(page.locator('details')).toHaveCount(50)
  await page.getByRole('button', { name: 'Процессы' }).click()
  await expect(page.getByText(/Показано 25 из \d+/)).toBeVisible()
  await expect(page.locator('details')).toHaveCount(25)
  await page.getByRole('button', { name: 'Аудит' }).click()

  for (const theme of ['light', 'gray', 'dark']) {
    await page.evaluate((value) => localStorage.setItem('theme', value), theme)
    await page.reload()
    await expect(page.locator('html')).toHaveAttribute('data-theme', theme)
    await expect(page.getByText(/Показано 25 из \d+/)).toBeVisible()
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
            ? `../../docs/assets/screens/2026-09-19-logs/${theme}-${width}.png`
            : `../../../.local/screenshots/fleet-logs-${theme}-${width}.png`,
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
