import { createHash } from 'node:crypto'
import { existsSync, readFileSync } from 'node:fs'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { VitePWA } from 'vite-plugin-pwa'

// La clave de IndexedDB cambia cuando cambia el contenido del grafo publicado.
const graphPath = 'public/data/bogota-graph.bin'
const graphHash = existsSync(graphPath) ? createHash('sha1').update(readFileSync(graphPath)).digest('hex').slice(0, 16) : 'missing'

// Las metaetiquetas Open Graph exigen URLs absolutas: VITE_SITE_URL o, en Netlify, la URL del sitio.
// Sin un valor, `%SITE_URL%/` quedaba en `href="/"` y Vite intentaba resolver ese
// href del <link rel="canonical"> como un asset local: leía el directorio raíz y
// el build moría con EISDIR. El fallback mantiene las URLs absolutas y válidas.
const configuredSiteUrl = (process.env.VITE_SITE_URL || process.env.URL || '').replace(/\/$/, '')
const siteUrl = configuredSiteUrl || 'https://ciclybog.netlify.app'

// Permite servir la app bajo un subpath, p.ej. embebida dentro de otra página.
// Sin la variable la base sigue siendo '/', así que el despliegue no cambia.
const base = process.env.VITE_BASE_PATH || '/'
// El build embebido no registra service worker: cachearía 24+ MB en el origen
// del contenedor y serviría versiones viejas tras recompilar.
const embed = process.env.VITE_EMBED === '1'

export default defineConfig(({ command }) => ({
  base,
  server: {
    allowedHosts: ['.ngrok-free.app']
  },
  // outDir se decide aquí y no con --outDir en la CLI: vite-plugin-pwa lo lee en
  // configResolved y el flag de línea de comandos lo dejaba sin resolver.
  build: {
    outDir: embed ? 'dist-embed' : 'dist'
  },
  define: { __GRAPH_HASH__: JSON.stringify(graphHash) },
  plugins: [
    {
      name: 'ciclybog-site-url',
      transformIndexHtml: {
        order: 'pre',
        handler: html => {
          if (command === 'build' && !configuredSiteUrl) console.warn(`[ciclybog] VITE_SITE_URL no definida; se usa ${siteUrl} para la vista previa en redes.`)
          return html.replaceAll('%SITE_URL%', siteUrl)
        }
      }
    },
    vue(),
    // Se mantiene aunque `embed` lo desactive: el plugin es quien provee el
    // módulo virtual `virtual:pwa-register` que importa src/main.ts. Excluirlo
    // del array dejaría ese import sin resolver.
    VitePWA({
      disable: embed,
      registerType: 'autoUpdate',
      includeAssets: ['favicon.svg', 'favicon.ico', 'apple-touch-icon-180x180.png'],
      manifest: {
        id: base,
        name: 'Ciclybog — rutas en bicicleta',
        short_name: 'Ciclybog',
        description: 'Planificador de rutas ciclistas offline para Bogotá',
        lang: 'es-CO',
        dir: 'ltr',
        theme_color: '#102a43',
        background_color: '#f5f7fa',
        display: 'standalone',
        orientation: 'any',
        start_url: base,
        scope: base,
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
        // La imagen de vista previa solo la usan las redes; no hace falta offline.
        globIgnores: ['og-image.png'],
        maximumFileSizeToCacheInBytes: 100 * 1024 * 1024,
        navigateFallback: `${base}index.html`,
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
}))
