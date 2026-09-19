import { render, screen } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router'
import { describe, expect, it } from 'vitest'
import { RequireAuth } from './require-auth'
import { useAuthStore } from './store'

describe('Fleet route guard', () => {
  it('lets the OIDC callback finish before an access token exists', () => {
    useAuthStore.getState().logout()
    render(
      <MemoryRouter initialEntries={['/sso/callback?code=example&state=example']}>
        <Routes>
          <Route element={<RequireAuth />}>
            <Route path="/sso/callback" element={<p>Callback ready</p>} />
            <Route path="/login" element={<p>Login redirect</p>} />
          </Route>
        </Routes>
      </MemoryRouter>,
    )
    expect(screen.getByText('Callback ready')).toBeTruthy()
    expect(screen.queryByText('Login redirect')).toBeNull()
  })
})
