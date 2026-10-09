import { StrictMode, useEffect, useState } from 'react'
import { createRoot } from 'react-dom/client'
import { I18nextProvider } from 'react-i18next'
import { Toaster } from 'sonner'
import i18n from './shared/i18n/config'
import { RouterProvider } from 'react-router'
import { router } from './app/router'
import { AuthBoundary } from './app/auth-boundary'
import { ThemeProvider, useTheme, PlatformProvider, PlatformServicesProvider } from '@sdlc/ui/lib'
import './index.css'

function AppToaster() {
  const { theme } = useTheme()
  return <Toaster theme={theme === 'light' ? 'light' : 'dark'} />
}

function Boot() {
  const [ready, setReady] = useState(i18n.isInitialized)
  useEffect(() => {
    if (i18n.isInitialized) {
      setReady(true)
      return
    }
    const handler = () => setReady(true)
    i18n.on('initialized', handler)
    return () => {
      i18n.off('initialized', handler)
    }
  }, [])
  if (!ready) return null
  return (
    <I18nextProvider i18n={i18n}>
      <AuthBoundary>
        <ThemeProvider>
          <PlatformProvider configUrl={import.meta.env.VITE_PLATFORM_BRANDING_URL ?? null}>
            <PlatformServicesProvider
              catalogUrl={import.meta.env.VITE_PLATFORM_SERVICES_URL ?? null}
            >
              <RouterProvider router={router} />
            </PlatformServicesProvider>
          </PlatformProvider>
          <AppToaster />
        </ThemeProvider>
      </AuthBoundary>
    </I18nextProvider>
  )
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <Boot />
  </StrictMode>,
)
