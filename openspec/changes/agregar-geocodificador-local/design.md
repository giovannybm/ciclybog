# Diseño: geocodificador local

## Pipeline

```text
data/bogota.osm.pbf
        ├── bogota-graph.bin
        ├── bogota-cycleways.geojson
        └── bogota-geocoder.bin
```

El pipeline Rust leerá nodos, vías y etiquetas OSM. Para cada entidad útil generará un registro con texto normalizado, texto visible, coordenadas, tipo y prioridad. Se incluirán principalmente `addr:*`, `place`, `amenity`, `shop`, `tourism`, `leisure`, `name` y `highway` con nombre.

## Índice

La primera versión puede usar un índice binario de términos a IDs y una lista compacta de registros. La búsqueda en el Web Worker combinará:

1. normalización Unicode y abreviaturas;
2. coincidencia de prefijo;
3. coincidencia por tokens;
4. distancia de Levenshtein limitada para errores pequeños;
5. distancia geográfica al centro visible del mapa.

El índice se cacheará en IndexedDB con una clave que incluya la versión del PBF. El contenido debe poder regenerarse con un comando reproducible.

## Integración con la interfaz

Los campos “Desde” y “Hasta” tendrán autocomplete local. Al elegir un resultado, la interfaz actualizará el marcador y conservará la selección manual en el mapa. La coordenada se entregará al ruteador, que hará snap a la arista navegable más cercana.

## Integración con el PWA

`bogota-geocoder.bin` se incluirá en el precache cuando su tamaño sea aceptable. Si el usuario no lo tiene cacheado, la aplicación mostrará estado de instalación y no bloqueará el mapa ni la selección manual.
