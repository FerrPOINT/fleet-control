import { apiRequest, jsonBody } from './client'
import type { SessionMessage } from './types'
import type { components } from './generated'

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
export const answerClarification = (id: string, question: string, input: AnswerInput) =>
  apiRequest<Schemas['TrackerAnswer']>(
    `${path(id)}/clarifications/${encodeURIComponent(question)}/answers`,
    jsonBody(input),
  )
export const confirmRequirements = (
  id: string,
  revision: number,
  contentHash: string,
  key: string,
) =>
  apiRequest<Schemas['TrackerConfirmation']>(
    `${path(id)}/requirements/${revision}/confirm`,
    jsonBody({ content_hash: contentHash, idempotency_key: key }),
  )

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
