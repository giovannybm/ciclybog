import { existsSync, readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { buildGeocoderIndex, parseAddress, parseStreetName, search, type RawGeocoderIndex } from './geocoder'

const LON = -74.03
const LAT = 4.75
const METERS_PER_DEGREE_LAT = 110_574
const horizontal = (lat: number, fromLon: number, toLon: number) => [fromLon, lat, toLon, lat]

function syntheticIndex(withCalle172B = true): RawGeocoderIndex {
  return {
    version: 2,
    streets: [
      { name: 'Carrera 10', lines: [[LON, LAT, LON, LAT + 0.01]] },
      { name: 'Calle 172', lines: [horizontal(LAT + 0.002, LON - 0.003, LON + 0.003)] },
      { name: 'Calle 172A', lines: [horizontal(LAT + 0.003, LON - 0.003, LON + 0.003)] },
      // Does not touch Carrera 10: it ends ~330 m west, as in OSM.
      ...(withCalle172B ? [{ name: 'Calle 172 B', lines: [horizontal(LAT + 0.004, LON - 0.006, LON - 0.003)] }] : []),
      { name: 'Calle 173', lines: [horizontal(LAT + 0.005, LON - 0.003, LON + 0.003)] }
    ],
    places: [['Museo Nacional', 'tourism=museum', -74.069, 4.615], ['Constructor toString', 'shop=hardware', -74.05, 4.7]],
    addresses: [['Avenida Carrera 19', '172B-90', -74.04797, 4.71045]]
  }
}

describe('parseAddress', () => {
  it('interpreta variantes de nomenclatura bogotana', () => {
    for (const query of ['Cra. 10 172b 50', 'Carrera 10 # 172B-50', 'KR 10 No. 172 B - 50', 'cra 10 n° 172b-50']) {
      expect(parseAddress(query), query).toMatchObject({
        street: { type: 'carrera', number: 10, letter: '', quadrant: '' },
        cross: { type: 'calle', number: 172, letter: 'b', quadrant: '' },
        plate: 50
      })
    }
  })

  it('assigns the quadrant to the correct road', () => {
    expect(parseAddress('Calle 26 Sur # 13-20')).toMatchObject({ street: { type: 'calle', number: 26, quadrant: 'sur' }, cross: { type: 'carrera', number: 13, quadrant: '' }, plate: 20 })
    expect(parseAddress('Carrera 10 # 17-20 Sur')).toMatchObject({ street: { quadrant: '' }, cross: { number: 17, quadrant: 'sur' } })
    expect(parseAddress('Calle 13 # 5-20 Este')).toMatchObject({ cross: { type: 'carrera', number: 5, quadrant: 'este' } })
  })

  it('recognizes bis, avenues, and roads without a cross street', () => {
    expect(parseAddress('Cl 72 bis a # 10 - 34')).toMatchObject({ street: { number: 72, bis: true, bisLetter: 'a' }, cross: { number: 10 }, plate: 34 })
    expect(parseAddress('Av. Calle 26 # 68-50')).toMatchObject({ street: { type: 'calle', number: 26 }, cross: { number: 68 } })
    expect(parseAddress('Cra 10')).toEqual({ street: expect.objectContaining({ type: 'carrera', number: 10 }) })
    expect(parseAddress('Museo Nacional')).toBeUndefined()
  })
})

describe('parseStreetName', () => {
  it('interpreta nombres OSM', () => {
    expect(parseStreetName('Avenida Carrera 10')).toMatchObject({ type: 'carrera', number: 10 })
    expect(parseStreetName('Calle 172 B')).toMatchObject({ type: 'calle', number: 172, letter: 'b' })
    expect(parseStreetName('Carrera 10 Bis A Este')).toMatchObject({ number: 10, bis: true, bisLetter: 'a', quadrant: 'este' })
    expect(parseStreetName('Avenida Boyacá')).toBeUndefined()
  })
})

describe('search', () => {
  it('extends the cross street to the carrera and applies the house number', () => {
    const [result] = search(buildGeocoderIndex(syntheticIndex()), 'Cra. 10 172b 50')
    expect(result).toMatchObject({ text: 'Carrera 10 # 172B-50', kind: 'estimated', confidence: 'medium' })
    expect(result.detail).toContain('Calle 172 B')
    expect(result.lon).toBeCloseTo(LON, 5)
    expect(result.lat).toBeCloseTo(LAT + 0.004 + 50 / METERS_PER_DEGREE_LAT, 5)
  })

  it('interpolates between neighboring streets when the cross street is missing', () => {
    const [result] = search(buildGeocoderIndex(syntheticIndex(false)), 'Carrera 10 # 172B-00')
    expect(result.detail).toBe('Interpolated between Calle 172A and Calle 173')
    // Between 172A and 173, only 172B is missing, so it lands in the middle.
    expect(result.lat).toBeCloseTo(LAT + 0.004, 5)
  })

  it('rejects extensions that contradict street order and advances the house number across segments', () => {
    // As in Usaquén: the extended Calle 172 B falls north of Calle 173, and the carrera is split.
    const raw: RawGeocoderIndex = {
      version: 2,
      streets: [
        { name: 'Carrera 10', lines: [[LON, LAT, LON, LAT + 0.0038]] },
        { name: 'Avenida Carrera 10', lines: [[LON, LAT + 0.01, LON, LAT + 0.0038]] },
        { name: 'Calle 172', lines: [horizontal(LAT + 0.002, LON - 0.003, LON + 0.003)] },
        { name: 'Calle 172A', lines: [horizontal(LAT + 0.003, LON - 0.003, LON + 0.003)] },
        { name: 'Calle 172 B', lines: [horizontal(LAT + 0.0044, LON - 0.006, LON - 0.003)] },
        { name: 'Calle 173', lines: [horizontal(LAT + 0.0042, LON - 0.003, LON + 0.003)] }
      ],
      places: [],
      addresses: []
    }
    const [result] = search(buildGeocoderIndex(raw), 'Cra. 10 172b 50')
    expect(result.detail).toBe('Interpolated between Calle 172A and Calle 173')
    // Mitad entre 172A (+0,003) y 173 (+0,0042), y 50 m al norte cruzando al otro tramo.
    expect(result.lat).toBeCloseTo(LAT + 0.0036 + 50 / METERS_PER_DEGREE_LAT, 5)
  })

  it('prefiere direcciones registradas en OSM', () => {
    const [result] = search(buildGeocoderIndex(syntheticIndex()), 'AK 19 # 172B-90')
    expect(result).toMatchObject({ kind: 'address', lon: -74.04797, confidence: 'high' })
  })

  it('searches roads and places with abbreviations and accents', () => {
    const index = buildGeocoderIndex(syntheticIndex())
    expect(search(index, 'Cra 10')[0]).toMatchObject({ text: 'Carrera 10', kind: 'street' })
    expect(search(index, 'museo nac')[0]).toMatchObject({ text: 'Museo Nacional', kind: 'place' })
    expect(search(index, 'zzz inexistente')).toEqual([])
    expect(search(index, 'constructor')[0]).toMatchObject({ text: 'Constructor toString' })
    expect(parseAddress('constructor 10 # 5-20')).toBeUndefined()
  })

  const realIndexPath = 'public/data/bogota-geocoder.json'
  const realIndex = existsSync(realIndexPath) ? JSON.parse(readFileSync(realIndexPath, 'utf8')) as RawGeocoderIndex : undefined
  it.runIf(realIndex?.version === 2)('geocodes Cra. 10 172b 50 with real OSM data', () => {
    const [result] = search(buildGeocoderIndex(realIndex!), 'Cra. 10 172b 50')
    expect(result.text).toBe('Carrera 10 # 172B-50')
    // On Carrera 10 (lon ≈ -74.033), between Calle 172 (4.7505) and 173 (4.7524), toward the north.
    expect(result.lon).toBeGreaterThan(-74.0345)
    expect(result.lon).toBeLessThan(-74.0315)
    expect(result.lat).toBeGreaterThan(4.7512)
    expect(result.lat).toBeLessThan(4.7530)
  })
})
