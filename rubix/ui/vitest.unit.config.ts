/// <reference types="vitest/config" />
import path from 'path'
import { defineConfig } from 'vitest/config'

/**
 * Node/jsdom test project for Rubix's own logic (API client, formatters, view
 * helpers). Kept separate from the template's playwright browser suite so these
 * run fast in CI without a browser install: `pnpm test:unit`.
 */
export default defineConfig({
  resolve: { alias: { '@': path.resolve(__dirname, './src') } },
  test: {
    globals: true,
    environment: 'jsdom',
    include: ['src/**/*.unit.test.{ts,tsx}'],
  },
})
