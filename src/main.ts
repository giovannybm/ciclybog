import { createApp } from 'vue'
import Clarity from '@microsoft/clarity'
import { registerSW } from 'virtual:pwa-register'
import './lib/install'
import App from './App.vue'
import './style.css'

// Embedded builds do not register a service worker: it would cache 24+ MB on the
// host origin and keep serving an old version after rebuilds.
if (!import.meta.env.VITE_EMBED) registerSW({ immediate: true })

const clarityProjectId = import.meta.env.VITE_CLARITY_PROJECT_ID
const clarityConsentKey = 'ciclybog-analytics-consent'
let clarityStarted = false

function startClarityIfConsented() {
  if (clarityStarted || !clarityProjectId || localStorage.getItem(clarityConsentKey) !== 'accepted') return
  Clarity.init(clarityProjectId)
  clarityStarted = true
}

startClarityIfConsented()
window.addEventListener('ciclybog:clarity-consent', startClarityIfConsented)

createApp(App).mount('#app')
