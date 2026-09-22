import type { Coordinate } from '../types'
import { asset } from './assets'

// Local projection in meters for Bogotá (latitude ~4.65°).
const METERS_PER_DEGREE_LON = 111_320 * Math.cos(4.65 * Math.PI / 180)
const METERS_PER_DEGREE_LAT = 110_574
/** Maximum distance for extending a cross street that does not reach the main street. */
const MAX_EXTENSION_METERS = 600
/** Maximum distance between two intersections used for interpolation. */
const MAX_INTERPOLATION_SPAN_METERS = 2000
const INTERPOLATION_WINDOW = 8
/** An intersection within this distance is considered real rather than extended. */
const RELIABLE_GAP_METERS = 20
const MAX_PLATE_METERS = 200
const DEFAULT_NEAR: Coordinate = [-74.0721, 4.711]

export type StreetType = 'calle' | 'carrera' | 'diagonal' | 'transversal'
export type Quadrant = '' | 'sur' | 'este'

export interface StreetNumber { number: number; letter: string; bis: boolean; bisLetter: string }
export interface StreetRef extends StreetNumber { type: StreetType; quadrant: Quadrant }
export interface ParsedAddress { street: StreetRef; cross?: StreetRef; plate?: number }

export interface GeocoderResult {
  text: string
  detail: string
  lon: number
  lat: number
  kind: 'address' | 'estimated' | 'street' | 'place'
  confidence: 'high' | 'medium' | 'low'
}

export interface RawGeocoderIndex {
  version: number
  source?: string
  streets: { name: string; lines: number[][] }[]
  places: [string, string, number, number][]
  addresses: [string, string, number, number][]
}

type Point = { x: number; y: number }
type Line = Point[]

interface Street { name: string; ref?: StreetRef; lines: Line[]; bbox: [number, number, number, number] }
interface Entry { text: string; detail: string; words: string[]; point: Point; kind: GeocoderResult['kind']; street?: Street }
interface Address { ref?: StreetRef; number: string; point: Point; text: string }

export interface GeocoderIndex {
  byKey: Map<string, Street[]>
  byFamily: Map<string, Street[]>
  entries: Entry[]
  addresses: Address[]
}

// Objetos sin prototipo: palabras como "constructor" no deben resolver a propiedades heredadas.
const TYPE_ALIASES: Record<string, StreetType> = Object.assign(Object.create(null), {
  calle: 'calle', cl: 'calle', cll: 'calle', clle: 'calle', call: 'calle', ac: 'calle',
  carrera: 'carrera', cra: 'carrera', cr: 'carrera', kra: 'carrera', kr: 'carrera', k: 'carrera', ak: 'carrera', carr: 'carrera',
  diagonal: 'diagonal', dg: 'diagonal', diag: 'diagonal',
  transversal: 'transversal', tv: 'transversal', tr: 'transversal', trans: 'transversal', transv: 'transversal', trv: 'transversal'
})
const WORD_ALIASES: Record<string, string> = Object.assign(Object.create(null), TYPE_ALIASES, { av: 'avenida', avda: 'avenida', cc: 'centro comercial', univ: 'universidad', hosp: 'hospital', pque: 'parque', oriente: 'este' })
const KIND_LABELS: Record<string, string> = Object.assign(Object.create(null), { amenity: 'Place', shop: 'Shop', tourism: 'Tourism', leisure: 'Leisure', office: 'Office', historic: 'Historic site', public_transport: 'Public transport', railway: 'Railway', healthcare: 'Healthcare', place: 'Area', building: 'Building' })
const AVENUE_WORDS = new Set(['avenida', 'av', 'avda'])
const NUMBER_SEPARATORS = new Set(['no', 'nro', 'num', 'numero', 'n'])

export function normalizeText(value: string): string {
  return value
    .toLowerCase()
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .replace(/n[°º]/g, ' ')
    .replace(/[^a-z0-9]+/g, ' ')
    .replace(/(\d)([a-z])/g, '$1 $2')
    .replace(/([a-z])(\d)/g, '$1 $2')
    .replace(/\s+/g, ' ')
    .trim()
}

function canonicalWords(value: string): string[] {
  return normalizeText(value).split(' ').filter(Boolean).flatMap(word => (WORD_ALIASES[word] ?? word).split(' '))
}

function readNumber(tokens: string[], start: number): (StreetNumber & { next: number }) | undefined {
  if (!/^\d+$/.test(tokens[start] ?? '')) return undefined
  let next = start + 1
  let letter = ''
  let bis = false
  let bisLetter = ''
  if (/^[a-h]$/.test(tokens[next] ?? '')) letter = tokens[next++]
  if (tokens[next] === 'bis') {
    bis = true
    next++
    if (/^[a-h]$/.test(tokens[next] ?? '')) bisLetter = tokens[next++]
  }
  return { number: Number(tokens[start]), letter, bis, bisLetter, next }
}

function readQuadrant(tokens: string[], index: number): [Quadrant, number] {
  if (tokens[index] === 'sur') return ['sur', index + 1]
  if (tokens[index] === 'este' || tokens[index] === 'oriente') return ['este', index + 1]
  return ['', index]
}

function family(type: StreetType): 'calle' | 'carrera' {
  return type === 'calle' || type === 'diagonal' ? 'calle' : 'carrera'
}

/** Valid quadrant for a street family: calles can be "sur" and carreras "este". */
function quadrantFor(type: StreetType, quadrant: Quadrant): Quadrant {
  if (quadrant === 'sur') return family(type) === 'calle' ? 'sur' : ''
  if (quadrant === 'este') return family(type) === 'carrera' ? 'este' : ''
  return ''
}

/** Parses Bogotá addresses: "Cra. 10 172b 50", "Calle 26 Sur # 13-20", "KR 10 No. 172 B - 50". */
export function parseAddress(query: string): ParsedAddress | undefined {
  const tokens = normalizeText(query).split(' ').filter(Boolean)
  let index = AVENUE_WORDS.has(tokens[0]) && TYPE_ALIASES[tokens[1]] ? 1 : 0
  const type = TYPE_ALIASES[tokens[index]]
  if (!type) return undefined
  const main = readNumber(tokens, index + 1)
  if (!main) return undefined
  const [afterMain, afterMainIndex] = readQuadrant(tokens, main.next)
  index = afterMainIndex
  while (NUMBER_SEPARATORS.has(tokens[index])) index++
  const crossType: StreetType = family(type) === 'calle' ? 'carrera' : 'calle'
  const street: StreetRef = { type, number: main.number, letter: main.letter, bis: main.bis, bisLetter: main.bisLetter, quadrant: quadrantFor(type, afterMain) }
  const cross = readNumber(tokens, index)
  if (!cross) return { street }
  index = cross.next
  let plate: number | undefined
  if (/^\d+$/.test(tokens[index] ?? '')) plate = Number(tokens[index++])
  const [trailing] = readQuadrant(tokens, index)
  // A quadrant that does not apply to the main street (e.g. "Carrera 10 Sur") belongs to the cross street.
  const crossQuadrant = quadrantFor(crossType, trailing || (street.quadrant ? '' : afterMain))
  return { street, cross: { type: crossType, number: cross.number, letter: cross.letter, bis: cross.bis, bisLetter: cross.bisLetter, quadrant: crossQuadrant }, plate }
}

/** Parses OSM street names: "Avenida Carrera 10", "Calle 172 B", "Carrera 10 Bis A Este". */
export function parseStreetName(name: string): StreetRef | undefined {
  const tokens = normalizeText(name).split(' ').filter(Boolean)
  const index = AVENUE_WORDS.has(tokens[0]) && TYPE_ALIASES[tokens[1]] ? 1 : 0
  const type = TYPE_ALIASES[tokens[index]]
  if (!type || tokens[index] === 'k') return undefined
  const number = readNumber(tokens, index + 1)
  if (!number || /^\d+$/.test(tokens[number.next] ?? '')) return undefined
  const rest = tokens.slice(number.next)
  const quadrant: Quadrant = rest.includes('sur') ? 'sur' : rest.includes('este') || rest.includes('oriente') ? 'este' : ''
  return { type, number: number.number, letter: number.letter, bis: number.bis, bisLetter: number.bisLetter, quadrant }
}

function refKey(ref: StreetRef): string {
  return [ref.type, ref.number, ref.letter, ref.bis ? 'bis' : '', ref.bisLetter, ref.quadrant].join('|')
}

function letterValue(letter: string): number {
  return letter ? letter.charCodeAt(0) - 96 : 0
}

function numberValue(ref: StreetNumber): number {
  return ref.number + letterValue(ref.letter) * 0.1 + (ref.bis ? 0.05 : 0) + letterValue(ref.bisLetter) * 0.01
}

function formatNumber(ref: StreetNumber): string {
  return `${ref.number}${ref.letter.toUpperCase()}${ref.bis ? ' Bis' : ''}${ref.bisLetter ? ` ${ref.bisLetter.toUpperCase()}` : ''}`
}

function formatQuadrant(quadrant: Quadrant): string {
  return quadrant ? ` ${quadrant === 'sur' ? 'Sur' : 'Este'}` : ''
}

export function formatAddress(address: ParsedAddress): string {
  const type = address.street.type[0].toUpperCase() + address.street.type.slice(1)
  const main = `${type} ${formatNumber(address.street)}${formatQuadrant(address.street.quadrant)}`
  if (!address.cross) return main
  return `${main} # ${formatNumber(address.cross)}${address.plate === undefined ? '' : `-${address.plate}`}${formatQuadrant(address.cross.quadrant)}`
}

const toPoint = (lon: number, lat: number): Point => ({ x: lon * METERS_PER_DEGREE_LON, y: lat * METERS_PER_DEGREE_LAT })
const toCoordinate = (point: Point): Coordinate => [point.x / METERS_PER_DEGREE_LON, point.y / METERS_PER_DEGREE_LAT]
const distance = (a: Point, b: Point) => Math.hypot(a.x - b.x, a.y - b.y)
const cross2d = (a: Point, b: Point) => a.x * b.y - a.y * b.x
const subtract = (a: Point, b: Point): Point => ({ x: a.x - b.x, y: a.y - b.y })
const lerp = (a: Point, b: Point, t: number): Point => ({ x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t })

export function buildGeocoderIndex(raw: RawGeocoderIndex): GeocoderIndex {
  if (raw.version !== 2) throw new Error('The address index is outdated; run pnpm geocoder:generate.')
  const byKey = new Map<string, Street[]>()
  const byFamily = new Map<string, Street[]>()
  const entries: Entry[] = []
  for (const { name, lines: rawLines } of raw.streets) {
    const lines = rawLines
      .map(flat => Array.from({ length: Math.floor(flat.length / 2) }, (_, index) => toPoint(flat[index * 2], flat[index * 2 + 1])))
      .filter(line => line.length >= 2)
    if (!lines.length) continue
    const points = lines.flat()
    const bbox: Street['bbox'] = [Math.min(...points.map(p => p.x)), Math.min(...points.map(p => p.y)), Math.max(...points.map(p => p.x)), Math.max(...points.map(p => p.y))]
    const street: Street = { name, ref: parseStreetName(name), lines, bbox }
    if (street.ref) {
      const key = refKey(street.ref)
      byKey.set(key, [...(byKey.get(key) ?? []), street])
      const familyKey = `${family(street.ref.type)}|${street.ref.quadrant}`
      byFamily.set(familyKey, [...(byFamily.get(familyKey) ?? []), street])
    }
    const longest = lines.reduce((best, line) => line.length > best.length ? line : best)
    entries.push({ text: name, detail: 'Street', words: canonicalWords(name), point: longest[Math.floor(longest.length / 2)], kind: 'street', street })
  }
  for (const [name, kind, lon, lat] of raw.places) {
    entries.push({ text: name, detail: KIND_LABELS[kind.split('=')[0]] ?? 'Place', words: canonicalWords(name), point: toPoint(lon, lat), kind: 'place' })
  }
  const addresses: Address[] = raw.addresses.map(([street, number, lon, lat]) => ({ ref: parseStreetName(street), number: normalizeText(number), point: toPoint(lon, lat), text: `${street} # ${number}` }))
  for (const address of addresses) entries.push({ text: address.text, detail: 'Address registered in OSM', words: canonicalWords(address.text), point: address.point, kind: 'address' })
  return { byKey, byFamily, entries, addresses }
}

function nearestOnStreet(street: Street, point: Point) {
  let best = { point: street.lines[0][0], line: 0, segment: 0, distance: Infinity }
  street.lines.forEach((line, lineIndex) => {
    for (let segment = 0; segment < line.length - 1; segment++) {
      const a = line[segment]
      const ab = subtract(line[segment + 1], a)
      const lengthSquared = ab.x ** 2 + ab.y ** 2
      const t = lengthSquared ? Math.min(1, Math.max(0, ((point.x - a.x) * ab.x + (point.y - a.y) * ab.y) / lengthSquared)) : 0
      const projected = lerp(a, line[segment + 1], t)
      const gap = distance(projected, point)
      if (gap < best.distance) best = { point: projected, line: lineIndex, segment, distance: gap }
    }
  })
  return best
}

/** Intersection between the main and cross streets; extends the cross street endpoints if needed. */
function intersect(main: Street, cross: Street): { point: Point; gap: number } | undefined {
  const [minX, minY, maxX, maxY] = main.bbox
  const [crossMinX, crossMinY, crossMaxX, crossMaxY] = cross.bbox
  if (crossMaxX < minX - MAX_EXTENSION_METERS || crossMinX > maxX + MAX_EXTENSION_METERS || crossMaxY < minY - MAX_EXTENSION_METERS || crossMinY > maxY + MAX_EXTENSION_METERS) return undefined
  let best: { point: Point; gap: number } | undefined
  const consider = (origin: Point, direction: Point, maxT: number, gapScale: number) => {
    for (const line of main.lines) {
      for (let index = 0; index < line.length - 1; index++) {
        const segment = subtract(line[index + 1], line[index])
        const denominator = cross2d(direction, segment)
        if (Math.abs(denominator) < 1e-9) continue
        const offset = subtract(line[index], origin)
        const t = cross2d(offset, segment) / denominator
        const u = cross2d(offset, direction) / denominator
        if (t < 0 || t > maxT || u < 0 || u > 1) continue
        const gap = t * gapScale
        if (!best || gap < best.gap) best = { point: lerp(line[index], line[index + 1], u), gap }
      }
    }
  }
  for (const line of cross.lines) {
    for (let index = 0; index < line.length - 1; index++) consider(line[index], subtract(line[index + 1], line[index]), 1, 0)
  }
  if (best) return best
  for (const line of cross.lines) {
    const ends: [Point, Point][] = [[line[0], line[1]], [line[line.length - 1], line[line.length - 2]]]
    for (const [end, previous] of ends) {
      const direction = subtract(end, previous)
      const length = Math.hypot(direction.x, direction.y)
      if (length < 1) continue
      consider(end, { x: direction.x / length, y: direction.y / length }, MAX_EXTENSION_METERS, 1)
    }
  }
  return best
}

const dot = (a: Point, b: Point) => a.x * b.x + a.y * b.y
const unit = (vector: Point): Point => {
  const length = Math.hypot(vector.x, vector.y) || 1
  return { x: vector.x / length, y: vector.y / length }
}

/** Walks along a street; OSM splits it into segments, so continue through adjacent segments. */
function walkAlong(street: Street, start: Point, direction: Point, meters: number): Point {
  const nearest = nearestOnStreet(street, start)
  let line = street.lines[nearest.line]
  let forward = dot(subtract(line[nearest.segment + 1], line[nearest.segment]), direction) >= 0
  let index = forward ? nearest.segment + 1 : nearest.segment
  let current = nearest.point
  let remaining = meters
  const visited = new Set([nearest.line])
  while (remaining > 0) {
    if (index < 0 || index >= line.length) {
      const next = street.lines.findIndex((candidate, candidateIndex) => !visited.has(candidateIndex)
        && (distance(candidate[0], current) < 5 || distance(candidate[candidate.length - 1], current) < 5))
      if (next < 0) break
      visited.add(next)
      line = street.lines[next]
      forward = distance(line[0], current) < 5
      index = forward ? 1 : line.length - 2
      continue
    }
    const step = distance(current, line[index])
    if (step >= remaining) return lerp(current, line[index], remaining / step)
    remaining -= step
    current = line[index]
    index += forward ? 1 : -1
  }
  return current
}

interface CrossHit { value: number; name: string; exactType: boolean; point: Point; gap: number }

/** Real intersections immediately below and above the requested number, as close as possible. */
function bracket(hits: CrossHit[], target: number): [CrossHit, CrossHit] | undefined {
  const below = hits.filter(hit => hit.value < target)
  const above = hits.filter(hit => hit.value > target)
  if (!below.length || !above.length) return undefined
  const lowValue = Math.max(...below.map(hit => hit.value))
  const highValue = Math.min(...above.map(hit => hit.value))
  let pair: [CrossHit, CrossHit] | undefined
  for (const low of below.filter(hit => hit.value === lowValue)) {
    for (const high of above.filter(hit => hit.value === highValue)) {
      const span = distance(low.point, high.point)
      if (span <= MAX_INTERPOLATION_SPAN_METERS && (!pair || span < distance(pair[0].point, pair[1].point))) pair = [low, high]
    }
  }
  return pair
}

function isBetween(point: Point, [low, high]: [CrossHit, CrossHit]): boolean {
  const axis = subtract(high.point, low.point)
  const lengthSquared = dot(axis, axis)
  const t = lengthSquared ? dot(subtract(point, low.point), axis) / lengthSquared : -1
  return t >= 0 && t <= 1
}

/** Relative position by letter order: between 172 and 173, a known 172A puts 172B at 2/3. */
function rankFraction(hits: CrossHit[], low: number, high: number, target: number): number {
  const values = [...new Set([...hits.map(hit => hit.value).filter(value => value > low && value < high), target])].sort((a, b) => a - b)
  return (values.indexOf(target) + 1) / (values.length + 1)
}

function trendDirection(street: Street, hits: CrossHit[], anchor: Point, target: number): Point | undefined {
  const nearest = nearestOnStreet(street, anchor)
  const line = street.lines[nearest.line]
  const tangent = unit(subtract(line[nearest.segment + 1], line[nearest.segment]))
  const slope = hits
    .filter(hit => distance(hit.point, anchor) < 1500)
    .reduce((sum, hit) => sum + (hit.value - target) * dot(subtract(hit.point, anchor), tangent), 0)
  if (Math.abs(slope) < 1e-6) return undefined
  return slope > 0 ? tangent : { x: -tangent.x, y: -tangent.y }
}

function geocodeNomenclature(index: GeocoderIndex, address: ParsedAddress): GeocoderResult | undefined {
  const cross = address.cross!
  const mains = index.byKey.get(refKey(address.street)) ?? []
  if (!mains.length) return undefined
  // "Carrera 10" and "Avenida Carrera 10" are the same corridor.
  const corridor: Street = {
    name: formatAddress({ street: address.street }),
    lines: mains.flatMap(main => main.lines),
    bbox: [Math.min(...mains.map(main => main.bbox[0])), Math.min(...mains.map(main => main.bbox[1])), Math.max(...mains.map(main => main.bbox[2])), Math.max(...mains.map(main => main.bbox[3]))]
  }
  const target = numberValue(cross)
  const hits: CrossHit[] = []
  for (const street of index.byFamily.get(`${family(cross.type)}|${cross.quadrant}`) ?? []) {
    const value = numberValue(street.ref!)
    if (Math.abs(value - target) > INTERPOLATION_WINDOW) continue
    const hit = intersect(corridor, street)
    if (hit) hits.push({ value, name: street.name, exactType: street.ref!.type === cross.type, point: hit.point, gap: hit.gap })
  }

  const reliable = hits.filter(hit => hit.gap <= RELIABLE_GAP_METERS)
  const bounds = bracket(reliable, target) ?? bracket(hits, target)
  const exact = hits.filter(hit => Math.abs(hit.value - target) < 1e-9).sort((a, b) => a.gap - b.gap || Number(b.exactType) - Number(a.exactType))[0]
  let anchor: Point
  let detail: string
  let confidence: GeocoderResult['confidence']
  if (exact && exact.gap <= RELIABLE_GAP_METERS) {
    anchor = exact.point
    detail = `Intersection with ${exact.name}`
    confidence = 'high'
  } else if (exact && (!bounds || isBetween(exact.point, bounds))) {
    anchor = exact.point
    detail = `Estimated by extending ${exact.name} (${Math.round(exact.gap)} m)`
    confidence = 'medium'
  } else if (bounds) {
    // The extension is unavailable or contradicts real intersection order: interpolate instead.
    const [low, high] = bounds
    anchor = nearestOnStreet(corridor, lerp(low.point, high.point, rankFraction(hits, low.value, high.value, target))).point
    detail = `Interpolated between ${low.name} and ${high.name}`
    confidence = low.gap <= RELIABLE_GAP_METERS && high.gap <= RELIABLE_GAP_METERS ? 'medium' : 'low'
  } else {
    return undefined
  }

  // The house number increases toward higher-numbered cross streets.
  const direction = (bounds && unit(subtract(bounds[1].point, bounds[0].point)))
    ?? trendDirection(corridor, hits, anchor, target)
    ?? (family(cross.type) === 'calle' ? { x: 0, y: cross.quadrant === 'sur' ? -1 : 1 } : { x: cross.quadrant === 'este' ? 1 : -1, y: 0 })
  const [lon, lat] = toCoordinate(walkAlong(corridor, anchor, direction, Math.min(address.plate ?? 0, MAX_PLATE_METERS)))
  return { text: formatAddress(address), detail, lon, lat, kind: 'estimated', confidence }
}

function textSearch(index: GeocoderIndex, query: string, near: Point, limit: number): GeocoderResult[] {
  const tokens = canonicalWords(query)
  if (tokens.join('').length < 3) return []
  const kindWeight = { place: 20, address: 15, street: 10, estimated: 0 }
  const scored: { entry: Entry; score: number }[] = []
  for (const entry of index.entries) {
    const matches = tokens.every(token => entry.words.some(word => /^\d+$/.test(token) || token.length === 1 ? word === token : word.startsWith(token)))
    if (!matches) continue
    const point = entry.street ? nearestOnStreet(entry.street, near).point : entry.point
    const score = (entry.words.length === tokens.length ? 30 : 0) + kindWeight[entry.kind] - entry.words.length - distance(point, near) / 500
    scored.push({ entry: { ...entry, point }, score })
  }
  return scored
    .sort((a, b) => b.score - a.score)
    .slice(0, limit)
    .map(({ entry }) => {
      const [lon, lat] = toCoordinate(entry.point)
      return { text: entry.text, detail: entry.detail, lon, lat, kind: entry.kind, confidence: entry.kind === 'street' ? 'low' : 'high' }
    })
}

export function search(index: GeocoderIndex, query: string, options: { near?: Coordinate; limit?: number } = {}): GeocoderResult[] {
  const limit = options.limit ?? 8
  const near = toPoint(...(options.near ?? DEFAULT_NEAR))
  const results: GeocoderResult[] = []
  const address = parseAddress(query)
  if (address?.cross) {
    const key = refKey(address.street)
    const number = normalizeText(`${formatNumber(address.cross)} ${address.plate ?? ''}`)
    for (const registered of index.addresses) {
      if (registered.ref && refKey(registered.ref) === key && registered.number === number) {
        const [lon, lat] = toCoordinate(registered.point)
        results.push({ text: registered.text, detail: 'Address registered in OSM', lon, lat, kind: 'address', confidence: 'high' })
      }
    }
    const estimated = geocodeNomenclature(index, address)
    if (estimated) results.push(estimated)
  } else if (address) {
    for (const street of index.byKey.get(refKey(address.street)) ?? []) {
      const [lon, lat] = toCoordinate(nearestOnStreet(street, near).point)
      results.push({ text: street.name, detail: 'Street', lon, lat, kind: 'street', confidence: 'low' })
    }
  }
  const seen = new Set(results.map(result => result.text))
  for (const result of textSearch(index, query, near, limit)) {
    if (!seen.has(result.text)) {
      seen.add(result.text)
      results.push(result)
    }
  }
  return results.slice(0, limit)
}

let indexPromise: Promise<GeocoderIndex> | undefined

export function loadGeocoder(): Promise<GeocoderIndex> {
  indexPromise ??= fetch(asset('data/bogota-geocoder.json'))
    .then(response => {
      if (!response.ok) throw new Error('Address search is not installed; select a point on the map.')
      return response.json() as Promise<RawGeocoderIndex>
    })
    .then(buildGeocoderIndex)
  indexPromise.catch(() => { indexPromise = undefined })
  return indexPromise
}

export async function searchGeocoder(query: string, options: { near?: Coordinate; limit?: number } = {}): Promise<GeocoderResult[]> {
  return search(await loadGeocoder(), query, options)
}
