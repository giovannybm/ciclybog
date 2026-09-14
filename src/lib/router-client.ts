import type { CalculatedRoute, Coordinate } from '../types'
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
      data.ok ? pending.resolve(data.routes || []) : pending.reject(new Error(data.error || 'No fue posible calcular la ruta'))
    }
    this.readyPromise = this.loadGraph()
  }

  private async loadGraph(): Promise<void> {
    let bytes = await getGraph()
    if (!bytes) {
      const response = await fetch(import.meta.env.VITE_ROUTE_GRAPH_URL || '/data/bogota-graph.bin')
      if (!response.ok) throw new Error('No se encontró el grafo ciclista de Bogotá')
      bytes = await response.arrayBuffer()
      await saveGraph(bytes)
    }
    await this.request({ type: 'load', bytes }, [bytes])
  }

  async route(origin: Coordinate, destination: Coordinate, alternatives = 1): Promise<CalculatedRoute[]> {
    await this.readyPromise
    // Vue puede entregar coordenadas reactivas (Proxy), que no son clonables
    // por structured clone al cruzar la frontera del Web Worker.
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
        reject(error instanceof Error ? error : new Error('No fue posible enviar la solicitud al motor local'))
      }
    })
  }
}
