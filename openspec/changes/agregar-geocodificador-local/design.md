# Design: local geocoder

## Pipeline

```text
data/bogota.osm.pbf
        ├── bogota-graph.bin
        ├── bogota-cycleways.geojson
        └── bogota-geocoder.bin
```

The Rust pipeline reads OSM nodes, roads, and tags. For each useful entity it generates a record with normalized text, display text, coordinates, type, and priority. It primarily includes `addr:*`, `place`, `amenity`, `shop`, `tourism`, `leisure`, `name`, and named `highway` features.

## Index

The first version may use a binary term-to-ID index and a compact record list. Search in the Web Worker combines:

1. Unicode and abbreviation normalization;
2. prefix matching;
3. token matching;
4. bounded Levenshtein distance for small errors;
5. geographic distance from the visible map center.

The index is cached in IndexedDB with a key that includes the PBF version. It must be reproducible with a single command.

## UI integration

The **From** and **To** fields provide local autocomplete. Selecting a result updates the marker while keeping manual map selection available. The coordinate is passed to the router, which snaps it to the nearest navigable edge.

## PWA integration

`bogota-geocoder.bin` is included in the precache when its size is acceptable. If it is not cached, the application reports its installation status without blocking the map or manual selection.
