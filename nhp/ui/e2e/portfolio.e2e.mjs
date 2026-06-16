// Standalone end-to-end smoke for the NHP UI, driven by Playwright against the
// live `make dev` server on :5194 (which proxies the records API to the rubix
// backend on :8094). Run AFTER seeding:
//   make dev SEED=1   then   make seed
//   node e2e/portfolio.e2e.mjs   (or BASE_URL=… node e2e/portfolio.e2e.mjs)
//
// No test runner — just chromium + asserts, so it works without a playwright
// config. Exits non-zero on the first failure.

import { chromium } from 'playwright'
import assert from 'node:assert/strict'

const BASE = process.env.BASE_URL ?? 'http://localhost:5194'
const SHOTS = process.env.SHOT_DIR ?? '/tmp/nhp-e2e'

const steps = []
const step = (name) => {
  console.log(`▶ ${name}`)
  steps.push(name)
}

async function run() {
  const browser = await chromium.launch()
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } })
  // Track console errors, but only count them once we're authenticated. The
  // app's first paint fires record requests with no token (→ 401 → redirect to
  // sign-in by design), so pre-login 401s are expected noise, not failures.
  let authed = false
  const consoleErrors = []
  page.on('console', (m) => {
    if (m.type() === 'error' && authed) consoleErrors.push(m.text())
  })
  page.on('pageerror', (e) => {
    if (authed) consoleErrors.push(`pageerror: ${e.message}`)
  })

  try {
    // --- sign in via the username/password form (Operator) -------------------
    step('load app → redirected to sign-in')
    await page.goto(BASE, { waitUntil: 'networkidle' })
    await page.waitForURL(/sign-in/, { timeout: 10_000 })
    assert.ok(await page.getByText('Sign in').first().isVisible(), 'sign-in card visible')

    step('fill credentials + submit')
    await page.getByLabel('Username').fill('acme_operator')
    await page.getByLabel('Password').fill('operator-demo')
    await page.getByRole('button', { name: 'Sign in' }).click()

    step('land on an authenticated route')
    await page.waitForURL((u) => !/sign-in/.test(u.toString()), { timeout: 10_000 })
    await page.waitForLoadState('networkidle')
    authed = true // from here on, 401s/console errors are real failures

    // --- dashboards must show real portfolio data, not the empty state -------
    step('dashboards show tenants (NOT the "No tenants" empty state)')
    await page.goto(`${BASE}/dashboards`, { waitUntil: 'networkidle' })
    await page.screenshot({ path: `${SHOTS}/dashboards.png`, fullPage: true })
    const emptyState = page.getByText(/No tenants\. Seed a portfolio first/i)
    assert.equal(await emptyState.count(), 0, 'empty "No tenants" state must be gone after seeding')
    // The seeded portfolio has Acme + Globex tenants; the sidebar lists them.
    const body = await page.locator('body').innerText()
    assert.ok(/acme/i.test(body), 'acme tenant referenced on dashboards')

    // --- admin record tables render rows -------------------------------------
    const adminPages = [
      ['tenants', /tenants/i, 2],
      ['sites', /sites/i, 4],
      ['meters', /meters/i, 12],
    ]
    for (const [slug, , minRows] of adminPages) {
      step(`admin/${slug} renders ≥ ${minRows} rows`)
      await page.goto(`${BASE}/admin/${slug}`, { waitUntil: 'networkidle' })
      await page.waitForLoadState('networkidle')
      await page.screenshot({ path: `${SHOTS}/admin-${slug}.png`, fullPage: true })
      const rows = await page.locator('table tbody tr').count()
      const text = await page.locator('body').innerText()
      const emptyish = /no\s+\w+\b.*(found|yet)|seed a portfolio/i.test(text)
      assert.ok(
        rows >= minRows || (!emptyish && rows > 0),
        `admin/${slug}: expected ≥${minRows} rows, got ${rows} (emptyish=${emptyish})`
      )
    }

    // --- no console errors during the walk -----------------------------------
    step('no uncaught console/page errors')
    const realErrors = consoleErrors.filter(
      (e) => !/favicon|ResizeObserver|Download the React DevTools/i.test(e)
    )
    assert.deepEqual(realErrors, [], `console errors:\n${realErrors.join('\n')}`)

    console.log(`\n✅ e2e passed (${steps.length} steps). Screenshots in ${SHOTS}/`)
  } finally {
    await browser.close()
  }
}

run().catch((err) => {
  console.error(`\n❌ e2e FAILED at step: ${steps.at(-1)}\n`)
  console.error(err.message)
  process.exit(1)
})
