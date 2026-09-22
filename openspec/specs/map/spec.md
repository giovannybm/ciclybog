# Capability: local Bogotá map

## Requirements

### Requirement: PMTiles source
The application MUST load the local vector map from `VITE_PMTILES_URL` through MapLibre's `pmtiles://` protocol.

### Requirement: OSM cycling layer
The application MUST show `public/data/bogota-cycleways.geojson` by default. It is generated from the same OSM PBF extract that feeds the graph, uses color `#3437eb`, and is visible from zoom 9.

### Requirement: geographic mask
The application MUST render map content only inside Bogotá's local administrative polygon (`bogota-boundary.geojson`) and cover exterior content in white through `bogota-mask.geojson`.

#### Scenario: outside the area
- GIVEN the viewport includes a margin around the data area
- WHEN the map renders
- THEN the area outside the configured boundary appears white, without tile labels or styles.

### Requirement: infrastructure styling
The application MUST visually distinguish route segments by `infrastructure`.

#### Scenario: map and route colors
- GIVEN a loaded PMTiles map and a calculated route
- WHEN their layers render
- THEN green areas use `#ddffc6`, water uses `#c6d9ff`, base-map cycleways use `#3437eb`, calculated-route cycleways use `#17601a`, and conventional roads keep a distinct color.

### Requirement: bounded navigation
The application MUST allow a panning margin around Bogotá without allowing indefinite navigation outside the configured area.

> Panning uses a wide rectangular window, while masking and selection use the local administrative polygon downloaded from OpenStreetMap.
