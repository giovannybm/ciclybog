# Proposal: fix routing, migrate to pnpm, and complete the PWA

## Context

The September 13, 2026 review against Bogotá's real graph found data and algorithm errors that degraded route quality:

- 1,443 roads with only `cycleway:*=no` were classified as cycleways.
- 874 turn restrictions with `except=bicycle` (and car-specific `restriction:<vehicle>` values) were applied to bicycles.
- 186 bicycle-contraflow roads (`oneway:bicycle=no`, `cycleway=opposite*`) remained one-way.
- `access=no` discarded roads with `bicycle=yes`; `bicycle=dismount` was not handled.
- Bidirectional A* stopped on the sum of both queue priorities and could terminate before the optimum.
- Turn penalties and restrictions were evaluated on node states, losing valid arrivals.
- Alternatives blocked the entire primary route and rarely appeared; the UI could not select them.
- Snapping had no maximum distance; the main component was undirected.
- Every query recomputed components, scanned the whole graph for snapping, rebuilt adjacency lists, and cloned the graph.
- The `.bin` repeated strings and geometries and had one node per OSM node (63 MB).
- The IndexedDB graph key was fixed and `vue-tsc -b` emitted `.js` files next to sources.

The project also needed to use pnpm exclusively, provide an installable PWA, and let users use their location.

## Proposed solution

1. **Pipeline:** correct cycleway classification, bicycle-aware restriction filtering, contraflow, correct `access`/`bicycle` precedence, high-cost `dismount`, and a contracted graph split only at intersections, endpoints, and restriction nodes with shared string and geometry tables.
2. **Engine:** load-time indexes (largest strongly connected component, spatial grid, indexed restrictions), edge-state unidirectional A* with correct turn handling, ≤ 250 m snapping that splits an edge and its twin without cloning, and overlap-controlled penalty alternatives.
3. **Frontend:** alternative selector with cycleway percentage, content-derived graph key, removal of generated `.js` artifacts, and `noEmit`.
4. **Tooling:** pnpm as the only package manager (`packageManager`, lockfile, scripts, and documentation).
5. **PWA:** 192/512/maskable/Apple Touch icons, complete manifest, and **Install app** button.
6. **Location:** map geolocation control and buttons to use location as origin or destination.

## Out of scope

- Turn-by-turn navigation, geocoding, and synchronization.
- Turn restrictions with a way-based intermediate `via` member.
- Including `trunk`/`motorway` roads.
