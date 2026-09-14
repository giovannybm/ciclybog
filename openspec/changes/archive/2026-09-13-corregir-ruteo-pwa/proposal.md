# Propuesta: corregir el ruteo, migrar a pnpm y completar la PWA

## Contexto

La revisión del motor (13 de septiembre de 2026) contra el grafo real de Bogotá encontró errores de datos y de algoritmo que degradan la calidad de las rutas:

- 1.443 vías con solo `cycleway:*=no` se clasificaban como ciclorruta.
- 874 restricciones de giro con `except=bicycle` (y las `restriction:<vehículo>` de carros) se aplicaban a bicicletas.
- 186 vías con contraflujo ciclista (`oneway:bicycle=no`, `cycleway=opposite*`) quedaban de un solo sentido.
- `access=no` descartaba vías con `bicycle=yes`; `bicycle=dismount` no se trataba.
- La condición de parada del A* bidireccional sumaba las prioridades de ambas colas y podía cortar la búsqueda antes del óptimo.
- Las penalizaciones y restricciones de giro se evaluaban sobre estados por nodo, perdiendo llegadas válidas.
- Las alternativas bloqueaban toda la ruta principal y casi nunca aparecían; la UI tampoco permitía elegirlas.
- El snap no tenía distancia máxima; la componente principal era no dirigida.
- Cada consulta recalculaba componentes, recorría todo el grafo para el snap, reconstruía adyacencias y clonaba el grafo.
- El `.bin` repetía textos y geometrías y tenía un nodo por cada nodo OSM (63 MB).
- La clave de IndexedDB del grafo era fija y `vue-tsc -b` emitía `.js` junto a las fuentes.

Además se requiere usar exclusivamente pnpm, que la PWA sea instalable y que el usuario pueda usar su ubicación.

## Solución propuesta

1. **Pipeline:** clasificación ciclista corregida, restricciones filtradas para bicicleta, contraflujo, `access`/`bicycle` con precedencia correcta, `dismount` con costo alto, y grafo contraído (se divide cada vía solo en intersecciones, extremos y nodos de restricción) con tablas compartidas de textos y geometrías.
2. **Motor:** índice preparado al cargar (componente fuertemente conexa principal, grilla espacial, restricciones indexadas), A* unidireccional sobre estados por arista (correcto con giros), snap a ≤ 250 m que divide la arista y su gemela sin clonar el grafo, y alternativas por penalización con control de solapamiento.
3. **Frontend:** selector de alternativas con porcentaje en ciclorruta, clave del grafo derivada de su contenido, eliminación de artefactos `.js` y `noEmit`.
4. **Herramientas:** pnpm como único gestor (`packageManager`, lockfile, scripts y documentación).
5. **PWA:** íconos 192/512/maskable/apple-touch, manifest completo, botón “Instalar app”.
6. **Ubicación:** control de geolocalización en el mapa y botones para usar la ubicación como origen o destino.

## Fuera de alcance

- Navegación giro a giro, geocodificación y sincronización.
- Restricciones de giro con vía intermedia (`via` de tipo way).
- Inclusión de vías `trunk`/`motorway`.
