import { describe, expect, it } from 'vitest'
import { canSubmitAnswer, type Question } from './task-chats'

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
