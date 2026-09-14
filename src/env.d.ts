/// <reference types="vite/client" />
/// <reference types="vite-plugin-pwa/client" />

declare const __GRAPH_HASH__: string

interface BeforeInstallPromptEvent extends Event {
  prompt(): Promise<void>
  userChoice: Promise<{ outcome: 'accepted' | 'dismissed'; platform: string }>
}

interface WindowEventMap {
  beforeinstallprompt: BeforeInstallPromptEvent
}

interface Navigator {
  standalone?: boolean
}
