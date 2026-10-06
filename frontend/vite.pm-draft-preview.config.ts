import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { resolve } from 'node:path'

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: { alias: { '@': resolve(__dirname, 'src') } },
  server: { host: '127.0.0.1', port: 55498, strictPort: true },
  preview: { host: '127.0.0.1', port: 55498, strictPort: true },
  build: {
    outDir: 'pm-draft-preview-dist',
    rollupOptions: { input: resolve(__dirname, 'pm-draft-preview.html') },
  },
})
