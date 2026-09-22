# Proposal: local cycling-routing PWA

## Context

The project needs an installable web application to draw bike routes on a map, calculate them on the device, and retain user data without relying on a backend.

## Proposed solution

Create a Vue 3 PWA with MapLibre GL JS and a custom Rust/WASM router. The application downloads a versioned Bogotá cycling graph, stores it in IndexedDB, calculates routes with A* inside a Web Worker, and saves routes as GeoJSON segments classified by infrastructure.

## Initial scope

- Navigable MapLibre/PMTiles map with basic controls.
- Origin/destination selection, plus primary and alternative routes calculated locally by Rust/WASM.
- Visual distinction between cycleways and conventional roads.
- Basic saved-route list.
- PWA installability and a cacheable shell.
- Clear messages when the graph is missing or connectivity is unavailable.

## Initial out of scope

- Cross-device synchronization, accounts, or a backend.
- Turn-by-turn navigation or road-safety guarantees.
- Processing the OSM PBF inside the app; generation happens in the Rust pipeline.
- Geocoding and address search; implemented by the independent `agregar-geocodificador-local` change.

## Risks and mitigations

- Results depend on the graph: version the area, OSM source, date, and cycling profile with the binary.
- PMTiles coverage and graph coverage must remain synchronized: version area, date, license, and OSM source before production.
- The binary may be large: measure size, progressive loading, and area limits.
