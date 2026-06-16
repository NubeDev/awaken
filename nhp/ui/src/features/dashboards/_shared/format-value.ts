/**
 * Display formatting for a register value (DASHBOARDS-SCOPE §1/§2: "present on the
 * frontend"). A register carries `precision` (decimals) and a `unit` LABEL.
 *
 * NOTE — units are rendered as FIXED labels, not converted. Live unit conversion
 * waits on the rubix `/prefs` endpoint (OVERVIEW gap #3 / DASHBOARDS.md §Query):
 * the converter exists in rubix-prefs but has no HTTP surface, so the POC shows the
 * register's stored `unit` string verbatim. Documented, not a bug.
 */
export function formatValue(
  value: number | null | undefined,
  opts: { precision?: number; unit?: string } = {}
): string {
  if (value === null || value === undefined || Number.isNaN(value)) return '—'
  const decimals = opts.precision ?? 2
  const num = value.toFixed(decimals)
  return opts.unit ? `${num} ${opts.unit}` : num
}

/** A short axis-tick number (no unit), trimming trailing zeros. */
export function formatTick(value: number): string {
  if (!Number.isFinite(value)) return ''
  return Number(value.toFixed(2)).toString()
}
