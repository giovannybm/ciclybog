# Capacidad: PWA offline

## Requisitos

### Requisito: instalabilidad
La aplicación DEBE publicar un manifest con `id`, `name`, `short_name`, `start_url`, `scope`, `display: standalone`, `lang`, colores e íconos PNG de 192 y 512 px (incluido uno `maskable`), un `apple-touch-icon` y registrar un service worker con manejador `fetch`. Cuando el navegador lo permita, la interfaz DEBE ofrecer un botón “Instalar app”; en iOS DEBE indicar cómo agregarla a la pantalla de inicio.

#### Escenario: instalación en Chrome/Android
- DADO que la app se sirve por HTTPS o localhost y el service worker está activo
- CUANDO el navegador emite `beforeinstallprompt`
- ENTONCES aparece “Instalar app” y al pulsarlo se muestra el diálogo nativo; tras instalar, el botón desaparece.

### Requisito: datos del usuario
La aplicación DEBE conservar las rutas guardadas localmente aunque el usuario cierre y vuelva a abrir la aplicación.

### Requisito: transparencia offline
La aplicación DEBE comunicar que el cálculo es local y que la disponibilidad geográfica depende del grafo instalado.

### Requisito: precache de artefactos locales
La aplicación DEBE incluir en la estrategia PWA el shell, los íconos, el módulo WASM, el grafo binario, las capas GeoJSON locales y el archivo PMTiles cuando estén publicados.

#### Escenario: primera carga sin conexión posterior
- DADO que el shell, el grafo y el PMTiles fueron publicados y cacheados durante una carga inicial
- CUANDO el usuario abre la PWA sin conexión
- ENTONCES la aplicación puede cargar el mapa local y el motor de ruteo sin solicitar un servicio de backend.

#### Escenario: actualización de artefactos
- DADO que existe una nueva versión del shell o de los artefactos locales
- CUANDO el service worker detecta la actualización
- ENTONCES la PWA debe actualizarse mediante `autoUpdate` sin perder las rutas guardadas en IndexedDB.

### Requisito: gestor de paquetes
El proyecto DEBE usar pnpm como único gestor de paquetes: `packageManager` declarado, `pnpm-lock.yaml` como único lockfile y scripts y documentación con `pnpm`.
