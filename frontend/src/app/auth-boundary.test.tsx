import { StrictMode, useState } from 'react'
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { useQuery, useQueryClient, type QueryClient } from '@tanstack/react-query'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { AuthBoundary } from './auth-boundary'
import { useAuthStore } from '@/shared/auth/store'

const readTranscript = vi.fn<() => Promise<string>>()
let clients: Set<QueryClient>
function Workspace() {
  const client = useQueryClient()
  clients.add(client)
  const [draft, setDraft] = useState('')
  const token = useAuthStore((state) => state.token)
  const transcript = useQuery({
    queryKey: ['transcript'],
    queryFn: readTranscript,
    enabled: Boolean(token),
    staleTime: Infinity,
  })
  return (
    <>
      <p>{transcript.data ?? 'No transcript'}</p>
      <input
        aria-label="Private draft"
        value={draft}
        onChange={(event) => setDraft(event.target.value)}
      />
    </>
  )
}

describe('authorization boundary', () => {
  beforeEach(() => {
    clients = new Set()
    readTranscript.mockReset().mockResolvedValue('Owner transcript')
    useAuthStore
      .getState()
      .setAuth({ token: 'old-token', userId: 'owner', email: 'owner@example.test' })
  })

  it.each([
    { token: 'new-token', userId: 'other' },
    { token: 'new-token', userId: 'owner' },
    { token: 'old-token', userId: 'owner' },
  ])('separates cache and drafts after reauthentication: %j', async (login) => {
    render(
      <AuthBoundary>
        <Workspace />
      </AuthBoundary>,
    )
    await screen.findByText('Owner transcript')
    const original = [...clients][0]
    if (!original) throw new Error('Expected the original query client')
    fireEvent.change(screen.getByRole('textbox', { name: 'Private draft' }), {
      target: { value: 'Owner private draft' },
    })
    let resolve!: (value: string) => void
    readTranscript.mockImplementationOnce(
      () =>
        new Promise((settle) => {
          resolve = settle
        }),
    )
    act(() => useAuthStore.getState().setAuth({ ...login, email: 'new@example.test' }))
    expect(screen.queryByText('Owner transcript')).not.toBeInTheDocument()
    expect(screen.getByRole('textbox', { name: 'Private draft' })).toHaveValue('')
    expect(original.getQueryData(['transcript'])).toBeUndefined()
    expect(clients.size).toBe(2)
    await act(async () => resolve('Current transcript'))
    await screen.findByText('Current transcript')
  })

  it('clears prior private data and forms on logout without fetching anonymously', async () => {
    render(
      <AuthBoundary>
        <Workspace />
      </AuthBoundary>,
    )
    await screen.findByText('Owner transcript')
    const original = [...clients][0]
    if (!original) throw new Error('Expected the original query client')
    fireEvent.change(screen.getByRole('textbox', { name: 'Private draft' }), {
      target: { value: 'Owner private draft' },
    })
    act(() => useAuthStore.getState().logout())
    expect(screen.getByText('No transcript')).toBeVisible()
    expect(screen.getByRole('textbox', { name: 'Private draft' })).toHaveValue('')
    expect(original.getQueryData(['transcript'])).toBeUndefined()
    expect(readTranscript).toHaveBeenCalledTimes(1)
  })

  it('does not discard an original draft when central sign-out is still pending', async () => {
    render(
      <AuthBoundary>
        <Workspace />
      </AuthBoundary>,
    )
    await screen.findByText('Owner transcript')
    fireEvent.change(screen.getByRole('textbox', { name: 'Private draft' }), {
      target: { value: 'Owner private draft' },
    })
    act(() => useAuthStore.getState().startSignOut())
    expect(screen.getByRole('textbox', { name: 'Private draft' })).toHaveValue(
      'Owner private draft',
    )
    expect(clients.size).toBe(1)
  })

  it('keeps the current cache live during StrictMode effect replay', async () => {
    render(
      <StrictMode>
        <AuthBoundary>
          <Workspace />
        </AuthBoundary>
      </StrictMode>,
    )
    await screen.findByText('Owner transcript')
    await waitFor(() =>
      expect(
        [...clients].some((client) => client.getQueryData(['transcript']) === 'Owner transcript'),
      ).toBe(true),
    )
  })
})
