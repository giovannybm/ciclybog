# Tareas

- [x] Definir el esquema versionado del índice geográfico (JSON v2: vías con geometría, lugares, direcciones).
- [x] Extraer `addr:*`, nombres de calles con geometría y lugares etiquetados desde el PBF, sin textos basura ni coordenadas ficticias.
- [x] Implementar normalización de acentos, abreviaturas y signos.
- [x] Interpretar nomenclatura bogotana (tipo, número, letra, bis, cuadrante, placa).
- [x] Ubicar direcciones por cruce, prolongación validada o interpolación entre cruces, y avanzar la placa.
- [x] Implementar búsqueda por prefijo de palabra y ranking por tipo y distancia al centro del mapa.
- [x] Crear `public/data/bogota-geocoder.json` desde `pnpm geocoder:generate` (6,4 MB; 1,8 MB gzip).
- [x] Agregar autocomplete “Origen” y “Destino” con retardo y descarte de respuestas obsoletas.
- [x] Conectar resultados con el snap y el ruteador Rust/WASM (validación de Bogotá y centrado del mapa).
- [x] Incluir el índice en el precache de la PWA.
- [x] Añadir pruebas de normalización, nomenclatura, interpolación, ausencia de resultados y datos reales (`pnpm test`).
- [ ] Cargar el índice en un Web Worker para no bloquear el hilo principal (hoy ~260 ms de preparación).
- [ ] Agregar geocodificación inversa básica.
- [ ] Medir tiempo de carga del índice en móvil.
