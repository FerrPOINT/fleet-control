import assert from 'node:assert/strict'
import test from 'node:test'
import { assertAnswerCommand, sameAnswerRequest } from '../src/api/clarification-custody.ts'

const request = () => ({
  expected_question_version: 2,
  requirement_revision: 3,
  selected_option_ids: ['a', 'b'],
  text: 'Synthetic only',
  comment: null,
  idempotency_key: 'original',
})
const receipt = (state = 'stored') => ({
  id: 'command',
  session_id: 'session',
  question_id: 'question',
  request: request(),
  payload_sha256: 'a'.repeat(64),
  state,
  answer: null,
  rejection_status: null,
  created_at: '2026-10-10T00:00:00Z',
  updated_at: '2026-10-10T00:00:00Z',
})

test('stored/delivering/uncertain receipts never claim answer delivery', () => {
  for (const state of ['stored', 'delivering', 'uncertain'])
    assertAnswerCommand(receipt(state), 'session')
})
test('reload readback retains original private body/key only in server receipt', () => {
  const restored = JSON.parse(JSON.stringify(receipt('uncertain')))
  assertAnswerCommand(restored, 'session')
  assert.equal(restored.request.idempotency_key, 'original')
  assert.equal(restored.request.text, 'Synthetic only')
})
test('another session and unknown state fail closed', () => {
  assert.throws(() => assertAnswerCommand(receipt(), 'foreign'))
  assert.throws(() => assertAnswerCommand(receipt('complete'), 'session'))
})
test('missing, malformed and array receipts fail closed', () => {
  for (const value of [
    null,
    [],
    {},
    { ...receipt(), request: [] },
    { ...receipt(), payload_sha256: 'bad' },
  ])
    assert.throws(() => assertAnswerCommand(value, 'session'))
})
test('original key/body are immutable while selection ordering is a set', () => {
  assert(sameAnswerRequest(request(), { ...request(), selected_option_ids: ['b', 'a'] }))
  for (const mutation of [
    { idempotency_key: 'new' },
    { text: 'changed' },
    { requirement_revision: 4 },
    { expected_question_version: 3 },
    { selected_option_ids: ['a'] },
    { comment: 'changed' },
  ])
    assert(!sameAnswerRequest(request(), { ...request(), ...mutation }))
})
test('unsafe version values cannot become recovery authority', () => {
  for (const value of [0, -1, 1.5, '2', true, 9007199254740992])
    assert.throws(() =>
      assertAnswerCommand(
        { ...receipt(), request: { ...request(), requirement_revision: value } },
        'session',
      ),
    )
})
test('delivered requires the exact original answer target/version/body', () => {
  const delivered = receipt('delivered')
  delivered.answer = {
    id: 'answer',
    question_id: 'question',
    question_version: 2,
    requirement_revision: 3,
    selected_option_ids: ['b', 'a'],
    text: 'Synthetic only',
    comment: null,
  }
  assertAnswerCommand(delivered, 'session')
  for (const mutation of [
    { question_id: 'other' },
    { question_version: 1 },
    { requirement_revision: 4 },
    { selected_option_ids: [] },
    { text: 'other' },
    { comment: 'other' },
    { id: '' },
  ])
    assert.throws(() =>
      assertAnswerCommand(
        { ...delivered, answer: { ...delivered.answer, ...mutation } },
        'session',
      ),
    )
})
test('uncertain cannot carry a delivered answer; rejected requires a definite status', () => {
  assert.throws(() => assertAnswerCommand({ ...receipt('uncertain'), answer: {} }, 'session'))
  assert.throws(() => assertAnswerCommand(receipt('rejected'), 'session'))
  assertAnswerCommand({ ...receipt('rejected'), rejection_status: 409 }, 'session')
})
