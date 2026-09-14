import type { SavedRoute } from '../types'

const DB_NAME = 'ciclybog'
const ROUTES_STORE = 'routes'
const GRAPH_STORE = 'graph'
const GRAPH_KEY = `bogota-graph-${__GRAPH_HASH__}`

function openDatabase(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, 2)
    request.onupgradeneeded = () => {
      if (!request.result.objectStoreNames.contains(ROUTES_STORE)) request.result.createObjectStore(ROUTES_STORE, { keyPath: 'id' })
      if (!request.result.objectStoreNames.contains(GRAPH_STORE)) request.result.createObjectStore(GRAPH_STORE)
    }
    request.onsuccess = () => resolve(request.result)
    request.onerror = () => reject(request.error)
  })
}

export async function listRoutes(): Promise<SavedRoute[]> {
  const db = await openDatabase()
  return new Promise((resolve, reject) => {
    const request = db.transaction(ROUTES_STORE).objectStore(ROUTES_STORE).getAll()
    request.onsuccess = () => resolve((request.result as SavedRoute[]).sort((a, b) => b.updatedAt.localeCompare(a.updatedAt)))
    request.onerror = () => reject(request.error)
  })
}

export async function saveRoute(route: SavedRoute): Promise<void> {
  const db = await openDatabase()
  return new Promise((resolve, reject) => {
    const request = db.transaction(ROUTES_STORE, 'readwrite').objectStore(ROUTES_STORE).put(route)
    request.onsuccess = () => resolve()
    request.onerror = () => reject(request.error)
  })
}

export async function getGraph(): Promise<ArrayBuffer | undefined> {
  const db = await openDatabase()
  return new Promise((resolve, reject) => {
    const request = db.transaction(GRAPH_STORE).objectStore(GRAPH_STORE).get(GRAPH_KEY)
    request.onsuccess = () => resolve(request.result as ArrayBuffer | undefined)
    request.onerror = () => reject(request.error)
  })
}

export async function saveGraph(bytes: ArrayBuffer): Promise<void> {
  const db = await openDatabase()
  return new Promise((resolve, reject) => {
    // Solo se conserva la versión vigente del grafo.
    const transaction = db.transaction(GRAPH_STORE, 'readwrite')
    const store = transaction.objectStore(GRAPH_STORE)
    store.clear()
    store.put(bytes, GRAPH_KEY)
    transaction.oncomplete = () => resolve()
    transaction.onerror = () => reject(transaction.error)
  })
}
