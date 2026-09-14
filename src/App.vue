<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from 'vue'
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
const searchCard = ref<HTMLElement | null>(null)
const sheet = ref<HTMLElement | null>(null)
const status = ref('Preparando tu mapa…')
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

// En móvil el mapa ocupa la pantalla: búsqueda flotante arriba y hoja deslizable abajo.
const mobileQuery = window.matchMedia('(max-width: 720px)')
const isMobile = ref(mobileQuery.matches)
const sheetExpanded = ref(false)
const searchOpen = ref(true)
const searchFocused = ref(false)
const searchOffset = ref(0)
const sheetOffset = ref(0)
const routeSummary = computed(() => `${originQuery.value || 'Origen'} → ${destinationQuery.value || 'Destino'}`)
let layoutObserver: ResizeObserver | undefined
let sheetDragStart: number | undefined
let suppressSheetClick = false
let map: Map | undefined
let router: RouterClient | undefined
let bogotaGeometry: GeoJSON.Geometry | undefined

const mapStyle = import.meta.env.VITE_MAP_STYLE_URL || 'https://demotiles.maplibre.org/style.json'
// Usa el extracto local por defecto; la variable solo permite reemplazarlo.
const pmtilesUrl = import.meta.env.VITE_PMTILES_URL || '/data/bogota.pmtiles'
const routeCollection: GeoJSON.FeatureCollection<GeoJSON.LineString, RouteSegmentProperties> = { type: 'FeatureCollection', features: [] }
const hasRoute = ref(false)

function handleViewportChange(event: MediaQueryListEvent) {
  isMobile.value = event.matches
  if (!event.matches) searchOpen.value = true
}

onMounted(async () => {
  mobileQuery.addEventListener('change', handleViewportChange)
  layoutObserver = new ResizeObserver(() => {
    searchOffset.value = searchCard.value?.offsetHeight ?? 0
    sheetOffset.value = sheet.value?.offsetHeight ?? 0
  })
  if (searchCard.value) layoutObserver.observe(searchCard.value)
  if (sheet.value) layoutObserver.observe(sheet.value)
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
  map.addControl(new maplibregl.NavigationControl({ showCompass: !isMobile.value }), 'top-right')
  map.addControl(new maplibregl.GeolocateControl({ positionOptions: { enableHighAccuracy: true }, trackUserLocation: true, showAccuracyCircle: true }), 'top-right')
  map.on('load', async () => {
    addMapLayers()
    map!.on('click', handleMapClick)
    try {
      router = new RouterClient()
      await router.ready()
      routerReady.value = true
      status.value = isMobile.value ? 'Busca un lugar o toca el mapa para elegir tu partida.' : 'Elige un punto de partida y uno de llegada para comenzar.'
      if (origin.value && destination.value) void calculateRoute()
    } catch (error) {
      status.value = 'No pudimos preparar el cálculo de rutas. Recarga la página e inténtalo de nuevo.'
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

onUnmounted(() => {
  mobileQuery.removeEventListener('change', handleViewportChange)
  layoutObserver?.disconnect()
  map?.remove()
  router?.destroy()
})

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

/** Margen visible del mapa: en móvil descuenta la búsqueda flotante y la hoja inferior. */
function mapPadding(): maplibregl.PaddingOptions {
  if (!isMobile.value || !mapElement.value) return { top: 60, bottom: 60, left: 60, right: 60 }
  const height = mapElement.value.clientHeight
  // Medidas de layout, no getBoundingClientRect: la hoja puede estar animando su transform.
  let top = (searchCard.value ? searchCard.value.offsetTop + searchCard.value.offsetHeight : 0) + 16
  // +36 px para que la atribución del mapa no tape el destino.
  let bottom = (sheet.value?.offsetHeight ?? 0) + 52
  // MapLibre ignora el encuadre si los márgenes superan el alto disponible.
  const scale = Math.min(1, (height * 0.75) / Math.max(top + bottom, 1))
  top *= scale
  bottom *= scale
  return { top, bottom, left: 24, right: 56 }
}

async function fitToCoordinates(coordinates: number[][]) {
  if (!map || !coordinates.length) return
  await nextTick()
  const first = coordinates[0] as [number, number]
  const bounds = coordinates.reduce((box, point) => box.extend(point as [number, number]), new maplibregl.LngLatBounds(first, first))
  map.fitBounds(bounds, { padding: mapPadding(), maxZoom: 17, duration: 700 })
}

async function focusPoint(point: Coordinate) {
  await nextTick()
  if (!map) return
  // `padding` en flyTo queda fijo en el mapa y desplazaría encuadres posteriores; `offset` no.
  const { top = 0, bottom = 0, left = 0, right = 0 } = mapPadding()
  map.flyTo({ center: point, zoom: Math.max(map.getZoom(), 16), offset: [(left - right) / 2, (top - bottom) / 2] })
}

function isSelectable(point: Coordinate) {
  return bogotaGeometry ? isInsideGeometry(point, bogotaGeometry) : isInsideBogota(point)
}

function handleMapClick(event: maplibregl.MapMouseEvent) {
  sheetExpanded.value = false
  const point: Coordinate = [event.lngLat.lng, event.lngLat.lat]
  if (!isSelectable(point)) { status.value = 'Selecciona un punto dentro de Bogotá.'; return }
  if (origin.value && destination.value) {
    destination.value = undefined
    destinationQuery.value = ''
    setPoint('origin', point, 'Punto en el mapa')
  } else {
    setPoint(origin.value ? 'destination' : 'origin', point, 'Punto en el mapa')
  }
}

function setPoint(target: PointTarget, point: Coordinate, label?: string) {
  if (target === 'origin') {
    origin.value = point
    if (label) originQuery.value = label
    originSuggestions.value = []
  } else {
    destination.value = point
    if (label) destinationQuery.value = label
    destinationSuggestions.value = []
  }
  selectedRoute.value = 0
  calculatedRoutes.value = []
  clearRoute()
  updatePointLayer()
  if (origin.value && destination.value) void calculateRoute()
  else status.value = origin.value ? 'Listo. Ahora elige tu destino.' : 'Elige primero tu punto de partida.'
}

function useMyLocation(target: PointTarget) {
  if (!('geolocation' in navigator)) { status.value = 'Tu navegador no permite compartir la ubicación.'; return }
  locating.value = target
  status.value = 'Buscando tu ubicación…'
  navigator.geolocation.getCurrentPosition(
    position => {
      locating.value = undefined
      const point: Coordinate = [position.coords.longitude, position.coords.latitude]
      if (!isSelectable(point)) { status.value = 'Tu ubicación está fuera de Bogotá. Elige un punto dentro de la ciudad.'; return }
      setPoint(target, point, 'Mi ubicación')
      if (!(origin.value && destination.value)) void focusPoint(point)
    },
    error => {
      locating.value = undefined
      status.value = describeGeolocationError(error)
    },
    { enableHighAccuracy: true, timeout: 15000, maximumAge: 30000 }
  )
}

function describeGeolocationError(error: GeolocationPositionError) {
  if (error.code === error.PERMISSION_DENIED) return 'No tenemos permiso para usar tu ubicación. Puedes elegir el punto directamente en el mapa.'
  if (error.code === error.TIMEOUT) return 'No encontramos tu ubicación a tiempo. Inténtalo de nuevo o elige el punto en el mapa.'
  return 'No pudimos determinar tu ubicación. Elige el punto directamente en el mapa.'
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
  status.value = 'Buscando la mejor ruta…'
  try {
    const calculated = await router.route(origin.value, destination.value, 2)
    if (!calculated.length) throw new Error('No encontramos una ruta en bicicleta entre esos puntos.')
    calculatedRoutes.value = calculated
    if (isMobile.value) {
      searchOpen.value = false
      sheetExpanded.value = false
    }
    selectRoute(0)
    status.value = calculated.length > 1 ? `Encontramos ${calculated.length} rutas. Elige la que prefieras.` : `Ruta calculada: ${formatDistance(calculated[0].distanceMeters)}.`
  } catch (error) {
    calculatedRoutes.value = []
    clearRoute()
    status.value = error instanceof Error ? error.message : 'No pudimos calcular la ruta. Prueba con otros puntos.'
  } finally { calculating.value = false }
}

function selectRoute(index: number) {
  selectedRoute.value = index
  const route = calculatedRoutes.value[index]
  if (!route) return
  renderRoute(route)
  void fitToCoordinates(route.segments.flatMap(segment => segment.geometry.coordinates))
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
  originQuery.value = ''
  destinationQuery.value = ''
  originSuggestions.value = []
  destinationSuggestions.value = []
  searchMessages.value = { origin: '', destination: '' }
  calculatedRoutes.value = []
  searchOpen.value = true
  clearRoute()
  updatePointLayer()
  status.value = 'Elige un punto de partida en el mapa o usa tu ubicación.'
}

function openSearch() {
  searchOpen.value = true
  sheetExpanded.value = false
}

function onSearchFocus() {
  searchFocused.value = true
  sheetExpanded.value = false
  loadGeocoder().catch(() => undefined)
}

function onSearchBlur() {
  searchFocused.value = false
}

function onSheetPointerDown(event: PointerEvent) {
  sheetDragStart = event.clientY
}

function onSheetPointerUp(event: PointerEvent) {
  if (sheetDragStart === undefined) return
  const deltaY = event.clientY - sheetDragStart
  sheetDragStart = undefined
  if (Math.abs(deltaY) < 24) return
  sheetExpanded.value = deltaY < 0
  suppressSheetClick = true
}

function toggleSheet() {
  if (suppressSheetClick) { suppressSheetClick = false; return }
  sheetExpanded.value = !sheetExpanded.value
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
    searchMessages.value[target] = results.length ? '' : 'No encontramos ese lugar. Revisa la dirección o selecciónalo en el mapa.'
  } catch (error) {
    if (sequence !== searchSequence[target]) return
    suggestions.value = []
    searchMessages.value[target] = 'No pudimos buscar ese lugar. Inténtalo de nuevo o selecciónalo en el mapa.'
  }
}

function selectPlace(target: PointTarget, place: GeocoderResult) {
  const point: Coordinate = [place.lon, place.lat]
  if (!isSelectable(point)) { searchMessages.value[target] = `${place.text} está fuera del Distrito Capital.`; return }
  searchSequence[target]++
  searchMessages.value[target] = place.kind === 'estimated' ? `Ubicación estimada: ${place.detail}.` : ''
  // Cierra el teclado en móvil para devolverle la pantalla al mapa.
  if (document.activeElement instanceof HTMLElement) document.activeElement.blur()
  setPoint(target, point, place.text)
  if (!(origin.value && destination.value)) void focusPoint(point)
}

function formatDistance(meters: number) { return meters >= 1000 ? `${(meters / 1000).toFixed(1)} km` : `${Math.round(meters)} m` }
function cyclewayShare(route: CalculatedRoute) { return Math.round(100 * route.cyclewayMeters / Math.max(route.distanceMeters, 1)) }

async function saveCurrentRoute() {
  if (!origin.value || !destination.value || !routeCollection.features.length) return
  const now = new Date().toISOString()
  const route: SavedRoute = { id: crypto.randomUUID(), name: `Ruta ${routes.value.length + 1}`, geometry: structuredClone(routeCollection), distanceMeters: routeCollection.features.reduce((total, feature) => total + feature.properties.distanceMeters, 0), alternative: selectedRoute.value > 0, createdAt: now, updatedAt: now }
  await saveRoute(route); routes.value = [route, ...routes.value]; status.value = 'Ruta guardada en este dispositivo.'
}

function selectSavedRoute(route: SavedRoute) {
  (map?.getSource('calculated-route') as GeoJSONSource | undefined)?.setData(route.geometry)
  status.value = `${route.name} cargada desde almacenamiento local.`
  sheetExpanded.value = false
  void fitToCoordinates(route.geometry.features.flatMap(feature => feature.geometry.coordinates))
}

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
  <main class="app-shell" :class="{ searching: isMobile && searchFocused, 'sheet-expanded': sheetExpanded }" :style="{ '--search-offset': `${searchOffset}px`, '--sheet-offset': `${sheetOffset}px` }">
    <header class="topbar">
      <div><p class="eyebrow">CICLYBOG</p><h1>Muévete en bici por Bogotá</h1></div>
      <div class="topbar-actions">
        <a class="github-link" href="https://github.com/giovannybm" target="_blank" rel="noreferrer" aria-label="Ver Ciclybog en GitHub" title="GitHub de Ciclybog">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 .5a12 12 0 0 0-3.79 23.39c.6.11.82-.26.82-.58v-2.03c-3.34.73-4.04-1.61-4.04-1.61-.55-1.39-1.34-1.76-1.34-1.76-1.09-.75.08-.74.08-.74 1.2.09 1.83 1.23 1.83 1.23 1.07 1.83 2.8 1.3 3.48.99.11-.77.42-1.3.76-1.6-2.67-.3-5.47-1.34-5.47-5.93 0-1.31.47-2.38 1.23-3.22-.12-.3-.53-1.52.12-3.17 0 0 1-.32 3.3 1.23a11.5 11.5 0 0 1 6 0c2.3-1.55 3.3-1.23 3.3-1.23.65 1.65.24 2.87.12 3.17.76.84 1.23 1.91 1.23 3.22 0 4.6-2.8 5.62-5.48 5.92.43.37.81 1.1.81 2.22v3.29c0 .32.22.69.83.57A12 12 0 0 0 12 .5Z" /></svg>
          <span>GitHub</span>
        </a>
        <button v-if="canInstall" type="button" class="install" @click="install">Instalar app</button>
        <span class="offline-badge">● Rutas disponibles sin conexión</span>
      </div>
    </header>
    <section class="workspace">
      <aside class="panel">
        <div ref="searchCard" class="search-card" :class="{ collapsed: isMobile && !searchOpen }">
          <button v-if="isMobile && !searchOpen" type="button" class="search-summary" aria-label="Editar origen y destino" @click="openSearch">
            <span class="route-dots" aria-hidden="true"><i class="dot origin"></i><i class="dot destination"></i></span>
            <span class="summary-text">{{ routeSummary }}</span>
            <span class="summary-edit">Editar</span>
          </button>
          <div v-else class="geocoder-fields">
            <div class="geocoder-field">
              <label for="origin-query">¿Desde dónde sales?</label>
              <div class="input-row">
                <i class="dot origin" aria-hidden="true"></i>
                <input id="origin-query" v-model="originQuery" type="search" autocomplete="off" enterkeyhint="search" placeholder="Desde: dirección o lugar" @focus="onSearchFocus" @blur="onSearchBlur" @input="searchPlace('origin')" @keydown.enter.prevent="originSuggestions[0] && selectPlace('origin', originSuggestions[0])" />
                <button type="button" class="locate-button" :class="{ busy: locating === 'origin' }" :disabled="Boolean(locating)" aria-label="Usar mi ubicación como origen" title="Usar mi ubicación" @click="useMyLocation('origin')">
                  <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="3.5" /><path d="M12 2v3M12 19v3M2 12h3M19 12h3" /><circle cx="12" cy="12" r="7" fill="none" /></svg>
                </button>
              </div>
              <ul v-if="originSuggestions.length" class="suggestions"><li v-for="place in originSuggestions" :key="`${place.text}-${place.lon}-${place.lat}`"><button type="button" @pointerdown.prevent @click="selectPlace('origin', place)"><span>{{ place.text }}</span><small>{{ place.detail }}</small></button></li></ul>
              <p v-else-if="searchMessages.origin" class="search-message">{{ searchMessages.origin }}</p>
            </div>
            <div class="geocoder-field">
              <label for="destination-query">¿A dónde vas?</label>
              <div class="input-row">
                <i class="dot destination" aria-hidden="true"></i>
                <input id="destination-query" v-model="destinationQuery" type="search" autocomplete="off" enterkeyhint="search" placeholder="Hasta: dirección o lugar" @focus="onSearchFocus" @blur="onSearchBlur" @input="searchPlace('destination')" @keydown.enter.prevent="destinationSuggestions[0] && selectPlace('destination', destinationSuggestions[0])" />
                <button type="button" class="locate-button" :class="{ busy: locating === 'destination' }" :disabled="Boolean(locating)" aria-label="Usar mi ubicación como destino" title="Usar mi ubicación" @click="useMyLocation('destination')">
                  <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="3.5" /><path d="M12 2v3M12 19v3M2 12h3M19 12h3" /><circle cx="12" cy="12" r="7" fill="none" /></svg>
                </button>
              </div>
              <ul v-if="destinationSuggestions.length" class="suggestions"><li v-for="place in destinationSuggestions" :key="`${place.text}-${place.lon}-${place.lat}`"><button type="button" @pointerdown.prevent @click="selectPlace('destination', place)"><span>{{ place.text }}</span><small>{{ place.detail }}</small></button></li></ul>
              <p v-else-if="searchMessages.destination" class="search-message">{{ searchMessages.destination }}</p>
            </div>
          </div>
        </div>

        <section ref="sheet" class="sheet" :class="{ expanded: sheetExpanded }" aria-label="Rutas y opciones">
          <button type="button" class="sheet-handle" :aria-expanded="sheetExpanded" :aria-label="sheetExpanded ? 'Contraer panel' : 'Ver más opciones'" @pointerdown="onSheetPointerDown" @pointerup="onSheetPointerUp" @click="toggleSheet"><span></span></button>
          <p class="status" :class="{ loading: calculating || locating }" role="status">{{ status }}</p>
          <ul v-if="calculatedRoutes.length" class="route-options">
            <li v-for="(route, index) in calculatedRoutes" :key="index">
              <button type="button" class="route-option" :class="{ selected: index === selectedRoute }" :aria-pressed="index === selectedRoute" @click="selectRoute(index)">
                <strong>{{ index === 0 ? 'Ruta principal' : `Alternativa ${index}` }}</strong>
                <span>{{ formatDistance(route.distanceMeters) }} · {{ cyclewayShare(route) }}% en cicloruta</span>
              </button>
            </li>
          </ul>
          <div v-if="!isMobile || origin || destination" class="actions">
            <button v-if="!hasRoute || !isMobile" type="button" :disabled="!origin || !destination || calculating || !routerReady" @click="calculateRoute">{{ calculating ? 'Buscando ruta…' : 'Encontrar ruta' }}</button>
            <button type="button" class="secondary" :disabled="!hasRoute" @click="saveCurrentRoute">Guardar</button>
            <button type="button" class="secondary" :disabled="!origin && !destination" @click="resetSelection">Limpiar</button>
          </div>
          <div class="sheet-details">
            <p v-if="showIosHint" class="install-hint">Para instalar en iPhone o iPad: toca <strong>Compartir</strong> y luego <strong>Agregar a inicio</strong>. <button type="button" class="link" @click="showIosHint = false">Cerrar</button></p>
            <button v-if="canInstall" type="button" class="install mobile-only" @click="install">Instalar app</button>
            <div class="legend"><span><i class="swatch cycleway"></i>Cicloruta</span><span><i class="swatch conventional"></i>Vía convencional</span><span><i class="swatch unknown"></i>Desconocida</span></div>
            <h2>Mis rutas guardadas</h2><p v-if="!routes.length" class="empty">Aquí aparecerán las rutas que guardes. Se almacenan solo en este dispositivo.</p><ul v-else class="route-list"><li v-for="route in routes" :key="route.id"><button class="route-button" type="button" @click="selectSavedRoute(route)">{{ route.name }} <small>{{ formatDistance(route.distanceMeters) }}</small></button></li></ul>
            <p class="hint">Las ciclorutas aparecen en azul. La ruta calculada muestra cada tramo según el tipo de vía.</p>
            <div class="sheet-links">
              <button type="button" class="link privacy-link" @click="managePrivacy">Configurar privacidad</button>
              <a class="link mobile-only" href="https://github.com/giovannybm" target="_blank" rel="noreferrer">GitHub</a>
            </div>
          </div>
        </section>
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
