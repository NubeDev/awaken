/**
 * The dashboard colour vocabulary, in ONE place (DASHBOARDS-SCOPE §7/§8: palette
 * via theme, threshold/severity colours portable). Two concerns:
 *  - a stable categorical series palette (multi-series charts: V L1/L2/L3),
 *  - the alarm severity ramp (DOMAIN-MODEL §Alarms: ok/warning/critical), which is
 *    the SAME ramp that both fires alarms and colours a chart.
 * Kept as literal hsl strings (not CSS-var lookups) so recharts `stroke`/`fill`
 * props take them directly; light enough for the POC, swap to theme vars later.
 */
import type { AlarmSeverity } from '@/api/records'

/** Categorical series colours, cycled by series index. */
export const SERIES_COLORS = [
  '#2563eb', // blue
  '#16a34a', // green
  '#ea580c', // orange
  '#9333ea', // violet
  '#0891b2', // cyan
  '#dc2626', // red
  '#ca8a04', // amber
  '#4f46e5', // indigo
] as const

export function seriesColor(index: number): string {
  return SERIES_COLORS[index % SERIES_COLORS.length]
}

/** Alarm severity → colour. `ok` is the neutral baseline (no alarm). */
export const SEVERITY_COLORS: Record<AlarmSeverity, string> = {
  ok: '#16a34a',
  warning: '#ca8a04',
  critical: '#dc2626',
}

/** Status pill colours (online/offline/unknown + a derived "degraded" rollup). */
export const STATUS_COLORS: Record<string, string> = {
  online: '#16a34a',
  offline: '#dc2626',
  degraded: '#ca8a04',
  unknown: '#9ca3af',
}
