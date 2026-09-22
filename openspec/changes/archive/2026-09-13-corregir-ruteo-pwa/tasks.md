# Tasks

## Pipeline
- [x] Classify cycleways only from positive `cycleway`, `cycleway:both|left|right` values.
- [x] Filter turn restrictions with `except=bicycle` and vehicle-specific restrictions; use `restriction:bicycle`.
- [x] Support bicycle contraflow (`oneway:bicycle`, `cycleway*=opposite*`).
- [x] Give `bicycle=*` precedence over `access`/`vehicle`; handle `bicycle=dismount`.
- [x] Contract the graph to intersections with shared geometry and strings (63 MB → 23 MB; 290,955 → 107,831 nodes).
- [x] Add unit tests for classification, direction, and restrictions.

## Engine
- [x] Prepare load-time indexes: main SCC, spatial grid, and indexed restrictions.
- [x] Run edge-state A* with correct turn penalties and restrictions.
- [x] Snap with a 250 m maximum, reverse twin, and non-cloning overlay.
- [x] Generate penalty alternatives with overlap control.
- [x] Test optimality against exhaustive search, forbidden turns with alternate arrivals, `only_*`, `no_u_turn`, two-way roads without return trips, far snaps, same-edge routes, alternatives, and SCC behavior.
- [x] Update the GeoJSON exporter and inspector.

## Frontend / PWA / tooling
- [x] Add an alternative selector with cycleway percentage.
- [x] Derive the graph key from content and clean previous keys.
- [x] Add `noEmit` to `tsconfig.app.json` and remove generated `.js` files from `src/`.
- [x] Migrate to pnpm (lockfile, `packageManager`, `only-allow`, scripts, README).
- [x] Add installable icons and manifest, **Install app** button, and iOS instructions.
- [x] Use my location as origin/destination and add `GeolocateControl`.
- [x] Regenerate graph and WASM; measure size and timings (native: 6 km route in ~3 ms, previously 169 ms).
