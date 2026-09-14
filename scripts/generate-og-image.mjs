// Genera la imagen de vista previa para redes (Open Graph / Twitter): `pnpm og:image`.
import { createRequire } from 'node:module'

// sharp llega como dependencia de @vite-pwa/assets-generator; pnpm 12 no enlaza bien
// la variante con peer opcional como dependencia directa, así que se resuelve desde ahí.
const require = createRequire(import.meta.url)
const sharp = createRequire(require.resolve('@vite-pwa/assets-generator/package.json'))('sharp')

const WIDTH = 1200
const HEIGHT = 630
const OUTPUT = 'public/og-image.png'

// Trazado ilustrativo de calles, ciclorrutas y una ruta calculada sobre el lado derecho.
const streets = Array.from({ length: 14 }, (_, index) => {
  const x = 640 + index * 44
  return `<path d="M${x} -20 L${x - 60} ${HEIGHT + 20}" />`
}).join('') + Array.from({ length: 12 }, (_, index) => {
  const y = 20 + index * 52
  return `<path d="M600 ${y} L${WIDTH + 20} ${y + 36}" />`
}).join('')

const svg = `
<svg xmlns="http://www.w3.org/2000/svg" width="${WIDTH}" height="${HEIGHT}" viewBox="0 0 ${WIDTH} ${HEIGHT}">
  <defs>
    <linearGradient id="background" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#102a43" />
      <stop offset="1" stop-color="#0b1f33" />
    </linearGradient>
    <linearGradient id="fade" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0.42" stop-color="#102a43" stop-opacity="1" />
      <stop offset="0.72" stop-color="#102a43" stop-opacity="0" />
    </linearGradient>
  </defs>
  <rect width="${WIDTH}" height="${HEIGHT}" fill="url(#background)" />

  <g stroke="#243b53" stroke-width="3" fill="none">${streets}</g>
  <g stroke="#3437eb" stroke-width="6" stroke-linecap="round" stroke-linejoin="round" fill="none" opacity="0.9">
    <path d="M700 90 L760 240 L745 420 L820 600" />
    <path d="M640 330 L900 380 L1180 430" />
    <path d="M980 40 L1010 250 L1090 640" />
  </g>
  <path d="M1040 110 L1020 250 L930 310 L880 450 L760 540" stroke="#0b1f33" stroke-width="22" stroke-linecap="round" stroke-linejoin="round" fill="none" opacity="0.6" />
  <path d="M1040 110 L1020 250 L930 310 L880 450 L760 540" stroke="#22c55e" stroke-width="12" stroke-linecap="round" stroke-linejoin="round" fill="none" />
  <circle cx="1040" cy="110" r="16" fill="#2563eb" stroke="#fff" stroke-width="5" />
  <circle cx="760" cy="540" r="16" fill="#dc2626" stroke="#fff" stroke-width="5" />
  <rect width="${WIDTH}" height="${HEIGHT}" fill="url(#fade)" />

  <g transform="translate(72 70) scale(1.5)">
    <circle cx="20" cy="45" r="10" fill="none" stroke="#ffffff" stroke-width="5" />
    <circle cx="48" cy="45" r="10" fill="none" stroke="#ffffff" stroke-width="5" />
    <path d="M20 45 30 22h9l9 23M27 30h17M30 22l-5-7h9" fill="none" stroke="#7dd3c7" stroke-linecap="round" stroke-linejoin="round" stroke-width="5" />
  </g>
  <g font-family="Helvetica Neue, Helvetica, Arial, sans-serif" fill="#ffffff">
    <text x="190" y="138" font-size="34" font-weight="800" letter-spacing="6" fill="#7dd3c7">CICLYBOG</text>
    <text x="72" y="270" font-size="68" font-weight="800">Muévete en bici</text>
    <text x="72" y="350" font-size="68" font-weight="800">por Bogotá</text>
    <text x="72" y="425" font-size="29" fill="#d9e2ec">Rutas por ciclorrutas y búsqueda de</text>
    <text x="72" y="465" font-size="29" fill="#d9e2ec">direcciones, calculadas en tu dispositivo.</text>
    <text x="72" y="555" font-size="24" font-weight="700" fill="#7dd3c7">● Funciona sin conexión · Datos de OpenStreetMap</text>
  </g>
</svg>`

await sharp(Buffer.from(svg)).png({ compressionLevel: 9 }).toFile(OUTPUT)
console.log(`Imagen Open Graph generada en ${OUTPUT} (${WIDTH}×${HEIGHT})`)
