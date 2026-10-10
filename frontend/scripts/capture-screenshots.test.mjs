import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import path from 'node:path'
import test from 'node:test'
import { fileURLToPath } from 'node:url'
import { Script } from 'node:vm'

function loadFixture() {
  const scriptUrl = new URL('./capture-screenshots.mjs', import.meta.url)
  const source = readFileSync(scriptUrl, 'utf8')
  const browserStart = source.indexOf('const browser = await chromium.launch()')
  assert(browserStart > 0)
  // Evaluate the real fixture declarations without importing Playwright or starting capture.
  const declarations = source
    .slice(0, browserStart)
    .replace(/^import .+ from .+\r?\n/gm, '')
    .replace('import.meta.url', JSON.stringify(scriptUrl.href))
  return new Script(`${declarations}\n;({ mockApi, ids, sessionRuns, coreScreens, viewports })`, {
    filename: fileURLToPath(scriptUrl),
  }).runInNewContext({ path, fileURLToPath, process: { env: {} }, URL })
}

async function installFixture() {
  const fixture = loadFixture()
  let handler
  const unhandled = await fixture.mockApi({
    route: async (pattern, callback) => {
      assert.equal(pattern, '**/api/v1/**')
      handler = callback
    },
  })
  const request = async (pathname, method = 'GET') => {
    let response
    await handler({
      request: () => ({ url: () => `http://fixture.test${pathname}`, method: () => method }),
      fulfill: async (value) => {
        response = value
      },
    })
    return response
  }
  return { ...fixture, request, unhandled }
}

test('chat capture can read the control journal for every fixture session run', async () => {
  const { request, unhandled, ids, sessionRuns } = await installFixture()
  for (const sessionId of [ids.sessionDev, ids.sessionQa]) {
    const controls = await request(`/api/v1/sessions/${sessionId}/chat-controls`)
    const activeRunId = JSON.parse(controls.body).active_run_id
    if (sessionId === ids.sessionDev) assert.equal(activeRunId, sessionRuns[sessionId][0].id)
    for (const run of sessionRuns[sessionId]) {
      const response = await request(`/api/v1/sessions/${sessionId}/runs/${run.id}/controls`)
      assert.equal(response.status, 200)
      assert.deepEqual(JSON.parse(response.body), [])
    }
  }
  assert.equal(unhandled.size, 0, 'capture must still reject any unhandled fixture API')
})

test('unknown and mismatched run targets remain unhandled', async () => {
  const { request, unhandled, ids, sessionRuns } = await installFixture()
  const paths = [
    `/api/v1/sessions/${ids.sessionDev}/runs/unknown/controls`,
    `/api/v1/sessions/unknown/runs/${sessionRuns[ids.sessionDev][0].id}/controls`,
    `/api/v1/sessions/${ids.sessionQa}/runs/${sessionRuns[ids.sessionDev][0].id}/controls`,
    `/api/v1/sessions/${ids.sessionDev}/runs/${sessionRuns[ids.sessionDev][0].id}/unknown`,
  ]
  for (const pathname of paths) {
    assert.equal((await request(pathname)).status, 404)
    assert(unhandled.has(pathname))
  }
  assert.equal(unhandled.size, paths.length)
})

test('read-only control fixture does not accept mutations or lookups', async () => {
  const { request, unhandled, ids, sessionRuns } = await installFixture()
  const runId = sessionRuns[ids.sessionDev][0].id
  const pathname = `/api/v1/sessions/${ids.sessionDev}/runs/${runId}/controls`
  for (const method of ['POST', 'PUT', 'PATCH', 'DELETE']) {
    assert.equal((await request(pathname, method)).status, 404)
    assert(unhandled.has(pathname))
  }
  assert.equal((await request(`${pathname}/lookup`)).status, 404)
  assert(unhandled.has(`${pathname}/lookup`))
})

test('capture retains 45 views at each of the three required viewports', () => {
  const { coreScreens, viewports } = loadFixture()
  const dimensions = viewports.map(({ name, width, height }) => ({ name, width, height }))
  assert.equal(coreScreens.length, 45)
  assert.equal(new Set(coreScreens.map(([name]) => name)).size, 45)
  assert.deepEqual(JSON.parse(JSON.stringify(dimensions)), [
    { name: '375x812', width: 375, height: 812 },
    { name: '1920x1080', width: 1920, height: 1080 },
    { name: '2560x1440', width: 2560, height: 1440 },
  ])
  assert(viewports.every(({ screens }) => screens === coreScreens))
  assert.equal(
    viewports.reduce((count, { screens }) => count + screens.length, 0),
    135,
  )
})
