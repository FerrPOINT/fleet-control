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

// These two legacy commands may no longer execute without caller-owned identity.
export function requireControlIdentity(base, current) {
  const adjusted = structuredClone(base)
  const migrated = []
  const expected = {
    name: 'Idempotency-Key',
    in: 'header',
    required: true,
    schema: { type: 'string' },
  }
  for (const operation of ['steer', 'stop']) {
    const path = `/api/v1/sessions/{session_id}/runs/{run_id}/${operation}`
    const parameters = current.paths?.[path]?.post?.parameters ?? []
    const keys = parameters.filter(
      (parameter) => parameter.name?.toLowerCase() === 'idempotency-key',
    )
    if (keys.length !== 1 || !isDeepStrictEqual(keys[0], expected))
      throw new Error('Runtime control identity differs from the documented security migration')
    const previous = adjusted.paths?.[path]?.post
    if (!previous) continue
    const existing = (previous.parameters ?? []).filter(
      (parameter) => parameter.name?.toLowerCase() === 'idempotency-key',
    )
    if (existing.length && (existing.length !== 1 || !isDeepStrictEqual(existing[0], expected)))
      throw new Error('Unexpected historical runtime control identity contract')
    if (!existing.length) {
      previous.parameters = [...(previous.parameters ?? []), structuredClone(expected)]
      migrated.push(path)
    }
  }
  return { base: adjusted, migrated }
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
  const { base, migrated } = requireControlIdentity(approval.base, current)
  const temporary = mkdtempSync(join(tmpdir(), 'fleet-openapi-compat-'))
  try {
    const basePath = join(temporary, 'base.json')
    writeFileSync(basePath, JSON.stringify(base))
    if (approval.retired)
      process.stdout.write(
        'Explicit security migration: run-wide approval 200 -> 409; see docs/API.md.\n',
      )
    if (migrated.length)
      process.stdout.write(
        'Explicit security migration: stop/steer require Idempotency-Key; see docs/API.md.\n',
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
