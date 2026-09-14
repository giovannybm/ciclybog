# Capacidad: geocodificación local de Bogotá

## Requisitos

### Requisito: índice local
La aplicación DEBE poder buscar direcciones y lugares de Bogotá sin depender de un servicio de geocodificación en tiempo de ejecución. El índice DEBE generarse desde el extracto OSM PBF con `pnpm geocoder:generate`, distribuirse como artefacto local versionado (`public/data/bogota-geocoder.json`, `version: 2`) e incluirse en el precache de la PWA.

El índice DEBE contener:
- vías con nombre y su geometría simplificada (tolerancia 2 m), agrupadas por nombre;
- lugares con nombre y etiqueta `amenity`, `shop`, `tourism`, `leisure`, `office`, `historic`, `public_transport`, `railway`, `healthcare`, `place` o `building`;
- direcciones `addr:street` + `addr:housenumber`.

NO DEBE contener textos sin letras ni coordenadas ficticias.

#### Escenario: índice disponible
- DADO que `bogota-geocoder.json` está publicado
- CUANDO el usuario enfoca un campo de búsqueda
- ENTONCES la aplicación carga el índice una sola vez y habilita la búsqueda.

#### Escenario: índice ausente o desactualizado
- DADO que el índice no está disponible o su versión no es 2
- CUANDO el usuario intenta buscar una dirección
- ENTONCES la interfaz informa que la búsqueda no está instalada y mantiene disponible la selección manual en el mapa.

### Requisito: normalización
El buscador DEBE ignorar diferencias de mayúsculas, acentos, signos (`#`, `-`, `.`, `No.`, `N°`) y separación entre números y letras (`172b` = `172 B`). DEBE reconocer abreviaturas comunes: `calle`/`cl`/`cll`/`ac`, `carrera`/`cra`/`cr`/`kr`/`k`/`ak`, `diagonal`/`dg`, `transversal`/`tv`/`tr` y `avenida`/`av`. Las palabras que coinciden con propiedades de objetos JavaScript (`constructor`, `toString`) NO DEBEN alterar la búsqueda.

### Requisito: nomenclatura bogotana
El buscador DEBE interpretar direcciones con la forma `<tipo> <número>[letra][bis [letra]][sur|este] [#] <número cruzado>[letra][bis [letra]] [placa] [sur|este]` y ubicarlas aunque OSM no tenga la dirección registrada:

1. Si existe una dirección OSM con la misma vía y número, DEBE devolverla primero con confianza alta.
2. Si la vía cruzada toca la vía principal a ≤ 20 m, el ancla DEBE ser ese cruce.
3. Si la vía cruzada no llega, DEBE prolongarse hasta 600 m, siempre que el punto resultante quede entre los cruces reales de número inmediatamente menor y mayor.
4. En otro caso DEBE interpolarse entre esos cruces, repartiendo por orden de letras y bis.
5. Desde el ancla DEBE avanzar la placa en metros (máximo 200) hacia los cruces de mayor número, continuando por los tramos contiguos de la misma vía.

Cada resultado DEBE indicar cómo se obtuvo (cruce, prolongación o interpolación).

#### Escenario: dirección sin registro en OSM
- DADO el índice real de Bogotá, donde la Calle 172 B no toca la Carrera 10 y la Calle 173 la cruza al sur de esa prolongación
- CUANDO el usuario busca `Cra. 10 172b 50`
- ENTONCES el primer resultado es `Carrera 10 # 172B-50`, interpolado entre la Calle 172 y la Calle 173, sobre la Carrera 10 y desplazado hacia el norte.

#### Escenario: cuadrantes
- DADO `Calle 26 Sur # 13-20` o `Carrera 10 # 17-20 Sur`
- CUANDO se interpreta la dirección
- ENTONCES `Sur` se aplica a la calle correspondiente y `Este` a la carrera correspondiente.

### Requisito: coincidencia tolerante
El buscador DEBE devolver resultados por prefijo de palabra sobre nombres de vías, lugares y direcciones, con coincidencia exacta para números. DEBE mostrar un máximo de ocho sugerencias, ordenadas por tipo de coincidencia y distancia al centro del mapa. Las búsquedas DEBEN aplicarse con retardo de escritura, y las respuestas de búsquedas anteriores DEBEN descartarse.

### Requisito: coordenadas utilizables para ruteo
Cada resultado DEBE devolver coordenadas WGS84, texto visible, detalle, tipo (`address`, `estimated`, `street`, `place`) y confianza. Al seleccionarlo, la aplicación DEBE validar que esté dentro de Bogotá, centrar el mapa y enviar la coordenada al snap sobre aristas del ruteador.

### Requisito: geocodificación inversa
El sistema DEBERÍA permitir convertir una coordenada seleccionada en el mapa a la dirección o lugar más cercano del índice local. Si no existe coincidencia, DEBE mostrar las coordenadas y permitir continuar con el ruteo.

### Requisito: cobertura y atribución
El índice DEBE conservar la fuente, el archivo PBF de origen y la atribución de OpenStreetMap. La ausencia de una dirección en OSM NO DEBE interpretarse como una dirección inválida.

### Requisito: privacidad y offline
La búsqueda DEBE ejecutarse localmente, sin enviar el texto ni las coordenadas del usuario a un tercero.
