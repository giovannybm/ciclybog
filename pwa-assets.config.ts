import { defineConfig, minimal2023Preset } from '@vite-pwa/assets-generator/config'

// Generates PWA icons (192, 512, maskable, apple-touch) from the SVG favicon: `pnpm pwa:assets`.
export default defineConfig({
  headLinkOptions: { preset: '2023' },
  preset: {
    ...minimal2023Preset,
    maskable: { ...minimal2023Preset.maskable, resizeOptions: { background: '#102a43' } },
    apple: { ...minimal2023Preset.apple, resizeOptions: { background: '#ffffff' } }
  },
  images: ['public/favicon.svg']
})
