import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { HashRouter } from 'react-router'
import { ThemeProvider } from '@sdlc/ui/lib'
import { PmDraftPreview } from './preview'
import '../../shared/i18n/config'
import '../../index.css'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ThemeProvider>
      <HashRouter>
        <PmDraftPreview />
      </HashRouter>
    </ThemeProvider>
  </StrictMode>,
)
