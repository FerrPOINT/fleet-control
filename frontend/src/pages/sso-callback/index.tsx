import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router'
import { completeSso } from '@sdlc/ui/sso'
import { Button, DelayedFallback } from '@sdlc/ui/ui'
import { apiBaseUrl } from '@/api/client'
import type { UserPermissionsResponse } from '@/api/types'
import { isCurrentAuth, ssoConfig, useAuthStore, type AuthScope } from '@/shared/auth/store'
import { AuthContextChangedError } from '@/api/client'

let pending: ReturnType<typeof completeSso> | null = null
let pendingScope: AuthScope | null = null
function completion(scope: AuthScope) {
  if (pending && (!pendingScope || !isCurrentAuth(pendingScope)))
    return Promise.reject(new AuthContextChangedError())
  if (!pending) {
    pendingScope = scope
    pending = completeSso(ssoConfig)
    void pending
      .finally(() => {
        pending = null
        pendingScope = null
      })
      .catch(() => undefined)
  }
  return pending
}

export function SsoCallbackPage() {
  const navigate = useNavigate()
  const [error, setError] = useState<string | null>(null)
  useEffect(() => {
    let active = true
    const scope = useAuthStore.getState()
    if (scope.token) {
      navigate('/', { replace: true })
      return
    }
    if (!isCurrentAuth(scope)) return
    void completion(scope)
      .then(async (session) => {
        if (!active || !isCurrentAuth(scope)) return
        const response = await fetch(`${apiBaseUrl}/api/v1/users/me`, {
          headers: { Authorization: `Bearer ${session.accessToken}` },
        })
        if (!response.ok) throw new Error('Не удалось открыть профиль Fleet Control.')
        const user = (await response.json()) as {
          id: string
          email: string
          username: string
          display_name: string
        }
        if (!active || !isCurrentAuth(scope)) return
        const permissionsResponse = await fetch(`${apiBaseUrl}/api/v1/users/me/permissions`, {
          headers: { Authorization: `Bearer ${session.accessToken}` },
        })
        if (!permissionsResponse.ok) throw new Error('Не удалось проверить права Fleet Control.')
        const permissions = (await permissionsResponse.json()) as UserPermissionsResponse
        if (permissions.user_id !== user.id)
          throw new Error('Профиль и права пользователя не совпадают.')
        if (!active || !isCurrentAuth(scope)) return
        useAuthStore.getState().setAuth({
          token: session.accessToken,
          userId: user.id,
          email: user.email,
          username: user.username,
          displayName: user.display_name,
          systemRole: permissions.role,
          isSystemAdmin: permissions.is_system_admin,
          permissions: permissions.permissions,
        })
        navigate(session.returnTo, { replace: true })
      })
      .catch((caught) => {
        if (active && isCurrentAuth(scope))
          setError(caught instanceof Error ? caught.message : 'Не удалось завершить вход')
      })
    return () => {
      active = false
    }
  }, [navigate])
  return (
    <main className="grid min-h-screen place-items-center bg-background p-4">
      {error ? (
        <div className="space-y-4 text-center">
          <p role="alert">{error}</p>
          <Button onClick={() => navigate('/login', { replace: true })}>Повторить вход</Button>
        </div>
      ) : (
        <DelayedFallback>
          <p role="status">Завершаем вход...</p>
        </DelayedFallback>
      )}
    </main>
  )
}
