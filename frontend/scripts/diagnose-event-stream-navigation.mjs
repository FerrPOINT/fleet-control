import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { createServer } from 'node:http'
import { readFile, writeFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { dirname, resolve } from 'node:path'
import ts from 'typescript'
import { webkit } from '@playwright/test'

// Diagnostic, not a passing product acceptance test. Uses the installed locked
// Base implementation, a disposable local HTTP server and fixture credentials.
const require = createRequire(import.meta.url)
const sdkSourcePath = resolve(dirname(require.resolve('@sdlc/ui/lib')), 'event-stream.ts')
const source = await readFile(sdkSourcePath, 'utf8')
const compiled = ts
  .transpileModule(source, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
  })
  .outputText.replaceAll('export function ', 'function ')
const iterations = 50
const results = []
const server = createServer((request, response) => {
  if (request.url === '/stream') {
    response.writeHead(200, {
      'content-type': 'text/event-stream',
      'cache-control': 'no-store',
    })
    response.end(': heartbeat\n\n')
    return
  }
  response.writeHead(200, { 'content-type': 'text/html' })
  response.end('<!doctype html><title>Owned WebKit transport diagnostic</title>')
})
await new Promise((resolve, reject) => {
  server.once('error', reject)
  server.listen(0, 'localhost', resolve)
})
const origin = `http://localhost:${server.address().port}`
let browser
try {
  browser = await webkit.launch()
  for (const mode of ['healthy', 'pending-navigation', 'disposed-before-navigation']) {
    const context = await browser.newContext()
    const page = await context.newPage()
    const errors = []
    const failedRequests = []
    page.on('pageerror', (error) => errors.push({ message: error.message, stack: error.stack }))
    page.on('requestfailed', (request) =>
      failedRequests.push({ url: request.url(), failure: request.failure() }),
    )
    for (let iteration = 0; iteration < iterations; iteration++) {
      await page.goto(origin)
      await page.addScriptTag({ content: compiled })
      await page.evaluate((mode) => {
        sessionStorage.setItem('unhandled', '0')
        addEventListener('unhandledrejection', () =>
          sessionStorage.setItem(
            'unhandled',
            String(Number(sessionStorage.getItem('unhandled')) + 1),
          ),
        )
        const options = {
          url: '/stream',
          token: 'fixture-token',
          eventTypes: ['session'],
          onEvent() {},
          initialDelayMs: 1,
        }
        if (mode === 'pending-navigation') {
          location.assign('/done')
          // This is the same boundary an EOF reconnect can race with. Keep the
          // transport's catch intact; do not preventDefault on browser errors.
          connectAuthenticatedEventStream(options)
        } else {
          const dispose = connectAuthenticatedEventStream(options)
          if (mode === 'disposed-before-navigation') {
            dispose()
            location.assign('/done')
          } else setTimeout(dispose, 10)
        }
      }, mode)
      if (mode === 'healthy') await page.waitForTimeout(30)
      else await page.waitForURL(`${origin}/done`)
      assert.equal(await page.evaluate(() => sessionStorage.getItem('unhandled')), '0')
    }
    results.push({ mode, iterations, errors, failedRequests, unhandledRejections: 0 })
    await context.close()
  }
} finally {
  await browser?.close()
  await new Promise((resolve) => server.close(resolve))
}
const reproduced = results[1].errors.some((error) =>
  /due to access control checks/.test(error.message),
)
const report = {
  purpose: 'WebKit pageerror while a caught authenticated SSE fetch races with navigation',
  source: '@sdlc/ui installed event-stream.ts, transpiled without semantic edits',
  sdkSourceSha256LF: createHash('sha256').update(source.replaceAll('\r\n', '\n')).digest('hex'),
  playwrightVersion: require('@playwright/test/package.json').version,
  nodeVersion: process.version,
  reproduced,
  productAcceptance: false,
  results,
}
if (process.argv[2]) await writeFile(process.argv[2], `${JSON.stringify(report, null, 2)}\n`)
console.log(
  JSON.stringify(
    {
      ...report,
      results: results.map(({ mode, iterations, errors, failedRequests, unhandledRejections }) => ({
        mode,
        iterations,
        pageErrors: errors.length,
        failedRequests: failedRequests.length,
        unhandledRejections,
        firstError: errors[0],
      })),
    },
    null,
    2,
  ),
)
assert.equal(results[0].errors.length, 0, 'Healthy transport must not emit pageerror')
assert.equal(results[2].errors.length, 0, 'Disposal before navigation must not emit pageerror')
if (!reproduced) process.exitCode = 2
