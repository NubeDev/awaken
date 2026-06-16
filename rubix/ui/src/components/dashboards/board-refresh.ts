// Board auto-refresh — visibility-aware polling + a snapped tick (§6).
//
// Two jobs, both about NOT busting the query cache on every render:
//
//   1. A refresh interval (Off · 5s · 10s · 30s · 1m · 5m) drives TanStack's
//      `refetchInterval`. The minimum live interval is 5s to match the backend's
//      scoped-context cache TTL (§4a) — polling faster than the cache turns over
//      buys nothing but load.
//   2. A monotonic *tick* that advances once per interval, but PAUSES while the
//      tab is hidden and catches up the moment it returns (document
//      visibilitychange). The tick is what re-derives the live window, so a board
//      in a background tab stops hammering the backend and resumes on focus.
//
// Snapping: a relative range ("last 1h") recomputed from `now` every render would
// produce a new `from`/`to` each time and bust the cache (and miss §4a's
// server-side hit, which keys on a resolved time snapshot). So we quantise both
// bounds DOWN to the current tick boundary before they enter the query key. Within
// a tick every render sees identical bounds → a guaranteed cache hit; the window
// only moves when the tick advances.

import { useEffect, useState } from 'react'

/** Refresh interval in milliseconds, or `null` for Off. */
export type RefreshInterval = number | null

export interface RefreshOption {
  label: string
  value: RefreshInterval
}

// 5s floor aligns with the backend cache TTL (§4a) — see module note.
export const REFRESH_OPTIONS: RefreshOption[] = [
  { label: 'Off', value: null },
  { label: '5s', value: 5_000 },
  { label: '10s', value: 10_000 },
  { label: '30s', value: 30_000 },
  { label: '1m', value: 60_000 },
  { label: '5m', value: 300_000 },
]

export const DEFAULT_REFRESH: RefreshInterval = null

/**
 * Quantise an epoch-ms instant DOWN to the nearest `interval` boundary, so a
 * relative window resolved from `now` is stable within a tick. With no live
 * interval (Off), snap to a coarse 5s grid so even a static board doesn't bust
 * the cache on incidental re-renders.
 */
export function snapInstant(epochMs: number, interval: RefreshInterval): number {
  const step = interval ?? 5_000
  return Math.floor(epochMs / step) * step
}

/**
 * A visibility-aware monotonic tick. Returns an integer that increments every
 * `interval` ms while the tab is visible; the timer is torn down when hidden and
 * a catch-up tick fires immediately on return. `interval === null` (Off) freezes
 * the tick so the board holds its last snapshot.
 *
 * The returned value is only meant to be folded into a query key / used as a
 * recompute trigger — its magnitude is not significant.
 */
export function useRefreshTick(interval: RefreshInterval): number {
  const [tick, setTick] = useState(0)

  useEffect(() => {
    if (interval === null) return
    let timer: ReturnType<typeof setInterval> | null = null

    const start = () => {
      if (timer !== null) return
      timer = setInterval(() => setTick((t) => t + 1), interval)
    }
    const stop = () => {
      if (timer !== null) {
        clearInterval(timer)
        timer = null
      }
    }
    const onVisibility = () => {
      if (document.visibilityState === 'visible') {
        // Catch up immediately on return, then resume ticking.
        setTick((t) => t + 1)
        start()
      } else {
        stop()
      }
    }

    if (document.visibilityState === 'visible') start()
    document.addEventListener('visibilitychange', onVisibility)
    return () => {
      stop()
      document.removeEventListener('visibilitychange', onVisibility)
    }
  }, [interval])

  return tick
}
