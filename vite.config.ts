import { createHash } from 'node:crypto'
import { existsSync, readFileSync } from 'node:fs'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { VitePWA } from 'vite-plugin-pwa'

// The IndexedDB key changes when the published graph content changes.
const graphPath = 'public/data/bogota-graph.bin'
const graphHash = existsSync(graphPath) ? createHash('sha1').update(readFileSync(graphPath)).digest('hex').slice(0, 16) : 'missing'

// Open Graph metadata requires absolute URLs: VITE_SITE_URL or Netlify's site URL.
// Without a value, `%SITE_URL%/` became `href="/"` and Vite tried to resolve the
// canonical link as a local asset: it read the root directory and the build failed
// with EISDIR. The fallback keeps URLs absolute and valid.
const configuredSiteUrl = (process.env.VITE_SITE_URL || process.env.URL || '').replace(/\/$/, '')
const siteUrl = configuredSiteUrl || 'https://ciclybog.netlify.app'

// Allows serving the app under a subpath, for example embedded in another page.
// Without the variable, the base remains '/', so root deployments do not change.
const base = process.env.VITE_BASE_PATH || '/'
// Embedded builds do not register a service worker: it would cache 24+ MB on the
// host origin and serve stale versions after rebuilds.
const embed = process.env.VITE_EMBED === '1'

export default defineConfig(({ command }) => ({
  base,
  server: {
    allowedHosts: ['.ngrok-free.app']
  },
  // Decide outDir here rather than with the CLI --outDir flag: vite-plugin-pwa
  // reads it in configResolved and the CLI flag left it unresolved.
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
          if (command === 'build' && !configuredSiteUrl) console.warn(`[ciclybog] VITE_SITE_URL is not defined; using ${siteUrl} for social previews.`)
          return html.replaceAll('%SITE_URL%', siteUrl)
        }
      }
    },
    vue(),
    // Keep the plugin even when `embed` disables it: the plugin provides the
    // virtual `virtual:pwa-register` module imported by src/main.ts. Removing it
    // from the array would leave that import unresolved.
    VitePWA({
      disable: embed,
      registerType: 'autoUpdate',
      includeAssets: ['favicon.svg', 'favicon.ico', 'apple-touch-icon-180x180.png'],
      manifest: {
        id: base,
        name: 'Ciclybog — bike routes',
        short_name: 'Ciclybog',
        description: 'Offline bike route planner for Bogotá',
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
        // The preview image is only used by social networks; it is not needed offline.
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
