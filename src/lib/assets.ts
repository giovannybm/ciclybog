/**
 * Resolves a `public/` asset against the deployment base.
 *
 * With the default base (`/`), the result is identical to the absolute path used
 * before, so root deployments are unchanged. Under a subpath
 * (`/demos/ciclybog/`, when embedding the app in another page), paths are rewritten
 * solas.
 *
 * All absolute project paths go through this function: keeping the risk in one
 * place makes it easy to audit.
 */
export function asset(path: string): string {
  return `${import.meta.env.BASE_URL}${path.replace(/^\//, '')}`
}

/**
 * Like `asset`, but honoring an environment variable when one is defined.
 *
 * Ojo: `.env` trae valores absolutos (`/data/bogota-graph.bin`) heredados de
 * when the app was only served at the root. A value starting with `/` is
 * interpreted relative to the base; an absolute URL (http…) is kept as-is.
 */
export function assetFromEnv(value: string | undefined, fallback: string): string {
  if (!value) return asset(fallback)
  if (/^[a-z]+:\/\//i.test(value)) return value
  return asset(value)
}
