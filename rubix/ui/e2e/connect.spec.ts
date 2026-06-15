import { test, expect, type Page } from '@playwright/test'

// The Connect → Portfolio golden path. A blank endpoint means "same origin", so
// every API call is a flat route (/records, ...) that the vite dev proxy must
// forward to the backend. If the proxy misses a route, vite returns index.html
// and the client's JSON.parse throws "Unexpected token '<'" — the regression
// this test guards. We seed the demo cast with `--seed-dev`, so acme_operator /
// operator-demo is a live credential and the acme tenant has a real portfolio.

const SEED = { tenant: 'acme', subject: 'acme_operator', secret: 'operator-demo' }

// Collect anything that smells like "got HTML where JSON was expected", from both
// the console and the network, so the assertion names the actual failure.
function watchForHtmlAsJson(page: Page): string[] {
  const problems: string[] = []
  page.on('console', (msg) => {
    const t = msg.text()
    if (/Unexpected token|is not valid JSON|<!doctype/i.test(t)) problems.push(`console: ${t}`)
  })
  page.on('pageerror', (err) => {
    if (/Unexpected token|is not valid JSON|<!doctype/i.test(err.message)) {
      problems.push(`pageerror: ${err.message}`)
    }
  })
  page.on('response', async (res) => {
    const url = res.url()
    if (!/\/(records|query|auth|datasources|health)(\/|$|\?)/.test(url)) return
    const ct = res.headers()['content-type'] || ''
    if (ct.includes('text/html')) problems.push(`response ${res.status()} text/html for ${url}`)
  })
  return problems
}

test('connect with blank endpoint loads the seeded portfolio without a JSON parse error', async ({
  page,
}) => {
  const problems = watchForHtmlAsJson(page)

  await page.goto('/')

  // The Connect screen.
  await expect(page.getByText('Connect to Rubix')).toBeVisible()

  // Leave Endpoint blank (same origin → proxied). Fill the seeded credential.
  await page.getByLabel('Tenant').fill(SEED.tenant)
  await page.getByLabel('Subject').fill(SEED.subject)
  await page.getByLabel('Secret').fill(SEED.secret)
  await page.getByRole('button', { name: 'Connect' }).click()

  // The Portfolio screen renders the tenant's real records.
  await expect(page.getByRole('heading', { name: 'Your portfolio' })).toBeVisible()
  await expect(page.getByText(`Tenant · ${SEED.tenant}`)).toBeVisible()

  // The seeded acme tenant has sites; assert the /records read succeeded and the
  // portfolio derived them. The subtitle only prints a site count once records
  // load (it says "your sites" while empty/loading), so a generous timeout here
  // also covers a cold vite first-compile. Then assert a site card rendered.
  await expect(page.getByText(/Rubix is watching \d+ sites?/)).toBeVisible({ timeout: 20_000 })
  await expect(page.getByText('No sites in this tenant yet')).toHaveCount(0)
  await expect(page.locator('text=equipment').first()).toBeVisible()

  expect(problems, `HTML-where-JSON problems:\n${problems.join('\n')}`).toEqual([])
})
