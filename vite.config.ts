import { createHash } from 'node:crypto'
import { existsSync, readFileSync } from 'node:fs'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { VitePWA } from 'vite-plugin-pwa'

// La clave de IndexedDB cambia cuando cambia el contenido del grafo publicado.
const graphPath = 'public/data/bogota-graph.bin'
const graphHash = existsSync(graphPath) ? createHash('sha1').update(readFileSync(graphPath)).digest('hex').slice(0, 16) : 'missing'

export default defineConfig({
  server: {
    allowedHosts: ['.ngrok-free.app']
  },
  define: { __GRAPH_HASH__: JSON.stringify(graphHash) },
  plugins: [
    vue(),
    VitePWA({
      registerType: 'autoUpdate',
      includeAssets: ['favicon.svg', 'favicon.ico', 'apple-touch-icon-180x180.png'],
      manifest: {
        id: '/',
        name: 'Ciclybog — rutas en bicicleta',
        short_name: 'Ciclybog',
        description: 'Planificador de rutas ciclistas offline para Bogotá',
        lang: 'es-CO',
        dir: 'ltr',
        theme_color: '#102a43',
        background_color: '#f5f7fa',
        display: 'standalone',
        orientation: 'any',
        start_url: '/',
        scope: '/',
        categories: ['navigation', 'travel', 'sports'],
        icons: [
          { src: 'pwa-64x64.png', sizes: '64x64', type: 'image/png' },
          { src: 'pwa-192x192.png', sizes: '192x192', type: 'image/png' },
          { src: 'pwa-512x512.png', sizes: '512x512', type: 'image/png', purpose: 'any' },
          { src: 'maskable-icon-512x512.png', sizes: '512x512', type: 'image/png', purpose: 'maskable' }
        ]
      },
      workbox: {
        globPatterns: ['**/*.{js,css,html,svg,png,ico,bin,wasm,pmtiles,geojson}', 'data/bogota-geocoder.json'],
        maximumFileSizeToCacheInBytes: 100 * 1024 * 1024,
        navigateFallback: '/index.html',
        runtimeCaching: [
          {
            urlPattern: /^https:\/\/demotiles\.maplibre\.org\/.*$/i,
            handler: 'CacheFirst',
            options: { cacheName: 'map-style', expiration: { maxEntries: 10 } }
          },
          {
            urlPattern: /^https:\/\/protomaps\.github\.io\/basemaps-assets\/.*$/i,
            handler: 'CacheFirst',
            options: { cacheName: 'map-fonts-sprites', expiration: { maxEntries: 200 } }
          }
        ]
      }
    })
  ]
})
