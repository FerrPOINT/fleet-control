import assert from 'node:assert/strict'
import { test } from 'node:test'
import { spawnSync } from 'node:child_process'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { requireRuntimeControlKeys, retireRunWideApproval } from './base-openapi-compat.mjs'

const path = '/api/v1/sessions/{session_id}/runs/{run_id}/approval'
const pair = () => {
  const base = {
    openapi: '3.1.0',
    paths: {
      [path]: {
        post: {
          requestBody: {
            content: { 'application/json': { schema: { type: 'string' } } },
          },
          responses: {
            200: {
              content: {
                'application/json': {
                  schema: { $ref: '#/components/schemas/RuntimeRunControlResponse' },
                },
              },
            },
            403: { description: 'Forbidden' },
          },
        },
      },
      '/api/v1/sessions': { get: { responses: { 200: { description: 'OK' } } } },
    },
    components: { schemas: { RuntimeRunControlResponse: { type: 'object' } } },
  }
  const current = structuredClone(base)
  current.paths[path].post.responses = {
    409: { description: 'Use the exact approval request decision endpoint' },
    403: { description: 'Forbidden' },
  }
  return { base, current }
}

const controlPaths = ['steer', 'stop'].map(
  (operation) => `/api/v1/sessions/{session_id}/runs/{run_id}/${operation}`,
)
function controlPair() {
  const specs = pair()
  const header = {
    name: 'Idempotency-Key',
    in: 'header',
    required: true,
    schema: { type: 'string' },
  }
  for (const route of controlPaths) {
    specs.base.paths[route] = {
      post: {
        parameters: [{ name: 'run_id', in: 'path', required: true, schema: { type: 'string' } }],
        responses: { 200: { description: 'OK' }, 403: { description: 'Forbidden' } },
      },
    }
    specs.current.paths[route] = structuredClone(specs.base.paths[route])
    specs.current.paths[route].post.parameters.push(structuredClone(header))
  }
  return specs
}

test('key migration adjusts only two missing legacy headers and never mutates the baseline', () => {
  const { base, current } = controlPair()
  const original = structuredClone(base)
  const result = requireRuntimeControlKeys(base, current)
  assert.deepEqual(base, original)
  assert.deepEqual(result.retired, ['steer', 'stop'])
  const expected = structuredClone(base)
  for (const route of controlPaths) {
    expected.paths[route].post.parameters.push(current.paths[route].post.parameters[1])
  }
  assert.deepEqual(result.base, expected)
  assert.deepEqual(requireRuntimeControlKeys(result.base, current), {
    base: result.base,
    retired: [],
  })
})

test('key migration rejects absent, duplicate, optional, renamed, typed or referenced headers', () => {
  for (const mutate of [
    (post) => post.parameters.pop(),
    (post) => post.parameters.push(structuredClone(post.parameters[1])),
    (post) => (post.parameters[1].required = false),
    (post) => (post.parameters[1].name = 'X-Command-Key'),
    (post) => (post.parameters[1].schema.type = 'integer'),
    (post) => (post.parameters[1].schema = { $ref: '#/components/schemas/CommandKey' }),
    (post) => (post.parameters[1].in = 'query'),
  ]) {
    for (const route of controlPaths) {
      const { base, current } = controlPair()
      mutate(current.paths[route].post)
      assert.throws(() => requireRuntimeControlKeys(base, current), /security migration/)
    }
  }
  const { base, current } = controlPair()
  delete current.paths[controlPaths[0]]
  assert.throws(() => requireRuntimeControlKeys(base, current), /security migration/)
  base.paths[controlPaths[0]].post.parameters.push({ name: 'Idempotency-Key', in: 'header' })
  assert.throws(() => requireRuntimeControlKeys(base, controlPair().current), /security migration/)
})

test('key migration still rejects restoring unkeyed controls after the baseline upgrade', () => {
  const { current: base } = controlPair()
  const current = structuredClone(base)
  current.paths[controlPaths[1]].post.parameters.pop()
  assert.throws(() => requireRuntimeControlKeys(base, current), /security migration/)
})

test('Base still rejects other control header, response and sibling operation changes', () => {
  const directory = mkdtempSync(join(tmpdir(), 'fleet-control-compat-test-'))
  try {
    for (const [change, message] of [
      [(spec) => delete spec.paths[controlPaths[1]].post.responses[403], 'removed response 403'],
      [
        (spec) =>
          spec.paths[controlPaths[0]].post.parameters.push({
            name: 'Other',
            in: 'header',
            required: true,
            schema: { type: 'string' },
          }),
        'added required parameter header:Other',
      ],
      [(spec) => delete spec.paths['/api/v1/sessions'], 'removed path /api/v1/sessions'],
    ]) {
      const { base, current } = controlPair()
      change(current)
      const adjusted = requireRuntimeControlKeys(
        retireRunWideApproval(base, current).base,
        current,
      ).base
      const baseline = join(directory, 'base.json')
      const candidate = join(directory, 'current.json')
      writeFileSync(baseline, JSON.stringify(adjusted))
      writeFileSync(candidate, JSON.stringify(current))
      const result = spawnSync(
        process.execPath,
        [
          resolve('../../services-base/frontend/scripts/sdlc-openapi-compat.mjs'),
          '--base',
          baseline,
          '--current',
          candidate,
        ],
        { encoding: 'utf8' },
      )
      assert.equal(result.status, 1, result.stderr)
      assert.ok(result.stderr.includes(message), result.stderr)
    }
  } finally {
    rmSync(directory, { recursive: true, force: true })
  }
})

test('retires only the exact historical response without mutating the source', () => {
  const { base, current } = pair()
  const original = structuredClone(base)
  const result = retireRunWideApproval(base, current)
  assert.equal(result.retired, true)
  assert.deepEqual(base, original)
  const expected = structuredClone(base)
  delete expected.paths[path].post.responses[200]
  assert.deepEqual(result.base, expected)
  assert.deepEqual(retireRunWideApproval(result.base, current), {
    base: result.base,
    retired: false,
  })
})

test('does not forgive removal, changed refusal, different schema or restored success', () => {
  for (const alter of [
    ({ current }) => delete current.paths[path],
    ({ current }) => (current.paths[path].post.responses[409].description = 'Different'),
    ({ base }) =>
      (base.paths[path].post.responses[200].content['application/json'].schema.$ref = '#/Wrong'),
    ({ current }) => (current.paths[path].post.responses[202] = { description: 'Accepted' }),
    ({ current }) => (current.paths[path].post.responses['2XX'] = { description: 'Success' }),
  ]) {
    const specs = pair()
    alter(specs)
    assert.throws(() => retireRunWideApproval(specs.base, specs.current), /security migration/)
  }
})

test('security guard remains enforced after the baseline includes retirement', () => {
  const { current: base } = pair()
  assert.deepEqual(retireRunWideApproval(base, structuredClone(base)), { base, retired: false })
  for (const status of ['200', '201', '2XX']) {
    const current = structuredClone(base)
    current.paths[path].post.responses[status] = { description: 'Restored success' }
    assert.throws(() => retireRunWideApproval(base, current), /security migration/)
  }
  const current = structuredClone(base)
  delete current.paths[path].post.responses[409]
  assert.throws(() => retireRunWideApproval(base, current), /security migration/)
})

test('Base still rejects removed sibling operations and changes on the retired route', () => {
  const directory = mkdtempSync(join(tmpdir(), 'fleet-openapi-test-'))
  try {
    for (const [change, expected] of [
      [(spec) => delete spec.paths['/api/v1/sessions'], 'removed path /api/v1/sessions'],
      [(spec) => delete spec.paths[path].post.requestBody, 'removed request body'],
      [(spec) => delete spec.paths[path].post.responses[403], 'removed response 403'],
      [
        (spec) => (spec.components.schemas.RuntimeRunControlResponse.type = 'string'),
        'changed type object -> string',
      ],
    ]) {
      const { base, current } = pair()
      const adjusted = retireRunWideApproval(base, current).base
      change(current)
      const baseline = join(directory, 'baseline.json')
      const candidate = join(directory, 'candidate.json')
      writeFileSync(baseline, JSON.stringify(adjusted))
      writeFileSync(candidate, JSON.stringify(current))
      const result = spawnSync(
        process.execPath,
        [
          resolve('../../services-base/frontend/scripts/sdlc-openapi-compat.mjs'),
          '--base',
          baseline,
          '--current',
          candidate,
        ],
        { encoding: 'utf8' },
      )
      assert.equal(result.status, 1)
      assert.ok(result.stderr.includes(expected), result.stderr)
    }
  } finally {
    rmSync(directory, { recursive: true, force: true })
  }
})
