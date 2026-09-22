# Delta: offline PWA

## MODIFIED

### Requirement: installability
The application MUST publish a manifest with `id`, `name`, `short_name`, `start_url`, `scope`, `display: standalone`, `lang`, colors, 192 and 512 px PNG icons (including one `maskable` icon), an Apple Touch icon, and a service worker with a `fetch` handler. When the browser allows it, the interface MUST offer an **Install app** button; on iOS it MUST explain how to add the app to the Home Screen.

#### Scenario: Chrome/Android installation
- GIVEN the app is served over HTTPS or localhost and the service worker is active
- WHEN the browser emits `beforeinstallprompt`
- THEN **Install app** appears and opens the native dialog; after installation, the button disappears.

## ADDED

### Requirement: package manager
The project MUST use pnpm as its only package manager: declare `packageManager`, keep `pnpm-lock.yaml` as the only lockfile, and use `pnpm` in scripts and documentation.
