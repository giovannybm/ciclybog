import { readFile, writeFile } from 'node:fs/promises'
import { FileSource, PMTiles } from 'pmtiles'
import { VectorTile } from '@mapbox/vector-tile'
import { PbfReader } from 'pbf'

const inputPath = process.argv[2] || 'public/data/bogota.pmtiles'
const outputPath = process.argv[3] || 'data/bogota-network.generated.geojson'
const zoom = Number(process.env.PMTILES_ZOOM || 15)
const bbox = (process.env.BOGOTA_BBOX || '-74.25,4.45,-73.95,4.90').split(',').map(Number)
const [west, south, east, north] = bbox
const tileCount = 2 ** zoom
const extent = 4096

const excluded = new Set(['steps', 'stairway', 'ladder', 'crossing', 'platform', 'corridor', 'bus_stop', 'taxiway', 'runway', 'rail', 'subway', 'funicular', 'cable_car', 'narrow_gauge', 'miniature', 'disused'])
const cycleways = new Set(['cycleway'])
const bicycleWays = new Set(['cycleway', 'path', 'track', 'footway', 'pedestrian', 'living_street', 'residential', 'service', 'unclassified', 'alley', 'tertiary', 'secondary', 'primary', 'tertiary_link', 'secondary_link', 'primary_link'])

function routingFactor(kind) {
  if (kind === 'cycleway') return 1.00
  if (['path', 'track', 'footway', 'pedestrian'].includes(kind)) return 1.04
  if (['living_street', 'residential', 'service', 'unclassified', 'alley'].includes(kind)) return 1.05
  if (kind === 'tertiary' || kind === 'tertiary_link') return 1.15
  if (kind === 'secondary' || kind === 'secondary_link') return 1.28
  if (kind === 'primary' || kind === 'primary_link') return 1.42
  return 1
}

const file = new File([await readFile(inputPath)], inputPath)
const archive = new PMTiles(new FileSource(file))
const rawSegments = []
const seen = new Set()

const lonToTile = lon => Math.floor((lon + 180) / 360 * tileCount)
const latToTile = lat => Math.floor((1 - Math.asinh(Math.tan(lat * Math.PI / 180)) / Math.PI) / 2 * tileCount)
const tileToLon = (x, localX) => ((x + localX / extent) / tileCount) * 360 - 180
const tileToLat = (y, localY) => 180 / Math.PI * Math.atan(Math.sinh(Math.PI * (1 - 2 * (y + localY / extent) / tileCount)))

for (let x = lonToTile(west); x <= lonToTile(east); x += 1) {
  for (let y = latToTile(north); y <= latToTile(south); y += 1) {
    const tile = await archive.getZxy(zoom, x, y)
    if (!tile) continue
    const vectorTile = new VectorTile(new PbfReader(new Uint8Array(tile.data)))
    const roads = vectorTile.layers.roads
    if (!roads) continue
    for (let index = 0; index < roads.length; index += 1) {
      const feature = roads.feature(index)
      const kind = feature.properties.kind_detail || feature.properties.kind
      if (!bicycleWays.has(kind) || excluded.has(kind)) continue
      const geometryParts = feature.loadGeometry()
      for (const part of geometryParts) {
        if (part.length < 2) continue
        const coordinates = part.map(point => [tileToLon(x, point.x), tileToLat(y, point.y)])
        for (let segmentIndex = 1; segmentIndex < coordinates.length; segmentIndex += 1) {
          const segment = [coordinates[segmentIndex - 1], coordinates[segmentIndex]]
          const oneway = feature.properties.oneway
          rawSegments.push({
            segment,
            kind,
            oneway,
            isBridge: Boolean(feature.properties.is_bridge),
            isTunnel: Boolean(feature.properties.is_tunnel),
            properties: {
              forward_cost: null,
              backward_cost: null,
              name: feature.properties.name || null,
              infrastructure: cycleways.has(kind) ? 'cycleway' : 'conventional',
              bicycle_access: cycleways.has(kind) ? 'designated' : 'yes',
              highway: kind
            }
          })
        }
      }
    }
  }
}

// Tiles may contain two roads that cross without sharing a vertex.
// Use a spatial grid to test only nearby segments and ignore bridges/tunnels
// so physically nonexistent connections are not created.
const intersectionCellSize = 0.0005
const grid = new Map()
const cuts = rawSegments.map(() => new Set([0, 1]))

function cellKey(x, y) { return `${x}:${y}` }
function segmentCells([[x1, y1], [x2, y2]]) {
  const minX = Math.floor(Math.min(x1, x2) / intersectionCellSize)
  const maxX = Math.floor(Math.max(x1, x2) / intersectionCellSize)
  const minY = Math.floor(Math.min(y1, y2) / intersectionCellSize)
  const maxY = Math.floor(Math.max(y1, y2) / intersectionCellSize)
  const cells = []
  for (let x = minX; x <= maxX; x += 1) for (let y = minY; y <= maxY; y += 1) cells.push(cellKey(x, y))
  return cells
}

function cross(ax, ay, bx, by) { return ax * by - ay * bx }
function intersectionParameters(first, second) {
  const [[x1, y1], [x2, y2]] = first
  const [[x3, y3], [x4, y4]] = second
  const rx = x2 - x1
  const ry = y2 - y1
  const sx = x4 - x3
  const sy = y4 - y3
  const denominator = cross(rx, ry, sx, sy)
  if (Math.abs(denominator) < 1e-12) return undefined
  const qx = x3 - x1
  const qy = y3 - y1
  const t = cross(qx, qy, sx, sy) / denominator
  const u = cross(qx, qy, rx, ry) / denominator
  if (t < 0 || t > 1 || u < 0 || u > 1) return undefined
  return [t, u]
}

for (let index = 0; index < rawSegments.length; index += 1) {
  for (const cell of segmentCells(rawSegments[index].segment)) {
    const candidates = grid.get(cell) || []
    for (const otherIndex of candidates) {
      const other = rawSegments[otherIndex]
      if (rawSegments[index].isBridge || rawSegments[index].isTunnel || other.isBridge || other.isTunnel) continue
      const parameters = intersectionParameters(rawSegments[index].segment, other.segment)
      if (!parameters) continue
      cuts[index].add(parameters[0])
      cuts[otherIndex].add(parameters[1])
    }
    candidates.push(index)
    grid.set(cell, candidates)
  }
}

const features = []
for (let index = 0; index < rawSegments.length; index += 1) {
  const raw = rawSegments[index]
  const [[startX, startY], [endX, endY]] = raw.segment
  const sortedCuts = [...cuts[index]].sort((a, b) => a - b)
  const factor = routingFactor(raw.kind)
  const isReverseOneway = raw.oneway === '-1' || raw.oneway === -1
  const isForwardOneway = raw.oneway === true || raw.oneway === 'yes' || raw.oneway === '1' || raw.oneway === 1
  for (let cutIndex = 1; cutIndex < sortedCuts.length; cutIndex += 1) {
    const fromT = sortedCuts[cutIndex - 1]
    const toT = sortedCuts[cutIndex]
    const segment = [[startX + (endX - startX) * fromT, startY + (endY - startY) * fromT], [startX + (endX - startX) * toT, startY + (endY - startY) * toT]]
    const key = JSON.stringify([segment, raw.kind, raw.oneway || 'no'])
    if (seen.has(key) || toT - fromT < 1e-9) continue
    seen.add(key)
    features.push({
      type: 'Feature',
      properties: { ...raw.properties, forward_cost: isReverseOneway ? null : factor, backward_cost: isForwardOneway ? null : factor },
      geometry: { type: 'LineString', coordinates: segment }
    })
  }
}

await writeFile(outputPath, JSON.stringify({ type: 'FeatureCollection', features }))
console.log(`Generated ${features.length} edges from ${inputPath}; processed intersections: ${rawSegments.length}`)
console.log(`Red normalizada guardada en ${outputPath}`)
