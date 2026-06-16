/**
 * The dashboard colour vocabulary, in ONE place (DASHBOARDS-SCOPE §7/§8: palette
 * via theme, threshold/severity colours portable). Two concerns:
 *  - a stable categorical series palette (multi-series charts: V L1/L2/L3),
 *  - the alarm severity ramp (DOMAIN-MODEL §Alarms: ok/warning/critical), which is
 *    the SAME ramp that both fires alarms and colours a chart.
 * Series colours are the theme's `--chart-1..5` CSS vars so the charts track
 * light/dark mode (styles/theme.css); recharts `stroke`/`fill` accept `var(--x)`
 * directly. Severity/status stay literal — they are SEMANTIC (ok/warn/critical,
 * online/offline) and must read the same in any theme.
 */
import type { AlarmSeverity } from '@/api/records'

/** Categorical series colours, cycled by series index — theme-driven (--chart-N). */
export const SERIES_COLORS = [
  'var(--chart-1)',
  'var(--chart-2)',
  'var(--chart-3)',
  'var(--chart-4)',
  'var(--chart-5)',
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
