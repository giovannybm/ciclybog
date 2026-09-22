# Ciclybog

An installable PWA for planning bike routes through Bogotá. Ciclybog combines Vue 3, MapLibre, and a custom Rust/WASM routing engine that runs directly on the device.

The app supports local address search, map and device-location point selection, route alternatives, and locally saved routes. Routing does not depend on a backend and can work offline after the local assets have been installed.

## Requirements

- Node.js 22 or compatible.
- pnpm 10 or newer.
- Stable Rust and `wasm-bindgen-cli` to regenerate the WASM engine or data assets.

Frontend development only requires Node.js and pnpm. Rust is required for router tests and data-pipeline commands.

## Quick start

The project uses **pnpm** as its only package manager (`packageManager` in `package.json`; `npm install` is rejected by `only-allow`).

```sh
corepack enable        # or install pnpm >= 10
pnpm install
cp .env.example .env
pnpm dev
```

Open the URL printed by Vite, normally `http://localhost:5173`.

The app calculates routes on the device with Rust/WASM inside a Web Worker. There is no routing backend.

### Microsoft Clarity analytics

To enable Clarity, create a Microsoft Clarity project and add its ID to the environment:

```sh
VITE_CLARITY_PROJECT_ID=your_project_id
```

The integration is optional. Clarity is not loaded when this variable is absent, and the app asks for user consent before starting it.

### Environment variables

All variables are optional during development. By default, the app loads the local assets included in `public/data/`.

| Variable | Purpose |
| --- | --- |
| `VITE_CLARITY_PROJECT_ID` | Enables Microsoft Clarity after user consent. |
| `VITE_MAP_STYLE_URL` | Remote MapLibre style when local PMTiles are not used. |
| `VITE_PMTILES_URL` | Path to a local or remote PMTiles file. |
| `VITE_ROUTE_GRAPH_URL` | Alternative path to the binary routing graph. |
| `VITE_SITE_URL` | Public URL used for canonical, Open Graph, and Twitter/X metadata. |

`.env.example` contains the starter template.

## Using the app

- Tap the map to set the origin and then the destination, search for an address or place, or use the location button beside either field. Browser location requires HTTPS or `localhost` and the user's permission.
- On mobile (≤ 720 px), the map fills the screen. Search floats at the top and collapses to a summary after a route is calculated; a bottom sheet contains status, routes, and actions. Swipe or tap the sheet handle to view the legend, saved routes, and privacy settings.
- The engine returns a primary route and up to two alternatives. Each route displays distance and cycleway coverage; the selected route is the one that gets saved.
- **Install app** appears when the browser supports installation (Chrome, Edge, Android). On iPhone/iPad, the app shows instructions for *Share → Add to Home Screen*.

## Routing graph

The Bogotá graph is generated from `data/bogota.osm.pbf` and stored at `public/data/bogota-graph.bin`. PMTiles is used only for map rendering.

```sh
pnpm map:download-osm   # download the BBBike extract
pnpm router:osm-data    # generate the graph and cycleway layer
pnpm router:wasm        # compile the routing engine to WASM
pnpm router:inspect     # inspect size, components, and sample route timings
pnpm router:test        # run router and pipeline tests
```

With another compatible PBF:

```sh
OSM_PBF_SOURCE_URL=https://.../bogota.osm.pbf pnpm map:download-osm
```

The default PBF comes from the [BBBike Bogotá extract](https://download.bbbike.org/osm/bbbike/Bogota/). OpenStreetMap data is subject to the ODbL; keep the required attribution when publishing the app.

### Pipeline rules

- **Cycleway classification:** a road is a cycleway when `highway=cycleway`, when `cycleway`/`cycleway:both|left|right` is `track`, `lane`, or `opposite_*`, or when a path has explicit bicycle access. `cycleway:*=no`, `separate`, and `shared_lane` do not count.
- **Access:** `bicycle=yes|designated|permissive|official` takes precedence over `access=no` and `vehicle=no`. `bicycle=dismount` is included with a ×3 cost.
- **Direction:** `oneway` and roundabouts are respected. Bicycle contraflow is enabled by `oneway:bicycle=no` or `cycleway*=opposite*`.
- **Turn restrictions:** restrictions with `except=bicycle` and restrictions specific to other vehicles (`restriction:motorcar`, …) are ignored. `restriction:bicycle` is used when available.
- **Contraction:** the graph keeps nodes at intersections, endpoints, and restriction nodes. Both directions of a road share geometry and strings are stored in a table. Current result: 107,831 nodes, 246,985 edges, 23 MB.

### Routing engine

- **Indexes:** the largest strongly connected component, a spatial grid, and a turn-restriction index are built at load time.
- **Snapping:** the point is projected onto the nearest road within 250 m. The edge and its twin are split in a temporary overlay without cloning the graph.
- **Search:** A* runs over edge states, so turn penalties and restrictions remain exact. The preferred route stays within 18% of the shortest route.
- **Alternatives:** alternatives penalize roads already used by the primary route. They are accepted when they are ≤ 1.4× the primary distance and share ≤ 80% of its distance.
- **Performance:** native release builds load and index the graph in ~75 ms; a 6 km route takes ~3 ms and a 20 km route ~16 ms.

The IndexedDB graph key is derived from the `.bin` SHA-1 hash during the build, so regenerating the graph automatically replaces the previous version.

To inspect the engine's exact edges in QGIS:

```sh
pnpm router:export-geojson   # → data/bogota-graph.geojson
```

Routing cost weights infrastructure; displayed distance remains physical distance.

## Address search

The **Origin** and **Destination** fields search a local index (`public/data/bogota-geocoder.json`) generated from the same PBF:

```sh
pnpm geocoder:generate   # roads with geometry, places, and OSM addresses
pnpm test                # geocoder tests, including real data
```

OSM contains relatively few numbered addresses in Bogotá, so street-number addresses (`Cra. 10 172b 50`, `Calle 26 Sur # 13-20`, `KR 10 No. 172 B - 50`) are resolved using:

- **Registered OSM address:** used directly when available.
- **Intersection:** the crossing point is used when the cross street touches the main street.
- **Extension:** the cross street is extended by up to 600 m when it does not reach the main street, unless that contradicts neighboring intersections.
- **Interpolation:** otherwise, the point is interpolated between the nearest lower and higher numbered cross streets.
- **House number:** the plate distance is advanced toward higher-numbered cross streets.

Each suggestion reports how it was resolved, for example *Interpolated between Calle 172 and Calle 173*. It is an estimate; accuracy depends on how roads are named in OSM.

## PWA

- `pnpm pwa:assets` regenerates the 64, 192, 512, maskable, and Apple Touch icons from `public/favicon.svg`.
- The manifest declares `id`, `scope`, `lang`, `display: standalone`, and icons. Workbox (`autoUpdate`) precaches the shell, WASM, graph, GeoJSON layers, geocoder index, and PMTiles.
- Installation and geolocation require a secure origin: HTTPS in production or `localhost` during development. To test on a phone over a local network, serve the app over HTTPS, for example through a tunnel.

## Build and deployment

```sh
pnpm typecheck
pnpm test
pnpm build
pnpm preview
```

The project is configured for Netlify with `pnpm build` and `dist/` as the publish directory. Set `VITE_SITE_URL` in production when the domain is not available automatically through Netlify's `URL` variable.

An embedded build is available under `/demos/ciclybog/`:

```sh
pnpm build:embed
```

This command generates `dist-embed/`, does not register a service worker, and disables analytics.

## Social previews

`index.html` includes Open Graph and Twitter (`summary_large_image`) metadata so shared links show a preview in WhatsApp, Facebook, LinkedIn, Slack, and X.

- The image is `public/og-image.png` (1200×630), regenerated with `pnpm og:image` from `scripts/generate-og-image.mjs`.
- Social networks require absolute URLs. During the build, `%SITE_URL%` is replaced by `VITE_SITE_URL` or Netlify's `URL` variable. If neither exists, the build prints a warning and uses the default site URL.
- To inspect a published link, use [Facebook Sharing Debugger](https://developers.facebook.com/tools/debug/) or [opengraph.xyz](https://www.opengraph.xyz/). WhatsApp and other networks cache previews.

## Offline map with PMTiles

Place the Bogotá PMTiles extract at `public/data/bogota.pmtiles` and set `VITE_PMTILES_URL=/data/bogota.pmtiles`. The app registers the `pmtiles://` protocol and generates a local Protomaps vector style.

- Green areas: `#ddffc6`.
- Water: `#c6d9ff`.
- `public/data/bogota-cycleways.geojson` (generated from the same PBF as the graph): `#3437eb`.
- Cycleway segments in a calculated route: `#17601a`.
- A white mask covers the area outside Bogotá.

Without this file, the app uses the demo style from `VITE_MAP_STYLE_URL`, which is not a complete offline experience.

To extract Bogotá from a global PMTiles file, install [go-pmtiles](https://github.com/protomaps/go-pmtiles/releases) and run:

```sh
PMTILES_SOURCE_URL=https://build.protomaps.com/YYYYMMDD.pmtiles pnpm map:download
```

`MAX_ZOOM` controls the maximum zoom level (15 by default).

The administrative boundary (`public/data/bogota-boundary.geojson` and `bogota-mask.geojson`) is updated with `pnpm map:boundary:update`.

## Specifications

- Current specifications: [`openspec/specs/`](openspec/specs/).
- Archived changes: [`openspec/changes/archive/`](openspec/changes/archive/).
