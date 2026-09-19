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
  updateRuntimeSettings: vi.fn(),
  getPortSettings: vi.fn(),
  updatePortSettings: vi.fn(),
  getIntegrationSettings: vi.fn(),
  updateIntegrationSettings: vi.fn(),
  getAuthSettings: vi.fn(),
  updateAuthSettings: vi.fn(),
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
  project_workflow_url: null,
  project_workflow_status: 'enabled',
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
  vi.mocked(fleet.updateRuntimeSettings).mockImplementation(async (value) => value)
  vi.mocked(fleet.getPortSettings).mockResolvedValue(ports)
  vi.mocked(fleet.updatePortSettings).mockImplementation(async (value) => value)
  vi.mocked(fleet.getIntegrationSettings).mockResolvedValue(integrations)
  vi.mocked(fleet.updateIntegrationSettings).mockImplementation(async (value) => value)
  vi.mocked(fleet.getAuthSettings).mockResolvedValue(authSettings)
  vi.mocked(fleet.updateAuthSettings).mockImplementation(async (value) => value)
  vi.mocked(auth.listUsers).mockResolvedValue([])
})

describe('SettingsPage', () => {
  it('keeps an unsaved draft through tab switches and a background refetch', async () => {
    const client = renderSettings()
    const root = await screen.findByRole('textbox', { name: 'Каталог агентов' })
    const form = screen.getByRole('form', { name: 'Источники и команды' })
    expect(within(form).getByRole('button', { name: 'Сохранить изменения' })).toBeDisabled()

    fireEvent.change(root, { target: { value: '/local/agents' } })
    expect(within(form).getByRole('button', { name: 'Сохранить изменения' })).toBeEnabled()
    fireEvent.keyDown(screen.getByRole('tab', { name: 'Порты' }), { key: 'Enter' })
    await screen.findByRole('spinbutton', { name: 'Порт API' })
    fireEvent.keyDown(screen.getByRole('tab', { name: 'Среда' }), { key: 'Enter' })
    expect(screen.getByRole('textbox', { name: 'Каталог агентов' })).toHaveValue('/local/agents')

    vi.mocked(fleet.getRuntimeSettings).mockResolvedValue({
      ...runtime,
      agents_root: '/server/agents',
    })
    await act(async () => {
      await client.refetchQueries({ queryKey: ['settings', 'runtime'] })
    })
    await waitFor(() =>
      expect(within(form).getByRole('button', { name: 'Сбросить' })).toBeEnabled(),
    )
    expect(root).toHaveValue('/local/agents')
    fireEvent.click(within(form).getByRole('button', { name: 'Сбросить' }))
    await waitFor(() => expect(root).toHaveValue('/server/agents'))
  })

  it('saves only changed settings and acknowledges success', async () => {
    renderSettings()
    const command = await screen.findByRole('textbox', { name: 'Команда Hermes' })
    fireEvent.change(command, { target: { value: 'hermes --quiet' } })
    fireEvent.click(screen.getByRole('button', { name: 'Сохранить изменения' }))

    await waitFor(() =>
      expect(fleet.updateRuntimeSettings).toHaveBeenCalledWith({
        ...runtime,
        hermes_command: 'hermes --quiet',
      }),
    )
    await waitFor(() => expect(screen.getByText('Изменения сохранены')).toBeInTheDocument())
    expect(screen.getByRole('button', { name: 'Сохранить изменения' })).toBeDisabled()
  })

  it('keeps the draft and shows a recoverable save error', async () => {
    vi.mocked(fleet.updateRuntimeSettings).mockRejectedValueOnce(new Error('network'))
    renderSettings()
    const command = await screen.findByRole('textbox', { name: 'Команда Hermes' })
    fireEvent.change(command, { target: { value: 'hermes --quiet' } })
    fireEvent.click(screen.getByRole('button', { name: 'Сохранить изменения' }))

    expect(await screen.findByRole('alert')).toHaveTextContent('Не удалось сохранить настройки')
    expect(command).toHaveValue('hermes --quiet')
    expect(screen.getByRole('button', { name: 'Сохранить изменения' })).toBeEnabled()
    fireEvent.click(screen.getByRole('button', { name: 'Сбросить' }))
    expect(command).toHaveValue('hermes')
    expect(screen.queryByRole('alert')).not.toBeInTheDocument()
  })

  it('disables inputs while a save is pending', async () => {
    let completeSave!: (value: typeof runtime) => void
    vi.mocked(fleet.updateRuntimeSettings).mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          completeSave = resolve
        }),
    )
    renderSettings()
    const command = await screen.findByRole('textbox', { name: 'Команда Hermes' })
    fireEvent.change(command, { target: { value: 'hermes --quiet' } })
    fireEvent.click(screen.getByRole('button', { name: 'Сохранить изменения' }))
    await waitFor(() => expect(command).toBeDisabled())
    expect(screen.getByRole('button', { name: 'Сохраняем...' })).toBeDisabled()
    await act(async () => completeSave({ ...runtime, hermes_command: 'hermes --quiet' }))
    await waitFor(() => expect(command).toBeEnabled())
  })

  it('preserves a draft when a background refresh fails and retries it', async () => {
    const client = renderSettings()
    const root = await screen.findByRole('textbox', { name: 'Каталог агентов' })
    fireEvent.change(root, { target: { value: '/local/agents' } })
    vi.mocked(fleet.getRuntimeSettings).mockRejectedValueOnce(new Error('network'))
    await act(async () => {
      await client.refetchQueries({ queryKey: ['settings', 'runtime'] })
    })
    expect(root).toHaveValue('/local/agents')
    expect(await screen.findByRole('alert')).toHaveTextContent('Не удалось загрузить настройки')
    fireEvent.click(screen.getByRole('button', { name: 'Повторить' }))
    await waitFor(() => expect(screen.queryByRole('alert')).not.toBeInTheDocument())
    expect(root).toHaveValue('/local/agents')
  })

  it('recovers from an initial settings load failure', async () => {
    vi.mocked(fleet.getRuntimeSettings).mockRejectedValueOnce(new Error('network'))
    renderSettings()
    expect(await screen.findByText('Не удалось загрузить настройки.')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Повторить' }))
    expect(await screen.findByRole('textbox', { name: 'Каталог агентов' })).toHaveValue('/agents')
  })

  it('keeps an empty or too-small port invalid instead of silently sending zero', async () => {
    renderSettings('/settings?tab=ports')
    const stride = (await screen.findByRole('spinbutton', {
      name: 'Шаг портов агентов',
    })) as HTMLInputElement
    fireEvent.change(stride, { target: { value: '' } })
    expect(stride).toHaveValue(null)
    expect(stride.checkValidity()).toBe(false)
    fireEvent.change(stride, { target: { value: '3' } })
    expect(stride.checkValidity()).toBe(false)
    expect(fleet.updatePortSettings).not.toHaveBeenCalled()
  })

  it('keeps auth choices constrained to supported values', async () => {
    renderSettings('/settings?tab=auth')
    const sameSite = await screen.findByRole('combobox', { name: 'Политика SameSite' })
    const mode = screen.getByRole('combobox', { name: 'Режим аутентификации' })
    expect(within(mode).getAllByRole('option')).toHaveLength(1)
    expect(within(sameSite).getAllByRole('option')).toHaveLength(3)
    fireEvent.change(sameSite, { target: { value: 'Strict' } })
    expect(screen.getByRole('button', { name: 'Сохранить изменения' })).toBeEnabled()
    fireEvent.click(screen.getByRole('button', { name: 'Сбросить' }))
    expect(sameSite).toHaveValue('Lax')
    expect(fleet.updateAuthSettings).not.toHaveBeenCalled()
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
    fireEvent.click(screen.getByRole('button', { name: 'Далее' }))
    expect(screen.getAllByText(/User \d+/)).toHaveLength(1)
    fireEvent.change(screen.getByRole('searchbox', { name: 'Найти пользователя' }), {
      target: { value: 'user-1@example.test' },
    })
    expect(screen.getByText('User 01')).toBeInTheDocument()
  })
})
