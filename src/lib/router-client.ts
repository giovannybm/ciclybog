import type { CalculatedRoute, Coordinate } from '../types'
import { assetFromEnv } from './assets'
import { getGraph, saveGraph } from './db'

interface RouterWorkerResponse { id: number; ok: boolean; routes?: CalculatedRoute[]; error?: string }

export class RouterClient {
  private worker = new Worker(new URL('../workers/router.worker.ts', import.meta.url), { type: 'module' })
  private sequence = 0
  private pending = new Map<number, { resolve: (routes: CalculatedRoute[]) => void; reject: (error: Error) => void }>()
  private readyPromise: Promise<void>

  constructor() {
    this.worker.onmessage = ({ data }: MessageEvent<RouterWorkerResponse>) => {
      const pending = this.pending.get(data.id)
      if (!pending) return
      this.pending.delete(data.id)
      data.ok ? pending.resolve(data.routes || []) : pending.reject(new Error(data.error || 'The route could not be calculated'))
    }
    this.readyPromise = this.loadGraph()
  }

  private async loadGraph(): Promise<void> {
    let bytes = await getGraph()
    if (!bytes) {
      const response = await fetch(assetFromEnv(import.meta.env.VITE_ROUTE_GRAPH_URL, 'data/bogota-graph.bin'))
      if (!response.ok) throw new Error('The Bogotá cycling graph was not found')
      bytes = await response.arrayBuffer()
      await saveGraph(bytes)
    }
    // The worker cannot infer the deployment base on its own, so pass it along.
    await this.request({ type: 'load', bytes, baseUrl: import.meta.env.BASE_URL }, [bytes])
  }

  async route(origin: Coordinate, destination: Coordinate, alternatives = 1): Promise<CalculatedRoute[]> {
    await this.readyPromise
    // Vue may provide reactive coordinates (Proxy), which cannot be cloned by
    // structured clone when crossing the Web Worker boundary.
    const plainOrigin: Coordinate = [Number(origin[0]), Number(origin[1])]
    const plainDestination: Coordinate = [Number(destination[0]), Number(destination[1])]
    return this.request({ type: 'route', origin: plainOrigin, destination: plainDestination, alternatives: Number(alternatives) })
  }

  async ready(): Promise<void> { await this.readyPromise }

  destroy() { this.worker.terminate() }

  private request(message: Record<string, unknown>, transfer: Transferable[] = []): Promise<CalculatedRoute[]> {
    const id = ++this.sequence
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject })
      try {
        this.worker.postMessage({ id, ...message }, transfer)
      } catch (error) {
        this.pending.delete(id)
        reject(error instanceof Error ? error : new Error('The request could not be sent to the local engine'))
      }
    })
  }
}
