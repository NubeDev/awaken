/// <reference types="vitest/config" />
import path from 'path'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { tanstackRouter } from '@tanstack/router-plugin/vite'
import { playwright } from '@vitest/browser-playwright'

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    tanstackRouter({
      target: 'react',
      autoCodeSplitting: true,
    }),
    react(),
    tailwindcss(),
  ],
  server: {
    // VITE_UI_PORT / VITE_API_PROXY are pinned by nhp/Makefile (BE_PORT 8094 /
    // UI_PORT 5194) so the dev server and its proxy always follow the backend.
    port: Number(process.env.VITE_UI_PORT) || 5194,
    // Watch fallback. Vite puts an inotify watch on every source file; on a
    // shared box where `fs.inotify.max_user_watches` is exhausted by other
    // processes, that crashes the dev server with `ENOSPC: file watchers
    // reached`. Set VITE_POLL=1 (the Makefile does this for `make dev-poll`) to
    // watch by polling instead — no inotify watches at all, at the cost of a
    // little idle CPU. Unset = native inotify (the fast default).
    watch: process.env.VITE_POLL
      ? { usePolling: true, interval: 300 }
      : undefined,
    // rubix-server serves flat, unprefixed routes (/records, /query, /auth/...,
    // /principals, /health, /ws/..., see rubix crates/rubix-server/src/http/
    // mod.rs). Proxy those to the backend so the UI calls them same-origin
    // without CORS; everything else falls through to vite's SPA index.html.
    // `/api` is kept for any future versioned surface. Override the target with
    // VITE_API_PROXY.
    proxy: {
      '^/(api|records|readings|query|auth|datasources|principals|tenants|devices|health|api-docs)(/|\\?|$)':
        {
          target: process.env.VITE_API_PROXY || 'http://127.0.0.1:8094',
          changeOrigin: true,
        },
      '/ws': {
        target: process.env.VITE_API_PROXY || 'http://127.0.0.1:8094',
        changeOrigin: true,
        ws: true,
      },
    },
  },
  // `react-grid-layout`'s CJS deps (`react-draggable`/`react-resizable`) read
  // `process.env.NODE_ENV` at runtime, which is undefined in the browser bundle
  // and throws on drag/resize. Shim it from Vite's mode so those handlers run.
  define: {
    'process.env': {
      NODE_ENV: process.env.NODE_ENV ?? 'development',
    },
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  test: {
    silent: 'passed-only',
    unstubEnvs: true,
    browser: {
      enabled: true,
      provider: playwright(),
      instances: [{ browser: 'chromium' }],
    },
    coverage: {
      // include: ['src/**/*.{js,jsx,ts,tsx}'], // Uncomment to expand the report to all src/**/* so untested modules appear as 0% coverage.
      exclude: [
        'src/components/ui/**',
        'src/assets/**',
        'src/tanstack-table.d.ts',
        'src/routeTree.gen.ts',
        'src/test-utils/**',
        'src/routes/**',
      ],
    },
  },
})
