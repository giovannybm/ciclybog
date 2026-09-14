import { mkdir, writeFile } from 'node:fs/promises'

const url = 'https://nominatim.openstreetmap.org/search?format=jsonv2&polygon_geojson=1&limit=1&q=Bogot%C3%A1%2C%20Colombia'
const response = await fetch(url, { headers: { 'user-agent': 'ciclybog/0.1 boundary updater' } })
if (!response.ok) throw new Error(`No se pudo descargar el límite de Bogotá: ${response.status}`)
const [place] = await response.json()
if (!place?.geojson?.coordinates) throw new Error('La respuesta no contiene geometría GeoJSON')

const geometry = { type: place.geojson.type, coordinates: place.geojson.coordinates }
const world = [[-180, -85], [180, -85], [180, 85], [-180, 85], [-180, -85]]
const rings = geometry.type === 'Polygon'
  ? geometry.coordinates
  : geometry.coordinates.flat()
const mask = { type: 'Feature', properties: { source: 'OSM relation 7426387' }, geometry: { type: 'Polygon', coordinates: [world, ...rings] } }
const feature = { type: 'Feature', properties: { source: 'OpenStreetMap relation 7426387', osm_id: place.osm_id }, geometry }

await mkdir('public/data', { recursive: true })
await writeFile('public/data/bogota-boundary.geojson', JSON.stringify(feature))
await writeFile('public/data/bogota-mask.geojson', JSON.stringify(mask))
console.log(`Límite Bogotá guardado desde relación OSM ${place.osm_id}`)
