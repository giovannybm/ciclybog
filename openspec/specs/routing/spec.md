# Capacidad: ruteo ciclista local

## Requisitos

### Requisito: cargar un grafo local
La aplicación DEBE cargar un archivo de grafo Rust/WASM configurable antes de habilitar el ruteo, persistirlo en IndexedDB y mostrar un estado accionable si el archivo falta o falla. El grafo de Bogotá DEBE generarse directamente desde un extracto OSM PBF; PMTiles se reserva para visualización.

#### Escenario: grafo disponible
- DADO que el mapa está cargado y el grafo responde correctamente
- CUANDO se inicializa el Web Worker Rust/WASM
- ENTONCES el usuario puede seleccionar origen y destino para calcular una ruta.

#### Escenario: grafo ausente
- DADO que la URL del grafo no responde
- CUANDO finaliza la inicialización
- ENTONCES la aplicación informa que debe instalarse el grafo y mantiene el mapa navegable.

### Requisito: invalidación del grafo en caché
La clave del grafo en IndexedDB DEBE derivarse del contenido del `.bin` publicado, y las versiones anteriores DEBEN eliminarse al guardar una nueva.

### Requisito: calcular en el cliente
La aplicación DEBE ejecutar una búsqueda A* de costo mínimo ponderado sobre estados por arista en un Web Worker mediante WebAssembly y NO DEBE requerir un endpoint de ruteo. La ruta devuelta DEBE ser óptima para el costo definido, incluidas penalizaciones y restricciones de giro.

#### Escenario: solicitud al worker
- DADO que el usuario selecciona origen y destino
- CUANDO la aplicación envía la solicitud al worker
- ENTONCES debe transferir coordenadas planas serializables y recibir segmentos GeoJSON clasificados.

#### Escenario: optimalidad
- DADO un grafo con penalizaciones de giro
- CUANDO se calcula una ruta
- ENTONCES su costo coincide con el de una búsqueda exhaustiva sobre estados por arista.

#### Escenario: error del motor
- DADO que Rust no encuentra ruta, el grafo es inválido o falla WASM
- CUANDO el worker informa la excepción
- ENTONCES la interfaz debe mostrar el mensaje específico devuelto por el motor y no un error genérico.

### Requisito: limitar Bogotá
La aplicación DEBE impedir selección fuera del límite geográfico configurado para Bogotá D.C. y limitar el paneo a una ventana con margen alrededor del área de datos.

#### Escenario: punto fuera del área
- DADO que el usuario pulsa fuera del sobre configurado
- CUANDO se procesa el clic
- ENTONCES la aplicación no debe crear origen ni destino y debe informar que el punto está fuera de Bogotá.

### Requisito: ubicación del usuario
La aplicación DEBE permitir usar la ubicación actual del dispositivo como origen o destino, validando que esté dentro de Bogotá y mostrando errores de permiso, disponibilidad o tiempo de espera.

#### Escenario: ubicación como origen
- DADO que el usuario concede el permiso de ubicación y está en Bogotá
- CUANDO pulsa “Mi ubicación como origen”
- ENTONCES el origen se ubica en su posición y el mapa se centra en ella; si ya hay destino, se calcula la ruta.

#### Escenario: permiso denegado
- DADO que el usuario rechaza el permiso
- CUANDO pulsa el botón de ubicación
- ENTONCES la aplicación explica que debe habilitar la ubicación y mantiene la selección manual.

### Requisito: clasificar segmentos
El resultado DEBE devolver segmentos con una clasificación `cycleway`, `conventional` o `unknown` para que el mapa los pinte con estilos distintos. Los segmentos consecutivos con la misma infraestructura, nombre y acceso DEBEN fusionarse, y cada ruta DEBE informar sus metros en cicloruta.

### Requisito: preferir infraestructura ciclista
El motor DEBE ponderar el costo de cada arista según su infraestructura y tipo de vía, y la distancia reportada DEBE seguir siendo la física.

- Una vía solo DEBE clasificarse como `cycleway` si es `highway=cycleway`, si `cycleway`, `cycleway:both`, `cycleway:left` o `cycleway:right` valen `track`, `lane`, `opposite_track` u `opposite_lane`, o si es un sendero o vía peatonal con acceso ciclista explícito.
- Las vías peatonales sin acceso ciclista explícito NO DEBEN formar parte del grafo, y los senderos compartidos DEBEN costar más que una ciclovía segregada.
- `bicycle=yes|designated|permissive|official|dismount` DEBE prevalecer sobre `access=no` y `vehicle=no`, y `bicycle=dismount` DEBE tener un costo alto.
- `oneway:bicycle=no` y `cycleway*=opposite*` DEBEN habilitar el contraflujo ciclista.
- La ruta preferida NO DEBE superar el 18% de desvío respecto a la más corta; si lo supera, el motor DEBE devolver la ruta corta.

### Requisito: rutas alternativas
El motor DEBE intentar devolver alternativas que no superen 1,4 veces la distancia de la ruta principal ni compartan más del 80% de su distancia con una ruta ya aceptada. La interfaz DEBE permitir elegir entre ellas y mostrar distancia y porcentaje en cicloruta.

#### Escenario: selección de alternativa
- DADO que el motor devuelve dos rutas
- CUANDO el usuario elige la segunda
- ENTONCES el mapa la pinta y “Guardar” persiste esa ruta marcada como alternativa.

### Requisito: evitar componentes aislados
El motor DEBE calcular una única vez, al cargar el grafo, la componente fuertemente conexa más grande y ajustar origen y destino solo a aristas cuyos dos extremos pertenezcan a ella.

#### Escenario: trampa de sentido único
- DADO un punto cercano a una vía de la que no se puede salir respetando los sentidos
- CUANDO se hace snap
- ENTONCES el motor elige la vía navegable más cercana dentro de la componente principal.

### Requisito: snap sobre aristas
El motor DEBE proyectar origen y destino sobre la arista navegable más cercana dentro de 250 m, usando un índice espacial, y dividir temporalmente esa arista y su gemela inversa sin modificar ni clonar el grafo base.

#### Escenario: clic fuera de un nodo OSM
- DADO que el usuario selecciona un punto cercano al centro de una vía
- CUANDO se calcula la ruta
- ENTONCES el motor inicia o termina en la posición proyectada sobre la vía.

#### Escenario: clic lejos de la red
- DADO un punto a más de 250 m de cualquier vía navegable
- CUANDO se calcula la ruta
- ENTONCES el motor devuelve un error que indica que no hay vía apta cerca del origen o destino.

#### Escenario: vía de doble sentido
- DADO un punto sobre una vía de doble sentido
- CUANDO la ruta debe salir hacia cualquiera de los dos extremos
- ENTONCES el motor sale directamente en ese sentido sin recorrer la cuadra y regresar.

#### Escenario: origen y destino en la misma vía
- DADO origen y destino sobre la misma arista en el sentido permitido
- CUANDO se calcula la ruta
- ENTONCES la ruta es el tramo directo entre ambas proyecciones.

### Requisito: formato preparado para optimización
El archivo binario DEBE conservar una versión explícita, índices CSR de entrada y salida, restricciones de giro, tablas compartidas de textos y geometrías, y nodos solo en puntos de decisión, sin depender de PMTiles.

### Requisito: penalización de giros
El motor DEBE aplicar un costo adicional según el ángulo entre el último tramo de la arista entrante y el primero de la saliente, sin modificar la distancia física reportada.

### Requisito: restricciones de giro OSM
El motor DEBE impedir transiciones `no_*` y permitir únicamente la vía indicada por `only_*`; un `no_u_turn` sobre la misma vía solo DEBE impedir el retorno. El pipeline DEBE ignorar restricciones con `except` que incluya `bicycle`, usar `restriction:bicycle` cuando exista e ignorar las restricciones específicas de otros vehículos o condicionales.

#### Escenario: llegada alternativa a un nodo
- DADO que la llegada más barata a un nodo tiene prohibido el giro necesario
- CUANDO existe otra llegada al mismo nodo que sí lo permite
- ENTONCES el motor encuentra la ruta usando esa otra llegada.

### Requisito: persistir rutas
La aplicación DEBE guardar cada GeoJSON LineString creado en almacenamiento local y recuperarlo al abrirla de nuevo.

#### Escenario: nueva ruta
- DADO que Rust/WASM devuelve una ruta calculada
- CUANDO el usuario la guarda
- ENTONCES se guarda con un identificador y timestamps y se muestra en “Mis rutas”.

### Requisito: inspeccionar el grafo
El proyecto DEBE proporcionar un exportador reproducible que convierta `bogota-graph.bin` a GeoJSON con una Feature por arista y conserve sus atributos topológicos y de ruteo.

#### Escenario: exportación para QGIS
- DADO un grafo binario generado desde el PBF
- CUANDO se ejecuta `pnpm router:export-geojson`
- ENTONCES se genera `data/bogota-graph.geojson` con `from`, `to`, `distance_meters`, `routing_cost`, `infrastructure`, `road_name`, `osm_way_id` y `bicycle_access`.
