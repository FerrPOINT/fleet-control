import { copyFile, mkdir, readFile, readdir, writeFile } from 'node:fs/promises'
import { createHash } from 'node:crypto'
import { resolve } from 'node:path'

const output = resolve('../docs/assets/design/chats-pm-consumer')
const manifestPath = resolve(output, 'manifest.json')
const views = ['dialogue', 'single', 'multiple', 'text', 'requirements', 'confirmation', 'diff']
const widths = [375, 1920, 2560]
const digest = (bytes) => createHash('sha256').update(bytes).digest('hex')
const imageInfo = async (path) => {
  const bytes = await readFile(path)
  if (!bytes.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10])))
    throw new Error(`Invalid PNG: ${path}`)
  return { sha256: digest(bytes), width: bytes.readUInt32BE(16), height: bytes.readUInt32BE(20) }
}
if (process.argv.includes('--verify')) {
  const manifest = JSON.parse(await readFile(manifestPath, 'utf8'))
  if (manifest.liveAcceptance !== false || manifest.screenshots.length !== 21)
    throw new Error('Expected 21 explicitly non-live consumer screenshots')
  const seen = new Set()
  for (const entry of manifest.screenshots) {
    if (!views.includes(entry.view) || !widths.includes(entry.viewport.width))
      throw new Error('Unexpected scenario or viewport')
    const expected = `preview/${entry.view}-${entry.viewport.width}.png`
    if (entry.file !== expected || seen.has(expected))
      throw new Error('Invalid screenshot identity')
    seen.add(expected)
    const actual = await imageInfo(resolve(output, entry.file))
    if (
      actual.sha256 !== entry.sha256 ||
      actual.width !== entry.size.width ||
      actual.height !== entry.size.height
    )
      throw new Error(`Screenshot mismatch: ${entry.file}`)
    if (actual.width !== entry.viewport.width || actual.height < entry.viewport.height)
      throw new Error('Screenshot does not cover the viewport')
  }
  for (const [file, sha256] of Object.entries(manifest.sourceSha256))
    if (digest(await readFile(resolve('..', file))) !== sha256)
      throw new Error(`Source changed: ${file}`)
  process.stdout.write(
    'Verified 21 consumer preview screenshots, hashes and source identity; liveAcceptance=false\n',
  )
} else {
  const results = resolve('test-results')
  const lastRun = JSON.parse(await readFile(resolve(results, '.last-run.json'), 'utf8'))
  if (lastRun.status !== 'passed')
    throw new Error('Browser run must pass before publishing evidence')
  const directories = await readdir(results)
  await mkdir(resolve(output, 'preview'), { recursive: true })
  const screenshots = []
  for (const width of widths) {
    const viewport = { width, height: width === 375 ? 812 : width === 1920 ? 1080 : 1440 }
    for (const view of views) {
      const name = ['dialogue', 'requirements'].includes(view)
        ? `pm-chat-${view}-${width}.png`
        : `consumer-${view}-${width}.png`
      for (const browser of ['chromium', 'firefox', 'webkit']) {
        const matches = directories.filter(
          (directory) =>
            directory.startsWith('fleet-control-PM-chat-clar-') &&
            directory.endsWith(`-${browser}`),
        )
        if (matches.length !== 1) throw new Error(`Ambiguous ${browser} evidence`)
        const source = resolve(results, matches[0], name)
        const actual = await imageInfo(source)
        if (actual.width !== width || actual.height < viewport.height)
          throw new Error('Invalid viewport')
        if (browser === 'chromium') {
          const file = `preview/${view}-${width}.png`
          await copyFile(source, resolve(output, file))
          screenshots.push({
            file,
            view,
            viewport,
            size: { width: actual.width, height: actual.height },
            sha256: actual.sha256,
          })
        }
      }
    }
  }
  const sourceSha256 = {}
  for (const file of [
    'frontend/src/pages/chat-detail/index.tsx',
    'frontend/e2e/fleet-control.spec.ts',
  ])
    sourceSha256[file] = digest(await readFile(resolve('..', file)))
  await writeFile(
    manifestPath,
    JSON.stringify(
      {
        evidence: 'production-controller-with-fixture-api',
        liveAcceptance: false,
        baseline: '08c6c56229ee308050176293e34d65f087b5b112',
        browsersVerified: ['chromium', 'firefox', 'webkit'],
        baseURL: process.env.PLAYWRIGHT_BASE_URL ?? 'http://localhost:4173',
        command:
          'pnpm exec playwright test e2e/fleet-control.spec.ts e2e/chats-directory.spec.ts --workers=1',
        sourceSha256,
        screenshots,
      },
      null,
      2,
    ) + '\n',
  )
  process.stdout.write('Published 21 consumer preview screenshots; liveAcceptance=false\n')
}
