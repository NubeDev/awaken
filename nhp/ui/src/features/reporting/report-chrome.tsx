/**
 * Shared print chrome for the reporting surfaces. PDF export is print-based (no
 * new dependency): `PrintStyles` injects a stylesheet that hides the app shell and
 * the on-screen controls and prints only the element with id `REPORT_ID`;
 * `printDocument` sets a sensible PDF filename (the document title) and opens the
 * browser print dialog. recharts draws SVG, so charts print crisp.
 *
 * `scopeSummary` turns a `ScopeFilter` (+ window) into the human labels the
 * printed header shows, so a handed-off PDF is self-describing.
 */
import type { PortfolioIndex, ScopeFilter } from './scope'

export const REPORT_ID = 'reporting-report'

export function PrintStyles() {
  return (
    <style>{`
      @media print {
        body * { visibility: hidden !important; }
        #${REPORT_ID}, #${REPORT_ID} * { visibility: visible !important; }
        #${REPORT_ID} { position: absolute !important; left: 0; top: 0; width: 100%; }
        .report-avoid-break { break-inside: avoid; }
        @page { margin: 14mm; }
      }
    `}</style>
  )
}

/** Open the print dialog with a meaningful default filename, then restore title. */
export function printDocument(filename: string) {
  const prev = document.title
  document.title = filename
  window.print()
  document.title = prev
}

export interface ScopeLabels {
  tenant: string
  site: string
  meterType: string
  quantity: string
}

export function scopeSummary(
  index: PortfolioIndex,
  filter: ScopeFilter
): ScopeLabels {
  const tenant = filter.tenantId
    ? (index.data.tenants.find((t) => t.id === filter.tenantId)?.content.name ??
      '—')
    : 'All tenants'
  const site = filter.siteId
    ? (index.data.sites.find((s) => s.id === filter.siteId)?.content.name ?? '—')
    : filter.tenantId
      ? 'All sites (tenant)'
      : 'All sites'
  const meterType = filter.meterTypeId
    ? index.meterTypeName(filter.meterTypeId)
    : 'All meter-types'
  const quantity = filter.quantity ?? 'All quantities'
  return { tenant, site, meterType, quantity }
}
