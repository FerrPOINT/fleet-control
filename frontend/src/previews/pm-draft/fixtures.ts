import type { CreatePmDraft, PmDraftCreation, PmDraftProjects } from '@/api/pm-drafts'

export const projects: PmDraftProjects = {
  enabled: true,
  tracker_instance_id: 'fictional-tracker',
  projects: [
    { id: '11111111-1111-4111-8111-111111111111', key: 'UX', name: 'Портал заявок' },
    { id: '11111111-1111-4111-8111-222222222222', key: 'OPS', name: 'Внутренние сервисы' },
  ],
  next_cursor: null,
}
export const agents = [
  { id: '22222222-2222-4222-8222-111111111111', name: 'agent3 · Project Manager · Hermes' },
  { id: '22222222-2222-4222-8222-222222222222', name: 'agent7 · PM внутренних сервисов · Hermes' },
]
export const initialRequest: CreatePmDraft = {
  agent_id: agents[0]!.id,
  title: 'Портал заявок сотрудников',
  description:
    'Внутренний портал: заявки, исполнители и история изменений. Доступ только участникам проекта. Уточнить критерии приёмки.',
  idempotency_key: 'fictional-original-key',
}
export type Step = PmDraftCreation['next_step']
export const steps: Step[] = ['draft', 'input', 'reservation', 'chat', 'admission']
export type Command =
  | { kind: 'create'; projectId: string; request: CreatePmDraft }
  | { kind: 'lookup'; projectId: string; key: string }
  | { kind: 'readback' | 'continue'; operationId: string }

export function operationAt(step: Step, projectId: string, agentId: string): PmDraftCreation {
  return {
    operation_id: '33333333-3333-4333-8333-333333333333',
    project_id: projectId,
    agent_id: agentId,
    task_id: step === 'draft' ? null : '44444444-4444-4444-8444-444444444444',
    session_id: step === 'admission' ? '55555555-5555-4555-8555-555555555555' : null,
    state: step === 'admission' ? 'awaiting_admission' : 'incomplete',
    next_step: step,
    dispatch_allowed: false,
  }
}
