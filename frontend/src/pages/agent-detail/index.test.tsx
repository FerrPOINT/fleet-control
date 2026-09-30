import { beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter, Route, Routes } from 'react-router'
import i18n from '@/shared/i18n/config'
import * as fleet from '@/api/fleet'
import { listUsers } from '@/api/auth'
import type { Agent, AgentConfig, AgentSkill } from '@/api/types'
import ru from '@/shared/i18n/locales/ru.json'
import en from '@/shared/i18n/locales/en.json'
import { AgentDetailPage } from './index'

vi.mock('@/api/fleet', () => ({
  getAgent: vi.fn(),
  getAgentConfig: vi.fn(),
  getAgentStorage: vi.fn(),
  listAgentSkills: vi.fn(),
  listLogs: vi.fn(),
  listSessions: vi.fn(),
  purgeAgentFiles: vi.fn(),
  runAgentOperation: vi.fn(),
  updateAgentConfig: vi.fn(),
  updateAgentSkill: vi.fn(),
}))
vi.mock('@/api/auth', () => ({ listUsers: vi.fn().mockResolvedValue([]) }))

const agent: Agent = {
  id: 'agent-qa',
  ordinal: 1,
  name: 'qa-agent',
  display_name: 'QA agent',
  kind: 'hermes',
  product_role: 'executor',
  role: 'developer',
  status: 'ready',
  description: null,
  namespace_id: null,
  workflow_id: null,
  runtime_version: null,
  dashboard_port: null,
  api_port: null,
  paths: {
    runtime: '/qa/runtime',
    config: '/qa/config',
    workspace: '/qa/workspace',
    logs: '/qa/logs',
  },
  runtime: {
    desired_state: 'stopped',
    pid: null,
    health_status: 'healthy',
    health_detail: null,
    command_preview: 'qa-command',
    env_preview: {},
    last_capabilities_json: {},
    startup_command_redacted: null,
    started_at: null,
    stopped_at: null,
    last_health_at: null,
  },
  created_at: '2026-10-01T00:00:00Z',
  updated_at: '2026-10-01T00:00:00Z',
}
const config: AgentConfig = {
  agent_id: agent.id,
  config_json: {},
  env_json: {},
  soul_md: 'Original',
  updated_at: agent.updated_at,
}
const skill: AgentSkill = {
  id: 'skill-1',
  agent_id: agent.id,
  name: 'qa-skill',
  title: 'QA skill',
  state: 'enabled',
  source: '/qa/skills',
  content: 'Original',
  updated_at: agent.updated_at,
}

function renderDetail(tab: Parameters<typeof AgentDetailPage>[0]['tab']) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={['/agents/agent-qa']}>
        <Routes>
          <Route path="/agents/:agentId" element={<AgentDetailPage tab={tab} />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  )
}

describe('localized agent detail', () => {
  beforeEach(async () => {
    vi.resetAllMocks()
    vi.mocked(listUsers).mockResolvedValue([])
    await i18n.changeLanguage('ru')
    vi.mocked(fleet.getAgent).mockResolvedValue(agent)
    vi.mocked(fleet.listLogs).mockResolvedValue([])
    vi.mocked(fleet.listAgentSkills).mockResolvedValue([])
    vi.mocked(fleet.listSessions).mockResolvedValue([])
    vi.mocked(fleet.getAgentConfig).mockResolvedValue(config)
  })

  it('uses all six translated tabs and preserves technical identifiers', async () => {
    renderDetail('overview')
    await screen.findByRole('heading', { name: 'QA agent', level: 1 })
    const nav = within(screen.getByRole('navigation', { name: 'Разделы агента' }))
    for (const title of Object.values(ru.agentDetail.tabs))
      expect(nav.getByRole('link', { name: title })).toBeVisible()
    expect(screen.getByText('Описание не задано')).toBeVisible()
    expect(screen.getByText('qa-agent — изолированная среда Hermes.')).toBeVisible()
    expect(screen.getByText('Обновлён')).toBeVisible()
    expect(screen.queryByText('never')).not.toBeInTheDocument()
  })

  it('has matching translation keys and switches the rendered detail to English', async () => {
    expect(Object.keys(en.agentDetail).sort()).toEqual(Object.keys(ru.agentDetail).sort())
    expect(Object.keys(en.agentDetail.tabs)).toEqual(Object.keys(ru.agentDetail.tabs))
    await i18n.changeLanguage('en')
    renderDetail('overview')
    await screen.findByRole('link', { name: 'Edit agent' })
    expect(screen.getByRole('navigation', { name: 'Agent sections' })).toBeVisible()
    expect(screen.getByText('No description')).toBeVisible()
  })

  it('retries failed agent loading instead of showing an empty page', async () => {
    vi.mocked(fleet.getAgent).mockRejectedValueOnce(new Error('offline')).mockResolvedValue(agent)
    renderDetail('overview')
    await screen.findByRole('alert')
    expect(screen.getByText('Не удалось загрузить агента.')).toBeVisible()
    fireEvent.click(screen.getByRole('button', { name: 'Повторить' }))
    await screen.findByRole('heading', { name: 'QA agent', level: 1 })
  })

  it.each([
    ['runtime', 'listLogs', 'Не удалось загрузить журнал.', 'Записей для этого агента пока нет'],
    ['skills', 'listAgentSkills', 'Не удалось загрузить навыки.', 'Навыки не выбраны'],
    ['sessions', 'listSessions', 'Не удалось загрузить сессии.', 'Сессий этого агента пока нет'],
  ] as const)(
    'distinguishes %s request failure from empty data and retries',
    async (tab, method, error, empty) => {
      vi.mocked(fleet[method]).mockRejectedValueOnce(new Error('offline')).mockResolvedValue([])
      renderDetail(tab)
      await screen.findByText(error)
      expect(screen.queryByText(empty)).not.toBeInTheDocument()
      fireEvent.click(screen.getByRole('button', { name: 'Повторить' }))
      await screen.findByText(empty)
      expect(fleet[method]).toHaveBeenCalledTimes(2)
    },
  )

  it('retries config loading rather than allowing an empty save', async () => {
    vi.mocked(fleet.getAgentConfig)
      .mockRejectedValueOnce(new Error('offline'))
      .mockResolvedValue(config)
    renderDetail('config')
    await screen.findByText('Не удалось загрузить конфигурацию.')
    expect(screen.queryByRole('button', { name: 'Сохранить конфигурацию' })).not.toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Повторить' }))
    await screen.findByRole('button', { name: 'Сохранить конфигурацию' })
  })

  it('keeps a failed config draft and blocks invalid JSON objects from submission', async () => {
    vi.mocked(fleet.updateAgentConfig).mockRejectedValue(new Error('offline'))
    renderDetail('config')
    const soul = await screen.findByRole('textbox', { name: 'SOUL.md' })
    const json = screen.getByRole('textbox', { name: 'config.json' })
    fireEvent.change(soul, { target: { value: 'Keep this draft' } })
    for (const invalid of ['{', 'null', '[]', '42']) {
      fireEvent.change(json, { target: { value: invalid } })
      expect(json).toHaveAttribute('aria-invalid', 'true')
      expect(screen.getByRole('button', { name: 'Сохранить конфигурацию' })).toBeDisabled()
    }
    expect(fleet.updateAgentConfig).not.toHaveBeenCalled()
    fireEvent.change(json, { target: { value: '{"qa":true}' } })
    fireEvent.click(screen.getByRole('button', { name: 'Сохранить конфигурацию' }))
    await screen.findByText('Не удалось сохранить конфигурацию. Изменения оставлены в форме.')
    expect(soul).toHaveValue('Keep this draft')
    expect(json).toHaveValue('{\n  "qa": true\n}')
    expect(fleet.updateAgentConfig).toHaveBeenCalledWith(agent.id, {
      config_json: { qa: true },
      env_json: {},
      soul_md: 'Keep this draft',
    })
  })

  it('locks the config while saving and shows success only after the response', async () => {
    let finish!: (value: AgentConfig) => void
    vi.mocked(fleet.updateAgentConfig).mockReturnValue(
      new Promise((resolve) => {
        finish = resolve
      }),
    )
    renderDetail('config')
    fireEvent.click(await screen.findByRole('button', { name: 'Сохранить конфигурацию' }))
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Сохранение...' })).toBeDisabled(),
    )
    for (const field of screen.getAllByRole('textbox')) expect(field).toBeDisabled()
    expect(screen.queryByText('Конфигурация сохранена')).not.toBeInTheDocument()
    finish(config)
    await screen.findByRole('status')
    expect(screen.getByRole('status')).toHaveTextContent('Конфигурация сохранена')
    fireEvent.change(screen.getByRole('textbox', { name: 'SOUL.md' }), {
      target: { value: 'New unsaved draft' },
    })
    expect(screen.queryByText('Конфигурация сохранена')).not.toBeInTheDocument()
  })

  it('locks skill controls during save and keeps the draft after failure', async () => {
    vi.mocked(fleet.listAgentSkills).mockResolvedValue([skill])
    let fail!: (error: Error) => void
    vi.mocked(fleet.updateAgentSkill).mockReturnValue(
      new Promise((_resolve, reject) => {
        fail = reject
      }),
    )
    renderDetail('skills')
    const editor = await screen.findByRole('textbox', { name: 'Изменить навык QA skill' })
    fireEvent.change(editor, { target: { value: 'Skill draft' } })
    fireEvent.click(screen.getByRole('button', { name: 'Сохранить навык' }))
    await waitFor(() => expect(editor).toBeDisabled())
    expect(screen.getByRole('button', { name: 'Выключить' })).toBeDisabled()
    fail(new Error('offline'))
    await screen.findByText('Не удалось сохранить навык. Изменения оставлены в форме.')
    expect(editor).toHaveValue('Skill draft')
    expect(editor).toBeEnabled()
  })

  it('localizes runtime health and operation feedback', async () => {
    vi.mocked(fleet.runAgentOperation).mockRejectedValueOnce(new Error('offline'))
    renderDetail('runtime')
    fireEvent.click(await screen.findByRole('button', { name: 'Проверить состояние' }))
    await screen.findByText('Не удалось выполнить операцию. Повторите попытку.')
    expect(screen.getByText('Доступен')).toBeVisible()
    expect(screen.getByText('Ещё не было')).toBeVisible()
    expect(fleet.runAgentOperation).toHaveBeenCalledWith(agent.id, 'health')
  })

  it('keeps file purge unavailable when storage verification fails', async () => {
    vi.mocked(fleet.getAgent).mockResolvedValue({ ...agent, status: 'archived' })
    vi.mocked(fleet.getAgentStorage).mockRejectedValue(new Error('offline'))
    renderDetail('workspace')
    await screen.findByText('Не удалось загрузить сведения о диске.')
    fireEvent.change(screen.getByLabelText('Введите имя агента для подтверждения'), {
      target: { value: agent.name },
    })
    expect(screen.getByRole('button', { name: 'Удалить файлы' })).toBeDisabled()
    expect(fleet.purgeAgentFiles).not.toHaveBeenCalled()
  })
})
