# Technical design

## Graph format (v3, magic `CICLYG02`)

- `nodes`: intersections, road endpoints, turn-restriction nodes, or cuts caused by missing nodes.
- `geometries`: shared polylines. Each edge references `geometry` and `reversed`, so both directions of a two-way road share coordinates.
- `strings`: text table; `road_name` and `bicycle_access` are indexes.
- `distance_meters` and `routing_cost` are `f32`; incoming and outgoing CSR lists; `turn_restrictions` use `via_node` as an internal index.

## Pipeline

| Rule | Behavior |
| --- | --- |
| Cycleway | `highway=cycleway`; `cycleway`, `cycleway:both`, `cycleway:left`, or `cycleway:right` with `track`, `lane`, `opposite_track`, or `opposite_lane`; `path`/`track`/`footway`/`pedestrian` with explicit bicycle access. `shared_lane`, `share_busway`, and `separate` do not turn a road into a cycleway. |
| Access | `bicycle=yes/designated/permissive/official/dismount` takes precedence over `access`/`vehicle=no/private`. `bicycle=no/private` excludes the road. |
| Dismount | `bicycle=dismount` is included with factor 3.0 and conventional classification. |
| Direction | `oneway:bicycle=no` or any `cycleway*=opposite*` creates two-way bicycle access; `oneway:bicycle=yes` forces one-way access; `junction=roundabout` implies one-way. |
| Restrictions | Ignore relations when `except` contains `bicycle`. Use `restriction:bicycle` when present, otherwise `restriction`. Ignore `restriction:<other vehicle>` and conditional restrictions. |

## Engine

`PreparedGraph` is built once at load time:

1. **Main component:** largest strongly connected component (iterative Kosaraju). Snapping only uses edges whose endpoints belong to it, guaranteeing directed reachability.
2. **Spatial grid:** 0.001° cells store indexes of edges whose bounding box touches them. Snapping checks cells within 250 m.
3. **Restrictions:** `(via_node, from_way) → [(to_way, only)]` map.

### Query

- **Snap:** project the point onto the nearest edge (≤ 250 m) and split that edge and its reverse twin into a virtual node/edge overlay. If origin and destination fall on the same geometry in compatible order, add a direct edge. The base graph is neither modified nor cloned.
- **Search:** A* over edge states. Transition cost is `distance × factor + turn penalty`; the turn uses local bearings from the last incoming-edge segment and first outgoing-edge segment. The haversine-to-destination heuristic is admissible and consistent because `factor ≥ 1` and penalties are non-negative. Search ends when the first edge reaching the destination is popped.
- **Primary:** calculate the weighted route and the shortest route (factor 1); use the weighted route when its detour does not exceed 18%.
- **Alternatives:** multiply the cost of edges in accepted routes (×1.6, then ×2.5) and include the shortest route as a candidate. Accept a candidate when its distance is ≤ 1.4 × the primary and it shares ≤ 80% of its distance with every accepted route.
- **Response:** each route includes `distance_meters`, `cycleway_meters`, and consecutive segments merged by infrastructure, name, and access.

## Frontend and PWA

- The IndexedDB key is `bogota-graph-<sha1 of .bin>`, injected by Vite during the build; saving a graph removes previous keys.
- The UI shows primary and alternative calculated routes; the selected route is rendered and can be saved.
- Location uses `navigator.geolocation` (high accuracy, 15-second timeout), validates that the point is in Bogotá, and assigns it as origin or destination; `GeolocateControl` shows the device position on the map.
- Installation uses icons generated with `@vite-pwa/assets-generator` from `public/favicon.svg`, plus `id`, `scope`, `lang`, `orientation`, and `categories` in the manifest. The button uses `beforeinstallprompt` and includes iOS instructions.
