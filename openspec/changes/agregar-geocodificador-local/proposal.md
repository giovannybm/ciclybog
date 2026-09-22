# Proposal: local geocoder for Bogotá

## Objective

Allow users to enter an address or place and get coordinates for the start or end of a bike route, without a backend and with offline support.

## Source

The index is generated from `data/bogota.osm.pbf`. PMTiles remains visual-only and `bogota-graph.bin` remains the routing source.

## Scope

- Extract addresses, roads, and OSM-tagged places.
- Normalize text and build a compact local index.
- Search with exact and tolerant matching.
- Show suggestions in Vue.
- Pass the selected coordinate to the router snap.
- Add basic reverse geocoding.

## Out of scope

- Complete coverage of addresses missing from OSM.
- A remote Nominatim/Pelias service in production.
- Turn-by-turn navigation.
- Automatic correction of incomplete OSM data.
