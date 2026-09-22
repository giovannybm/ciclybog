# Technical design

## Flow

1. Vite serves the Vue shell and registers the service worker.
2. MapLibre creates the map with a wide `maxBounds` window; the mask and selection validation use Bogotá's local administrative polygon.
3. The PWA downloads `VITE_ROUTE_GRAPH_URL`, stores it in IndexedDB, and passes it to the Rust/WASM worker.
4. The worker projects points onto navigable edges and runs weighted minimum-cost bidirectional A* on a directed graph, returning GeoJSON segments with infrastructure metadata.
5. The route receives an ID and timestamps, is rendered with per-segment colors, and is persisted in IndexedDB.

The visual layer is built from `public/data/bogota.pmtiles` through `pmtiles://`. `public/data/bogota-mask.geojson` covers the outside of `public/data/bogota-boundary.geojson` in white. Green areas use `#ddffc6`, water `#c6d9ff`, base-map cycleways `#3437eb`, and calculated-route `cycleway` segments `#17601a`.

## Data limits

The `SavedRoute` entity contains `id`, `name`, `geometry`, `createdAt`, and `updatedAt`. GeoJSON is the portable source for future GPX/GeoJSON export features.

## Offline/PWA

The precache includes the shell and published `.bin` files. The MapLibre demo style uses cache-first only as a starter; production must review licenses, terms of use, tiles, and offline coverage.

## Graph generation

The `scripts/download-bogota-osm-pbf.sh` script downloads the Bogotá OSM PBF extract. The `rust/data-pipeline` pipeline reads OSM nodes and roads directly, splits each road at its original nodes, respects `oneway`/`junction=roundabout`, filters bicycle-incompatible access, and produces the custom binary format. PMTiles remains reserved for map rendering. The extractor keeps bicycle-relevant classes, preserves directions, and assigns cost multipliers by infrastructure and road type. The engine compares the physically shortest route with the preferred cycling route and limits preferred detour to 18%, allowing cycleways and conventional roads to combine without excessive detours. Visual classification does not imply that a road is safe.

The administrative boundary is updated with `pnpm map:boundary:update`; `MAP_VIEW_BOUNDS` only controls how far the map can zoom and pan.

The format retains the directed graph, CSR adjacency lists, and turn restrictions, and is versioned through `Graph.version`. `pnpm router:export-geojson` allows exact-edge inspection in QGIS.
