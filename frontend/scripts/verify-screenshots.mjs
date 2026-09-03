import { existsSync, readFileSync, statSync } from 'node:fs'
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

if (entries.length !== expectedTotal) {
  throw new Error(
    `Screenshot manifest total mismatch: declared ${expectedTotal}, found ${entries.length} rows.`,
  )
}

if (entries.length < 120) {
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
  '/settings?tab=users',
  '/access-denied',
  '/not-a-fleet-route',
]

for (const route of requiredRoutes) {
  if (!entries.some((entry) => entry.route === route)) {
    throw new Error(`Screenshot manifest is missing required route ${route}.`)
  }
}

const pngMagic = Buffer.from([0x89, 0x50, 0x4e, 0x47])
const seen = new Set()

for (const entry of entries) {
  const absolutePath = path.join(repoRoot, entry.filePath)
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

  const header = readFileSync(absolutePath, { encoding: null, length: 4 })
  if (!header.subarray(0, 4).equals(pngMagic)) {
    throw new Error(`Screenshot file is not a PNG: ${entry.filePath}.`)
  }
}

console.log(`Verified ${entries.length} screenshots across ${foundViewports.size} viewports.`)
