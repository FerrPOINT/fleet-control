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

test('executor edit avoids leader requests and exposes named editors', async ({
  page,
  request,
}) => {
  test.setTimeout(120_000)
  const login = await request.post('http://localhost:7701/auth/login', {
    data: { email: account.email, password: account.password },
  })
  expect(login.ok(), await login.text()).toBeTruthy()
  const { access_token } = (await login.json()) as { access_token: string }
  const response = await request.get('http://localhost:7742/api/v1/executors', {
    headers: { Authorization: `Bearer ${access_token}` },
  })
  expect(response.ok(), await response.text()).toBeTruthy()
  const executors = (await response.json()) as { id: string; display_name: string }[]
  expect(executors.length).toBeGreaterThan(0)
  const executor = executors[0]!

  const badRequests: string[] = []
  page.on('response', (result) => {
    if (result.url().includes(`/api/v1/leaders/${executor.id}/executors`)) {
      badRequests.push(`${result.status()} ${result.url()}`)
    }
  })
  await page.goto(`http://localhost:7742/executors/${executor.id}/edit`)
  await page.getByLabel('Email').fill(account.email)
  await page.getByLabel('Пароль').fill(account.password)
  await page.getByRole('button', { name: 'Войти', exact: true }).click()

  await expect(page.getByRole('heading', { name: `Edit ${executor.display_name}` })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Save agent' })).toBeEnabled()
  for (const width of [1280, 375]) {
    await page.setViewportSize({ width, height: 812 })
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= document.documentElement.clientWidth,
      ),
    ).toBeTruthy()
  }
  const screenshot = fileURLToPath(
    new URL('../../../.local/screenshots/fleet-agent-edit-375.png', import.meta.url),
  )
  mkdirSync(dirname(screenshot), { recursive: true })
  await page.screenshot({ path: screenshot, fullPage: true })
  expect(badRequests).toEqual([])

  await page.goto(`http://localhost:7742/agents/${executor.id}/config`)
  await expect(page.getByRole('textbox', { name: 'SOUL.md' })).toBeVisible()
  await expect(page.getByRole('textbox', { name: 'config.json' })).toBeVisible()
  await expect(page.getByRole('textbox', { name: 'env.json' })).toBeVisible()

  const skillsResponse = await request.get(
    `http://localhost:7742/api/v1/agents/${executor.id}/skills`,
    {
      headers: { Authorization: `Bearer ${access_token}` },
    },
  )
  expect(skillsResponse.ok(), await skillsResponse.text()).toBeTruthy()
  const skills = (await skillsResponse.json()) as { title: string }[]
  expect(skills.length).toBeGreaterThan(0)
  await page.goto(`http://localhost:7742/agents/${executor.id}/skills`)
  await expect(page.getByRole('textbox', { name: `Edit ${skills[0]!.title}` })).toBeVisible()
})
