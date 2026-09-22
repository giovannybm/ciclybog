# Delta: local cycling routing

## MODIFIED

### Requirement: client-side calculation
The application MUST run a weighted minimum-cost A* search over edge states in a Web Worker through WebAssembly and MUST NOT require a routing endpoint. The returned route MUST be optimal for the defined cost, including turn penalties and restrictions.

#### Scenario: optimality
- GIVEN a graph with turn penalties
- WHEN a route is calculated
- THEN its cost matches an exhaustive Dijkstra search over edge states.

### Requirement: avoid isolated components
The engine MUST calculate the largest strongly connected component once when loading the graph and snap origin and destination only to edges whose endpoints belong to it.

#### Scenario: one-way trap
- GIVEN a point near a road that cannot be exited while respecting directions
- WHEN snapping occurs
- THEN the engine chooses the nearest navigable road inside the main component.

### Requirement: snap to edges
The engine MUST project the origin and destination onto the nearest navigable edge within 250 m using a spatial index, and temporarily split that edge and its reverse twin without modifying or cloning the base graph.

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

### Requirement: OSM turn restrictions
The engine MUST prevent `no_*` transitions and allow only the road specified by `only_*`. The pipeline MUST ignore restrictions whose `except` includes `bicycle`, use `restriction:bicycle` when present, and ignore restrictions specific to other vehicles or conditionals.

#### Scenario: alternate arrival at a node
- GIVEN the cheapest arrival at a node forbids the required turn
- WHEN another arrival at the same node allows it
- THEN the engine finds the route using that other arrival.

### Requirement: prefer cycling infrastructure
The engine MUST weight each edge by infrastructure and road type, while reported distance MUST remain physical distance.

- A road MUST be classified as `cycleway` only when it is `highway=cycleway`, when `cycleway`, `cycleway:both`, `cycleway:left`, or `cycleway:right` is `track`, `lane`, `opposite_track`, or `opposite_lane`, or when it is a path or pedestrian road with explicit bicycle access.
- Pedestrian roads without explicit bicycle access MUST NOT enter the graph.
- `bicycle=yes|designated|permissive|official|dismount` MUST take precedence over `access=no` and `vehicle=no`, and `bicycle=dismount` MUST have a high cost.
- `oneway:bicycle=no` and `cycleway*=opposite*` MUST enable bicycle contraflow.
- The preferred route MUST NOT exceed 18% detour from the shortest route.

### Requirement: optimization-ready format
The binary file MUST retain an explicit version, CSR indexes, turn restrictions, shared string and geometry tables, and nodes only at decision points.

## ADDED

### Requirement: alternative routes
The engine MUST attempt to return alternatives no more than 1.4 times the primary route distance and sharing no more than 80% of its distance with an accepted route. The interface MUST allow choosing among them and show distance and cycleway percentage.

#### Scenario: select an alternative
- GIVEN the engine returns two routes
- WHEN the user chooses the second
- THEN the map displays it and **Save** persists it marked as an alternative.

### Requirement: cached graph invalidation
The IndexedDB graph key MUST derive from the published `.bin` content, and older versions MUST be removed when a new one is saved.

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
