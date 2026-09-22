/// <reference types="vite/client" />
/// <reference types="vite-plugin-pwa/client" />

declare const __GRAPH_HASH__: string

interface ImportMetaEnv {
  readonly VITE_MAP_STYLE_URL?: string
  readonly VITE_ROUTE_GRAPH_URL?: string
  readonly VITE_PMTILES_URL?: string
  readonly VITE_CLARITY_PROJECT_ID?: string
  readonly VITE_SITE_URL?: string
  /** '1' when the app is built for embedding under a subpath: no PWA or analytics. */
  readonly VITE_EMBED?: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}

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
