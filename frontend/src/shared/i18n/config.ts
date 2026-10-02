import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'
import en from './locales/en.json'
import { sdlcLocales } from '@sdlc/ui/i18n'
import ru from './locales/ru.json'

i18n.use(initReactI18next).init({
  // Fleet-shared translations (services-base @sdlc/ui/i18n) fill gaps;
  // local keys win on conflicts.
  defaultNS: 'fleet-control',
  fallbackNS: 'base',
  resources: {
    en: { base: sdlcLocales.en, 'fleet-control': en },
    ru: { base: sdlcLocales.ru, 'fleet-control': ru },
  },
  lng: 'ru',
  fallbackLng: 'en',
  interpolation: { escapeValue: false },
  react: { useSuspense: false },
})

export default i18n
