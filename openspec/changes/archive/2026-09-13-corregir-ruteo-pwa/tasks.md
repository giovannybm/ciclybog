# Tareas

## Pipeline
- [x] Clasificar ciclorruta solo con valores positivos de `cycleway`, `cycleway:both|left|right`.
- [x] Filtrar restricciones de giro con `except=bicycle` y las específicas de otros vehículos; usar `restriction:bicycle`.
- [x] Soportar contraflujo ciclista (`oneway:bicycle`, `cycleway*=opposite*`).
- [x] Dar precedencia a `bicycle=*` sobre `access`/`vehicle`; tratar `bicycle=dismount`.
- [x] Contraer el grafo a intersecciones con geometrías y textos compartidos (63 MB → 23 MB; 290.955 → 107.831 nodos).
- [x] Pruebas unitarias de clasificación, sentido y restricciones.

## Motor
- [x] Preparar índice al cargar: SCC principal, grilla espacial, restricciones indexadas.
- [x] A* por aristas con penalización y restricciones de giro correctas.
- [x] Snap con distancia máxima de 250 m, arista gemela y overlay sin clonar.
- [x] Alternativas por penalización con control de solapamiento.
- [x] Pruebas: optimalidad frente a búsqueda exhaustiva, giros prohibidos con llegada alternativa, `only_*`, `no_u_turn`, doble sentido sin retorno, snap lejano, misma arista, alternativas, SCC.
- [x] Actualizar exportador GeoJSON e inspector.

## Frontend / PWA / herramientas
- [x] Selector de alternativas con porcentaje en ciclorruta.
- [x] Clave del grafo por hash de contenido y limpieza de claves anteriores.
- [x] `noEmit` en `tsconfig.app.json` y eliminación de `.js` generados en `src/`.
- [x] Migrar a pnpm (lockfile, `packageManager`, `only-allow`, scripts, README).
- [x] Íconos y manifest instalables; botón “Instalar app” e instrucciones iOS.
- [x] Usar mi ubicación como origen/destino y `GeolocateControl`.
- [x] Regenerar grafo y WASM; medir tamaño y tiempos (nativo: ruta de 6 km en ~3 ms, antes 169 ms).
