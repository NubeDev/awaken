/**
 * Look-freeze reference capture.
 *
 * With `rubix-server --seed-dev` running and the UI served against the live
 * API, screenshot the four operator surfaces into `docs/reference/`. These are
 * the baseline every later UI gate eyeballs against
 * (docs/sessions/ui/STATUS.md "Look-freeze gate") — a populated page may not
 * lose widgets, density, or chart series versus its reference.
 *
 * Usage (dev loop, three terminals):
 *   1. cd rubix && cargo run -p rubix-server -- --seed-dev   # :8088
 *   2. pnpm -C rubix/ui dev                                  # :5180, /api proxied
 *   3. node rubix/ui/scripts/screenshot.mjs                  # captures references
 *
 * Override the UI origin with UI_BASE (default http://127.0.0.1:5180).
 */
import { mkdir } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium } from 'playwright'

const HERE = dirname(fileURLToPath(import.meta.url))
const OUT = join(HERE, '..', 'docs', 'reference')
const BASE = process.env.UI_BASE ?? 'http://127.0.0.1:5180'

/** The four surfaces UI-02 freezes: route path → reference file name. */
const SURFACES = [
  { path: '/', name: 'dashboard' },
  { path: '/points', name: 'points' },
  { path: '/sparks', name: 'sparks' },
  { path: '/flows', name: 'flows' },
]

async function main() {
  await mkdir(OUT, { recursive: true })
  const browser = await chromium.launch()
  const context = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    deviceScaleFactor: 2,
    colorScheme: 'dark',
  })
  const page = await context.newPage()
  try {
    for (const surface of SURFACES) {
      const url = new URL(surface.path, BASE).toString()
      await page.goto(url, { waitUntil: 'networkidle' })
      // Let charts/sparklines settle after data resolves.
      await page.waitForTimeout(1500)
      const file = join(OUT, `${surface.name}.png`)
      await page.screenshot({ path: file, fullPage: true })
      console.log(`captured ${surface.name} → ${file}`)
    }
  } finally {
    await browser.close()
  }
}

main().catch((err) => {
  console.error(err)
  process.exit(1)
})
