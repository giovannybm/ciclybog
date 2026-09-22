// Generates the social preview image (Open Graph / Twitter): `pnpm og:image`.
import { createRequire } from 'node:module'

// sharp arrives as a dependency of @vite-pwa/assets-generator; pnpm 12 does not
// link the optional-peer variant correctly as a direct dependency, so resolve it there.
const require = createRequire(import.meta.url)
const sharp = createRequire(require.resolve('@vite-pwa/assets-generator/package.json'))('sharp')

const WIDTH = 1200
const HEIGHT = 630
const OUTPUT = 'public/og-image.png'

// Illustrative drawing of streets, cycleways, and a calculated route on the right.
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
    <text x="72" y="270" font-size="68" font-weight="800">Bike through Bogotá</text>
    <text x="72" y="350" font-size="68" font-weight="800">with Ciclybog</text>
    <text x="72" y="425" font-size="29" fill="#d9e2ec">Cycleway-first routes and local search</text>
    <text x="72" y="465" font-size="29" fill="#d9e2ec">address search and on-device routing.</text>
    <text x="72" y="555" font-size="24" font-weight="700" fill="#7dd3c7">● Works offline · OpenStreetMap data</text>
  </g>
</svg>`

await sharp(Buffer.from(svg)).png({ compressionLevel: 9 }).toFile(OUTPUT)
console.log(`Imagen Open Graph generada en ${OUTPUT} (${WIDTH}×${HEIGHT})`)
