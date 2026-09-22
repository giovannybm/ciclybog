# Routing graph data

`public/data/bogota-graph.bin` is generated from a Bogotá OSM PBF extract with `pnpm router:osm-data`. PMTiles is used only for visualization.

## Format (v3, magic `CICLYG02`, version `osm-pbf-v4-contracted`)

- **Nodes:** only decision points: intersections between accepted roads, road endpoints, and nodes involved in turn restrictions.
- **Edges:** directed, with `distance_meters` and `routing_cost` (`f32`), a reference to shared geometry (`geometry` + `reversed`), infrastructure, indexes into the `strings` table for names and access, and `osm_way_id`.
- **Indexes and restrictions:** incoming and outgoing CSR adjacency lists, plus turn restrictions using `via_node` as an internal index.

The same command generates `public/data/bogota-cycleways.geojson`, which feeds the map's blue lines so the cycleway layer and graph use the same OSM version.

## Classification rules

- **`cycleway`:**
  - `highway=cycleway`.
  - `cycleway`, `cycleway:both`, `cycleway:left`, or `cycleway:right` with `track`, `lane`, `opposite_track`, or `opposite_lane`.
  - `path`/`track`/`footway`/`pedestrian` with `bicycle=yes|designated|permissive|official`.
- **Not a cycleway:** `cycleway:*=no`, `separate` (the cycleway is mapped as a separate road), `shared_lane`, and detail keys such as `cycleway:left:width`.
- **Excluded:** `footway`/`pedestrian` without explicit bicycle access; `bicycle=no|private`; `access` or `vehicle` `no|private` without explicit bicycle permission.
- **Dismount:** `bicycle=dismount` is routable with a factor of 3.0.
- **Contraflow:** `oneway:bicycle=no` or `cycleway*=opposite*` makes the road two-way for bicycles.
- **Restrictions:** relations with `except` containing `bicycle`, restrictions specific to other vehicles, and way-based intermediate restrictions are discarded.

### Cost factors

| Road | Factor |
| --- | ---: |
| `cycleway` | 1.00 |
| Bicycle `path`/`track` | 1.08 |
| Local streets | 1.05 |
| `tertiary` | 1.15 |
| Bicycle `footway` | 1.18 |
| Shared `track` | 1.22 |
| `secondary` | 1.28 |
| Shared `path` | 1.30 |
| `primary` | 1.42 |
| `dismount` | 3.00 |

## Latest generation result (September 13, 2026)

- 73,479 accepted roads.
- 107,831 nodes (104,563 in the largest strongly connected component).
- 246,985 edges, including 13,911 cycleway edges.
- 142,474 geometries and 2,038 applied restrictions.
- 23.0 MB.

The default reproducible extract is `https://download.bbbike.org/osm/bbbike/Bogota/Bogota.osm.pbf`, updated by the provider on September 5, 2026. OpenStreetMap data is distributed under the ODbL; preserve OSM/BBBike attribution when publishing the app.

To inspect the graph's exact edges in QGIS, run `pnpm router:export-geojson` and open `data/bogota-graph.geojson`. Its attributes are `from`, `to`, `distance_meters`, `routing_cost`, `infrastructure`, `road_name`, `osm_way_id`, and `bicycle_access`.

The PMTiles file and PBF must have compatible coverage and dates. The sample fixture (`pnpm router:sample-data`) is only for local validation.
