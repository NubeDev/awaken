import { defineConfig } from 'vite'
import { fileURLToPath, URL } from 'node:url'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  // Relative base so the same static build serves from the backend (/admin) and
  // loads inside a Tauri desktop webview (file://) without a host-aware rewrite.
  base: './',
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  server: {
    port: Number(process.env.VITE_UI_PORT) || 5192,
    // The backend serves flat, unprefixed routes (/records, /query, /auth/...,
    // /health, /datasources, /ws/..., /api-docs) — see crates/rubix-server/src/
    // http/mod.rs. The UI client hits those same paths with a blank ("same
    // origin") endpoint, so the dev server must proxy each one to the backend;
    // anything not matched here falls through to vite's SPA index.html, which is
    // what produced the "Unexpected token '<'" JSON parse error. `/api` is kept
    // for the future versioned surface (client.ts TENANT_ROUTES_LIVE).
    proxy: {
      '^/(api|records|query|auth|datasources|principals|tenants|devices|health|api-docs)(/|$)': {
        target: process.env.VITE_API_PROXY || 'http://127.0.0.1:8092',
        changeOrigin: true,
      },
      '/ws': {
        target: process.env.VITE_API_PROXY || 'http://127.0.0.1:8092',
        changeOrigin: true,
        ws: true,
      },
    },
  },
  build: {
    outDir: 'dist',
  },
})
