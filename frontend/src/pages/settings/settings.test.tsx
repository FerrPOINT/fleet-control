import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { MemoryRouter } from 'react-router'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import * as fleet from '@/api/fleet'
import * as auth from '@/api/auth'
import { SettingsPage } from './index'

vi.mock('@/api/fleet', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/fleet')>()),
  getRuntimeSettings: vi.fn(),
  getPortSettings: vi.fn(),
  getIntegrationSettings: vi.fn(),
  getAuthSettings: vi.fn(),
}))
vi.mock('@/api/auth', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/auth')>()),
  listUsers: vi.fn(),
}))

const runtime = {
  agents_root: '/agents',
  hermes_source: '../hermes',
  hermes_command: 'hermes',
  java_agent_source: '../java-agent',
  java_agent_command: 'java',
}
const ports = {
  backend_port: 23801,
  frontend_port: 23802,
  agent_port_base: 24000,
  agent_port_stride: 10,
}
const integrations = {
  project_workflow_url: 'http://project-workflow:8000',
  project_workflow_status: 'configured',
  github_remote: null,
}
const authSettings = {
  mode: 'hmac',
  jwt_issuer: 'fleet-control',
  jwt_audience: 'sdlc',
  access_token_ttl_minutes: 15,
  refresh_token_ttl_days: 7,
  refresh_cookie_name: 'refresh_token',
  refresh_cookie_secure: false,
  refresh_cookie_same_site: 'Lax',
  refresh_cookie_domain: null,
  refresh_cookie_path: '/api/v1/auth',
}

function renderSettings(initialEntry = '/settings') {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })
  render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[initialEntry]}>
        <SettingsPage />
      </MemoryRouter>
    </QueryClientProvider>,
  )
  return client
}

beforeEach(() => {
  vi.clearAllMocks()
  vi.mocked(fleet.getRuntimeSettings).mockResolvedValue(runtime)
  vi.mocked(fleet.getPortSettings).mockResolvedValue(ports)
  vi.mocked(fleet.getIntegrationSettings).mockResolvedValue(integrations)
  vi.mocked(fleet.getAuthSettings).mockResolvedValue(authSettings)
  vi.mocked(auth.listUsers).mockResolvedValue([])
})

describe('SettingsPage', () => {
  it('shows effective runtime values without fake editing controls', async () => {
    renderSettings()

    const panel = await screen.findByRole('region', { name: 'Источники и команды' })
    expect(within(panel).getByText('/agents')).toBeInTheDocument()
    expect(within(panel).getByText('hermes')).toBeInTheDocument()
    expect(within(panel).getByText('Только чтение')).toBeInTheDocument()
    expect(within(panel).getByText(/deployment Fleet Control/)).toBeInTheDocument()
    expect(screen.queryByRole('form')).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Сохранить изменения' })).not.toBeInTheDocument()
    expect(screen.getByRole('note')).toHaveTextContent('Показаны активные значения запуска')
  })

  it('loads only the selected settings tab and keeps its state in the URL', async () => {
    renderSettings('/settings?tab=ports')

    const panel = await screen.findByRole('region', { name: 'Сетевые порты' })
    expect(within(panel).getByText('24000')).toBeInTheDocument()
    expect(fleet.getRuntimeSettings).not.toHaveBeenCalled()
    expect(fleet.getPortSettings).toHaveBeenCalledTimes(1)

    fireEvent.keyDown(screen.getByRole('tab', { name: 'Интеграции' }), { key: 'Enter' })
    expect(await screen.findByRole('region', { name: 'Интеграции' })).toHaveTextContent(
      'http://project-workflow:8000',
    )
    expect(fleet.getIntegrationSettings).toHaveBeenCalledTimes(1)
  })

  it('recovers from an initial settings load failure', async () => {
    vi.mocked(fleet.getRuntimeSettings).mockRejectedValueOnce(new Error('network'))
    renderSettings()

    expect(await screen.findByText('Не удалось загрузить настройки.')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Повторить' }))
    expect(await screen.findByRole('region', { name: 'Источники и команды' })).toHaveTextContent(
      '/agents',
    )
  })

  it('keeps the last effective snapshot when a refresh fails and can retry', async () => {
    const client = renderSettings()
    await screen.findByText('/agents')
    vi.mocked(fleet.getRuntimeSettings).mockRejectedValueOnce(new Error('network'))

    await act(async () => {
      await client.refetchQueries({ queryKey: ['settings', 'runtime'] })
    })

    expect(screen.getByText('/agents')).toBeInTheDocument()
    expect(await screen.findByRole('alert')).toHaveTextContent('последний успешный снимок')
    fireEvent.click(screen.getByRole('button', { name: 'Повторить' }))
    await waitFor(() => expect(screen.queryByRole('alert')).not.toBeInTheDocument())
  })

  it('explains ownership for integrations and platform authentication', async () => {
    renderSettings('/settings?tab=integrations')
    const integrationsPanel = await screen.findByRole('region', { name: 'Интеграции' })
    expect(within(integrationsPanel).getByText('Настроено')).toBeInTheDocument()
    expect(within(integrationsPanel).queryByText('Репозиторий GitHub')).not.toBeInTheDocument()

    fireEvent.keyDown(screen.getByRole('tab', { name: 'Доступ' }), { key: 'Enter' })
    const authPanel = await screen.findByRole('region', { name: 'Политика аутентификации' })
    expect(within(authPanel).getByText(/Central Auth/)).toBeInTheDocument()
    expect(within(authPanel).getByText('HMAC')).toBeInTheDocument()
    expect(within(authPanel).getByText('Нет')).toBeInTheDocument()
  })

  it('shows a searchable users list with an Admin Panel handoff', async () => {
    vi.mocked(auth.listUsers).mockResolvedValue(
      Array.from({ length: 13 }, (_, index) => ({
        id: `user-${index + 1}`,
        display_name: `User ${String(index + 1).padStart(2, '0')}`,
        email: `user-${index + 1}@example.test`,
      })) as never,
    )
    renderSettings('/settings?tab=users')

    expect(await screen.findByRole('link', { name: 'Открыть в Admin Panel' })).toHaveAttribute(
      'href',
      'http://localhost:7772/users',
    )
    expect(screen.getAllByText(/User \d+/)).toHaveLength(12)
    expect(screen.queryByRole('note')).not.toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Далее' }))
    expect(screen.getAllByText(/User \d+/)).toHaveLength(1)
    fireEvent.change(screen.getByRole('searchbox', { name: 'Найти пользователя' }), {
      target: { value: 'user-1@example.test' },
    })
    expect(screen.getByText('User 01')).toBeInTheDocument()
  })
})
