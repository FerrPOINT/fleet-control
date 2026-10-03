import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { HashRouter } from 'react-router'
import { ThemeProvider } from '@sdlc/ui/lib'
import { Preview } from './preview'
import '../../shared/i18n/config'
import '../../index.css'
import './preview.css'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ThemeProvider>
      <HashRouter>
        <Preview />
      </HashRouter>
    </ThemeProvider>
  </StrictMode>,
)
