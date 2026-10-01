import { beforeEach, describe, expect, it, vi } from 'vitest'
import { getChatsDirectory } from './chats-directory'
import { apiRequest } from './client'
vi.mock('./client', () => ({ apiRequest: vi.fn() }))
beforeEach(() => vi.clearAllMocks())
describe('chats directory wire query', () => {
  it('omits user_id for server mine default', async () => {
    await getChatsDirectory()
    expect(apiRequest).toHaveBeenCalledWith('/api/v1/chats/directory?limit=50')
  })
  it('encodes scoped literal search and cursor', async () => {
    await getChatsDirectory({
      agentId: 'agent',
      userIds: ['one', 'two'],
      search: '  50%_ & login  ',
      before: 'cursor',
      limit: 100,
    })
    const query = new URL(vi.mocked(apiRequest).mock.calls[0]![0], 'http://local').searchParams
    expect(Object.fromEntries(query)).toEqual({
      agent_id: 'agent',
      user_id: 'one,two',
      q: '50%_ & login',
      before: 'cursor',
      limit: '100',
    })
  })
  it('represents explicit all separately from omission', async () => {
    await getChatsDirectory({ userIds: [] })
    expect(apiRequest).toHaveBeenCalledWith('/api/v1/chats/directory?user_id=all&limit=50')
  })
})
