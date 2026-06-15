import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  // Relative base so the same static build serves from the backend (/admin) and
  // loads inside a Tauri desktop webview (file://) without a host-aware rewrite.
  base: './',
  plugins: [react(), tailwindcss()],
  server: {
    port: 5180,
    proxy: {
      '/api': {
        target: process.env.VITE_API_PROXY || 'http://127.0.0.1:8088',
        changeOrigin: true,
      },
    },
  },
  build: {
    outDir: 'dist',
  },
})
