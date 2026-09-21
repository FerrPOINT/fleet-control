import { describe, expect, it } from 'vitest'
import { render, screen } from '@testing-library/react'
import type { Agent } from '@/api/types'
import { AgentIdentity, StatusBadge } from './common'

describe('fleet status labels', () => {
  it('localizes agent roles, state and profile metadata', () => {
    const agent = {
      name: 'agent1',
      display_name: 'QA Agent',
      kind: 'hermes',
      product_role: 'executor',
      role: 'developer',
      status: 'ready',
      namespace_id: 'qa',
      runtime: { health_status: 'not started' },
    } as Agent

    render(<AgentIdentity agent={agent} />)

    expect(screen.getByText('Исполнитель')).toBeInTheDocument()
    expect(screen.getByText('Готов')).toBeInTheDocument()
    expect(screen.getByText('Не запущен')).toBeInTheDocument()
    expect(screen.getByText(/профиль: разработчик/)).toBeInTheDocument()
    expect(screen.getByText(/пространство: qa/)).toBeInTheDocument()
  })

  it('keeps unknown external states readable', () => {
    render(<StatusBadge value="custom_state" />)
    expect(screen.getByText('custom state')).toBeInTheDocument()
  })

  it('keeps success badges readable on the light theme surface', () => {
    render(<StatusBadge value="running" />)
    expect(screen.getByText('Работает')).toHaveClass('text-text-primary')
  })
})
