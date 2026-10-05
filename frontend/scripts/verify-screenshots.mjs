import { existsSync, readFileSync, statSync } from 'node:fs'
import { createHash } from 'node:crypto'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const repoRoot = path.resolve(__dirname, '../..')
const manifestPath = path.join(repoRoot, 'docs/assets/screens/manifest.md')
const manifest = readFileSync(manifestPath, 'utf8')
const lines = manifest.split(/\r?\n/)
const totalMatch = manifest.match(/Total screenshots:\s+(\d+)\./)

if (!totalMatch) {
  throw new Error('Screenshot manifest is missing the total screenshot count.')
}

const entries = lines
  .map((line) => line.match(/^\|\s*([^|]+?)\s*\|\s*`([^`]+\.png)`\s*\|\s*`([^`]+)`\s*\|$/))
  .filter(Boolean)
  .map((match) => ({
    viewport: match[1].trim(),
    filePath: match[2],
    route: match[3],
  }))

const expectedTotal = Number(totalMatch[1])
const evidence = JSON.parse(
  readFileSync(path.join(repoRoot, 'docs/assets/screens/manifest.json'), 'utf8'),
)
if (
  evidence.evidence !== 'production-pages-with-fixture-api' ||
  evidence.liveAcceptance !== false ||
  evidence.count !== expectedTotal ||
  evidence.screenshots.length !== expectedTotal
)
  throw new Error('Screenshot JSON evidence does not match its declared fixture scope/count.')
const proofs = new Map(evidence.screenshots.map((entry) => [entry.filePath, entry]))
if (proofs.size !== expectedTotal) throw new Error('Screenshot JSON evidence contains duplicates.')

if (entries.length !== expectedTotal) {
  throw new Error(
    `Screenshot manifest total mismatch: declared ${expectedTotal}, found ${entries.length} rows.`,
  )
}

if (entries.length < 80) {
  throw new Error(`Screenshot manifest is unexpectedly small: ${entries.length} rows.`)
}

const requiredViewports = new Set(['375x812', '1920x1080', '2560x1440'])
const foundViewports = new Set(entries.map((entry) => entry.viewport))

for (const viewport of requiredViewports) {
  if (!foundViewports.has(viewport)) {
    throw new Error(`Screenshot manifest is missing required viewport ${viewport}.`)
  }
}

const requiredRoutes = [
  '/chats',
  '/leaders',
  '/leaders/new',
  '/executors',
  '/executors/new',
  '/agents',
  '/agents/new',
  '/sessions',
  '/workflows',
  '/deployments',
  '/deployments?tab=jobs',
  '/logs',
  '/logs?tab=audit',
  '/settings',
  '/settings?tab=retention',
  '/settings?tab=history',
  '/settings?tab=users',
  '/access-denied',
  '/not-a-fleet-route',
]

for (const route of requiredRoutes) {
  for (const viewport of requiredViewports) {
    if (!entries.some((entry) => entry.route === route && entry.viewport === viewport)) {
      throw new Error(`Screenshot manifest is missing required route ${route} at ${viewport}.`)
    }
  }
}

const pngMagic = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10])
const seen = new Set()
const screensRoot = path.join(repoRoot, 'docs/assets/screens')

for (const entry of entries) {
  const absolutePath = path.resolve(repoRoot, entry.filePath)
  if (!absolutePath.startsWith(screensRoot + path.sep)) {
    throw new Error(`Screenshot path must stay within the screenshot directory: ${entry.filePath}.`)
  }
  if (!requiredViewports.has(entry.viewport)) {
    throw new Error(`Screenshot manifest contains an unsupported viewport ${entry.viewport}.`)
  }
  if (seen.has(entry.filePath)) {
    throw new Error(`Screenshot manifest contains duplicate file ${entry.filePath}.`)
  }
  seen.add(entry.filePath)

  if (!existsSync(absolutePath)) {
    throw new Error(`Screenshot file listed in manifest does not exist: ${entry.filePath}.`)
  }

  const stats = statSync(absolutePath)
  if (stats.size < 1024) {
    throw new Error(`Screenshot file is too small to be a useful capture: ${entry.filePath}.`)
  }

  const header = readFileSync(absolutePath)
  if (!header.subarray(0, 8).equals(pngMagic)) {
    throw new Error(`Screenshot file is not a PNG: ${entry.filePath}.`)
  }
  const proof = proofs.get(entry.filePath)
  const [width, height] = entry.viewport.split('x').map(Number)
  if (
    !proof ||
    proof.viewport !== entry.viewport ||
    proof.route !== entry.route ||
    proof.sha256 !== createHash('sha256').update(header).digest('hex') ||
    proof.size.width !== header.readUInt32BE(16) ||
    proof.size.height !== header.readUInt32BE(20) ||
    proof.size.width !== width ||
    proof.size.height < height
  )
    throw new Error(`Screenshot content/route/viewport proof mismatch: ${entry.filePath}.`)
}

console.log(`Verified ${entries.length} screenshots across ${foundViewports.size} viewports.`)
