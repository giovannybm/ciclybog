# Tasks

- [x] Define the versioned geographic-index schema (JSON v2: roads with geometry, places, addresses).
- [x] Extract `addr:*`, named streets with geometry, and tagged places from the PBF without junk text or fabricated coordinates.
- [x] Implement accent, abbreviation, and punctuation normalization.
- [x] Parse Bogotá street nomenclature (type, number, letter, bis, quadrant, plate).
- [x] Locate addresses by intersection, validated extension, or interpolation, then advance the plate distance.
- [x] Implement word-prefix search and ranking by type and distance from the map center.
- [x] Create `public/data/bogota-geocoder.json` with `pnpm geocoder:generate` (6.4 MB; 1.8 MB gzip).
- [x] Add **Origin** and **Destination** autocomplete with debounce and stale-response rejection.
- [x] Connect results to the Rust/WASM router snap (Bogotá validation and map centering).
- [x] Include the index in the PWA precache.
- [x] Add normalization, nomenclature, interpolation, no-result, and real-data tests (`pnpm test`).
- [ ] Load the index in a Web Worker to avoid blocking the main thread (currently ~260 ms preparation).
- [ ] Add basic reverse geocoding.
- [ ] Measure index load time on mobile.
