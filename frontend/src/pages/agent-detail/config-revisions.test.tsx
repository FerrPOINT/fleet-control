import { beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
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

describe('effective configuration readiness', () => {
  beforeEach(() => {
    vi.resetAllMocks()
    vi.mocked(fleet.listAgentConfigRevisions).mockResolvedValue([])
    vi.mocked(fleet.getAgentSdlcReadiness).mockResolvedValue({
      agent_id: 'agent-qa',
      runtime_healthy: true,
      ready_for_sdlc: false,
      effective_revision: 7,
      blockers: ['effective_configuration_readback_failed'],
    })
  })

  it.each([
    ['ru', 'Файлы runtime не соответствуют активной конфигурации'],
    ['en', 'Runtime files do not match the active configuration'],
  ])(
    'shows the verified readback failure in %s without claiming readiness',
    async (locale, text) => {
      await i18n.changeLanguage(locale)
      const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
      render(
        <QueryClientProvider client={client}>
          <ConfigRevisions agentId="agent-qa" />
        </QueryClientProvider>,
      )
      expect(await screen.findByText(text)).toBeVisible()
      expect(screen.queryByText('effective configuration readback failed')).not.toBeInTheDocument()
      expect(fleet.getAgentSdlcReadiness).toHaveBeenCalledWith('agent-qa')
      client.clear()
    },
  )
})
