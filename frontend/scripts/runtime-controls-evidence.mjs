import { createHash } from 'node:crypto'
import { access, copyFile, mkdir, readFile, readdir, writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { parseArgs } from 'node:util'

const { values } = parseArgs({
  options: {
    verify: { type: 'boolean', default: false },
    input: { type: 'string', default: 'test-results/runtime-controls-css' },
    scenario: { type: 'string', default: 'runtime-controls' },
  },
})

const scenarios = {
  'runtime-controls': {
    prefix: 'control-uncertain',
    route: '/chats/session1',
    grep: 'HTTP-success uncertainty',
  },
  'clarification-uncertain': {
    prefix: 'clarification-uncertain',
    route: '/chats/session1?tab=clarification&question=q2',
    grep: 'uncertain clarification',
  },
}
if (!Object.hasOwn(scenarios, values.scenario)) throw new Error('Unknown fixture scenario')
const scenario = scenarios[values.scenario]
const output = resolve(`../docs/assets/design/${values.scenario}`)
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
    (manifest.scenario ?? 'runtime-controls') !== values.scenario ||
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
      entry.route !== scenario.route ||
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
  process.stdout.write(
    `Verified nine ${values.scenario} fixture screenshots; liveAcceptance=false\n`,
  )
} else {
  const input = resolve(values.input)
  const run = JSON.parse(await readFile(resolve(input, '.last-run.json'), 'utf8'))
  if (run.status !== 'passed') throw new Error('Only a passed Playwright run may be published')
  const directories = await readdir(input)
  const screenshots = []
  await mkdir(output, { recursive: true })
  for (const browser of browsers) {
    const candidates = directories.filter(
      (name) => name.startsWith('runtime-controls-fixture-') && name.endsWith(`-${browser}`),
    )
    const matches = []
    for (const candidate of candidates) {
      try {
        await access(resolve(input, candidate, `${scenario.prefix}-375.png`))
        matches.push(candidate)
      } catch (error) {
        if (error.code !== 'ENOENT') throw error
      }
    }
    if (matches.length !== 1) throw new Error(`Missing unambiguous ${browser} test result`)
    for (const viewport of viewports) {
      const source = resolve(input, matches[0], `${scenario.prefix}-${viewport.width}.png`)
      const image = await inspect(source, viewport)
      const file = `${browser}-${viewport.width}.png`
      await copyFile(source, resolve(output, file))
      screenshots.push({ browser, file, route: scenario.route, viewport, ...image })
    }
  }
  await writeFile(
    resolve(output, 'manifest.json'),
    JSON.stringify(
      {
        evidence: 'production-chat-with-fixture-api',
        scenario: values.scenario,
        liveAcceptance: false,
        command: `PLAYWRIGHT_BASE_URL=http://127.0.0.1:4173 pnpm exec playwright test e2e/runtime-controls.spec.ts --grep "${scenario.grep}" --workers=1 --output=${values.input}`,
        screenshots,
      },
      null,
      2,
    ) + '\n',
  )
  process.stdout.write('Published nine fixture screenshots; no live runtime acceptance claimed\n')
}
