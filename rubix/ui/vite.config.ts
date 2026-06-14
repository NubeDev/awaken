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
    port: 5180,
    // Proxy the real rubix-server API in dev so the UI calls /api/v1/* without
    // CORS. Override the target with VITE_API_PROXY.
    proxy: {
      '/api': {
        target: process.env.VITE_API_PROXY ?? 'http://127.0.0.1:8088',
        changeOrigin: true,
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
