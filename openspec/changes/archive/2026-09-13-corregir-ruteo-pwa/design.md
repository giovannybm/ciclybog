# Diseño técnico

## Formato del grafo (v3, magic `CICLYG02`)

- `nodes`: solo intersecciones, extremos de vía, nodos con restricción de giro o cortes por nodos ausentes.
- `geometries`: polilíneas compartidas. Cada arista referencia `geometry` y `reversed`, de modo que las dos direcciones de una vía de doble sentido comparten coordenadas.
- `strings`: tabla de textos; `road_name` y `bicycle_access` son índices.
- `distance_meters` y `routing_cost` en `f32`; CSR de salida y entrada; `turn_restrictions` con `via_node` como índice interno.

## Pipeline

| Regla | Comportamiento |
| --- | --- |
| Ciclorruta | `highway=cycleway`; `cycleway`, `cycleway:both`, `cycleway:left` o `cycleway:right` con `track`, `lane`, `opposite_track` u `opposite_lane`; `path`/`track`/`footway`/`pedestrian` con acceso ciclista explícito. `shared_lane`, `share_busway` y `separate` no convierten la calzada en ciclorruta. |
| Acceso | `bicycle=yes/designated/permissive/official/dismount` prevalece sobre `access`/`vehicle=no/private`. `bicycle=no/private` excluye. |
| Desmonte | `bicycle=dismount` entra con factor 3.0, clasificación convencional. |
| Sentido | `oneway:bicycle=no` o cualquier `cycleway*=opposite*` produce doble sentido; `oneway:bicycle=yes` fuerza sentido único; `junction=roundabout` implica sentido único. |
| Restricciones | Se ignoran si `except` contiene `bicycle`. Se usa `restriction:bicycle` si existe; si no, `restriction`. Las `restriction:<otro vehículo>` y condicionales se ignoran. |

## Motor

`PreparedGraph` se construye una vez al cargar:

1. **Componente principal:** componente fuertemente conexa más grande (Kosaraju iterativo). Solo se hace snap sobre aristas con ambos extremos en ella, lo que garantiza alcanzabilidad dirigida.
2. **Grilla espacial:** celdas de 0,001° con los índices de aristas cuyo rectángulo las toca. El snap revisa las celdas dentro de 250 m.
3. **Restricciones:** mapa `(via_node, from_way) → [(to_way, only)]`.

### Consulta

- **Snap:** se proyecta el punto sobre la arista más cercana (≤ 250 m) y se divide esa arista y su gemela inversa en un overlay de nodos y aristas virtuales. Si origen y destino caen en la misma geometría y en orden compatible, se agrega una arista directa. El grafo base no se modifica ni se clona.
- **Búsqueda:** A* sobre estados por arista. El costo de una transición es `distancia × factor + penalización(giro)`, y el giro se mide con los rumbos locales del último tramo de la arista entrante y el primero de la saliente. La heurística es la distancia haversine al destino, admisible y consistente porque `factor ≥ 1` y las penalizaciones son ≥ 0. La búsqueda termina al extraer la primera arista que llega al destino.
- **Principal:** se calculan la ruta ponderada y la ruta más corta (factor 1); la ponderada se usa si no supera el 18% de desvío.
- **Alternativas:** se multiplica el costo de las aristas de las rutas ya aceptadas (×1,6 y luego ×2,5) y se incluye la ruta más corta como candidata. Se acepta una candidata si su distancia ≤ 1,4 × la principal y comparte ≤ 80% de su distancia con cada ruta aceptada.
- **Respuesta:** cada ruta incluye `distance_meters`, `cycleway_meters` y segmentos consecutivos fusionados por infraestructura, nombre y acceso.

## Frontend y PWA

- La clave de IndexedDB es `bogota-graph-<sha1 del .bin>`, inyectada por Vite en build; al guardar se eliminan claves anteriores.
- Se muestra una lista de rutas calculadas (principal y alternativas); la seleccionada se pinta y se puede guardar.
- La ubicación usa `navigator.geolocation` (alta precisión, timeout de 15 s), valida que el punto esté en Bogotá y lo asigna como origen o destino; `GeolocateControl` muestra la posición en el mapa.
- La instalación usa íconos generados con `@vite-pwa/assets-generator` desde `public/favicon.svg`, `id`, `scope`, `lang`, `orientation` y `categories` en el manifest, y un botón basado en `beforeinstallprompt`, con instrucciones para iOS.
