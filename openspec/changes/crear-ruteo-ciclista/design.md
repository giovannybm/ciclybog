# Diseño técnico

## Flujo

1. Vite sirve el shell Vue y registra el service worker.
2. MapLibre crea el mapa con una ventana `maxBounds` amplia; la máscara y la validación de selección usan el polígono administrativo local de Bogotá.
3. La PWA descarga `VITE_ROUTE_GRAPH_URL`, lo guarda en IndexedDB y lo entrega al worker Rust/WASM.
4. El worker proyecta los puntos sobre aristas navegables y ejecuta A* bidireccional con costo mínimo ponderado sobre un grafo dirigido; devuelve segmentos GeoJSON con metadata de infraestructura.
5. La ruta recibe identidad y timestamps, se muestra con colores por segmento y se persiste en IndexedDB.

La capa visual se construye desde `public/data/bogota.pmtiles` mediante `pmtiles://`. `public/data/bogota-mask.geojson` cubre con blanco el exterior de `public/data/bogota-boundary.geojson`. Las zonas verdes usan `#ddffc6`, el agua `#c6d9ff`, las ciclovías del mapa base `#3437eb` y los segmentos `cycleway` de una ruta calculada `#17601a`.

## Límites de datos

La entidad `SavedRoute` contiene `id`, `name`, `geometry`, `createdAt` y `updatedAt`. El GeoJSON es la fuente portable para futuras funciones de exportación GPX/GeoJSON.

## Offline/PWA

El precache incluye el shell y archivos `.bin` publicados. El estilo de demostración MapLibre usa cache-first solo como starter; producción debe revisar licencias, términos de uso, tiles y cobertura offline.

## Generación del grafo

El script `scripts/download-bogota-osm-pbf.sh` descarga el extracto OSM PBF de Bogotá. El pipeline `rust/data-pipeline` lee nodos y vías OSM directamente, divide cada vía en sus nodos originales, respeta `oneway`/`junction=roundabout`, filtra accesos incompatibles con bicicleta y produce el formato binario propio. PMTiles queda reservado para la representación cartográfica. El extractor filtra clases relevantes para bicicletas, conserva sentidos y asigna multiplicadores de costo por infraestructura y tipo de vía. El motor compara la ruta físicamente más corta con la ruta preferida para bicicleta y limita el desvío preferido al 18%; así puede combinar ciclovías y vías convencionales sin forzar desvíos grandes. La clasificación visual no implica que una vía sea segura.

El límite administrativo se actualiza con `npm run map:boundary:update`; la ventana rectangular `MAP_VIEW_BOUNDS` solo controla cuánto puede alejarse y desplazarse el mapa.

El formato conserva el grafo dirigido, adyacencias CSR y restricciones de giro, y se versiona mediante `Graph.version`. `npm run router:export-geojson` permite inspeccionar las aristas exactas en QGIS.
