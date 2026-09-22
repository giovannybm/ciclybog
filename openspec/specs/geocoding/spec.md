# Capability: local Bogotá geocoding

## Requirements

### Requirement: local index
The application MUST search Bogotá addresses and places without depending on a runtime geocoding service. The index MUST be generated from the OSM PBF extract with `pnpm geocoder:generate`, distributed as a versioned local artifact (`public/data/bogota-geocoder.json`, `version: 2`), and included in the PWA precache.

The index MUST contain:
- named roads and their simplified geometry (2 m tolerance), grouped by name;
- named places with an `amenity`, `shop`, `tourism`, `leisure`, `office`, `historic`, `public_transport`, `railway`, `healthcare`, `place`, or `building` tag;
- `addr:street` + `addr:housenumber` addresses.

It MUST NOT contain text without letters or fabricated coordinates.

#### Scenario: index available
- GIVEN `bogota-geocoder.json` is published
- WHEN the user focuses a search field
- THEN the application loads the index once and enables search.

#### Scenario: missing or outdated index
- GIVEN the index is unavailable or its version is not 2
- WHEN the user searches for an address
- THEN the interface reports that search is not installed and keeps manual map selection available.

### Requirement: normalization
The search MUST ignore differences in case, accents, punctuation (`#`, `-`, `.`, `No.`, `N°`), and spacing between numbers and letters (`172b` = `172 B`). It MUST recognize common abbreviations: `calle`/`cl`/`cll`/`ac`, `carrera`/`cra`/`cr`/`kr`/`k`/`ak`, `diagonal`/`dg`, `transversal`/`tv`/`tr`, and `avenida`/`av`. Words matching JavaScript object properties (`constructor`, `toString`) MUST NOT alter the search.

### Requirement: Bogotá address format
The search MUST parse addresses in the form `<type> <number>[letter][bis [letter]][sur|este] [#] <cross number>[letter][bis [letter]] [plate] [sur|este]` and locate them even when OSM has no registered address:

1. If an OSM address with the same road and number exists, it MUST be returned first with high confidence.
2. If the cross street touches the main road within 20 m, the anchor MUST be that intersection.
3. If the cross street does not reach it, it MUST be extended up to 600 m, as long as the resulting point falls between the immediately lower and higher real-numbered intersections.
4. Otherwise it MUST be interpolated between those intersections, distributing by letter and bis order.
5. From the anchor, the plate distance MUST be advanced in meters (maximum 200) toward higher-numbered intersections, continuing through adjacent segments of the same road.

Every result MUST explain how it was obtained (intersection, extension, or interpolation).

#### Scenario: address not registered in OSM
- GIVEN the real Bogotá index, where Calle 172 B does not reach Carrera 10 and Calle 173 crosses it south of that extension
- WHEN the user searches for `Cra. 10 172b 50`
- THEN the first result is `Carrera 10 # 172B-50`, interpolated between Calle 172 and Calle 173 on Carrera 10 and shifted north.

#### Scenario: quadrants
- GIVEN `Calle 26 Sur # 13-20` or `Carrera 10 # 17-20 Sur`
- WHEN the address is parsed
- THEN `Sur` applies to the corresponding calle and `Este` to the corresponding carrera.

### Requirement: tolerant matching
The search MUST return word-prefix results for road, place, and address names, with exact matching for numbers. It MUST show at most eight suggestions, ordered by match type and distance from the map center. Searches MUST be debounced, and responses from older searches MUST be discarded.

### Requirement: routable coordinates
Each result MUST return WGS84 coordinates, visible text, detail, type (`address`, `estimated`, `street`, `place`), and confidence. When selected, the application MUST validate that it is inside Bogotá, center the map, and send the coordinate to the router's edge snap.

### Requirement: reverse geocoding
The system SHOULD convert a coordinate selected on the map into the nearest address or place from the local index. If there is no match, it MUST show the coordinates and allow routing to continue.

### Requirement: coverage and attribution
The index MUST preserve its source, input PBF, and OpenStreetMap attribution. The absence of an address in OSM MUST NOT be interpreted as an invalid address.

### Requirement: privacy and offline use
Search MUST run locally without sending the user's text or coordinates to a third party.
