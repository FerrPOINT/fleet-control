import { createHash } from 'node:crypto'
import { copyFile, mkdir, readFile, readdir, writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { parseArgs } from 'node:util'

const { values } = parseArgs({
  options: {
    verify: { type: 'boolean', default: false },
    input: { type: 'string', default: 'test-results/runtime-controls-css' },
  },
})

const output = resolve('../docs/assets/design/runtime-controls')
const browsers = ['chromium', 'firefox', 'webkit']
const viewports = [
  { width: 375, height: 812 },
  { width: 1920, height: 1080 },
  { width: 2560, height: 1440 },
]
const digest = (bytes) => createHash('sha256').update(bytes).digest('hex')
const signature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10])

async function inspect(path, viewport) {
  const png = await readFile(path)
  if (!png.subarray(0, 8).equals(signature)) throw new Error(`Not a PNG: ${path}`)
  const size = { width: png.readUInt32BE(16), height: png.readUInt32BE(20) }
  if (size.width !== viewport.width || size.height < viewport.height)
    throw new Error(`Screenshot viewport mismatch: ${path}`)
  return { size, sha256: digest(png) }
}

if (values.verify) {
  const manifest = JSON.parse(await readFile(resolve(output, 'manifest.json'), 'utf8'))
  if (
    manifest.evidence !== 'production-chat-with-fixture-api' ||
    manifest.liveAcceptance !== false ||
    manifest.screenshots.length !== 9
  )
    throw new Error('Invalid fixture evidence metadata')
  const remaining = new Set(
    browsers.flatMap((browser) => viewports.map(({ width }) => `${browser}-${width}.png`)),
  )
  for (const entry of manifest.screenshots) {
    const viewport = viewports.find(({ width }) => width === entry.viewport.width)
    if (
      !viewport ||
      viewport.height !== entry.viewport.height ||
      entry.route !== '/chats/session1' ||
      entry.file !== `${entry.browser}-${viewport.width}.png` ||
      !remaining.delete(entry.file)
    )
      throw new Error('Invalid or duplicate screenshot entry')
    const actual = await inspect(resolve(output, entry.file), viewport)
    if (
      actual.sha256 !== entry.sha256 ||
      JSON.stringify(actual.size) !== JSON.stringify(entry.size)
    )
      throw new Error(`Changed screenshot: ${entry.file}`)
  }
  if (remaining.size) throw new Error('Missing browser/viewport evidence')
  process.stdout.write('Verified nine runtime-control fixture screenshots; liveAcceptance=false\n')
} else {
  const input = resolve(values.input)
  const run = JSON.parse(await readFile(resolve(input, '.last-run.json'), 'utf8'))
  if (run.status !== 'passed') throw new Error('Only a passed Playwright run may be published')
  const directories = await readdir(input)
  const screenshots = []
  await mkdir(output, { recursive: true })
  for (const browser of browsers) {
    const matches = directories.filter(
      (name) => name.startsWith('runtime-controls-fixture-') && name.endsWith(`-${browser}`),
    )
    if (matches.length !== 1) throw new Error(`Missing unambiguous ${browser} test result`)
    for (const viewport of viewports) {
      const source = resolve(input, matches[0], `control-uncertain-${viewport.width}.png`)
      const image = await inspect(source, viewport)
      const file = `${browser}-${viewport.width}.png`
      await copyFile(source, resolve(output, file))
      screenshots.push({ browser, file, route: '/chats/session1', viewport, ...image })
    }
  }
  await writeFile(
    resolve(output, 'manifest.json'),
    JSON.stringify(
      {
        evidence: 'production-chat-with-fixture-api',
        liveAcceptance: false,
        command:
          'PLAYWRIGHT_BASE_URL=http://127.0.0.1:4189 pnpm exec playwright test e2e/runtime-controls.spec.ts --workers=1 --output=test-results/runtime-controls-css',
        screenshots,
      },
      null,
      2,
    ) + '\n',
  )
  process.stdout.write('Published nine fixture screenshots; no live runtime acceptance claimed\n')
}
