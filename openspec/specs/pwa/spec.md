# Capability: offline PWA

## Requirements

### Requirement: installability
The application MUST publish a manifest with `id`, `name`, `short_name`, `start_url`, `scope`, `display: standalone`, `lang`, colors, 192 and 512 px PNG icons (including one `maskable` icon), an Apple Touch icon, and a service worker with a `fetch` handler. When the browser allows it, the interface MUST offer an **Install app** button; on iOS it MUST explain how to add the app to the Home Screen.

#### Scenario: Chrome/Android installation
- GIVEN the app is served over HTTPS or localhost and the service worker is active
- WHEN the browser emits `beforeinstallprompt`
- THEN **Install app** appears and opens the native dialog; after installation, the button disappears.

### Requirement: user data
The application MUST retain saved routes locally after the user closes and reopens it.

### Requirement: offline transparency
The application MUST communicate that routing is local and that geographic availability depends on the installed graph.

### Requirement: local artifact precache
The PWA strategy MUST include the shell, icons, WASM module, binary graph, local GeoJSON layers, and PMTiles when published.

#### Scenario: first offline load
- GIVEN the shell, graph, and PMTiles were published and cached during an initial load
- WHEN the user opens the PWA offline
- THEN the application can load the local map and routing engine without requesting a backend service.

#### Scenario: artifact update
- GIVEN a new version of the shell or local artifacts exists
- WHEN the service worker detects the update
- THEN the PWA updates through `autoUpdate` without losing routes stored in IndexedDB.

### Requirement: package manager
The project MUST use pnpm as its only package manager: declare `packageManager`, keep `pnpm-lock.yaml` as the only lockfile, and use `pnpm` in scripts and documentation.
