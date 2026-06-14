/**
 * Time-range and refresh presets (docs/design/time-range-and-refresh.md §§1-2):
 * the quick-range list the picker offers and the auto-refresh interval options.
 * Kept as pure data so the store, picker, and tests share one source of truth.
 */

/** A relative `{from, to}` range token pair plus its menu label. */
export interface QuickRange {
  label: string
  from: string
  to: string
}

/**
 * Quick ranges, in menu order. Relative tokens (resolved client- and server-side
 * against one frozen `now`). "Today"/"Yesterday" use `now/d` day-floor rounding.
 */
export const QUICK_RANGES: readonly QuickRange[] = [
  { label: 'Last 5 minutes', from: 'now-5m', to: 'now' },
  { label: 'Last 15 minutes', from: 'now-15m', to: 'now' },
  { label: 'Last 1 hour', from: 'now-1h', to: 'now' },
  { label: 'Last 6 hours', from: 'now-6h', to: 'now' },
  { label: 'Last 24 hours', from: 'now-24h', to: 'now' },
  { label: 'Last 7 days', from: 'now-7d', to: 'now' },
  { label: 'Last 30 days', from: 'now-30d', to: 'now' },
  { label: 'Today', from: 'now/d', to: 'now' },
  { label: 'Yesterday', from: 'now-1d/d', to: 'now/d' },
] as const

/**
 * Auto-refresh interval, seconds (0 = off). `5` preserves the prior blanket
 * `LIVE_INTERVAL` 5 s behaviour as the default "live" preset.
 */
export type RefreshSecs = 0 | 5 | 10 | 30 | 60 | 300 | 900

export interface RefreshOption {
  label: string
  secs: RefreshSecs
}

export const REFRESH_OPTIONS: readonly RefreshOption[] = [
  { label: 'Off', secs: 0 },
  { label: '5s', secs: 5 },
  { label: '10s', secs: 10 },
  { label: '30s', secs: 30 },
  { label: '1m', secs: 60 },
  { label: '5m', secs: 300 },
  { label: '15m', secs: 900 },
] as const

/** The default refresh interval — 5 s, matching the prior live-poll behaviour. */
export const DEFAULT_REFRESH: RefreshSecs = 5

/** The default range when no `?from/to` is present — last 6 hours. */
export const DEFAULT_RANGE: QuickRange = {
  label: 'Last 6 hours',
  from: 'now-6h',
  to: 'now',
}

/** Whether a refresh-secs value is one of the offered presets. */
export function isRefreshSecs(n: number): n is RefreshSecs {
  return REFRESH_OPTIONS.some((o) => o.secs === n)
}
