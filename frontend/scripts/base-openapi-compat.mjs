import { execFileSync, spawnSync } from 'node:child_process'
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'
import { isDeepStrictEqual } from 'node:util'

const retiredPath = '/api/v1/sessions/{session_id}/runs/{run_id}/approval'

// This one security retirement is explicit; Base checks every other contract field.
export function retireRunWideApproval(base, current) {
  const response = base.paths?.[retiredPath]?.post?.responses?.['200']
  const replacement = current.paths?.[retiredPath]?.post?.responses
  if (
    replacement?.['409']?.description !== 'Use the exact approval request decision endpoint' ||
    Object.keys(replacement).some((status) => /^2(?:\d\d|XX)$/i.test(status)) ||
    (response &&
      response.content?.['application/json']?.schema?.$ref !==
        '#/components/schemas/RuntimeRunControlResponse')
  )
    throw new Error('Run-wide approval retirement differs from the documented security migration')
  if (!response) return { base, retired: false }
  const adjusted = structuredClone(base)
  delete adjusted.paths[retiredPath].post.responses['200']
  return { base: adjusted, retired: true }
}

// Legacy controls without a durable command key are deliberately refused, not replayed.
export function requireRuntimeControlKeys(base, current) {
  const expected = {
    name: 'Idempotency-Key',
    in: 'header',
    required: true,
    schema: { type: 'string' },
  }
  const adjusted = structuredClone(base)
  const retired = []
  for (const operation of ['steer', 'stop']) {
    const route = `/api/v1/sessions/{session_id}/runs/{run_id}/${operation}`
    const candidate = current.paths?.[route]?.post
    const headers = candidate?.parameters?.filter(
      (parameter) =>
        parameter.in === 'header' && parameter.name.toLowerCase() === 'idempotency-key',
    )
    if (headers?.length !== 1 || !isDeepStrictEqual(headers[0], expected))
      throw new Error('Runtime control key differs from the documented security migration')
    const previous = adjusted.paths?.[route]?.post
    const old =
      previous?.parameters?.filter(
        (parameter) =>
          parameter.in === 'header' && parameter.name.toLowerCase() === 'idempotency-key',
      ) ?? []
    if (old.length && (old.length !== 1 || !isDeepStrictEqual(old[0], expected)))
      throw new Error(
        'Historical runtime control key differs from the documented security migration',
      )
    if (previous && !old.length) {
      previous.parameters = [...(previous.parameters ?? []), expected]
      retired.push(operation)
    }
  }
  return { base: adjusted, retired }
}

export function checkCompatibility() {
  const baseline = JSON.parse(
    execFileSync('git', ['show', 'origin/main:openapi/openapi.json'], {
      encoding: 'utf8',
      maxBuffer: 20 * 1024 * 1024,
    }),
  )
  const currentPath = resolve('../openapi/openapi.json')
  const current = JSON.parse(readFileSync(currentPath, 'utf8'))
  const approval = retireRunWideApproval(baseline, current)
  const controls = requireRuntimeControlKeys(approval.base, current)
  const base = controls.base
  const temporary = mkdtempSync(join(tmpdir(), 'fleet-openapi-compat-'))
  try {
    const basePath = join(temporary, 'base.json')
    writeFileSync(basePath, JSON.stringify(base))
    if (approval.retired)
      process.stdout.write(
        'Explicit security migration: run-wide approval 200 -> 409; see docs/API.md.\n',
      )
    if (controls.retired.length)
      process.stdout.write(
        `Explicit security migration: ${controls.retired.join('/')} requires Idempotency-Key; see docs/API_VERSIONING.md.\n`,
      )
    const result = spawnSync(
      process.execPath,
      [
        resolve('../../services-base/frontend/scripts/sdlc-openapi-compat.mjs'),
        '--base',
        basePath,
        '--current',
        currentPath,
      ],
      { stdio: 'inherit' },
    )
    return result.status ?? 1
  } finally {
    rmSync(temporary, { recursive: true, force: true })
  }
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url)
  process.exit(checkCompatibility())
