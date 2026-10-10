import type { AnswerInput } from '@/api/task-chats'

export function canonicalAnswerPayload(input: AnswerInput): AnswerInput {
  // Match the stored server request without changing the draft's click order.
  return {
    expected_question_version: input.expected_question_version,
    requirement_revision: input.requirement_revision,
    selected_option_ids: [...input.selected_option_ids].sort(),
    text: input.text,
    comment: input.comment,
    idempotency_key: input.idempotency_key,
  }
}
