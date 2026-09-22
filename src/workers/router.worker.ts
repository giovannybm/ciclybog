import type { CalculatedRoute, Coordinate, Infrastructure } from '../types'

type Message = { id: number; type: 'load'; bytes: ArrayBuffer; baseUrl?: string } | { id: number; type: 'route'; origin: Coordinate; destination: Coordinate; alternatives: number }
type RawSegment = { geometry: { lon: number; lat: number }[]; distance_meters: number; infrastructure: Infrastructure; road_name?: string; bicycle_access?: string }
type WasmRouter = { route: (originLon: number, originLat: number, destinationLon: number, destinationLat: number, alternatives: number) => RawRoute[] }
type RawRoute = { distance_meters: number; cycleway_meters: number; segments: RawSegment[] }
let router: WasmRouter | undefined

self.onmessage = async ({ data }: MessageEvent<Message>) => {
  try {
    if (data.type === 'load') { router = await loadWasmRouter(data.bytes, data.baseUrl); self.postMessage({ id: data.id, ok: true, routes: [] }); return }
    if (!router) throw new Error('El motor Rust todavía no está cargado')
    const routes = router.route(data.origin[0], data.origin[1], data.destination[0], data.destination[1], data.alternatives).map(toCalculatedRoute)
    self.postMessage({ id: data.id, ok: true, routes })
  } catch (error) {
    self.postMessage({ id: data.id, ok: false, error: describeWorkerError(error) })
  }
}

function describeWorkerError(error: unknown): string {
  if (error instanceof Error) return error.message
  if (typeof error === 'string') return error
  if (error && typeof error === 'object') {
    const message = (error as { message?: unknown }).message
    if (typeof message === 'string') return message
    try { return JSON.stringify(error) } catch { return String(error) }
  }
  return String(error)
}

function toCalculatedRoute(route: RawRoute): CalculatedRoute {
  return { distanceMeters: route.distance_meters, cyclewayMeters: route.cycleway_meters, segments: route.segments.map(segment => ({ type: 'Feature' as const, geometry: { type: 'LineString', coordinates: segment.geometry.map(point => [point.lon, point.lat]) }, properties: { infrastructure: segment.infrastructure, roadName: segment.road_name, distanceMeters: segment.distance_meters, bicycleAccess: segment.bicycle_access } })) }
}

async function loadWasmRouter(bytes: ArrayBuffer, baseUrl = '/'): Promise<WasmRouter> {
  // La base la manda el hilo principal: resolver contra self.location.origin
  // ignoraría el subpath y rompería la app al servirla fuera de la raíz.
  const base = new URL(baseUrl, self.location.origin)
  const moduleUrl = new URL('wasm/ciclybog_router_wasm.js', base).href
  const module = await import(/* @vite-ignore */ moduleUrl) as { default: (input?: unknown) => Promise<void>; Router: new (bytes: Uint8Array) => WasmRouter }
  await module.default(new URL('wasm/ciclybog_router_wasm_bg.wasm', base).href)
  return new module.Router(new Uint8Array(bytes))
}
