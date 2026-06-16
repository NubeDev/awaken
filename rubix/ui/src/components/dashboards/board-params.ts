// Board-level time range → a structured, UTC time scope (DASHBOARDS-SCOPE.md §5).
// One time control at the top of a board re-scopes every panel at once. The board
// no longer formats a locale datetime string into SQL — that was the timezone bug
// (a UTC+10 browser asking for "last 1h" compared a local window against UTC
// `created`). Instead it sends absolute UTC epoch milliseconds plus a grain, and
// the backend injects the window/bucket by expanding the chart's `$__timeFilter`
// / `$__timeBucket` / `$__interval` macros. Charts opt in by using those macros;
// a panel with none ignores the range.

import { subDays, subHours } from 'date-fns'

import type { Grain, TimeScope } from '../../api/query'

export type IntervalUnit = Grain

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

// Convert a range into the structured TimeScope the query API sends. A JS Date's
// getTime() is epoch milliseconds in UTC regardless of the browser's timezone, so
// this is timezone-correct by construction — the fix for the old wall-clock bug.
export function boardTimeScope(range: BoardTimeRange): TimeScope {
  return {
    from: range.start.getTime(),
    to: range.end.getTime(),
    grain: range.interval,
  }
}
