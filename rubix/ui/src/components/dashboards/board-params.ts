// Board-level time range → query parameters (§2/§3, LAMINAR-BORROW.md). This is
// the signature dashboard feature Laminar has and we lacked: one time control at
// the top of a board re-scopes every panel at once. It reuses the parameter
// substitution the SQL console already ships (applyParameters), so a panel's
// chart SQL referencing `{{start_time}}` / `{{end_time}}` / `{{interval_unit}}`
// is filled from the board's range before it runs. Panels with no placeholders
// are untouched — the range is opt-in per chart.

import { format, subDays, subHours } from 'date-fns'

export type IntervalUnit = 'minute' | 'hour' | 'day' | 'week'

export interface BoardTimeRange {
  start: Date
  end: Date
  interval: IntervalUnit
}

// Quick ranges, each with a sensible bucket width — the picker most dashboards
// lead with. `compute` is called when the range is selected (browser time).
export interface QuickRange {
  label: string
  interval: IntervalUnit
  compute: () => { start: Date; end: Date }
}

export const QUICK_RANGES: QuickRange[] = [
  { label: 'Last 1h', interval: 'minute', compute: () => ({ start: subHours(new Date(), 1), end: new Date() }) },
  { label: 'Last 24h', interval: 'hour', compute: () => ({ start: subHours(new Date(), 24), end: new Date() }) },
  { label: 'Last 7d', interval: 'day', compute: () => ({ start: subDays(new Date(), 7), end: new Date() }) },
  { label: 'Last 30d', interval: 'day', compute: () => ({ start: subDays(new Date(), 30), end: new Date() }) },
]

export const DEFAULT_RANGE: BoardTimeRange = (() => {
  const r = QUICK_RANGES[2] // Last 7d
  const { start, end } = r.compute()
  return { start, end, interval: r.interval }
})()

// Format a range into the `{{…}}` substitution map applyParameters consumes.
// Datetimes are emitted as 'yyyy-MM-dd HH:mm:ss.SSS' (quoted on substitution);
// interval_unit is a bare word (spliced unquoted, e.g. date_trunc('hour', …)).
export function formatBoardParams(range: BoardTimeRange): Record<string, string | number> {
  return {
    start_time: format(range.start, 'yyyy-MM-dd HH:mm:ss.SSS'),
    end_time: format(range.end, 'yyyy-MM-dd HH:mm:ss.SSS'),
    interval_unit: range.interval,
  }
}
