# Capability: local cycling routing

## Requirements

### Requirement: load a local graph
The application MUST load a configurable Rust/WASM graph before enabling routing, persist it in IndexedDB, and show an actionable state when the file is missing or fails. The Bogotá graph MUST be generated directly from an OSM PBF extract; PMTiles is reserved for visualization.

#### Scenario: graph available
- GIVEN the map is loaded and the graph responds correctly
- WHEN the Rust/WASM Web Worker initializes
- THEN the user can select an origin and destination to calculate a route.

#### Scenario: graph missing
- GIVEN the graph URL does not respond
- WHEN initialization finishes
- THEN the application reports that the graph must be installed and keeps the map usable.

### Requirement: cached graph invalidation
The IndexedDB graph key MUST derive from the published `.bin` content, and older versions MUST be removed when a new one is saved.

### Requirement: client-side calculation
The application MUST run a weighted minimum-cost A* search over edge states in a Web Worker through WebAssembly and MUST NOT require a routing endpoint. The returned route MUST be optimal for the defined cost, including penalties and turn restrictions.

#### Scenario: worker request
- GIVEN the user selects an origin and destination
- WHEN the application sends the request to the worker
- THEN it transfers plain serializable coordinates and receives classified GeoJSON segments.

#### Scenario: optimality
- GIVEN a graph with turn penalties
- WHEN a route is calculated
- THEN its cost matches an exhaustive search over edge states.

#### Scenario: engine error
- GIVEN Rust cannot find a route, the graph is invalid, or WASM fails
- WHEN the worker reports the exception
- THEN the interface shows the specific message returned by the engine rather than a generic error.

### Requirement: constrain Bogotá
The application MUST prevent selection outside the configured Bogotá D.C. boundary and limit panning to a window with a margin around the data area.

#### Scenario: point outside the area
- GIVEN the user clicks outside the configured boundary
- WHEN the click is processed
- THEN the application does not create an origin or destination and reports that the point is outside Bogotá.

### Requirement: user location
The application MUST allow the device's current location to be used as an origin or destination, validate that it is inside Bogotá, and show permission, availability, and timeout errors.

#### Scenario: location as origin
- GIVEN the user grants location permission and is in Bogotá
- WHEN they press **Use my location as origin**
- THEN the origin is placed at their position and the map centers there; if a destination already exists, the route is calculated.

#### Scenario: permission denied
- GIVEN the user rejects permission
- WHEN they press the location button
- THEN the application explains that location must be enabled and keeps manual selection available.

### Requirement: classify segments
The result MUST return segments classified as `cycleway`, `conventional`, or `unknown` so the map can use distinct styles. Consecutive segments with the same infrastructure, name, and access MUST be merged, and every route MUST report its cycleway meters.

### Requirement: prefer cycling infrastructure
The engine MUST weight each edge by infrastructure and road type, while reported distance MUST remain physical distance.

- A road MUST be classified as `cycleway` only when it is `highway=cycleway`, when `cycleway`, `cycleway:both`, `cycleway:left`, or `cycleway:right` is `track`, `lane`, `opposite_track`, or `opposite_lane`, or when it is a path or pedestrian road with explicit bicycle access.
- Pedestrian roads without explicit bicycle access MUST NOT enter the graph, and shared paths MUST cost more than a separated cycleway.
- `bicycle=yes|designated|permissive|official|dismount` MUST take precedence over `access=no` and `vehicle=no`, and `bicycle=dismount` MUST have a high cost.
- `oneway:bicycle=no` and `cycleway*=opposite*` MUST enable bicycle contraflow.
- The preferred route MUST NOT exceed 18% detour from the shortest route; if it does, the engine MUST return the shortest route.

### Requirement: alternative routes
The engine MUST attempt to return alternatives no more than 1.4 times the primary route distance and sharing no more than 80% of its distance with an accepted route. The interface MUST allow choosing among them and show distance and cycleway percentage.

#### Scenario: select an alternative
- GIVEN the engine returns two routes
- WHEN the user chooses the second
- THEN the map displays it and **Save** persists it marked as an alternative.

### Requirement: avoid isolated components
The engine MUST calculate the largest strongly connected component once when loading the graph and snap origin and destination only to edges whose endpoints both belong to it.

#### Scenario: one-way trap
- GIVEN a point near a road that cannot be exited while respecting directions
- WHEN snapping occurs
- THEN the engine chooses the nearest navigable road inside the main component.

### Requirement: snap to edges
The engine MUST project the origin and destination onto the nearest navigable edge within 250 m using a spatial index, and temporarily split that edge and its reverse twin without modifying or cloning the base graph.

#### Scenario: click away from an OSM node
- GIVEN the user selects a point near the middle of a road
- WHEN a route is calculated
- THEN the engine starts or ends at the projected position on the road.

#### Scenario: click far from the network
- GIVEN a point is more than 250 m from any navigable road
- WHEN a route is calculated
- THEN the engine returns an error stating that no suitable road is near the origin or destination.

#### Scenario: two-way road
- GIVEN a point on a two-way road
- WHEN the route must leave toward either endpoint
- THEN the engine leaves directly in that direction without traversing and returning along the block.

#### Scenario: origin and destination on one road
- GIVEN origin and destination lie on the same edge in the permitted direction
- WHEN a route is calculated
- THEN the route is the direct segment between both projections.

### Requirement: optimization-ready format
The binary file MUST retain an explicit version, incoming and outgoing CSR indexes, turn restrictions, shared string and geometry tables, and nodes only at decision points, without depending on PMTiles.

### Requirement: turn penalties
The engine MUST apply an additional cost based on the angle between the last segment of the incoming edge and the first segment of the outgoing edge without changing reported physical distance.

### Requirement: OSM turn restrictions
The engine MUST prevent `no_*` transitions and allow only the road specified by `only_*`; a `no_u_turn` on the same road MUST only prevent returning. The pipeline MUST ignore restrictions whose `except` includes `bicycle`, use `restriction:bicycle` when present, and ignore restrictions specific to other vehicles or conditionals.

#### Scenario: alternate arrival at a node
- GIVEN the cheapest arrival at a node forbids the required turn
- WHEN another arrival at the same node allows it
- THEN the engine finds the route using that other arrival.

### Requirement: persist routes
The application MUST save every generated GeoJSON LineString in local storage and restore it when opened again.

#### Scenario: new route
- GIVEN Rust/WASM returns a calculated route
- WHEN the user saves it
- THEN it is stored with an ID and timestamps and appears under **My saved routes**.

### Requirement: inspect the graph
The project MUST provide a reproducible exporter that converts `bogota-graph.bin` to GeoJSON with one feature per edge and preserves its topological and routing attributes.

#### Scenario: QGIS export
- GIVEN a binary graph generated from the PBF
- WHEN `pnpm router:export-geojson` runs
- THEN `data/bogota-graph.geojson` is generated with `from`, `to`, `distance_meters`, `routing_cost`, `infrastructure`, `road_name`, `osm_way_id`, and `bicycle_access`.
