import { defineConfig, devices } from '@playwright/test'

// End-to-end config for the Rubix UI. The test drives the real Connect flow in a
// browser against a live backend — the same path a user takes — so it exercises
// the vite dev proxy, the gate's header credential, and the seeded portfolio.
//
// The backend is NOT started here (it is a cargo build/run with its own data
// dir); bring it up first, e.g.:
//   RUBIX_BIND=127.0.0.1:8090 RUBIX_DATA_DIR=/tmp/rubix-e2e cargo run \
//     --bin rubix-server -- --seed-dev
// then point the UI proxy at it via RUBIX_E2E_PROXY. Vite IS started here so the
// proxy rewrite under test is the one that actually serves the run.
const PROXY = process.env.RUBIX_E2E_PROXY || 'http://127.0.0.1:8088'
const PORT = Number(process.env.RUBIX_E2E_UI_PORT || 5180)

export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: 0,
  reporter: [['list']],
  use: {
    baseURL: `http://127.0.0.1:${PORT}`,
    trace: 'on-first-retry',
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: {
    command: `pnpm dev --port ${PORT}`,
    url: `http://127.0.0.1:${PORT}`,
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
    env: { VITE_API_PROXY: PROXY },
  },
})
