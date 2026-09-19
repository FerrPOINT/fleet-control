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

test('dashboard and navigation use Russian labels on the live fleet', async ({ page }) => {
  test.setTimeout(150_000)
  for (let attempt = 0; attempt < 2; attempt++) {
    await page.goto('http://localhost:7742/')
    await page.getByLabel('Email').fill(account.email)
    await page.getByLabel('Пароль').fill(account.password)
    const loginResponse = page.waitForResponse(
      (response) =>
        response.url().startsWith('http://localhost:7701/oidc/login') &&
        response.request().method() === 'POST',
    )
    await page.getByRole('button', { name: 'Войти', exact: true }).click()
    const response = await loginResponse
    if (response.status() === 429 && attempt === 0) {
      await page.waitForTimeout(62_000)
      continue
    }
    expect(response.status()).toBeLessThan(400)
    break
  }

  for (const [width, height] of [
    [375, 812],
    [1280, 800],
    [1920, 1080],
    [2560, 1440],
  ]) {
    await page.setViewportSize({ width, height })
    await expect(page.getByRole('heading', { name: 'Обзор агентов' })).toBeVisible()
    await expect(page.getByRole('link', { name: 'Создать агента' })).toBeVisible()
    await expect(page.getByText('Последние события')).toBeVisible()
    await expect(page.getByRole('button', { name: 'Выйти' })).toBeVisible()
    await expect(page.getByText('Исполнитель').first()).toBeVisible()
    await expect(page.getByText('Готов').first()).toBeVisible()
    await expect(page.getByText('Executor', { exact: true })).toHaveCount(0)
    await expect(page.getByText('ready', { exact: true })).toHaveCount(0)
    if (width === 375) {
      await expect(page.getByRole('combobox', { name: 'Раздел Fleet Control' })).toHaveValue('/')
    } else {
      await expect(page.getByRole('link', { name: 'Обзор', exact: true })).toBeVisible()
    }
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= document.documentElement.clientWidth,
      ),
    ).toBeTruthy()
    const screenshot = fileURLToPath(
      new URL(`../../../.local/screenshots/fleet-dashboard-ru-${width}.png`, import.meta.url),
    )
    mkdirSync(dirname(screenshot), { recursive: true })
    await page.screenshot({ path: screenshot, fullPage: true })
  }
})
