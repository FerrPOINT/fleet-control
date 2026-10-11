import { readdir, readFile, mkdir, copyFile, writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { createHash } from 'node:crypto'

const output = resolve('../docs/assets/design/chat-controller')
if (process.argv.includes('--verify')) {
  const manifest = JSON.parse(await readFile(resolve(output, 'manifest.json'), 'utf8'))
  if (
    manifest.liveAcceptance !== false ||
    manifest.count !== 9 ||
    manifest.screenshots.length !== 9
  )
    throw new Error('Invalid controller fixture manifest')
  const seen = new Set()
  for (const entry of manifest.screenshots) {
    const expected = `pm-chat-${entry.view}-${entry.viewport.width}.png`
    if (
      !['dialogue', 'clarification', 'requirements'].includes(entry.view) ||
      ![375, 1920, 2560].includes(entry.viewport.width) ||
      entry.file !== expected ||
      seen.has(expected)
    )
      throw new Error('Invalid or duplicate controller screenshot')
    seen.add(expected)
    if (entry.route !== `/chats/00000000-0000-4000-8000-000000000201?tab=${entry.view}`)
      throw new Error('Controller screenshot route mismatch')
    const png = await readFile(resolve(output, entry.file))
    if (
      createHash('sha256').update(png).digest('hex') !== entry.sha256 ||
      png.readUInt32BE(16) !== entry.size.width ||
      png.readUInt32BE(20) !== entry.size.height
    )
      throw new Error(`Controller screenshot content mismatch: ${entry.file}`)
  }
  process.stdout.write('Verified nine fixture controller screenshots and hashes\n')
  process.exit(0)
}
const results = resolve('test-results')
const run = JSON.parse(await readFile(resolve(results, '.last-run.json'), 'utf8'))
if (run.status !== 'passed') throw new Error('Only passed Playwright evidence may be published')
const directories = await readdir(results)
await mkdir(output, { recursive: true })
const screenshots = []
for (const width of [375, 1920, 2560]) {
  for (const view of ['dialogue', 'clarification', 'requirements']) {
    const name = `pm-chat-${view}-${width}.png`
    for (const browser of ['chromium', 'firefox', 'webkit']) {
      const matches = directories.filter(
        (directory) =>
          directory.startsWith('fleet-control-PM-chat-clar-') && directory.endsWith(`-${browser}`),
      )
      if (matches.length !== 1) throw new Error(`Missing unambiguous ${browser} evidence`)
      const source = resolve(results, matches[0], name)
      const png = await readFile(source)
      if (!png.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10])))
        throw new Error(`Invalid PNG: ${source}`)
      const size = { width: png.readUInt32BE(16), height: png.readUInt32BE(20) }
      const height = width === 375 ? 812 : width === 1920 ? 1080 : 1440
      if (size.width !== width || size.height < height)
        throw new Error(`Invalid viewport evidence: ${source}`)
      if (browser === 'chromium') {
        await copyFile(source, resolve(output, name))
        screenshots.push({
          file: name,
          view,
          route: `/chats/00000000-0000-4000-8000-000000000201?tab=${view}`,
          viewport: { width, height },
          size,
          sha256: createHash('sha256').update(png).digest('hex'),
        })
      }
    }
  }
}
await writeFile(
  resolve(output, 'manifest.json'),
  JSON.stringify(
    {
      evidence: 'production-controller-with-fixture-api',
      liveAcceptance: false,
      browsersVerified: ['chromium', 'firefox', 'webkit'],
      command:
        'pnpm exec playwright test e2e/fleet-control.spec.ts --grep "PM chat clarification" --workers=1',
      count: screenshots.length,
      screenshots,
    },
    null,
    2,
  ) + '\n',
)
process.stdout.write(`Published ${screenshots.length} fixture images; liveAcceptance=false\n`)
