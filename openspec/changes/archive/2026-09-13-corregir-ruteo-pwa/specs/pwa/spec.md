# Delta: PWA offline

## MODIFICADO

### Requisito: instalabilidad
La aplicación DEBE publicar un manifest con `id`, `name`, `short_name`, `start_url`, `scope`, `display: standalone`, `lang`, colores e íconos PNG de 192 y 512 px (incluido uno `maskable`), un `apple-touch-icon` y registrar un service worker con manejador `fetch`. Cuando el navegador lo permita, la interfaz DEBE ofrecer un botón “Instalar app”; en iOS DEBE indicar cómo agregarla a la pantalla de inicio.

#### Escenario: instalación en Chrome/Android
- DADO que la app se sirve por HTTPS o localhost y el service worker está activo
- CUANDO el navegador emite `beforeinstallprompt`
- ENTONCES aparece “Instalar app” y al pulsarlo se muestra el diálogo nativo; tras instalar, el botón desaparece.

## AGREGADO

### Requisito: gestor de paquetes
El proyecto DEBE usar pnpm como único gestor de paquetes: `packageManager` declarado, `pnpm-lock.yaml` como único lockfile y scripts y documentación con `pnpm`.
