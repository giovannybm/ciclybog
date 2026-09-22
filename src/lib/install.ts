import { ref } from 'vue'

// Register on import so we do not miss `beforeinstallprompt`, which may fire
// before the app is mounted.
export const installPrompt = ref<BeforeInstallPromptEvent | undefined>()
export const isStandalone = ref(window.matchMedia('(display-mode: standalone)').matches || navigator.standalone === true)
export const needsIosInstructions = /iphone|ipad|ipod/i.test(navigator.userAgent) && !isStandalone.value

window.addEventListener('beforeinstallprompt', event => {
  event.preventDefault()
  installPrompt.value = event
})

window.addEventListener('appinstalled', () => {
  installPrompt.value = undefined
  isStandalone.value = true
})

export async function promptInstall(): Promise<boolean> {
  const event = installPrompt.value
  if (!event) return false
  await event.prompt()
  const { outcome } = await event.userChoice
  installPrompt.value = undefined
  return outcome === 'accepted'
}
