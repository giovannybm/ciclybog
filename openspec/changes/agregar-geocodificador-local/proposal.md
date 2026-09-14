# Propuesta: geocodificador local de Bogotá

## Objetivo

Permitir que el usuario escriba una dirección o lugar y obtenga coordenadas para iniciar o terminar una ruta ciclista, sin backend y con soporte offline.

## Fuente

El índice se generará desde `data/bogota.osm.pbf`. PMTiles seguirá siendo únicamente la fuente visual y `bogota-graph.bin` la fuente de ruteo.

## Alcance

- Extraer direcciones, calles y lugares con etiquetas OSM.
- Normalizar texto y construir un índice compacto local.
- Buscar con coincidencia exacta y tolerante.
- Mostrar sugerencias en Vue.
- Pasar la coordenada elegida al snap del ruteador.
- Agregar geocodificación inversa básica.

## Fuera de alcance

- Cobertura completa de direcciones no presentes en OSM.
- Servicio Nominatim/Pelias remoto en producción.
- Navegación giro a giro.
- Corrección automática de datos OSM incompletos.
