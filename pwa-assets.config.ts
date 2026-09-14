import { defineConfig, minimal2023Preset } from '@vite-pwa/assets-generator/config'

// Genera íconos PWA (192, 512, maskable, apple-touch) desde el favicon SVG: `pnpm pwa:assets`.
export default defineConfig({
  headLinkOptions: { preset: '2023' },
  preset: {
    ...minimal2023Preset,
    maskable: { ...minimal2023Preset.maskable, resizeOptions: { background: '#102a43' } },
    apple: { ...minimal2023Preset.apple, resizeOptions: { background: '#ffffff' } }
  },
  images: ['public/favicon.svg']
})
