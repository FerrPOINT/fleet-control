import { test } from 'node:test'
import assert from 'node:assert/strict'
import { wireSchema } from './verify-chat-contract.mjs'

test('wire schema resolves refs and ignores documentation or constraint annotations', () => {
  const document = { components: { schemas: { Version: { type: 'integer', format: 'int64' } } } }
  assert.deepEqual(wireSchema({ $ref: '#/components/schemas/Version' }, document), {
    type: 'integer',
    format: 'int64',
  })
  assert.deepEqual(wireSchema({ type: 'string', description: 'updated', maxLength: 32 }, {}), {
    type: 'string',
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
})
