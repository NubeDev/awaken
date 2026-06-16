/**
 * Throwaway e2e check: drive the real login flow in a browser and assert the
 * dashboard shows seeded data. Verbose: logs console + network failures.
 *   node scripts/e2e-login-check.mjs   (UI_BASE override, default :5194)
 */
import { chromium } from 'playwright'

const BASE = process.env.UI_BASE ?? 'http://127.0.0.1:5194'

async function main() {
  const browser = await chromium.launch()
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } })

  page.on('console', (m) =>
    console.log(`  [console.${m.type()}] ${m.text()}`)
  )
  page.on('pageerror', (e) => console.log(`  [pageerror] ${e.message}`))
  page.on('requestfailed', (r) =>
    console.log(`  [reqfailed] ${r.method()} ${r.url()} — ${r.failure()?.errorText}`)
  )
  // Log auth + records responses with status.
  page.on('response', (res) => {
    const u = res.url()
    if (/\/auth\/|\/records|\/principals/.test(u)) {
      console.log(`  [resp ${res.status()}] ${res.request().method()} ${u.replace(BASE, '')}`)
    }
  })

  try {
    await page.goto(BASE, { waitUntil: 'networkidle' })
    console.log('url after load:', page.url())

    if (/sign-in/.test(page.url())) {
      console.log('clicking Admin demo button...')
      await page.getByRole('button', { name: /Admin/ }).first().click()
      // Give the login round-trip time, then report where we are.
      await page.waitForTimeout(4000)
      console.log('url 4s after click:', page.url())
    }

    // Don't wait for networkidle — the dashboard polls /readings continuously, so
    // the network never goes idle. Wait for DOM + a fixed settle instead.
    await page.goto(new URL('/dashboards', BASE).toString(), {
      waitUntil: 'domcontentloaded',
    })
    await page.waitForTimeout(4000)

    const bodyText = await page.locator('body').innerText()
    await page
      .screenshot({ path: '/tmp/e2e-dashboard.png', timeout: 5000 })
      .catch((e) => console.log('  (screenshot skipped:', e.message, ')'))
    console.log('screenshot → /tmp/e2e-dashboard.png')
    console.log('final url:', page.url())

    if (/No tenants/i.test(bodyText)) throw new Error('DASHBOARD EMPTY: "No tenants"')
    if (!(/Acme Industries/i.test(bodyText) || /Globex/i.test(bodyText)))
      throw new Error('No seeded tenant name on dashboard. Body: ' + bodyText.slice(0, 500))

    console.log('\n✅ PASS: dashboard shows tenants')
  } finally {
    await browser.close()
  }
}

main().catch((err) => {
  console.error('\n❌ FAIL:', err.message)
  process.exit(1)
})
