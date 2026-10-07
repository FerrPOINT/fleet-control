import { test } from 'node:test'
import assert from 'node:assert/strict'
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import { spawnSync } from 'node:child_process'
import { wireSchema, trackerContract, verifyContract } from './verify-chat-contract.mjs'

test('wire schema resolves refs and ignores documentation only', () => {
  const document = { components: { schemas: { Version: { type: 'integer', format: 'int64' } } } }
  assert.deepEqual(wireSchema({ $ref: '#/components/schemas/Version' }, document), {
    type: 'integer',
    format: 'int64',
  })
  assert.deepEqual(wireSchema({ type: 'string', description: 'updated', maxLength: 32 }, {}), {
    type: 'string',
    maxLength: 32,
  })
})

test('required, nullability, formats and property names remain part of the contract', () => {
  const required = {
    type: 'object',
    required: ['id'],
    properties: { id: { type: 'string', format: 'uuid' } },
  }
  assert.notDeepEqual(wireSchema(required, {}), wireSchema({ ...required, required: [] }, {}))
  assert.notDeepEqual(
    wireSchema({ type: ['string', 'null'] }, {}),
    wireSchema({ type: 'string' }, {}),
  )
  assert.notDeepEqual(
    wireSchema({ type: 'string', format: 'uuid' }, {}),
    wireSchema({ type: 'string' }, {}),
  )
  assert.throws(() => wireSchema({ $ref: '#/components/schemas/Missing' }, {}), /Missing schema/)
  assert.throws(() => wireSchema({ $ref: '#/components/schemas/__proto__' }, {}), /Missing schema/)
  assert.throws(
    () => wireSchema({ $ref: '#/components/schemas/constructor' }, {}),
    /Missing schema/,
  )
})

test('validation constraints cannot disappear from a compatible contract', () => {
  for (const [key, value] of Object.entries({
    minimum: 1,
    maximum: 9,
    exclusiveMinimum: 0,
    exclusiveMaximum: 10,
    multipleOf: 2,
    minLength: 1,
    maxLength: 32,
    pattern: '^[a-z]+$',
    minItems: 1,
    maxItems: 8,
    uniqueItems: true,
    minProperties: 1,
    maxProperties: 4,
    additionalProperties: false,
    nullable: true,
    readOnly: true,
    writeOnly: true,
    const: 'fixed',
  })) {
    assert.notDeepEqual(
      wireSchema({ type: 'string', [key]: value }, {}),
      wireSchema({ type: 'string' }, {}),
      key,
    )
  }
})

test('nested maps retain field names and additional property schemas', () => {
  const document = { components: { schemas: { Value: { type: 'string', maxLength: 5 } } } }
  const schema = {
    type: 'object',
    properties: { description: { type: 'string', maxLength: 10 } },
    additionalProperties: { $ref: '#/components/schemas/Value' },
  }
  const actual = wireSchema(schema, document)
  assert.deepEqual(actual.properties.description, { maxLength: 10, type: 'string' })
  assert.deepEqual(actual.additionalProperties, { maxLength: 5, type: 'string' })
  assert.notDeepEqual(actual, wireSchema({ ...schema, additionalProperties: true }, document))
  const named = JSON.parse(
    '{"__proto__":{"type":"string"},"properties":{"__proto__":{"type":"integer"}}}',
  )
  assert.deepEqual(wireSchema(named, {}), named)
  assert.equal(Object.getPrototypeOf(wireSchema(named, {})), Object.prototype)
})

test('reference siblings preserve constraints and reject unresolved or recursive references', () => {
  const document = { components: { schemas: { Value: { type: 'string' } } } }
  const ref = { $ref: '#/components/schemas/Value' }
  assert.deepEqual(wireSchema({ ...ref, description: 'docs' }, document), { type: 'string' })
  assert.notDeepEqual(wireSchema({ ...ref, maxLength: 5 }, document), wireSchema(ref, document))
  assert.throws(() => wireSchema({ $ref: 'https://external/schema' }, document), /External/)
  document.components.schemas.Value = ref
  assert.throws(() => wireSchema(ref, document), /Recursive/)
})

test('boolean schemas and ordered array constraints remain distinct', () => {
  assert.equal(wireSchema(false, {}), false)
  assert.equal(wireSchema(true, {}), true)
  assert.equal(
    wireSchema(
      { $ref: '#/components/schemas/Denied' },
      { components: { schemas: { Denied: false } } },
    ),
    false,
  )
  assert.notDeepEqual(wireSchema({ items: false }, {}), wireSchema({ items: true }, {}))
  assert.deepEqual(
    wireSchema({ required: ['b', 'a'], enum: [2, 1], type: ['null', 'string'] }, {}),
    wireSchema({ required: ['a', 'b'], enum: [1, 2], type: ['string', 'null'] }, {}),
  )
  for (const value of [null, [], 'object', 1]) assert.throws(() => wireSchema(value, {}), /Invalid/)
})

function documents() {
  const pairs = {
    TrackerTaskContext: 'SdlcContext',
    TrackerQuestion: 'Question',
    TrackerAnswer: 'Answer',
    TrackerRequirementsRevision: 'RequirementsRevision',
    TrackerConfirmation: 'Confirmation',
    ClarificationAnswerRequest: 'AnswerCommand',
    ConfirmRequirementsRequest: 'ConfirmCommand',
  }
  const schema = {
    type: 'object',
    additionalProperties: false,
    required: ['revision'],
    properties: { revision: { type: 'integer', format: 'int64', minimum: 1, maximum: 9 } },
  }
  return {
    fleet: {
      components: {
        schemas: Object.fromEntries(
          Object.keys(pairs).map((name) => [name, structuredClone(schema)]),
        ),
      },
    },
    tracker: {
      components: {
        schemas: Object.fromEntries(
          Object.values(pairs).map((name) => [name, structuredClone(schema)]),
        ),
      },
    },
  }
}

test('seven DTO verifier rejects a loosened requirements revision even when shape matches', () => {
  const { fleet, tracker } = documents()
  const contract = trackerContract(tracker)
  verifyContract(fleet, contract)
  for (const change of ['maximum', 'additionalProperties']) {
    const altered = structuredClone(fleet)
    if (change === 'maximum')
      delete altered.components.schemas.TrackerRequirementsRevision.properties.revision.maximum
    else delete altered.components.schemas.TrackerRequirementsRevision.additionalProperties
    assert.throws(() => verifyContract(altered, contract), /drift: TrackerRequirementsRevision/)
  }
})

test('record rejects incompatible producer before replacing the accepted snapshot', () => {
  const home = mkdtempSync(join(tmpdir(), 'fleet-chat-contract-'))
  try {
    for (const folder of ['frontend/scripts', 'openapi', 'docs/contracts'])
      mkdirSync(join(home, folder), { recursive: true })
    const script = join(home, 'frontend/scripts/verify-chat-contract.mjs')
    copyFileSync(new URL('./verify-chat-contract.mjs', import.meta.url), script)
    const { fleet, tracker } = documents()
    const snapshot = join(home, 'docs/contracts/tracker-clarification-v1.json')
    writeFileSync(snapshot, JSON.stringify(trackerContract(tracker)))
    const original = readFileSync(snapshot)
    writeFileSync(join(home, 'openapi/openapi.json'), JSON.stringify(fleet))
    tracker.components.schemas.RequirementsRevision.properties.revision.maximum = 8
    const source = join(home, 'tracker.json')
    writeFileSync(source, JSON.stringify(tracker))
    const result = spawnSync(process.execPath, [script, '--record', source], { encoding: 'utf8' })
    assert.notEqual(result.status, 0)
    assert.match(result.stderr, /drift: TrackerRequirementsRevision/)
    assert.deepEqual(readFileSync(snapshot), original)
  } finally {
    rmSync(home, { recursive: true, force: true })
  }
})
