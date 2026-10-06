import { expect, test } from '@playwright/test'
import AxeBuilder from '@axe-core/playwright'
import { agents, projects } from '../src/previews/pm-draft/fixtures'

const entry = '/pm-draft-preview.html#/'
test.beforeEach(async ({ page }) => {
  await page.route('**/*', async (route) => {
    const url = new URL(route.request().url())
    if (url.origin !== 'http://127.0.0.1:55498' || url.pathname.startsWith('/api/')) {
      throw new Error(`Preview attempted API/external request: ${url.origin}${url.pathname}`)
    }
    await route.continue()
  })
})

test('explicit creation, saved preparation and chat remain non-dispatching', async ({
  page,
}, info) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto(`${entry}?view=list`)
  await page.getByRole('button', { name: 'Создать задачу с PM' }).click()
  await expect(page.getByLabel('Проект', { exact: true })).toHaveValue('')
  await expect(page.getByLabel('Project Manager', { exact: true })).toHaveValue('')
  await expect(page.getByRole('button', { name: 'Создать задачу', exact: true })).toBeDisabled()
  await page.getByLabel('Проект', { exact: true }).selectOption(projects.projects[0]!.id)
  await page.getByLabel('Project Manager', { exact: true }).selectOption(agents[0]!.id)
  await page.getByLabel('Название задачи').fill('Портал заявок сотрудников')
  await page.getByLabel('Исходный запрос').fill('Длинный исходный запрос. '.repeat(25))
  for (const viewport of [
    { width: 375, height: 812 },
    { width: 768, height: 1024 },
    { width: 1920, height: 1080 },
    { width: 2560, height: 1440 },
  ]) {
    await page.setViewportSize(viewport)
    await page.getByRole('button', { name: 'Создать задачу', exact: true }).scrollIntoViewIfNeeded()
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
    await page.screenshot({
      path: info.outputPath(`form-${viewport.width}.png`),
      fullPage: true,
      animations: 'disabled',
    })
  }
  await page.getByRole('button', { name: 'Создать задачу', exact: true }).click()
  await expect(page.getByText('Задача создана, подготовка не завершена')).toBeVisible()
  await page.getByRole('button', { name: 'Продолжить подготовку' }).click()
  await page.getByRole('button', { name: 'Продолжить подготовку' }).click()
  await expect(page.getByText('Задача и чат сохранены')).toBeVisible()
  await page.getByRole('button', { name: 'Открыть чат' }).click()
  await expect(page).toHaveURL(/view=chat/)
  await expect(page.getByRole('status')).toHaveText(
    'Ожидает допуска PM к запуску. Агент ещё не запущен.',
  )
  await expect(page.getByRole('button', { name: 'Отправить сообщение' })).toBeDisabled()
  await page.getByRole('button', { name: 'Вернуться в чаты' }).click()
  await expect(
    page.getByRole('button', { name: /Портал заявок сотрудников · Ожидает допуска PM/ }),
  ).toBeVisible()
  await page.goBack()
  await expect(page.getByRole('status')).toContainText('Агент ещё не запущен')
  expect(errors).toEqual([])
  expect(
    await page.evaluate(() => ({
      local: Object.keys(localStorage),
      session: Object.keys(sessionStorage),
    })),
  ).toEqual({ local: [], session: [] })
})

test('lost acknowledgement and empty filtered page preserve the input', async ({ page }) => {
  await page.goto(`${entry}?access=empty-page&result=absent`)
  await page.getByRole('button', { name: 'Ещё проекты' }).click()
  await expect(page.getByLabel('Проект', { exact: true })).toHaveValue('')
  await page.getByLabel('Проект', { exact: true }).selectOption(projects.projects[0]!.id)
  await page.getByLabel('Project Manager', { exact: true }).selectOption(agents[0]!.id)
  await page.getByLabel('Название задачи').fill('Запрос без повторного создания')
  await page.getByLabel('Исходный запрос').fill('Сохранить именно этот текст')
  await page.getByRole('button', { name: 'Создать задачу', exact: true }).click()
  await expect(page.getByRole('heading', { name: 'Ответ не получен' })).toBeVisible()
  await expect(page.getByLabel('Исходный запрос')).toHaveValue('Сохранить именно этот текст')
  await expect(page.getByRole('button', { name: 'Повторить исходное создание' })).toHaveCount(0)
  await page.getByRole('button', { name: 'Проверить состояние' }).click()
  await page.getByRole('button', { name: 'Повторить исходное создание' }).click()
  await expect(page.getByRole('heading', { name: 'Подготовка задачи не завершена' })).toBeVisible()
  await expect(page.getByText(/UX-102/)).toHaveCount(0)
  expect(page.url()).not.toContain('Сохранить')
})

test('all proposed states remain readable, isolated and accessible', async ({ page }, info) => {
  test.setTimeout(90000)
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  for (const [name, query] of [
    ['list', 'view=list'],
    ['uncertain', 'state=uncertain'],
    ['incomplete', 'state=incomplete&step=draft'],
    ['awaiting', 'state=awaiting'],
    ['chat', 'state=awaiting&view=chat'],
    ['readonly', 'state=incomplete&access=readonly'],
    ['denied', 'state=uncertain&access=denied'],
    ['unavailable', 'state=uncertain&access=unavailable'],
  ]) {
    await page.goto(`${entry}?${query}`)
    // State query parameters seed fictional scenarios on document load;
    // actual in-memory operation state survives normal hash navigation.
    await page.reload()
    if (name === 'unavailable')
      await page.getByRole('button', { name: 'Проверить состояние' }).click()
    if (name === 'readonly') {
      await expect(page.getByRole('button', { name: 'Продолжить подготовку' })).toBeDisabled()
      await page.getByRole('button', { name: 'Вернуться в чаты' }).click()
      await page.getByRole('button', { name: 'Вернуться к подготовке задачи' }).click()
      await expect(page.getByRole('button', { name: 'Продолжить подготовку' })).toBeDisabled()
    }
    if (name === 'denied')
      await expect(page.getByRole('button', { name: 'Проверить состояние' })).toBeDisabled()
    for (const viewport of [
      { width: 375, height: 812 },
      { width: 1920, height: 1080 },
    ]) {
      await page.setViewportSize(viewport)
      const action = page
        .getByRole('button', {
          name: /^(Проверить состояние|Продолжить подготовку|Открыть чат|Отправить сообщение)$/,
        })
        .first()
      if (await action.count()) await action.scrollIntoViewIfNeeded()
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
        true,
      )
      await page.screenshot({
        path: info.outputPath(`${name}-${viewport.width}.png`),
        fullPage: true,
        animations: 'disabled',
      })
    }
    expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([])
  }
  expect(errors).toEqual([])
})

test('keyboard navigation reaches creation and returns to its retained draft', async ({ page }) => {
  await page.goto(entry)
  await page.getByLabel('Проект', { exact: true }).focus()
  await page.keyboard.press('ArrowDown')
  await page.keyboard.press('Tab')
  await expect(page.getByLabel('Project Manager', { exact: true })).toBeFocused()
  await page.keyboard.press('ArrowDown')
  await page.keyboard.press('Tab')
  await expect(page.getByLabel('Название задачи')).toBeFocused()
  await page.keyboard.type('Keyboard draft')
  await page.keyboard.press('Tab')
  await expect(page.getByLabel('Исходный запрос')).toBeFocused()
  await page.keyboard.type('Original keyboard request')
  await page.keyboard.press('Tab')
  await expect(page.getByRole('button', { name: 'Создать задачу', exact: true })).toBeFocused()
  await page.getByRole('button', { name: 'Вернуться в чаты' }).click()
  await page.getByRole('button', { name: 'Вернуться к черновику' }).click()
  await expect(page.getByLabel('Исходный запрос')).toHaveValue('Original keyboard request')
})
