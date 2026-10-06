import { readdir, readFile, mkdir, copyFile, writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { createHash } from 'node:crypto'

const output = resolve('../docs/assets/design/session-composer')
const route = '/sessions/00000000-0000-4000-8000-000000000201'
const browsers = ['chromium', 'firefox', 'webkit']
const viewports = [
  { width: 375, height: 812 },
  { width: 1920, height: 1080 },
  { width: 2560, height: 1440 },
]
function inspect(png, viewport, scale = 1) {
  if (!png.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10])))
    throw new Error('Invalid screenshot PNG')
  const size = { width: png.readUInt32BE(16), height: png.readUInt32BE(20) }
  if (size.width !== viewport.width * scale || size.height < viewport.height * scale)
    throw new Error('Session composer full-page screenshot viewport mismatch')
  return { size, sha256: createHash('sha256').update(png).digest('hex') }
}
if (process.argv.includes('--verify')) {
  const manifest = JSON.parse(await readFile(resolve(output, 'manifest.json'), 'utf8'))
  if (
    manifest.evidence !== 'production-controller-with-fixture-api' ||
    manifest.liveAcceptance !== false ||
    manifest.count !== 3 ||
    manifest.screenshots.length !== 3 ||
    JSON.stringify(manifest.browsersVerified) !== JSON.stringify(browsers)
  )
    throw new Error('Invalid session composer fixture manifest')
  for (const [index, viewport] of viewports.entries()) {
    const entry = manifest.screenshots[index]
    if (
      entry.route !== route ||
      entry.view !== 'legacy-first-prompt' ||
      entry.file !== `legacy-first-prompt-${viewport.width}.png` ||
      JSON.stringify(entry.viewport) !== JSON.stringify(viewport)
    )
      throw new Error('Invalid session composer screenshot identity')
    const proof = inspect(await readFile(resolve(output, entry.file)), viewport)
    if (entry.sha256 !== proof.sha256 || JSON.stringify(entry.size) !== JSON.stringify(proof.size))
      throw new Error('Session composer screenshot hash mismatch')
  }
  process.stdout.write('Verified three session composer fixture screenshots\n')
  process.exit(0)
}
const results = resolve('test-results')
const run = JSON.parse(await readFile(resolve(results, '.last-run.json'), 'utf8'))
if (run.status !== 'passed') throw new Error('Only passed Playwright evidence may be published')
const directories = await readdir(results)
await mkdir(output, { recursive: true })
const screenshots = []
for (const viewport of viewports) {
  const file = `legacy-first-prompt-${viewport.width}.png`
  for (const browser of browsers) {
    const matches = directories.filter(
      (directory) =>
        directory.startsWith('fleet-control-legacy-sessi-') && directory.endsWith(`-${browser}`),
    )
    if (matches.length !== 1) throw new Error(`Missing unambiguous ${browser} composer evidence`)
    const source = resolve(results, matches[0], file)
    const proof = inspect(await readFile(source), viewport, browser === 'webkit' ? 2 : 1)
    if (browser === 'chromium') {
      await copyFile(source, resolve(output, file))
      screenshots.push({ file, view: 'legacy-first-prompt', route, viewport, ...proof })
    }
  }
}
await writeFile(
  resolve(output, 'manifest.json'),
  JSON.stringify(
    {
      evidence: 'production-controller-with-fixture-api',
      liveAcceptance: false,
      browsersVerified: browsers,
      command:
        'pnpm exec playwright test e2e/fleet-control.spec.ts --grep "legacy session permits"',
      count: screenshots.length,
      screenshots,
    },
    null,
    2,
  ) + '\n',
)
process.stdout.write('Published three session composer fixture images; liveAcceptance=false\n')
