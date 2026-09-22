/**
 * Resuelve un recurso de `public/` contra la base de despliegue.
 *
 * Con la base por defecto (`/`) el resultado es idéntico a la ruta absoluta que
 * había antes, así que el despliegue en la raíz no cambia. Bajo un subpath
 * (`/demos/ciclybog/`, al embeber la app en otra página) las rutas se reescriben
 * solas.
 *
 * Todas las rutas absolutas del proyecto pasan por aquí: concentrar el riesgo en
 * una función lo hace auditable de un vistazo.
 */
export function asset(path: string): string {
  return `${import.meta.env.BASE_URL}${path.replace(/^\//, '')}`
}

/**
 * Igual que `asset`, pero respetando una variable de entorno si está definida.
 *
 * Ojo: `.env` trae valores absolutos (`/data/bogota-graph.bin`) heredados de
 * cuando la app solo se servía en la raíz. Un valor que empieza por `/` se
 * reinterpreta como relativo a la base; una URL absoluta (http…) se respeta tal cual.
 */
export function assetFromEnv(value: string | undefined, fallback: string): string {
  if (!value) return asset(fallback)
  if (/^[a-z]+:\/\//i.test(value)) return value
  return asset(value)
}
