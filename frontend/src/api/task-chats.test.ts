import { describe, expect, it, vi } from 'vitest'
import { canSubmitAnswer, listPendingAnswerCommands, type Question } from './task-chats'
import { apiRequest } from './client'

vi.mock('./client', () => ({ apiRequest: vi.fn() }))

const question: Question = {
  id: 'q1',
  request_id: 'request',
  task_id: 'task',
  root_task_id: 'task',
  assignment_id: 'assignment',
  execution_id: 'execution',
  agent_id: 'agent',
  assignment_version: 1,
  checkpoint_id: 'checkpoint',
  author_subject: 'pm',
  created_at: '2026-10-01T12:00:00Z',
  version: 1,
  requirement_revision: 3,
  text: 'Доступ?',
  rationale: 'Модель доступа',
  required: true,
  mode: 'single',
  state: 'open',
  requirement_reference: 'REQ-04',
  recommended_option_id: 'members',
  answer: null,
  options: [
    { id: 'members', label: 'Участники', consequences: 'Доступ проекту', is_custom: false },
    { id: 'custom', label: 'Свой вариант', consequences: 'Опишите', is_custom: true },
  ],
}
describe('clarification answer validation', () => {
  it('does not submit a recommendation without explicit selection', () => {
    expect(canSubmitAnswer(question, [], '')).toBe(false)
    expect(canSubmitAnswer(question, ['members'], '')).toBe(true)
  })
  it('requires custom text and rejects unknown or duplicate options', () => {
    expect(canSubmitAnswer(question, ['custom'], ' ')).toBe(false)
    expect(canSubmitAnswer(question, ['custom'], 'Сотрудники отдела')).toBe(true)
    expect(canSubmitAnswer(question, ['foreign'], 'text')).toBe(false)
    expect(canSubmitAnswer({ ...question, mode: 'multiple' }, ['members', 'members'], '')).toBe(
      false,
    )
  })
  it('supports multiple and text modes but not a closed question', () => {
    expect(
      canSubmitAnswer({ ...question, mode: 'multiple' }, ['members', 'custom'], 'Details'),
    ).toBe(true)
    expect(canSubmitAnswer({ ...question, mode: 'text' }, [], 'Answer')).toBe(true)
    expect(canSubmitAnswer({ ...question, state: 'answered' }, ['members'], '')).toBe(false)
  })
})

describe('answer continuation recovery inventory', () => {
  const command = {
    id: 'original',
    session_id: 'session',
    question_id: 'question',
    request: {
      idempotency_key: 'original-key',
      expected_question_version: 1,
      requirement_revision: 2,
      selected_option_ids: [],
      text: 'Original answer',
      comment: null,
    },
    payload_sha256: 'a'.repeat(64),
    state: 'delivered',
    continuation_state: 'pending',
    answer: {
      id: 'answer',
      question_id: 'question',
      question_version: 1,
      requirement_revision: 2,
      selected_option_ids: [],
      text: 'Original answer',
      comment: null,
    },
    rejection_status: null,
    created_at: '2026-10-10T00:00:00Z',
    updated_at: '2026-10-10T00:00:00Z',
  }
  it('accepts delivered pending without weakening original answer custody', async () => {
    vi.mocked(apiRequest).mockResolvedValueOnce([command])
    expect(await listPendingAnswerCommands('session')).toEqual([command])
    vi.mocked(apiRequest).mockResolvedValueOnce([
      { ...command, answer: { ...command.answer, text: 'Different' } },
    ])
    await expect(listPendingAnswerCommands('session')).rejects.toThrow(
      'Answer delivery is not proven',
    )
  })
  it.each(['confirmed', 'not_required', 'unknown'])(
    'rejects delivered %s from pending inventory',
    async (state) => {
      vi.mocked(apiRequest).mockResolvedValueOnce([{ ...command, continuation_state: state }])
      await expect(listPendingAnswerCommands('session')).rejects.toThrow()
    },
  )
  it('rejects confirmed without delivered answer and duplicate pending receipts', async () => {
    vi.mocked(apiRequest).mockResolvedValueOnce([
      { ...command, state: 'stored', answer: null, continuation_state: 'confirmed' },
    ])
    await expect(listPendingAnswerCommands('session')).rejects.toThrow()
    vi.mocked(apiRequest).mockResolvedValueOnce([command, command])
    await expect(listPendingAnswerCommands('session')).rejects.toThrow(
      'Invalid pending answer command inventory',
    )
  })
})
