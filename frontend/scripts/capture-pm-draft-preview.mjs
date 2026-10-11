import { chromium } from '@playwright/test'
import { mkdir, writeFile, readFile } from 'node:fs/promises'
import { createHash } from 'node:crypto'
import { fileURLToPath } from 'node:url'

const origin = 'http://127.0.0.1:55496'
const directory = new URL('../../docs/assets/design/pm-draft/', import.meta.url)
await mkdir(directory, { recursive: true })
const browser = await chromium.launch()
const entries = []
const external = []
const errors = []
try {
  for (const [width, height] of [
    [375, 812],
    [768, 1024],
    [1920, 1080],
    [2560, 1440],
  ]) {
    for (const state of ['form', 'uncertain', 'incomplete', 'awaiting']) {
      const page = await browser.newPage({ viewport: { width, height } })
      page.on('request', (request) => {
        const url = new URL(request.url())
        if (url.origin !== origin || url.pathname.startsWith('/api/')) external.push(request.url())
      })
      page.on('pageerror', (error) => errors.push(error.message))
      const route = `/pm-draft-preview.html#/?state=${state}`
      await page.goto(origin + route, { waitUntil: 'networkidle' })
      await page.getByRole('heading', { name: 'Новая задача с PM' }).waitFor()
      await page.evaluate(() => document.fonts.ready)
      if (state === 'form') {
        await page.getByLabel('Проект', { exact: true }).selectOption('portal')
        await page.getByLabel('Project Manager', { exact: true }).selectOption('agent3')
        await page.getByLabel('Название задачи').fill('Портал заявок сотрудников')
        await page
          .getByLabel('Исходный запрос')
          .fill(
            'Нужен внутренний портал: создание заявки, назначение исполнителя, история изменений. Доступ только участникам проекта. Уточнить критерии приёмки перед разработкой.',
          )
      }
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)
      if (overflow) throw new Error(`Horizontal overflow: ${state} ${width}`)
      const file = `${state}-${width}x${height}.png`
      const path = new URL(file, directory)
      await page.screenshot({ path: fileURLToPath(path), fullPage: true })
      const bytes = await readFile(path)
      entries.push({
        file,
        route,
        viewport: { width, height },
        sha256: createHash('sha256').update(bytes).digest('hex'),
      })
      await page.close()
    }
  }
  if (external.length || errors.length) throw new Error(JSON.stringify({ external, errors }))
  await writeFile(
    new URL('manifest.json', directory),
    JSON.stringify(
      {
        generatedAt: new Date().toISOString(),
        evidence: 'isolated fictional design proposal',
        approved: false,
        liveAcceptance: false,
        entries,
      },
      null,
      2,
    ) + '\n',
  )
  console.log(
    `Captured ${entries.length} isolated proposal screens; no API/external requests or page errors`,
  )
} finally {
  await browser.close()
}
