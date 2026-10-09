import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import i18n from '@/shared/i18n/config'
import * as fleet from '@/api/fleet'
import { ConfigRevisions } from './config-revisions'

vi.mock('@/api/fleet', () => ({
  activateAgentConfigRevision: vi.fn(),
  getAgentSdlcReadiness: vi.fn(),
  listAgentConfigRevisions: vi.fn(),
  validateAgentConfigRevision: vi.fn(),
}))

describe('configuration readiness refresh', () => {
  let client: QueryClient

  beforeEach(async () => {
    vi.resetAllMocks()
    await i18n.changeLanguage('en')
    client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    vi.mocked(fleet.listAgentConfigRevisions).mockResolvedValue([])
  })

  afterEach(() => {
    client.clear()
  })

  it.each([
    { runtime: 'running', sdlc: 'ready', runtimeHealthy: true, sdlcReady: true },
    { runtime: 'running', sdlc: 'blocked', runtimeHealthy: true, sdlcReady: false },
    { runtime: 'stopped', sdlc: 'blocked', runtimeHealthy: false, sdlcReady: false },
  ])(
    'holds cached $runtime/$sdlc as unknown after failure and recovers with keyboard retry',
    async ({ runtime, sdlc, runtimeHealthy, sdlcReady }) => {
      const agentId = `agent-${runtime}-${sdlc}`
    const latest = {
        agent_id: agentId,
        runtime_healthy: runtimeHealthy,
        ready_for_sdlc: sdlcReady,
        effective_revision: 7,
        blockers: [],
      }
      vi.mocked(fleet.getAgentSdlcReadiness)
      .mockResolvedValueOnce(latest)
        .mockRejectedValueOnce(new Error('Readiness unavailable'))
      .mockResolvedValue(latest)
      const user = userEvent.setup()
      render(
        <QueryClientProvider client={client}>
          <ConfigRevisions agentId={agentId} />
        </QueryClientProvider>,
      )
      expect(await screen.findByText(i18n.t(`statuses.${sdlc}`))).toBeVisible()
      expect(screen.getByText(i18n.t(`statuses.${runtime}`))).toBeVisible()
      const effective = `${i18n.t('configRevisions.effective')}: 7`
      expect(screen.getByText(effective)).toBeVisible()
      const retry = screen.getByRole('button', { name: i18n.t('sessions.retry') })
      await user.click(retry)
      expect(await screen.findByText(i18n.t('configRevisions.loadError'))).toBeVisible()
      expect(screen.queryByText(i18n.t(`statuses.${sdlc}`))).not.toBeInTheDocument()
      expect(screen.queryByText(i18n.t(`statuses.${runtime}`))).not.toBeInTheDocument()
      const unknown = screen.getAllByText(i18n.t('statuses.unknown'))
      expect(unknown).toHaveLength(2)
      unknown.forEach((badge) => expect(badge).toBeVisible())
      expect(screen.getByText(effective)).toBeVisible()
      expect(retry).toBeEnabled()
      await user.tab({ shift: true })
      expect(retry).not.toHaveFocus()
      await user.tab()
      expect(retry).toHaveFocus()
      await user.keyboard('{Enter}')
      expect(await screen.findByText(i18n.t(`statuses.${sdlc}`))).toBeVisible()
      expect(screen.getByText(i18n.t(`statuses.${runtime}`))).toBeVisible()
      expect(screen.queryByText(i18n.t('statuses.unknown'))).not.toBeInTheDocument()
      expect(screen.queryByText(i18n.t('configRevisions.loadError'))).not.toBeInTheDocument()
      expect(screen.getByText(effective)).toBeVisible()
      expect(fleet.getAgentSdlcReadiness).toHaveBeenCalledTimes(3)
      expect(fleet.listAgentConfigRevisions).toHaveBeenCalledTimes(3)
    },
  )
})
