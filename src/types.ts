export type Coordinate = [number, number]

export type Infrastructure = 'cycleway' | 'conventional' | 'unknown'

export interface RouteSegmentProperties {
  infrastructure: Infrastructure
  roadName?: string
  distanceMeters: number
  bicycleAccess?: string
}

export interface RouteSegment {
  type: 'Feature'
  geometry: GeoJSON.LineString
  properties: RouteSegmentProperties
}

export interface CalculatedRoute {
  distanceMeters: number
  cyclewayMeters: number
  segments: RouteSegment[]
}

export interface SavedRoute {
  id: string
  name: string
  geometry: GeoJSON.FeatureCollection<GeoJSON.LineString, RouteSegmentProperties>
  distanceMeters: number
  alternative: boolean
  createdAt: string
  updatedAt: string
}
