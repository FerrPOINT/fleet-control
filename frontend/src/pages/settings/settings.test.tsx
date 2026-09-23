import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { MemoryRouter } from 'react-router'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import * as fleet from '@/api/fleet'
import * as auth from '@/api/auth'
import type { ManagedSettingsSnapshot, ManagedSettingsVersion } from '@/api/types'
import { SettingsPage } from './index'

vi.mock('@/api/fleet', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/fleet')>()),
  getManagedSettings: vi.fn(),
  previewManagedSettings: vi.fn(),
  applyManagedSettings: vi.fn(),
  listManagedSettingsVersions: vi.fn(),
  rollbackManagedSettings: vi.fn(),
}))
vi.mock('@/api/auth', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/auth')>()),
  listUsers: vi.fn(),
}))

const snapshot: ManagedSettingsSnapshot = {
  runtime: {
    agents_root: '/agents',
    hermes_source: '../hermes',
    hermes_command: 'hermes',
    java_agent_source: '../java-agent',
    java_agent_command: 'java',
  },
  ports: { agent_port_base: 24000, agent_port_stride: 10 },
  integrations: {
    forge_api_url: 'http://ci-cd:22801',
    forge_project: 'fleet-control',
    project_workflow_url: 'http://project-workflow:8000',
  },
  auth: {
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
  },
  retention: { stale_archived_days: 30, review_interval_secs: 3600 },
}

const activeVersion: ManagedSettingsVersion = {
  id: 'version-2',
  version: 2,
  snapshot,
  created_by_user_id: 'user-1',
  rollback_of_version: null,
  created_at: '2026-09-23T10:00:00+00:00',
  is_active: true,
}
const historicalVersion: ManagedSettingsVersion = {
  id: 'version-1',
  version: 1,
  snapshot: { ...snapshot, runtime: { ...snapshot.runtime, hermes_command: 'hermes-old' } },
  created_by_user_id: 'user-1',
  rollback_of_version: null,
  created_at: '2026-09-22T10:00:00+00:00',
  is_active: false,
}
const versions: ManagedSettingsVersion[] = [activeVersion, historicalVersion]

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
  vi.mocked(fleet.getManagedSettings).mockResolvedValue({ active_version: 2, snapshot })
  vi.mocked(fleet.previewManagedSettings).mockImplementation(async (proposed) => ({
    active_version: 2,
    restart_required: true,
    changes: [
      {
        path: 'runtime.hermes_command',
        before: 'hermes',
        after: proposed.runtime.hermes_command,
        requires_restart: true,
      },
    ],
  }))
  vi.mocked(fleet.applyManagedSettings).mockResolvedValue({
    restart_scheduled: true,
    version: { ...activeVersion, version: 3, id: 'version-3' },
  })
  vi.mocked(fleet.listManagedSettingsVersions).mockResolvedValue(versions)
  vi.mocked(fleet.rollbackManagedSettings).mockResolvedValue({
    restart_scheduled: true,
    version: { ...activeVersion, version: 3, id: 'version-3', rollback_of_version: 1 },
  })
  vi.mocked(auth.listUsers).mockResolvedValue([])
})

describe('SettingsPage', () => {
  it('edits a shared draft and previews every change before apply', async () => {
    renderSettings()
    const command = await screen.findByLabelText('Команда Hermes')
    expect(command).toHaveValue('hermes')
    expect(screen.getByText('Активная версия №2')).toBeInTheDocument()

    fireEvent.change(command, { target: { value: 'hermes-next' } })
    fireEvent.click(screen.getByRole('button', { name: 'Проверить изменения' }))

    expect(await screen.findByRole('alertdialog')).toHaveTextContent('runtime.hermes_command')
    expect(fleet.previewManagedSettings).toHaveBeenCalledWith(
      expect.objectContaining({
        runtime: expect.objectContaining({ hermes_command: 'hermes-next' }),
      }),
    )
    fireEvent.click(screen.getByRole('button', { name: 'Применить и перезапустить' }))
    await waitFor(() =>
      expect(fleet.applyManagedSettings).toHaveBeenCalledWith(expect.anything(), 2),
    )
    expect(await screen.findByRole('status')).toHaveTextContent('перезапускается с новой версией')
  })

  it('keeps the draft and confirmation open when apply fails, then retries', async () => {
    vi.mocked(fleet.applyManagedSettings)
      .mockRejectedValueOnce(new Error('apply failed'))
      .mockResolvedValueOnce({ restart_scheduled: true, version: activeVersion })
    renderSettings()
    const command = await screen.findByLabelText('Команда Hermes')
    fireEvent.change(command, { target: { value: 'hermes-next' } })
    fireEvent.click(screen.getByRole('button', { name: 'Проверить изменения' }))
    await screen.findByRole('alertdialog')

    fireEvent.click(screen.getByRole('button', { name: 'Применить и перезапустить' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('apply failed')
    expect(screen.getByRole('alertdialog')).toBeInTheDocument()
    expect(command).toHaveValue('hermes-next')

    fireEvent.click(screen.getByRole('button', { name: 'Применить и перезапустить' }))
    await waitFor(() => expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument())
    expect(fleet.applyManagedSettings).toHaveBeenCalledTimes(2)
  })

  it('keeps the draft when preview validation fails and allows retry', async () => {
    vi.mocked(fleet.previewManagedSettings)
      .mockRejectedValueOnce(new Error('preview failed'))
      .mockImplementationOnce(async (proposed) => ({
        active_version: 2,
        restart_required: true,
        changes: [
          {
            path: 'runtime.hermes_command',
            before: 'hermes',
            after: proposed.runtime.hermes_command,
            requires_restart: true,
          },
        ],
      }))
    renderSettings()
    const command = await screen.findByLabelText('Команда Hermes')
    fireEvent.change(command, { target: { value: 'invalid-command' } })
    fireEvent.click(screen.getByRole('button', { name: 'Проверить изменения' }))

    expect(await screen.findByRole('alert')).toHaveTextContent('preview failed')
    expect(command).toHaveValue('invalid-command')
    fireEvent.click(screen.getByRole('button', { name: 'Проверить изменения' }))
    expect(await screen.findByRole('alertdialog')).toHaveTextContent('runtime.hermes_command')
  })

  it('resets an unsaved draft to the effective snapshot', async () => {
    renderSettings()
    const command = await screen.findByLabelText('Команда Hermes')
    fireEvent.change(command, { target: { value: 'temporary' } })
    fireEvent.click(screen.getByRole('button', { name: 'Сбросить черновик' }))
    expect(command).toHaveValue('hermes')
  })

  it('previews a historical version before rollback and preserves the expected version', async () => {
    renderSettings('/settings?tab=history')
    expect(await screen.findByText('Версия №2')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Вернуть' }))
    expect(await screen.findByRole('alertdialog')).toHaveTextContent('Вернуть настройки версии №1')
    expect(fleet.previewManagedSettings).toHaveBeenCalledWith(historicalVersion.snapshot)

    fireEvent.click(screen.getByRole('button', { name: 'Подтвердить rollback' }))
    await waitFor(() => expect(fleet.rollbackManagedSettings).toHaveBeenCalledWith(1, 2))
    expect(await screen.findByRole('status')).toHaveTextContent('Rollback сохранён')
  })

  it('keeps rollback confirmation open after an error and retries the same version', async () => {
    vi.mocked(fleet.rollbackManagedSettings)
      .mockRejectedValueOnce(new Error('rollback failed'))
      .mockResolvedValueOnce({ restart_scheduled: true, version: activeVersion })
    renderSettings('/settings?tab=history')
    await screen.findByText('Версия №2')
    fireEvent.click(screen.getByRole('button', { name: 'Вернуть' }))
    await screen.findByRole('alertdialog')

    fireEvent.click(screen.getByRole('button', { name: 'Подтвердить rollback' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('rollback failed')
    expect(screen.getByRole('alertdialog')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Подтвердить rollback' }))
    await waitFor(() => expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument())
    expect(fleet.rollbackManagedSettings).toHaveBeenNthCalledWith(1, 1, 2)
    expect(fleet.rollbackManagedSettings).toHaveBeenNthCalledWith(2, 1, 2)
  })

  it('recovers from an initial load failure and keeps a successful snapshot on refresh error', async () => {
    vi.mocked(fleet.getManagedSettings).mockRejectedValueOnce(new Error('network'))
    const client = renderSettings()
    expect(await screen.findByText('Не удалось загрузить настройки.')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Повторить' }))
    expect(await screen.findByLabelText('Команда Hermes')).toHaveValue('hermes')

    vi.mocked(fleet.getManagedSettings).mockRejectedValueOnce(new Error('refresh failed'))
    await act(async () => {
      await client.refetchQueries({ queryKey: ['settings', 'managed'] })
    })
    expect(screen.getByLabelText('Команда Hermes')).toHaveValue('hermes')
    expect(await screen.findByRole('alert')).toHaveTextContent('последний успешный снимок')
  })

  it('shows managed agent ports while explaining deployment-owned service ports', async () => {
    renderSettings('/settings?tab=ports')
    expect(await screen.findByLabelText('Первый порт агента')).toHaveValue(24000)
    expect(screen.getByText(/Порты backend\/frontend/)).toBeInTheDocument()
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
