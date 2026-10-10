import type { AnswerInput } from './task-chats'
import type { components } from './generated'

export type ClarificationCommand = components['schemas']['ClarificationAnswerCommand'] & {
  continuation_state?: 'not_required' | 'pending' | 'confirmed'
}

const object = (value: unknown): value is Record<string, unknown> =>
  value !== null && typeof value === 'object' && !Array.isArray(value)
const safeVersion = (value: unknown) => Number.isSafeInteger(value) && Number(value) > 0

export function sameAnswerRequest(left: AnswerInput, right: AnswerInput) {
  return (
    left.idempotency_key === right.idempotency_key &&
    left.expected_question_version === right.expected_question_version &&
    left.requirement_revision === right.requirement_revision &&
    (left.text ?? null) === (right.text ?? null) &&
    (left.comment ?? null) === (right.comment ?? null) &&
    JSON.stringify([...left.selected_option_ids].sort()) ===
      JSON.stringify([...right.selected_option_ids].sort())
  )
}

export function assertAnswerCommand(
  value: unknown,
  session: string,
): asserts value is ClarificationCommand {
  if (!object(value) || !object(value.request)) throw new Error('Invalid answer command receipt')
  const request = value.request
  if (
    value.session_id !== session ||
    typeof value.id !== 'string' ||
    !value.id ||
    typeof value.question_id !== 'string' ||
    !value.question_id ||
    typeof value.payload_sha256 !== 'string' ||
    !/^[a-f0-9]{64}$/.test(value.payload_sha256) ||
    typeof value.created_at !== 'string' ||
    typeof value.updated_at !== 'string' ||
    !['stored', 'delivering', 'uncertain', 'delivered', 'rejected'].includes(String(value.state)) ||
    (value.continuation_state !== undefined &&
      !['not_required', 'pending', 'confirmed'].includes(String(value.continuation_state))) ||
    (value.continuation_state === 'confirmed' && value.state !== 'delivered') ||
    typeof request.idempotency_key !== 'string' ||
    !request.idempotency_key ||
    !safeVersion(request.expected_question_version) ||
    !safeVersion(request.requirement_revision) ||
    !Array.isArray(request.selected_option_ids) ||
    request.selected_option_ids.some((id) => typeof id !== 'string') ||
    (request.text != null && typeof request.text !== 'string') ||
    (request.comment != null && typeof request.comment !== 'string')
  )
    throw new Error('Invalid answer command scope or payload')
  if (value.state === 'delivered') {
    const answer = value.answer
    if (
      !object(answer) ||
      typeof answer.id !== 'string' ||
      !answer.id ||
      answer.question_id !== value.question_id ||
      answer.question_version !== request.expected_question_version ||
      answer.requirement_revision !== request.requirement_revision ||
      (answer.text ?? null) !== (request.text ?? null) ||
      (answer.comment ?? null) !== (request.comment ?? null) ||
      !Array.isArray(answer.selected_option_ids) ||
      JSON.stringify([...answer.selected_option_ids].sort()) !==
        JSON.stringify([...request.selected_option_ids].sort()) ||
      value.rejection_status !== null
    )
      throw new Error('Answer delivery is not proven by this receipt')
  } else if (value.answer !== null) throw new Error('Unconfirmed answer cannot be delivered')
  if (value.state === 'rejected') {
    if (
      !Number.isInteger(value.rejection_status) ||
      Number(value.rejection_status) < 400 ||
      Number(value.rejection_status) > 499
    )
      throw new Error('Invalid answer rejection')
  } else if (value.rejection_status !== null) throw new Error('Unexpected answer rejection')
}
