import { apiRequest } from './client'
import { ApiError } from '@sdlc/ui/lib'
import type { SessionMessage } from './types'
import type { components } from './generated'
import {
  assertAnswerCommand,
  sameAnswerRequest,
  type ClarificationCommand,
} from './clarification-custody'

type Schemas = components['schemas']
export type TaskBinding = Schemas['TaskChatBinding']
export type TaskContext = Schemas['TrackerTaskContext']
export type TaskContextResponse = Schemas['SessionTaskContext']
export type AnswerInput = Schemas['ClarificationAnswerRequest']
export type Question = Schemas['TrackerQuestion']
export type RequirementsRevision = Schemas['TrackerRequirementsRevision']
export type RequirementsDocument = Omit<
  RequirementsRevision,
  'revision' | 'content_hash' | 'created_at' | 'author_subject'
>
export type HistoryPage = { items: SessionMessage[]; next_before: string | null }
export type ChatControls = Schemas['ChatControls']

const path = (id: string) => `/api/v1/sessions/${encodeURIComponent(id)}`
export const getTaskContext = (id: string) =>
  apiRequest<TaskContextResponse>(`${path(id)}/task-context`)
export const getChatControls = (id: string) => apiRequest<ChatControls>(`${path(id)}/chat-controls`)
export const getChatHistory = (id: string, before?: string) =>
  apiRequest<HistoryPage>(
    `${path(id)}/history?limit=50${before ? `&before=${encodeURIComponent(before)}` : ''}`,
  )
export const getClarifications = (id: string) =>
  apiRequest<Schemas['TrackerClarifications']>(`${path(id)}/clarifications`)
export const getRequirements = (id: string) =>
  apiRequest<Schemas['TrackerRequirements']>(`${path(id)}/requirements`)
export async function storeAnswerCommand(id: string, question: string, input: AnswerInput) {
  const value = await apiRequest<unknown>(
    `${path(id)}/clarifications/${encodeURIComponent(question)}/answer-commands`,
    { method: 'POST', body: JSON.stringify(input) },
  )
  assertAnswerCommand(value, id)
  if (value.question_id !== question || !sameAnswerRequest(value.request, input))
    throw new Error('Original answer command changed')
  return value
}
export async function listPendingAnswerCommands(id: string) {
  const values = await apiRequest<unknown>(`${path(id)}/clarification-answer-commands`)
  if (!Array.isArray(values) || values.length > 100)
    throw new Error('Invalid answer command inventory')
  const result: ClarificationCommand[] = []
  for (const value of values) {
    assertAnswerCommand(value, id)
    if (
      !['stored', 'delivering', 'uncertain'].includes(value.state) ||
      result.some((item) => item.id === value.id)
    )
      throw new Error('Invalid pending answer command inventory')
    result.push(value)
  }
  return result
}
export async function deliverAnswerCommand(id: string, command: string) {
  const value = await apiRequest<unknown>(
    `${path(id)}/clarification-answer-commands/${encodeURIComponent(command)}/delivery`,
    { method: 'POST' },
  )
  assertAnswerCommand(value, id)
  if (value.id !== command) throw new Error('Answer command identity changed')
  return value
}
export async function answerClarification(id: string, question: string, input: AnswerInput) {
  const stored = await storeAnswerCommand(id, question, input)
  const result = await deliverAnswerCommand(id, stored.id)
  if (
    result.payload_sha256 !== stored.payload_sha256 ||
    !sameAnswerRequest(result.request, stored.request)
  )
    throw new Error('Original answer command changed during delivery')
  if (result.state === 'delivered' && result.answer) return result.answer
  if (result.state === 'rejected')
    throw new ApiError(result.rejection_status ?? 409, 'Исходный ответ отклонён Tracker')
  // Even a definite new rejection has a retained receipt. No implicit new key.
  throw new Error('Original answer delivery is not confirmed; read the server journal')
}
export const confirmRequirements = (
  id: string,
  revision: number,
  contentHash: string,
  key: string,
) =>
  apiRequest<Schemas['TrackerConfirmation']>(`${path(id)}/requirements/${revision}/confirm`, {
    method: 'POST',
    body: JSON.stringify({ content_hash: contentHash, idempotency_key: key }),
  })

export function canSubmitAnswer(question: Question, selected: string[], text: string) {
  if (question.state !== 'open') return false
  if (question.mode === 'text') return Boolean(text.trim())
  if (question.mode === 'single' && selected.length !== 1) return false
  if (question.mode === 'multiple' && !selected.length) return false
  if (
    new Set(selected).size !== selected.length ||
    selected.some((id) => !question.options.some((option) => option.id === id))
  )
    return false
  return (
    !question.options.some((option) => selected.includes(option.id) && option.is_custom) ||
    Boolean(text.trim())
  )
}

export const blockedLabels: Record<string, string> = {
  read_only: 'Только чтение',
  workflow_assignment_required: 'Ожидается проверенное назначение workflow',
  java_chat_phase_2: 'Чат Java Agent пока недоступен',
  dispatch_pending_or_uncertain: 'Доставка ожидается или требует сверки',
  runtime_stopped_messages_queue: 'Агент остановлен: сообщение будет сохранено в очереди',
}
