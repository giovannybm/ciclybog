# Capacidad: mapa local de Bogotá

## Requisitos

### Requisito: fuente PMTiles
La aplicación DEBE poder cargar el mapa vectorial local desde `VITE_PMTILES_URL` mediante el protocolo `pmtiles://` de MapLibre.

### Requisito: capa ciclista OSM
La aplicación DEBE mostrar por defecto la capa `public/data/bogota-cycleways.geojson`, generada desde el mismo extracto OSM PBF que alimenta el grafo, con color `#3437eb` y visibilidad desde zoom 9.

### Requisito: máscara geográfica
La aplicación DEBE mostrar el mapa coloreado únicamente dentro del polígono administrativo local de Bogotá (`bogota-boundary.geojson`) y cubrir con blanco la información exterior mediante `bogota-mask.geojson`.

#### Escenario: exterior del área
- DADO que el viewport incluye el margen alrededor del área de datos
- CUANDO se renderiza el mapa
- ENTONCES el exterior del límite configurado aparece blanco y sin etiquetas ni estilos de las tiles.

### Requisito: estilo de infraestructura
La aplicación DEBE diferenciar visualmente los segmentos de ruta según `infrastructure`.

#### Escenario: colores de mapa y ruta
- DADO un mapa PMTiles cargado y una ruta calculada
- CUANDO se renderizan sus capas
- ENTONCES las zonas verdes usan `#ddffc6`, el agua usa `#c6d9ff`, las ciclovías del mapa base usan `#3437eb`, las ciclovías de una ruta calculada usan `#17601a` y las vías convencionales conservan un color distinto.

### Requisito: navegación acotada
La aplicación DEBE permitir un margen de paneo alrededor del área de Bogotá sin permitir que el usuario navegue indefinidamente fuera de la zona configurada.

> El paneo usa una ventana amplia rectangular, pero la máscara y la selección usan el polígono administrativo local descargado desde OpenStreetMap.
