import { describe, expect, it } from 'vitest'
import { canonicalAnswerPayload } from './answer-payload'

const low = '00000000-0000-4000-8000-000000000001'
const high = '00000000-0000-4000-8000-000000000002'
const input = {
  expected_question_version: 7,
  requirement_revision: 11,
  selected_option_ids: [high, low],
  text: 'Original custom answer',
  comment: 'Original comment',
  idempotency_key: 'original-key',
}

describe('canonical clarification answer payload', () => {
  it('matches sorted server readback without mutating click order or other identity fields', () => {
    const result = canonicalAnswerPayload(input)
    expect(result).toEqual({ ...input, selected_option_ids: [low, high] })
    expect(input.selected_option_ids).toEqual([high, low])
    expect(result.selected_option_ids).not.toBe(input.selected_option_ids)
    expect(JSON.stringify(result)).toBe(
      JSON.stringify(canonicalAnswerPayload({ ...input, selected_option_ids: [low, high] })),
    )
    expect(canonicalAnswerPayload(result)).toEqual(result)
  })

  it('keeps nulls, whitespace and duplicates for the existing server validation', () => {
    expect(
      canonicalAnswerPayload({
        ...input,
        selected_option_ids: [high, low, high],
        text: null,
        comment: '  Original comment  ',
      }),
    ).toEqual({
      ...input,
      selected_option_ids: [low, high, high],
      text: null,
      comment: '  Original comment  ',
    })
  })
})
