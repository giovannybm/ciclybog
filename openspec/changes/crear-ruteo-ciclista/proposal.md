# Propuesta: app PWA de ruteo ciclista local

## Contexto

Se necesita una aplicación web instalable para dibujar rutas de bicicleta sobre un mapa, calcular el trazado en el dispositivo y conservar los datos del usuario sin depender de un backend.

## Solución propuesta

Crear una PWA en Vue 3 con MapLibre GL JS y un ruteador propio en Rust/WASM. La aplicación descargará un grafo binario ciclista versionado para Bogotá, lo guardará en IndexedDB, calculará rutas con A* dentro de un Web Worker y guardará las rutas segmentadas como GeoJSON con clasificación de infraestructura.

## Alcance inicial

- Mapa navegable con MapLibre/PMTiles y controles básicos.
- Selección de origen/destino, ruta principal y alternativa calculadas localmente por Rust/WASM.
- Diferenciación visual entre cicloruta y vía convencional.
- Listado básico de rutas guardadas.
- Instalabilidad PWA y shell cacheable.
- Mensajes claros cuando falta el grafo o no hay conectividad.

## Fuera de alcance inicial

- Sincronización entre dispositivos, cuentas o backend.
- Navegación giro a giro o garantía de seguridad vial.
- Procesamiento del PBF de OSM dentro de la app; la generación se realiza en el pipeline Rust.
- Geocodificación y búsqueda de direcciones; se implementará como el cambio independiente `agregar-geocodificador-local`.

## Riesgos y mitigaciones

- El resultado depende del grafo: versionar el área, fuente OSM, fecha y perfil ciclista junto al binario.
- La cobertura PMTiles y el grafo deben mantenerse sincronizados: versionar área, fecha, licencia y fuente OSM antes de producción.
- El binario puede ser grande: medir tamaño, carga progresiva y límites de área.
