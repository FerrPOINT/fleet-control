import { useEffect, useState } from 'react'
import { Navigate, useLocation } from 'react-router'
import { beginSso } from '@sdlc/ui/sso'
import { Button, PlatformMark } from '@sdlc/ui/ui'
import { ssoConfig, useAuthStore } from '@/shared/auth/store'

export function LoginPage() {
  const location = useLocation()
  const token = useAuthStore((state) => state.token)
  const [error, setError] = useState<string | null>(null)
  const destination = (location.state as { from?: { pathname?: string; search?: string } } | null)?.from
  const returnTo = destination ? `${destination.pathname ?? '/'}${destination.search ?? ''}` : '/'
  const loggedOut = new URLSearchParams(location.search).has('logged_out')
  useEffect(() => {
    if (token || loggedOut) return
    void beginSso(ssoConfig, returnTo).catch(() => setError('Central Auth временно недоступен.'))
  }, [token, loggedOut, returnTo])
  if (token) return <Navigate to={returnTo} replace />
  return <main className="grid min-h-screen place-items-center bg-background p-4">
    <div className="w-full max-w-sm space-y-5 text-center">
      <PlatformMark withName />
      <h1 className="text-xl font-semibold">Вход в Fleet Control</h1>
      {error && <p role="alert" className="text-sm text-destructive">{error}</p>}
      <Button className="w-full" onClick={() => void beginSso(ssoConfig, returnTo).catch(() => setError('Central Auth временно недоступен.'))}>Войти через SDLC</Button>
    </div>
  </main>
}
