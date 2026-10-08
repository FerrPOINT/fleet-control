import { beforeEach, describe, expect, it } from 'vitest'
import { ApiError } from '@sdlc/ui/lib'
import type { AgentSession, SessionAgentRun, SessionMessage } from '@/api/types'
import {
  chatActivity,
  chatBackTo,
  clearDispatch,
  commandService,
  dispatchHeld,
  legacyControlHeld,
  markDispatch,
  unknownOutcome,
} from './core'

describe('core contract guards', () => {
  beforeEach(() => sessionStorage.clear())
  it('accepts only Chats return paths and keeps filters', () => {
    expect(chatBackTo('/chats?agent=a&q=work&users=u')).toBe('/chats?agent=a&q=work&users=u')
    for (const value of [
      '//evil.example/chats',
      'https://evil.example/chats',
      '/sessions',
      'javascript:alert(1)',
    ])
      expect(chatBackTo(value)).toBe('/chats')
  })
  it('keeps server order and excludes other agents and sessions', () => {
    const session = { id: 's', primary_agent_id: 'a' } as AgentSession
    const messages = [
      { id: 'z', session_id: 's', author_type: 'user', created_at: '2099-01-01' },
      {
        id: 'a',
        session_id: 's',
        author_type: 'agent',
        author_agent_id: 'a',
        created_at: '2000-01-01',
      },
      { id: 'foreign', session_id: 's', author_type: 'agent', author_agent_id: 'b' },
      { id: 'foreign-session', session_id: 't', author_type: 'user' },
    ] as SessionMessage[]
    const runs = [
      { id: 'other', session_id: 's', agent_id: 'b', state: 'running' },
    ] as SessionAgentRun[]
    expect(chatActivity(session, messages, runs)).toMatchObject({
      messages: [{ id: 'z' }, { id: 'a' }],
      runs: [],
      busy: false,
    })
  })
  it.each(['pending', 'running', 'waiting', 'stopping'])(
    'blocks a primary run in %s state',
    (state) => {
      expect(
        chatActivity(
          { id: 's', primary_agent_id: 'a' } as AgentSession,
          [],
          [{ session_id: 's', agent_id: 'a', state } as SessionAgentRun],
        ).busy,
      ).toBe(true)
    },
  )
  it.each(['pending', 'dispatched'])(
    'holds %s delivery independently of runs',
    (delivery_state) => {
      expect(
        chatActivity(
          { id: 's', primary_agent_id: 'a' } as AgentSession,
          [{ session_id: 's', delivery_state } as SessionMessage],
          [],
        ).busy,
      ).toBe(true)
    },
  )
  it('treats transport, malformed success and server failure as unknown', () => {
    for (const failure of [
      new Error('offline'),
      new ApiError(200, 'malformed'),
      new ApiError(408, 'timeout'),
      new ApiError(503, 'unavailable'),
    ])
      expect(unknownOutcome(failure)).toBe(true)
    expect(unknownOutcome(new ApiError(403, 'denied'))).toBe(false)
  })
  it('persists an allowlist and does not release another scope or key', () => {
    const marker = {
      actor: 'u',
      agent: 'a',
      service: 'fleet',
      key: 'k',
      digest: '0'.repeat(64),
      body: 'private text',
      token: 'secret',
    }
    markDispatch('s', marker)
    const raw = sessionStorage.getItem('fleet-control.chat-dispatch.v1:s')!
    expect(raw).not.toContain('private text')
    expect(raw).not.toContain('secret')
    expect(dispatchHeld('s')).toBe(true)
    expect(clearDispatch('s', { ...marker, actor: 'other' })).toBe(false)
    expect(() => markDispatch('s', marker)).toThrow()
    expect(
      clearDispatch('s', {
        actor: 'u',
        agent: 'a',
        service: 'fleet',
        key: 'k',
        digest: '0'.repeat(64),
      }),
    ).toBe(true)
  })
  it('keeps unreadable legacy metadata held', () => {
    sessionStorage.setItem('fleet-control.control-recovery.v1:s', 'malformed')
    expect(legacyControlHeld('s')).toBe(true)
    expect(legacyControlHeld('other')).toBe(false)
  })
  it('strips userinfo, query and fragment from the service identity', () => {
    const service = commandService(
      'https://user:secret@example.test/api?token=secret#private',
      'https://auth.test',
    )
    expect(service).toContain('https://example.test/api')
    expect(service).not.toContain('secret')
    expect(service).not.toContain('private')
  })
})
