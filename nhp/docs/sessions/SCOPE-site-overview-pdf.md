# Scope — Site Overview PDF export

**One-liner:** a new "Site Overview" report that prints a full, single-document
snapshot of one site — status, KPIs, per-gateway breakdown, energy, and active
alarms — exported to PDF through the existing browser-print path.

## Why / what the user gets

Today reports are metric-slices (consumption, trend, alarms, raw). There's no
single "give me everything about this site" document — the thing you hand a facility
manager or attach to a ticket. This adds that: pick a site, click **Export PDF**,
get a clean multi-section overview on one print job.

## Guiding constraints (don't re-litigate)

- **Reuse, don't rebuild.** The PDF mechanism (`PrintStyles` + `printDocument`,
  `report-chrome.tsx`), the data layer (`usePortfolio`, `useWindowedHistory`,
  `useLatestReadings`), and the assembled site view (`buildSiteBoard` →
  `SiteBoard` with `SiteKpis` / `GatewayCard[]` / `powerPanel`) all already exist.
  The site overview is a new **report variant**, not new infrastructure.
- **No new queries / no backend.** Everything comes from the records + readings
  already fetched for the dashboards (frozen-rubix rule holds). `buildSiteBoard`
  and `activeAlarms` are pure over data we already have.
- **Browser-print, DOM-based.** Same as the other reports — recharts SVG prints
  crisp; `.report-avoid-break` keeps sections whole; `@page { margin: 14mm }`.
- **Honest rollups only.** Every number is a rollup of fetched records (the
  existing builders already guarantee this — no fabricated values).

## In scope

A `SiteOverviewReport` registered as a new report `type` (`'site-overview'`),
selected like the others, requiring a **site** in the scope filter. Its body, top
to bottom:

1. **Header band** — site name, tenant, address/timezone, generated-at, window.
   (Report chrome already renders tenant/site/window/generated; add the site's
   address + timezone + an overall status pill.)
2. **Status + KPI strip** — site rollup status (`rollupStatus` over gateways),
   then `SiteKpis`: gateways, meters, **active alarms**, total energy. Reuse the
   dashboard `KpiTile` look (or a print-friendly equivalent).
3. **Active alarms section** — `activeAlarms(index, registersInSite, latest)`
   with the severity counts chip + table. Reuse `AlarmCounts` / `AlarmTable`
   (the same `alarmsOnly + includeNoHistory` selection the console uses, so
   low-battery/gauge alarms show). Empty state: "No active alarms."
4. **Gateways breakdown** — one row/card per `GatewayCard`: name, status +
   last_seen, meter count, alarm count, energy sparkline + latest. This is the
   site's spine; `buildSiteBoard` returns it ready.
5. **Energy / power panel** — the `powerPanel` (cross-meter power) trend if the
   site logs power, plus the site energy total from `SiteKpis.energy`. Skip the
   chart cleanly when there's no power/energy (LoRa-only sites).
6. **Device inventory table** — a flat per-meter list under the site: meter,
   gateway, network **protocol** (modbus/lora), meter-type, headline reading
   (`primaryMetric` value+unit), status. Walk `index` filtered to the site
   (`selectMeters` + `meterLocation` + `registersByMeter`). This is the part not
   already on any single dashboard page and the main new assembly work.

Each numbered section is wrapped in `.report-avoid-break` so it doesn't split
across a page.

## Out of scope (say no)

- No new export engine (jspdf/@react-pdf), no server-side PDF, no scheduled/email
  delivery, no multi-site batch ("portfolio book").
- No new endpoints, no readings beyond the window already fetched.
- No editable/branded templates, logos, cover page (a later polish pass).
- No per-meter deep detail (that's the meter dashboard page); the inventory table
  is one row per meter, not a sub-report each.

## How (build outline)

1. **`reports/site-overview.tsx`** — `SiteOverviewReport({ index, filter, token })`.
   - Guard: if `!filter.siteId`, render a "Pick a site to generate an overview"
     prompt (this report is site-required, unlike consumption/trend).
   - Resolve the site key from `filter.siteId`; gather the site's meters/registers
     from `index`; call `buildSiteBoard(siteKey, …, token, timezone)` with the
     records from `index.data` + windowed history (`useWindowedHistory` over the
     site's registers) — mirrors how `consumption.tsx` pulls history.
   - For alarms: `selectRegisters(index, filter, { alarmsOnly: true,
     includeNoHistory: true })` + `useLatestReadings` + `activeAlarms`.
   - Render the six sections from reused widgets.
2. **Register the variant** — add `'site-overview'` to the report `type` union /
   the report picker in `reports-page.tsx`, with a label ("Site overview") and the
   `windowed: true` flag. Wire it into the same `id={REPORT_ID}` body so
   `exportPdf()` already covers it; filename → `site-overview-<site>-<date>`.
3. **Site-required UX** — when this type is chosen, the filter bar should make the
   site picker prominent / the Export button disabled until a site is set (small
   guard in `reports-page.tsx`).
4. **(Optional) one-click entry** — an "Export overview" button on the **site
   dashboard page** (`site-page.tsx`) that deep-links to
   `/reports?type=site-overview&tenant=…&site=…`, so the export is reachable from
   where you're already looking at the site. Nice-to-have, not required for done.

## Files

- New: `ui/src/features/reporting/reports/site-overview.tsx`
- Edit: `ui/src/features/reporting/reports-page.tsx` (register type + label +
  site-required guard + filename), report `type` union wherever it's declared.
- Reuse (no change): `report-chrome.tsx`, `use-portfolio.ts`, `scope.ts`,
  `alarms.ts`, `alarm-view.tsx`, `auto-build/site-board.ts`,
  `auto-build/primary-metric.ts`, `auto-build/rollup.ts`.
- Optional edit: `ui/src/features/dashboards/pages/site-page.tsx` (entry button).

## Done = (acceptance)

1. Reports → choose **Site overview**, pick a site → the page renders all six
   sections with real numbers (status, KPIs, alarms, gateways, energy, inventory).
2. **Export PDF** produces a single document containing every section, no section
   split mid-page, charts crisp.
3. A **LoRa-heavy site** (e.g. Acme HQ / Acme Plant carpark) renders correctly:
   inventory shows lora protocol + °C/ppm/% units, the low-battery / CO / temp
   alarms appear in the alarms section, and the energy panel degrades gracefully
   when there's little/no power.
4. With no site selected, the report prompts for one and Export is disabled.
5. `pnpm test:unit`, `tsc -b`, and `pnpm lint` stay green; any new pure assembly
   helper gets a small unit test (matches the `primary-metric.unit.test.ts`
   pattern).

## Estimate

~½ day. One new component + a small registration edit; all data + PDF plumbing is
reuse. The only genuinely new assembly is the device-inventory table (a straight
walk of `index` filtered to the site).
