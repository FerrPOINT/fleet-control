import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import test from 'node:test'

const scripts = dirname(fileURLToPath(import.meta.url))
const original = JSON.parse(
  readFileSync(resolve(scripts, '../../docs/assets/screens/manifest.json')),
)

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), 'fleet-screenshot-verifier-'))
  t.after(() => rmSync(root, { recursive: true, force: true }))
  const directory = join(root, 'frontend/scripts')
  mkdirSync(directory, { recursive: true })
  copyFileSync(join(scripts, 'verify-screenshots.mjs'), join(directory, 'verify-screenshots.mjs'))
  const manifest = structuredClone(original)
  for (const entry of manifest.screenshots) {
    // Header fixtures exercise evidence checks, not image decoding or visual acceptance.
    const png = Buffer.alloc(1024)
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]).copy(png)
    const [width, height] = entry.viewport.split('x').map(Number)
    png.writeUInt32BE(width, 16)
    png.writeUInt32BE(height, 20)
    entry.size = { width, height }
    entry.sha256 = createHash('sha256').update(png).digest('hex')
    mkdirSync(dirname(join(root, entry.filePath)), { recursive: true })
    writeFileSync(join(root, entry.filePath), png)
  }
  function run() {
    const output = join(root, 'docs/assets/screens')
    writeFileSync(join(output, 'manifest.json'), JSON.stringify(manifest))
    writeFileSync(
      join(output, 'manifest.md'),
      `Total screenshots: ${manifest.count}.\n` +
        manifest.screenshots
          .map((entry) => `| ${entry.viewport} | \`${entry.filePath}\` | \`${entry.route}\` |`)
          .join('\n'),
    )
    return spawnSync(process.execPath, [join(directory, 'verify-screenshots.mjs')], {
      encoding: 'utf8',
    })
  }
  return { root, manifest, run }
}

test('complete three-viewport fixture passes hash, route and dimension checks', (t) => {
  const f = fixture(t)
  const result = f.run()
  assert.equal(result.status, 0, result.stderr)
  assert.match(result.stdout, /Verified 135 screenshots across 3 viewports/)
})

for (const [name, mutate, message] of [
  [
    'changed content',
    (f) => writeFileSync(join(f.root, f.manifest.screenshots[0].filePath), Buffer.alloc(1024)),
    /not a PNG/,
  ],
  [
    'changed hash',
    (f) => {
      f.manifest.screenshots[0].sha256 = '0'.repeat(64)
    },
    /proof mismatch/,
  ],
  [
    'changed dimension',
    (f) => {
      f.manifest.screenshots[0].size.width += 1
    },
    /proof mismatch/,
  ],
  [
    'duplicate path',
    (f) => {
      f.manifest.screenshots[1].filePath = f.manifest.screenshots[0].filePath
    },
    /duplicates/,
  ],
  [
    'wrong count',
    (f) => {
      f.manifest.count += 1
    },
    /scope\/count/,
  ],
  [
    'live evidence claim',
    (f) => {
      f.manifest.liveAcceptance = true
    },
    /scope\/count/,
  ],
  [
    'missing mobile route',
    (f) => {
      f.manifest.screenshots.find(
        (entry) => entry.route === '/chats' && entry.viewport === '375x812',
      ).route = '/replacement'
    },
    /missing required route \/chats at 375x812/,
  ],
  [
    'path traversal',
    (f) => {
      f.manifest.screenshots[0].filePath = 'docs/assets/screens/../outside.png'
    },
    /path must stay within/,
  ],
]) {
  test(`rejects ${name}`, (t) => {
    const f = fixture(t)
    mutate(f)
    const result = f.run()
    assert.notEqual(result.status, 0)
    assert.match(result.stderr, message)
  })
}
