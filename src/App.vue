<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import maplibregl, { type Map, type GeoJSONSource } from 'maplibre-gl'
import * as pmtiles from 'pmtiles'
import * as basemaps from '@protomaps/basemaps'
import { listRoutes, saveRoute } from './lib/db'
import { BOGOTA_CENTER, isInsideBogota, MAP_VIEW_BOUNDS } from './lib/bogota-boundary'
import { installPrompt, isStandalone, needsIosInstructions, promptInstall } from './lib/install'
import { RouterClient } from './lib/router-client'
import { loadGeocoder, searchGeocoder, type GeocoderResult } from './lib/geocoder'
import type { CalculatedRoute, Coordinate, RouteSegmentProperties, SavedRoute } from './types'

type PointTarget = 'origin' | 'destination'

const mapElement = ref<HTMLElement | null>(null)
const status = ref('Cargando mapa y motor local…')
const routes = ref<SavedRoute[]>([])
const calculatedRoutes = ref<CalculatedRoute[]>([])
const selectedRoute = ref(0)
const origin = ref<Coordinate | undefined>()
const destination = ref<Coordinate | undefined>()
const calculating = ref(false)
const locating = ref<PointTarget | undefined>()
const routerReady = ref(false)
const showIosHint = ref(needsIosInstructions)
const originQuery = ref('')
const destinationQuery = ref('')
const originSuggestions = ref<GeocoderResult[]>([])
const destinationSuggestions = ref<GeocoderResult[]>([])
const searchMessages = ref<Record<PointTarget, string>>({ origin: '', destination: '' })
const searchTimers: Partial<Record<PointTarget, number>> = {}
const searchSequence: Record<PointTarget, number> = { origin: 0, destination: 0 }
const canInstall = computed(() => Boolean(installPrompt.value) && !isStandalone.value)
const consentKey = 'ciclybog-analytics-consent'
const showConsent = ref(false)
let map: Map | undefined
let router: RouterClient | undefined
let bogotaGeometry: GeoJSON.Geometry | undefined

const mapStyle = import.meta.env.VITE_MAP_STYLE_URL || 'https://demotiles.maplibre.org/style.json'
const pmtilesUrl = import.meta.env.VITE_PMTILES_URL
const routeCollection: GeoJSON.FeatureCollection<GeoJSON.LineString, RouteSegmentProperties> = { type: 'FeatureCollection', features: [] }
const hasRoute = ref(false)

onMounted(async () => {
  showConsent.value = !localStorage.getItem(consentKey)
  routes.value = await listRoutes()
  try {
    const boundaryResponse = await fetch('/data/bogota-boundary.geojson')
    if (boundaryResponse.ok) {
      const boundary = await boundaryResponse.json() as GeoJSON.Feature<GeoJSON.Geometry>
      bogotaGeometry = boundary.geometry
    }
  } catch {
    status.value = 'No se pudo cargar el límite oficial; se usará el límite rectangular.'
  }
  const style = createMapStyle()
  map = new maplibregl.Map({ container: mapElement.value!, style, center: BOGOTA_CENTER, zoom: 11, minZoom: 7, maxBounds: MAP_VIEW_BOUNDS })
  map.addControl(new maplibregl.NavigationControl(), 'top-right')
  map.addControl(new maplibregl.GeolocateControl({ positionOptions: { enableHighAccuracy: true }, trackUserLocation: true, showAccuracyCircle: true }), 'top-right')
  map.on('load', async () => {
    addMapLayers()
    map!.on('click', handleMapClick)
    try {
      router = new RouterClient()
      await router.ready()
      routerReady.value = true
      status.value = 'Motor Rust listo. Selecciona el origen en el mapa o usa tu ubicación.'
      if (origin.value && destination.value) void calculateRoute()
    } catch (error) {
      status.value = error instanceof Error ? error.message : 'No fue posible cargar el grafo de Bogotá.'
    }
  })
})

function createMapStyle(): maplibregl.StyleSpecification | string {
  if (!pmtilesUrl) return mapStyle
  const protocol = new pmtiles.Protocol()
  maplibregl.addProtocol('pmtiles', protocol.tile)
  protocol.add(new pmtiles.PMTiles(pmtilesUrl))
  return {
    version: 8,
    glyphs: 'https://protomaps.github.io/basemaps-assets/fonts/{fontstack}/{range}.pbf',
    sprite: 'https://protomaps.github.io/basemaps-assets/sprites/v4/light',
    sources: {
      protomaps: { type: 'vector', url: `pmtiles://${pmtilesUrl}`, attribution: '© OpenStreetMap contributors · Protomaps' }
    },
    layers: basemaps.layers('protomaps', basemaps.namedFlavor('light'), { lang: 'es' }).map(customizeBasemapLayer)
  }
}

type BasemapLayer = maplibregl.StyleSpecification['layers'][number]

function customizeBasemapLayer(layer: BasemapLayer): BasemapLayer {
  const customized = { ...layer } as BasemapLayer & { paint?: Record<string, unknown> }
  const paint: Record<string, unknown> = { ...(customized.paint as Record<string, unknown> | undefined) }

  if (customized.type === 'background') paint['background-color'] = '#ffffff'
  if (['landcover', 'landuse_park', 'landuse_urban_green'].includes(customized.id)) paint['fill-color'] = '#ddffc6'
  if (['water', 'water_stream', 'water_river'].includes(customized.id)) {
    paint[customized.type === 'fill' ? 'fill-color' : 'line-color'] = '#c6d9ff'
  }
  customized.paint = paint
  return customized
}

onUnmounted(() => { map?.remove(); router?.destroy() })

function addMapLayers() {
  map!.addSource('bogota-cycleways-osm', { type: 'geojson', data: '/data/bogota-cycleways.geojson' })
  map!.addLayer({
    id: 'bogota-cycleways-default',
    type: 'line',
    source: 'bogota-cycleways-osm',
    minzoom: 9,
    layout: { 'line-cap': 'round', 'line-join': 'round' },
    paint: {
      'line-color': '#3437eb',
      'line-opacity': 0.95,
      'line-width': ['interpolate', ['linear'], ['zoom'], 9, 0.8, 11, 1.4, 13, 2.2, 15, 4, 18, 8]
    }
  })
  map!.addSource('bogota-mask', { type: 'geojson', data: '/data/bogota-mask.geojson' })
  map!.addLayer({ id: 'bogota-mask', type: 'fill', source: 'bogota-mask', paint: { 'fill-color': '#ffffff', 'fill-opacity': 1 } })
  map!.addSource('bogota-boundary', { type: 'geojson', data: '/data/bogota-boundary.geojson' })
  map!.addLayer({ id: 'bogota-boundary', type: 'line', source: 'bogota-boundary', paint: { 'line-color': '#102a43', 'line-width': 2, 'line-dasharray': [2, 2] } })
  map!.addSource('calculated-route', { type: 'geojson', data: routeCollection })
  map!.addLayer({ id: 'route-segments-casing', type: 'line', source: 'calculated-route', layout: { 'line-cap': 'round', 'line-join': 'round' }, paint: { 'line-width': 9, 'line-opacity': 0.25, 'line-color': '#102a43' } })
  map!.addLayer({ id: 'route-segments', type: 'line', source: 'calculated-route', layout: { 'line-cap': 'round', 'line-join': 'round' }, paint: { 'line-width': 6, 'line-opacity': 0.95, 'line-color': ['match', ['get', 'infrastructure'], 'cycleway', '#17601a', 'conventional', '#f59e0b', '#64748b'] } })
  map!.addSource('route-points', { type: 'geojson', data: { type: 'FeatureCollection', features: [] } })
  map!.addLayer({ id: 'route-points', type: 'circle', source: 'route-points', paint: { 'circle-radius': 7, 'circle-color': ['match', ['get', 'kind'], 'origin', '#2563eb', '#dc2626'], 'circle-stroke-color': '#fff', 'circle-stroke-width': 2 } })
}

function isSelectable(point: Coordinate) {
  return bogotaGeometry ? isInsideGeometry(point, bogotaGeometry) : isInsideBogota(point)
}

function handleMapClick(event: maplibregl.MapMouseEvent) {
  const point: Coordinate = [event.lngLat.lng, event.lngLat.lat]
  if (!isSelectable(point)) { status.value = 'Selecciona un punto dentro del Distrito Capital.'; return }
  if (origin.value && destination.value) {
    destination.value = undefined
    setPoint('origin', point)
  } else {
    setPoint(origin.value ? 'destination' : 'origin', point)
  }
}

function setPoint(target: PointTarget, point: Coordinate) {
  if (target === 'origin') origin.value = point
  else destination.value = point
  selectedRoute.value = 0
  calculatedRoutes.value = []
  clearRoute()
  updatePointLayer()
  if (origin.value && destination.value) void calculateRoute()
  else status.value = origin.value ? 'Origen seleccionado. Ahora selecciona el destino.' : 'Destino seleccionado. Ahora selecciona el origen.'
}

function useMyLocation(target: PointTarget) {
  if (!('geolocation' in navigator)) { status.value = 'Este navegador no permite obtener tu ubicación.'; return }
  locating.value = target
  status.value = 'Obteniendo tu ubicación…'
  navigator.geolocation.getCurrentPosition(
    position => {
      locating.value = undefined
      const point: Coordinate = [position.coords.longitude, position.coords.latitude]
      if (!isSelectable(point)) { status.value = 'Tu ubicación está fuera del Distrito Capital; selecciona el punto en el mapa.'; return }
      map?.flyTo({ center: point, zoom: Math.max(map.getZoom(), 15) })
      setPoint(target, point)
    },
    error => {
      locating.value = undefined
      status.value = describeGeolocationError(error)
    },
    { enableHighAccuracy: true, timeout: 15000, maximumAge: 30000 }
  )
}

function describeGeolocationError(error: GeolocationPositionError) {
  if (error.code === error.PERMISSION_DENIED) return 'Permiso de ubicación denegado. Habilítalo en la configuración del navegador o selecciona el punto en el mapa.'
  if (error.code === error.TIMEOUT) return 'La ubicación tardó demasiado. Inténtalo de nuevo o selecciona el punto en el mapa.'
  return 'No fue posible determinar tu ubicación. Selecciona el punto en el mapa.'
}

function isInsideGeometry(point: Coordinate, geometry: GeoJSON.Geometry): boolean {
  if (geometry.type === 'Polygon') return isInsidePolygon(point, geometry.coordinates as number[][][])
  if (geometry.type === 'MultiPolygon') return (geometry.coordinates as number[][][][]).some(polygon => isInsidePolygon(point, polygon))
  return false
}

function isInsidePolygon(point: Coordinate, polygon: number[][][]): boolean {
  const insideRing = (ring: number[][]) => {
    let inside = false
    for (let index = 0, previous = ring.length - 1; index < ring.length; previous = index++) {
      const [xi, yi] = ring[index]
      const [xj, yj] = ring[previous]
      const intersects = yi > point[1] !== yj > point[1] && point[0] < (xj - xi) * (point[1] - yi) / (yj - yi) + xi
      if (intersects) inside = !inside
    }
    return inside
  }
  return insideRing(polygon[0]) && !polygon.slice(1).some(insideRing)
}

async function calculateRoute() {
  if (!router || !routerReady.value || !origin.value || !destination.value) return
  calculating.value = true
  status.value = 'Calculando ruta en Rust/WASM…'
  try {
    const calculated = await router.route(origin.value, destination.value, 2)
    if (!calculated.length) throw new Error('No existe una ruta ciclista entre esos puntos.')
    calculatedRoutes.value = calculated
    selectRoute(0)
    status.value = calculated.length > 1 ? `Se encontraron ${calculated.length} rutas. Elige una en la lista.` : `Ruta calculada: ${formatDistance(calculated[0].distanceMeters)}.`
  } catch (error) {
    calculatedRoutes.value = []
    clearRoute()
    status.value = error instanceof Error ? error.message : 'No se pudo calcular la ruta.'
  } finally { calculating.value = false }
}

function selectRoute(index: number) {
  selectedRoute.value = index
  const route = calculatedRoutes.value[index]
  if (route) renderRoute(route)
}

function renderRoute(route: CalculatedRoute) {
  routeCollection.features = route.segments.map(segment => ({ type: 'Feature', geometry: segment.geometry, properties: segment.properties }))
  hasRoute.value = routeCollection.features.length > 0
  ;(map!.getSource('calculated-route') as GeoJSONSource).setData(routeCollection)
}

function updatePointLayer() {
  const points: [PointTarget, Coordinate | undefined][] = [['origin', origin.value], ['destination', destination.value]]
  const features = points.filter(([, point]) => point).map(([kind, point]) => ({ type: 'Feature' as const, geometry: { type: 'Point' as const, coordinates: point! }, properties: { kind } }))
  ;(map?.getSource('route-points') as GeoJSONSource | undefined)?.setData({ type: 'FeatureCollection', features })
}

function clearRoute() { routeCollection.features = []; hasRoute.value = false; (map?.getSource('calculated-route') as GeoJSONSource | undefined)?.setData(routeCollection) }

function resetSelection() {
  origin.value = undefined
  destination.value = undefined
  calculatedRoutes.value = []
  clearRoute()
  updatePointLayer()
  status.value = 'Selecciona el origen en el mapa o usa tu ubicación.'
}

function warmGeocoder() {
  loadGeocoder().catch(() => undefined)
}

function searchPlace(target: PointTarget) {
  window.clearTimeout(searchTimers[target])
  searchTimers[target] = window.setTimeout(() => void runSearch(target), 180)
}

async function runSearch(target: PointTarget) {
  const query = target === 'origin' ? originQuery.value : destinationQuery.value
  const suggestions = target === 'origin' ? originSuggestions : destinationSuggestions
  const sequence = ++searchSequence[target]
  if (query.trim().length < 3) { suggestions.value = []; searchMessages.value[target] = ''; return }
  try {
    const center = map?.getCenter()
    const results = await searchGeocoder(query, { near: center ? [center.lng, center.lat] : undefined })
    // Ignora respuestas de búsquedas anteriores que terminan tarde.
    if (sequence !== searchSequence[target]) return
    suggestions.value = results
    searchMessages.value[target] = results.length ? '' : 'Sin coincidencias. Revisa la dirección (p. ej. Cra. 10 # 172B-50) o toca el mapa.'
  } catch (error) {
    if (sequence !== searchSequence[target]) return
    suggestions.value = []
    searchMessages.value[target] = error instanceof Error ? error.message : 'No fue posible buscar la dirección.'
  }
}

function selectPlace(target: PointTarget, place: GeocoderResult) {
  const point: Coordinate = [place.lon, place.lat]
  if (!isSelectable(point)) { searchMessages.value[target] = `${place.text} está fuera del Distrito Capital.`; return }
  searchSequence[target]++
  if (target === 'origin') { originQuery.value = place.text; originSuggestions.value = [] }
  else { destinationQuery.value = place.text; destinationSuggestions.value = [] }
  searchMessages.value[target] = place.kind === 'estimated' ? `Ubicación estimada: ${place.detail}.` : ''
  map?.flyTo({ center: point, zoom: Math.max(map.getZoom(), 16) })
  setPoint(target, point)
}

function formatDistance(meters: number) { return meters >= 1000 ? `${(meters / 1000).toFixed(1)} km` : `${Math.round(meters)} m` }
function cyclewayShare(route: CalculatedRoute) { return Math.round(100 * route.cyclewayMeters / Math.max(route.distanceMeters, 1)) }

async function saveCurrentRoute() {
  if (!origin.value || !destination.value || !routeCollection.features.length) return
  const now = new Date().toISOString()
  const route: SavedRoute = { id: crypto.randomUUID(), name: `Ruta ${routes.value.length + 1}`, geometry: structuredClone(routeCollection), distanceMeters: routeCollection.features.reduce((total, feature) => total + feature.properties.distanceMeters, 0), alternative: selectedRoute.value > 0, createdAt: now, updatedAt: now }
  await saveRoute(route); routes.value = [route, ...routes.value]; status.value = 'Ruta guardada en este dispositivo.'
}

function selectSavedRoute(route: SavedRoute) { (map?.getSource('calculated-route') as GeoJSONSource | undefined)?.setData(route.geometry); status.value = `${route.name} cargada desde almacenamiento local.` }

async function install() {
  if (await promptInstall()) status.value = 'Ciclybog quedó instalada en este dispositivo.'
}

function setAnalyticsConsent(consent: 'accepted' | 'rejected') {
  localStorage.setItem(consentKey, consent)
  showConsent.value = false
  if (consent === 'accepted') window.dispatchEvent(new Event('ciclybog:clarity-consent'))
}

function managePrivacy() {
  showConsent.value = true
}
</script>

<template>
  <main class="app-shell">
    <header class="topbar">
      <div><p class="eyebrow">CICLYBOG · RUTEO LOCAL</p><h1>Rutas de bicicleta en Bogotá</h1></div>
      <div class="topbar-actions">
        <button v-if="canInstall" type="button" class="install" @click="install">Instalar app</button>
        <span class="offline-badge">● Sin backend de ruteo</span>
      </div>
    </header>
    <section class="workspace">
      <aside class="panel">
        <p v-if="showIosHint" class="install-hint">Para instalar en iPhone o iPad: toca <strong>Compartir</strong> y luego <strong>Agregar a inicio</strong>. <button type="button" class="link" @click="showIosHint = false">Cerrar</button></p>
        <p class="status" :class="{ loading: calculating || locating }" role="status">{{ status }}</p>
        <div class="geocoder-fields">
          <div class="geocoder-field">
            <label for="origin-query">Origen</label>
            <input id="origin-query" v-model="originQuery" type="search" autocomplete="off" placeholder="Ej.: Cra. 10 # 172B-50 o un lugar" @focus="warmGeocoder" @input="searchPlace('origin')" @keydown.enter.prevent="originSuggestions[0] && selectPlace('origin', originSuggestions[0])" />
            <ul v-if="originSuggestions.length" class="suggestions"><li v-for="place in originSuggestions" :key="`${place.text}-${place.lon}-${place.lat}`"><button type="button" @click="selectPlace('origin', place)"><span>{{ place.text }}</span><small>{{ place.detail }}</small></button></li></ul>
            <p v-else-if="searchMessages.origin" class="search-message">{{ searchMessages.origin }}</p>
          </div>
          <div class="geocoder-field">
            <label for="destination-query">Destino</label>
            <input id="destination-query" v-model="destinationQuery" type="search" autocomplete="off" placeholder="Ej.: Calle 26 # 68-50 o un lugar" @focus="warmGeocoder" @input="searchPlace('destination')" @keydown.enter.prevent="destinationSuggestions[0] && selectPlace('destination', destinationSuggestions[0])" />
            <ul v-if="destinationSuggestions.length" class="suggestions"><li v-for="place in destinationSuggestions" :key="`${place.text}-${place.lon}-${place.lat}`"><button type="button" @click="selectPlace('destination', place)"><span>{{ place.text }}</span><small>{{ place.detail }}</small></button></li></ul>
            <p v-else-if="searchMessages.destination" class="search-message">{{ searchMessages.destination }}</p>
          </div>
        </div>
        <div class="location-actions">
          <button type="button" class="secondary" :disabled="Boolean(locating)" @click="useMyLocation('origin')">{{ locating === 'origin' ? 'Ubicando…' : '📍 Mi ubicación como origen' }}</button>
          <button type="button" class="secondary" :disabled="Boolean(locating)" @click="useMyLocation('destination')">{{ locating === 'destination' ? 'Ubicando…' : '🏁 Mi ubicación como destino' }}</button>
        </div>
        <div class="actions">
          <button type="button" :disabled="!origin || !destination || calculating || !routerReady" @click="calculateRoute">{{ calculating ? 'Calculando…' : 'Calcular ruta' }}</button>
          <button type="button" class="secondary" :disabled="!hasRoute" @click="saveCurrentRoute">Guardar</button>
          <button type="button" class="secondary" :disabled="!origin && !destination" @click="resetSelection">Limpiar</button>
        </div>
        <ul v-if="calculatedRoutes.length" class="route-options">
          <li v-for="(route, index) in calculatedRoutes" :key="index">
            <button type="button" class="route-option" :class="{ selected: index === selectedRoute }" :aria-pressed="index === selectedRoute" @click="selectRoute(index)">
              <strong>{{ index === 0 ? 'Ruta principal' : `Alternativa ${index}` }}</strong>
              <span>{{ formatDistance(route.distanceMeters) }} · {{ cyclewayShare(route) }}% en cicloruta</span>
            </button>
          </li>
        </ul>
        <div class="legend"><span><i class="swatch cycleway"></i>Cicloruta</span><span><i class="swatch conventional"></i>Vía convencional</span><span><i class="swatch unknown"></i>Desconocida</span></div>
        <h2>Mis rutas</h2><p v-if="!routes.length" class="empty">Tus rutas se almacenan localmente.</p><ul v-else class="route-list"><li v-for="route in routes" :key="route.id"><button class="route-button" type="button" @click="selectSavedRoute(route)">{{ route.name }} <small>{{ formatDistance(route.distanceMeters) }}</small></button></li></ul>
        <p class="hint">El motor usa un grafo OSM de Bogotá descargado en la primera carga. El color indica la infraestructura registrada en cada tramo.</p>
        <button type="button" class="link privacy-link" @click="managePrivacy">Configurar privacidad</button>
      </aside>
      <div ref="mapElement" class="map" aria-label="Mapa de rutas ciclistas"></div>
    </section>
    <aside v-if="showConsent" class="consent-banner" role="dialog" aria-label="Consentimiento de analítica">
      <div>
        <strong>Privacidad y analítica</strong>
        <p>Usamos Microsoft Clarity de forma opcional para entender cómo se usa Ciclybog y mejorar la aplicación. Puedes aceptar o rechazar; el ruteo funciona igual.</p>
      </div>
      <div class="consent-actions">
        <button type="button" class="secondary" @click="setAnalyticsConsent('rejected')">Rechazar</button>
        <button type="button" @click="setAnalyticsConsent('accepted')">Aceptar</button>
      </div>
    </aside>
  </main>
</template>
