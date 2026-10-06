import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { MemoryRouter } from 'react-router'
import { SsoLogoutPendingError } from '@sdlc/ui/sso'
import { LoginPage } from './'
import { useAuthStore } from '@/shared/auth/store'

const beginSso = vi.hoisted(() => vi.fn<(...args: unknown[]) => Promise<void>>(async () => {}))
vi.mock('@sdlc/ui/sso', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@sdlc/ui/sso')>()),
  beginSso,
}))

function renderLogin(path = '/login') {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <LoginPage />
    </MemoryRouter>,
  )
}

describe('fleet-control login recovery', () => {
  it('presents one platform login without product branding or legacy warnings', async () => {
    renderLogin('/login?logged_out=1')
    expect(await screen.findByRole('heading', { name: 'Вход в платформу', exact: true })).toBeInTheDocument()
    expect(screen.queryByText(/Base|SDLC|Task Tracker|Fleet Control|Wiki|CI[/]CD|Admin Panel|второй фактор|защита входа/i)).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Войти через SSO', exact: true })).toBeInTheDocument()
  })

  beforeEach(() => {
    beginSso.mockReset().mockResolvedValue(undefined)
    useAuthStore.getState().logout()
  })
  afterEach(cleanup)

  it('keeps automatic guards non-interactive', async () => {
    renderLogin()
    await waitFor(() =>
      expect(beginSso).toHaveBeenCalledWith(
        expect.objectContaining({ clientId: 'fleet-control' }),
        '/',
      ),
    )
  })

  it('requires explicit login after logout and marks it interactive', async () => {
    renderLogin('/login?logged_out=1')
    expect(beginSso).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: 'Войти через SSO' }))
    await waitFor(() =>
      expect(beginSso).toHaveBeenCalledWith(
        expect.objectContaining({ clientId: 'fleet-control' }),
        '/',
        { interactive: true },
      ),
    )
  })

  it.each([new SsoLogoutPendingError(), new DOMException('Cancelled', 'AbortError')])(
    'does not report interrupted automatic navigation as auth failure: %s',
    async (error) => {
      beginSso.mockRejectedValueOnce(error)
      renderLogin()
      await waitFor(() => expect(beginSso).toHaveBeenCalledOnce())
      fireEvent.click(screen.getByRole('button', { name: 'Войти через SSO' }))
      await waitFor(() => expect(beginSso).toHaveBeenCalledTimes(2))
      expect(screen.queryByRole('alert')).not.toBeInTheDocument()
      expect(beginSso).toHaveBeenLastCalledWith(
        expect.objectContaining({ clientId: 'fleet-control' }),
        '/',
        { interactive: true },
      )
    },
  )

  it('keeps real auth failure visible and clears it after retry', async () => {
    beginSso.mockRejectedValueOnce(new Error('private auth details'))
    renderLogin('/login?logged_out=1')
    fireEvent.click(screen.getByRole('button', { name: 'Войти через SSO' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Central Auth временно недоступен.')
    expect(screen.queryByText(/private auth details/)).not.toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Войти через SSO' }))
    await waitFor(() => expect(screen.queryByRole('alert')).not.toBeInTheDocument())
  })
})
