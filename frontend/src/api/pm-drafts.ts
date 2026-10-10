import { ApiError } from '@sdlc/ui/lib'
import { apiRequest } from './client'
import type { components } from './generated'

export type CreatePmDraft = components['schemas']['CreatePmDraftRequest']
export type PmDraftCreation = components['schemas']['PmDraftCreationResponse']
export type PmDraftProjects = components['schemas']['PmDraftProjectDirectory']

const uuid = (value: unknown): value is string =>
  typeof value === 'string' &&
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/.test(value) &&
  value !== '00000000-0000-0000-0000-000000000000'

function readCreation(
  value: unknown,
  expected: { projectId?: string; agentId?: string; operationId?: string },
): PmDraftCreation {
  const fields = [
    'operation_id',
    'project_id',
    'agent_id',
    'task_id',
    'session_id',
    'state',
    'next_step',
    'dispatch_allowed',
  ]
  if (!value || typeof value !== 'object' || Array.isArray(value))
    throw new ApiError(502, 'Invalid PM Draft operation response')
  const result = value as PmDraftCreation
  if (
    Object.keys(result).length !== fields.length ||
    fields.some((field) => !Object.hasOwn(result, field)) ||
    !uuid(result.operation_id) ||
    !uuid(result.project_id) ||
    !uuid(result.agent_id) ||
    (result.task_id !== null && !uuid(result.task_id)) ||
    (result.session_id !== null && !uuid(result.session_id)) ||
    result.dispatch_allowed !== false ||
    (expected.projectId !== undefined && result.project_id !== expected.projectId) ||
    (expected.agentId !== undefined && result.agent_id !== expected.agentId) ||
    (expected.operationId !== undefined && result.operation_id !== expected.operationId) ||
    (result.state === 'awaiting_admission'
      ? result.next_step !== 'admission' || result.task_id === null || result.session_id === null
      : result.state !== 'incomplete' ||
        !['draft', 'input', 'reservation', 'chat'].includes(result.next_step) ||
        result.session_id !== null ||
        (result.next_step === 'draft' ? result.task_id !== null : result.task_id === null))
  )
    throw new ApiError(502, 'Invalid PM Draft operation response')
  return result
}

export async function createPmDraft(projectId: string, request: CreatePmDraft) {
  return readCreation(
    await apiRequest<unknown>(`/api/v1/projects/${encodeURIComponent(projectId)}/pm-drafts`, {
      method: 'POST',
      body: JSON.stringify(request),
    }),
    { projectId, agentId: request.agent_id },
  )
}

export async function getPmDraftCreation(operationId: string) {
  return readCreation(
    await apiRequest<unknown>(`/api/v1/pm-drafts/operations/${encodeURIComponent(operationId)}`),
    { operationId },
  )
}

export async function findPmDraftCreation(projectId: string, key: string) {
  try {
    return readCreation(
      await apiRequest<unknown>(
        `/api/v1/projects/${encodeURIComponent(projectId)}/pm-drafts/operation?${new URLSearchParams({ idempotency_key: key })}`,
      ),
      { projectId },
    )
  } catch (error) {
    if (error instanceof ApiError && error.status === 404) return null
    throw error
  }
}

export async function continuePmDraftCreation(operationId: string) {
  return readCreation(
    await apiRequest<unknown>(
      `/api/v1/pm-drafts/operations/${encodeURIComponent(operationId)}/continue`,
      { method: 'POST', body: '{}' },
    ),
    { operationId },
  )
}

export async function getPmDraftProjects(after?: string): Promise<PmDraftProjects> {
  const value = await apiRequest<unknown>(
    `/api/v1/pm-drafts/projects${after ? `?${new URLSearchParams({ after })}` : ''}`,
  )
  const invalid = () => new ApiError(502, 'Invalid PM Draft project directory')
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw invalid()
  const page = value as PmDraftProjects
  if (
    Object.keys(page).length !== 4 ||
    !['enabled', 'tracker_instance_id', 'projects', 'next_cursor'].every((key) =>
      Object.hasOwn(page, key),
    ) ||
    typeof page.enabled !== 'boolean' ||
    typeof page.tracker_instance_id !== 'string' ||
    !Array.isArray(page.projects) ||
    page.projects.length > 50 ||
    (page.next_cursor !== null &&
      (!uuid(page.next_cursor) || (after !== undefined && page.next_cursor <= after))) ||
    (!page.enabled && (page.projects.length !== 0 || page.next_cursor !== null))
  )
    throw invalid()
  let previous = after
  for (const project of page.projects) {
    if (
      !project ||
      typeof project !== 'object' ||
      Array.isArray(project) ||
      Object.keys(project).length !== 3 ||
      !['id', 'key', 'name'].every((key) => Object.hasOwn(project, key)) ||
      !uuid(project.id) ||
      typeof project.key !== 'string' ||
      typeof project.name !== 'string' ||
      (previous !== undefined && project.id <= previous) ||
      (page.next_cursor !== null && project.id > page.next_cursor)
    )
      throw invalid()
    previous = project.id
  }
  return page
}
