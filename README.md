# Ciclybog

PWA instalable en Vue 3 + MapLibre para calcular y guardar rutas ciclistas en Bogotá, con un motor de ruteo propio en Rust/WASM que corre en el dispositivo.

## Inicio rápido

El proyecto usa **pnpm** como único gestor de paquetes (`packageManager` en `package.json`; `npm install` es rechazado por `only-allow`).

```sh
corepack enable        # o instala pnpm >= 10
pnpm install
cp .env.example .env
pnpm dev
```

La aplicación calcula las rutas en el dispositivo con Rust/WASM dentro de un Web Worker. No existe un backend de ruteo.

## Uso

- Toca el mapa para fijar el origen y luego el destino, o usa **📍 Mi ubicación como origen** / **🏁 Mi ubicación como destino**. El botón de geolocalización del mapa muestra y sigue tu posición. La ubicación requiere HTTPS o `localhost` y permiso del navegador.
- El motor devuelve la ruta principal y hasta dos alternativas. La lista muestra la distancia y el porcentaje en cicloruta de cada una; la seleccionada es la que se guarda.
- **Instalar app** aparece cuando el navegador lo permite (Chrome, Edge, Android). En iPhone/iPad se muestran instrucciones para *Compartir → Agregar a inicio*.

## Grafo de ruteo

El grafo de Bogotá se genera desde `data/bogota.osm.pbf` y queda en `public/data/bogota-graph.bin`. PMTiles solo se usa para el mapa.

```sh
pnpm map:download-osm   # descarga el extracto de BBBike
pnpm router:osm-data    # genera el .bin y la capa de ciclorutas
pnpm router:wasm        # compila el motor a WASM
pnpm router:inspect     # tamaño, componentes y tiempos de ruta de ejemplo
pnpm router:test        # pruebas del motor y del pipeline
```

Con otro PBF compatible con OSM:

```sh
OSM_PBF_SOURCE_URL=https://.../bogota.osm.pbf pnpm map:download-osm
```

El PBF por defecto proviene del extracto de Bogotá de [BBBike](https://download.bbbike.org/osm/bbbike/Bogota/). Los datos de OpenStreetMap están sujetos a ODbL y deben conservar la atribución.

### Pipeline

- **Clasificación de ciclorutas:** una vía es cicloruta si es `highway=cycleway`, si `cycleway`/`cycleway:both|left|right` es `track`, `lane` u `opposite_*`, o si es un sendero con acceso ciclista explícito. `cycleway:*=no`, `separate` y `shared_lane` no cuentan.
- **Acceso:** `bicycle=yes|designated|permissive|official` prevalece sobre `access=no` y `vehicle=no`. `bicycle=dismount` entra con costo ×3.
- **Sentido:** se respetan `oneway` y las glorietas, y se habilita el contraflujo ciclista con `oneway:bicycle=no` o `cycleway*=opposite*`.
- **Restricciones de giro:** se ignoran las que tienen `except=bicycle` y las específicas de otros vehículos (`restriction:motorcar`, …). Se usa `restriction:bicycle` si existe.
- **Contracción:** el grafo solo tiene nodos en intersecciones, extremos y nodos de restricción. Las dos direcciones de una vía comparten geometría y los textos van en una tabla. Resultado actual: 107.831 nodos, 246.985 aristas, 23 MB.

### Motor

- **Índices:** al cargar se calculan la componente fuertemente conexa principal, una grilla espacial y un índice de restricciones.
- **Snap:** se proyecta el punto sobre la vía más cercana a menos de 250 m y se divide la arista y su gemela en un overlay temporal, sin clonar el grafo.
- **Búsqueda:** A* sobre estados por arista, de modo que las penalizaciones y restricciones de giro son exactas. La ruta preferida (ponderada por infraestructura) no supera el 18% de desvío respecto a la más corta.
- **Alternativas:** se obtienen penalizando las vías ya usadas. Se aceptan si miden ≤ 1,4× la principal y comparten ≤ 80% de su distancia.
- **Rendimiento:** en release nativo, carga e índices tardan ~75 ms y una ruta de 6 km tarda ~3 ms (una de 20 km, ~16 ms).

La clave del grafo en IndexedDB se deriva del hash SHA-1 del `.bin` durante el build, así que un grafo regenerado reemplaza automáticamente al anterior.

Para inspeccionar en QGIS las aristas exactas del motor:

```sh
pnpm router:export-geojson   # → data/bogota-graph.geojson
```

El costo de ruteo pondera la infraestructura; la distancia mostrada es la física.

## Búsqueda de direcciones

Los campos **Origen** y **Destino** buscan en un índice local (`public/data/bogota-geocoder.json`) generado desde el mismo PBF:

```sh
pnpm geocoder:generate   # vías con geometría, lugares y direcciones de OSM
pnpm test                # pruebas del geocodificador (incluye datos reales)
```

OSM tiene pocas direcciones con número en Bogotá, así que las direcciones con nomenclatura (`Cra. 10 172b 50`, `Calle 26 Sur # 13-20`, `KR 10 No. 172 B - 50`) se calculan así:

- **Registrada en OSM:** si la dirección existe, se usa directamente.
- **Cruce:** si la vía cruzada toca la principal, se usa el punto de cruce.
- **Prolongación:** si la cruzada no llega, se prolonga hasta 600 m, siempre que no contradiga el orden de los cruces reales vecinos.
- **Interpolación:** en otro caso se interpola entre las cruzadas de número inmediatamente menor y mayor.
- **Placa:** desde ese punto se avanza la cantidad de metros de la placa hacia las cruzadas de mayor número.

Cada sugerencia indica cómo se obtuvo, por ejemplo *Interpolada entre Calle 172 y Calle 173*. Es una estimación: la precisión depende de cómo estén nombradas las vías en OSM.

## PWA

- `pnpm pwa:assets` regenera los íconos (64, 192, 512, maskable y apple-touch) desde `public/favicon.svg`.
- El manifest declara `id`, `scope`, `lang`, `display: standalone` e íconos; el service worker (Workbox, `autoUpdate`) precachea el shell, el WASM, el grafo, las capas GeoJSON y el PMTiles.
- La instalación y la geolocalización requieren un origen seguro: HTTPS en producción o `localhost` en desarrollo. Para probar en un teléfono dentro de la red local, sirve la app por HTTPS (por ejemplo con un túnel).

## Mapa offline con PMTiles

Coloca el extracto PMTiles de Bogotá en `public/data/bogota.pmtiles` y activa `VITE_PMTILES_URL=/data/bogota.pmtiles`. La aplicación registra el protocolo `pmtiles://` y genera el estilo vectorial Protomaps localmente.

- Zonas verdes: `#ddffc6`.
- Agua: `#c6d9ff`.
- Capa `public/data/bogota-cycleways.geojson` (generada desde el mismo PBF que el grafo): `#3437eb`.
- Tramos en cicloruta de una ruta calculada: `#17601a`.
- Una máscara blanca cubre lo que queda fuera de Bogotá.

Sin ese archivo se usa el estilo demo de `VITE_MAP_STYLE_URL`, que no es una experiencia offline completa.

Para extraer Bogotá desde un PMTiles global instala [go-pmtiles](https://github.com/protomaps/go-pmtiles/releases) y ejecuta:

```sh
PMTILES_SOURCE_URL=https://build.protomaps.com/YYYYMMDD.pmtiles pnpm map:download
```

`MAX_ZOOM` controla el nivel máximo (por defecto `15`).

El límite administrativo (`public/data/bogota-boundary.geojson` y `bogota-mask.geojson`) se actualiza con `pnpm map:boundary:update`.

## Especificación

- Especificaciones vigentes: [`openspec/specs/`](openspec/specs/).
- Cambios archivados: [`openspec/changes/archive/`](openspec/changes/archive/).
