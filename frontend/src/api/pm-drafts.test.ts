import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ApiError } from '@sdlc/ui/lib'
import { apiRequest } from './client'
import {
  continuePmDraftCreation,
  createPmDraft,
  findPmDraftCreation,
  getPmDraftCreation,
  getPmDraftProjects,
} from './pm-drafts'

vi.mock('./client', () => ({ apiRequest: vi.fn() }))
const project = '11111111-1111-4111-8111-111111111111'
const agent = '22222222-2222-4222-8222-222222222222'
const operation = '33333333-3333-4333-8333-333333333333'
const pending = {
  operation_id: operation,
  project_id: project,
  agent_id: agent,
  task_id: null,
  session_id: null,
  state: 'incomplete',
  next_step: 'draft',
  dispatch_allowed: false,
}

describe('PM Draft recovery contract', () => {
  beforeEach(() => {
    vi.mocked(apiRequest).mockReset()
  })

  it('preserves a source cursor even for an empty rollout-filtered project page', async () => {
    const page = {
      enabled: true,
      tracker_instance_id: 'tracker',
      projects: [],
      next_cursor: project,
    }
    vi.mocked(apiRequest).mockResolvedValue(page)
    expect(await getPmDraftProjects()).toEqual(page)
    const final = {
      ...page,
      projects: [{ id: agent, key: 'PM', name: 'Owner project' }],
      next_cursor: null,
    }
    vi.mocked(apiRequest).mockResolvedValue(final)
    expect(await getPmDraftProjects(project)).toEqual(final)
    expect(apiRequest).toHaveBeenLastCalledWith(`/api/v1/pm-drafts/projects?after=${project}`)
    for (const wrong of [
      { ...page, next_cursor: undefined },
      { ...page, enabled: false },
      { ...final, projects: [{ id: agent, key: 'PM', name: 'Project', private: 'hidden' }] },
      { ...page, next_cursor: project.toUpperCase() },
      { ...final, projects: [{ id: project, key: 'PM', name: 'Stale' }] },
    ]) {
      vi.mocked(apiRequest).mockResolvedValue(wrong)
      await expect(getPmDraftProjects(project)).rejects.toMatchObject({ status: 502 })
    }
  })

  it('keeps the supplied command key and resolves an incomplete saved operation', async () => {
    const request = {
      agent_id: agent,
      title: 'Требования',
      description: 'Исходный запрос',
      idempotency_key: 'exact-key',
    }
    vi.mocked(apiRequest).mockResolvedValue(pending)
    expect(await createPmDraft(project, request)).toEqual(pending)
    expect(apiRequest).toHaveBeenCalledWith(`/api/v1/projects/${project}/pm-drafts`, {
      method: 'POST',
      body: JSON.stringify(request),
    })
    expect(await findPmDraftCreation(project, 'utf8-ключ&?/')).toEqual(pending)
    const path = vi.mocked(apiRequest).mock.calls[1]![0]
    expect(new URL(path, 'https://fleet.test').searchParams.get('idempotency_key')).toBe(
      'utf8-ключ&?/',
    )
  })

  it('continues the same operation without resubmitting or changing its original input', async () => {
    const awaiting = {
      ...pending,
      task_id: agent,
      session_id: project,
      state: 'awaiting_admission',
      next_step: 'admission',
    }
    vi.mocked(apiRequest).mockResolvedValue(awaiting)
    expect(await getPmDraftCreation(operation)).toEqual(awaiting)
    expect(await continuePmDraftCreation(operation)).toEqual(awaiting)
    expect(apiRequest).toHaveBeenLastCalledWith(
      `/api/v1/pm-drafts/operations/${operation}/continue`,
      { method: 'POST', body: '{}' },
    )
  })

  it('treats only authoritative 404 as absent, not a dependency or permission failure', async () => {
    vi.mocked(apiRequest).mockRejectedValue(new ApiError(404, 'Not found'))
    expect(await findPmDraftCreation(project, 'key')).toBeNull()
    for (const status of [401, 403, 409, 503]) {
      const error = new ApiError(status, 'Rejected')
      vi.mocked(apiRequest).mockRejectedValue(error)
      await expect(findPmDraftCreation(project, 'key')).rejects.toMatchObject({ status })
    }
  })

  it.each(['awaiting_runtime_acceptance', 'runtime_accepted'] as const)(
    'accepts the exact %s runtime receipt for creation and recovery',
    async (state) => {
      const response = {
        ...pending,
        task_id: agent,
        session_id: project,
        state,
        next_step: 'runtime',
        dispatch_allowed: state === 'runtime_accepted',
      }
      vi.mocked(apiRequest).mockResolvedValue(response)
      expect(
        await createPmDraft(project, {
          agent_id: agent,
          title: 'Task',
          description: '',
          idempotency_key: 'original-key',
        }),
      ).toEqual(response)
      expect(await getPmDraftCreation(operation)).toEqual(response)
      expect(await findPmDraftCreation(project, 'original-key')).toEqual(response)
      expect(await continuePmDraftCreation(operation)).toEqual(response)
      expect(apiRequest).toHaveBeenLastCalledWith(
        `/api/v1/pm-drafts/operations/${operation}/continue`,
        { method: 'POST', body: '{}' },
      )
    },
  )

  it('rejects inconsistent runtime acceptance without weakening identity or closed fields', async () => {
    const response = {
      ...pending,
      task_id: agent,
      session_id: project,
      state: 'runtime_accepted',
      next_step: 'runtime',
      dispatch_allowed: true,
    }
    for (const changed of [
      { ...response, dispatch_allowed: false },
      { ...response, dispatch_allowed: 'true' },
      { ...response, state: 'awaiting_runtime_acceptance' },
      { ...response, state: 'awaiting_runtime_acceptance', dispatch_allowed: null },
      { ...response, state: 'awaiting_admission', next_step: 'admission' },
      { ...response, next_step: 'admission' },
      { ...response, task_id: null },
      { ...response, session_id: null },
      { ...response, operation_id: agent },
      { ...response, state: 'completed' },
      { ...response, machine_secret: 'must-not-be-rendered' },
    ]) {
      vi.mocked(apiRequest).mockResolvedValue(changed)
      await expect(getPmDraftCreation(operation)).rejects.toMatchObject({ status: 502 })
    }
  })

  it('rejects invented dispatch, incomplete identities and invalid partial-success states', async () => {
    for (const changed of [
      { ...pending, dispatch_allowed: true },
      { ...pending, project_id: agent },
      { ...pending, agent_id: project },
      { ...pending, state: 'running' },
      { ...pending, state: 'awaiting_admission', next_step: 'admission' },
      { ...pending, next_step: 'reservation' },
      { ...pending, session_id: agent },
      { ...pending, operation_id: '00000000-0000-0000-0000-000000000000' },
      { ...pending, machine_secret: 'must-not-be-rendered' },
    ]) {
      vi.mocked(apiRequest).mockResolvedValue(changed)
      await expect(
        createPmDraft(project, {
          agent_id: agent,
          title: 'Task',
          description: '',
          idempotency_key: 'key',
        }),
      ).rejects.toMatchObject({ status: 502 })
    }
    const omitted = { ...pending } as Partial<typeof pending>
    delete omitted.task_id
    vi.mocked(apiRequest).mockResolvedValue(omitted)
    await expect(getPmDraftCreation(operation)).rejects.toMatchObject({ status: 502 })
    vi.mocked(apiRequest).mockResolvedValue({ ...pending, operation_id: agent })
    await expect(continuePmDraftCreation(operation)).rejects.toMatchObject({ status: 502 })
  })
})
