# Datos del grafo

`public/data/bogota-graph.bin` es un artefacto generado desde un extracto OSM PBF de Bogotá con `pnpm router:osm-data`. PMTiles se usa únicamente para visualización.

## Formato (v3, magic `CICLYG02`, versión `osm-pbf-v4-contracted`)

- **Nodos:** solo puntos de decisión: intersecciones entre vías aceptadas, extremos de vía y nodos vía de restricciones de giro.
- **Aristas:** dirigidas, con `distance_meters` y `routing_cost` (`f32`), referencia a una geometría compartida (`geometry` + `reversed`), infraestructura, índices a la tabla `strings` para nombre y acceso, y `osm_way_id`.
- **Índices y restricciones:** adyacencias CSR de entrada y salida, y restricciones de giro con `via_node` como índice interno.

El mismo comando genera `public/data/bogota-cycleways.geojson`, que alimenta las líneas azules del mapa, así la capa y el grafo usan la misma versión de OSM.

## Reglas de clasificación

- **`cycleway`:**
  - `highway=cycleway`.
  - `cycleway`, `cycleway:both`, `cycleway:left` o `cycleway:right` con `track`, `lane`, `opposite_track` u `opposite_lane`.
  - `path`/`track`/`footway`/`pedestrian` con `bicycle=yes|designated|permissive|official`.
- **No son cicloruta:** `cycleway:*=no`, `separate` (la cicloruta está mapeada como vía aparte), `shared_lane` y claves de detalle como `cycleway:left:width`.
- **Excluidas:** `footway`/`pedestrian` sin acceso ciclista explícito; `bicycle=no|private`; `access` o `vehicle` `no|private` sin permiso ciclista explícito.
- **Desmonte:** `bicycle=dismount` es transitable con factor 3.0.
- **Contraflujo:** `oneway:bicycle=no` o `cycleway*=opposite*` hacen la vía de doble sentido para bicicleta.
- **Restricciones:** se descartan las que tienen `except` con `bicycle`, las solo específicas de otros vehículos y las de vía intermedia tipo way.

### Factores de costo

| Vía | Factor |
| --- | --- |
| `cycleway` | 1.00 |
| `path`/`track` ciclista | 1.08 |
| Calles locales | 1.05 |
| `tertiary` | 1.15 |
| `footway` ciclista | 1.18 |
| `track` compartido | 1.22 |
| `secondary` | 1.28 |
| `path` compartido | 1.30 |
| `primary` | 1.42 |
| `dismount` | 3.00 |

## Resultado de la última generación (13 de septiembre de 2026)

- 73.479 vías aceptadas.
- 107.831 nodos (104.563 en la componente fuertemente conexa principal).
- 246.985 aristas, de las cuales 13.911 son cicloruta.
- 142.474 geometrías y 2.038 restricciones aplicadas.
- 23,0 MB.

El extracto reproducible por defecto es `https://download.bbbike.org/osm/bbbike/Bogota/Bogota.osm.pbf`, actualizado por el proveedor el 5 de septiembre de 2026. Los datos de OpenStreetMap se distribuyen bajo ODbL; conserva la atribución de OSM/BBBike al publicar la aplicación.

Para inspeccionar en QGIS las aristas exactas del grafo: `pnpm router:export-geojson` y abre `data/bogota-graph.geojson`. Sus atributos son `from`, `to`, `distance_meters`, `routing_cost`, `infrastructure`, `road_name`, `osm_way_id` y `bicycle_access`.

El archivo PMTiles y el PBF deben tener cobertura y fecha compatibles. El fixture de muestra (`pnpm router:sample-data`) solo sirve para validar la aplicación localmente.
