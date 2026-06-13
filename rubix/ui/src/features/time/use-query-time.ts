/**
 * The resolved time arguments a widget data fetch threads into its query
 * (docs/design/time-range-and-refresh.md §4). Reads the live `{from, to}` tokens
 * and the refresh `tick`/frozen `nowMs` from the store and derives:
 * - `timeRange` — the raw tokens passed to the server, which resolves `now`
 *   authoritatively against its own clock (the body bounds bind as parameters).
 * - `intervalSecs` — a bucket width derived from the resolved span and a target
 *   data-point count, so `$__timeGroup`/`$__interval` yield ~N points.
 * - `tickKey` — the resolved span snapped to the refresh tick, the cache-key
 *   discriminator that folds the range into invalidation without busting it every
 *   render (raw `now` ms would).
 *
 * The server is the source of truth for `now`; the client resolution here only
 * drives the interval derivation and the snapped cache key, never the bound
 * values sent on the wire.
 */
import type { TimeRangeBody } from '@/api/types'
import { useTimeStore } from '@/stores/time-store'
import { resolveRange, snapToTick } from './resolve'

/** Target points per series — the interval is sized to land near this count. */
const TARGET_POINTS = 300

/** Bucket the resolved span into ~`TARGET_POINTS` buckets, min 1 second. */
export function deriveIntervalSecs(fromMs: number, toMs: number): number {
  const spanSecs = Math.max(1, Math.round((toMs - fromMs) / 1000))
  return Math.max(1, Math.round(spanSecs / TARGET_POINTS))
}

export interface QueryTime {
  timeRange: TimeRangeBody
  intervalSecs: number
  /** Resolved bounds (client mirror), for `point_history` start/end wiring. */
  fromMs: number
  toMs: number
  /** Stable cache discriminator: snapped span + tick. */
  tickKey: string
}

export function useQueryTime(): QueryTime {
  const from = useTimeStore((s) => s.from)
  const to = useTimeStore((s) => s.to)
  const refresh = useTimeStore((s) => s.refresh)
  const tick = useTimeStore((s) => s.tick)
  const nowMs = useTimeStore((s) => s.nowMs)

  const { fromMs, toMs } = resolveRange(from, to, nowMs)
  const intervalSecs = deriveIntervalSecs(fromMs, toMs)

  // Snap to the refresh interval (or a 30 s floor when paused) so the key is
  // stable between refreshes; the `tick` makes a manual refresh re-fetch.
  const tickMs = (refresh > 0 ? refresh : 30) * 1000
  const snappedFrom = snapToTick(fromMs, tickMs)
  const snappedTo = snapToTick(toMs, tickMs)
  const tickKey = `${snappedFrom}:${snappedTo}:${intervalSecs}:${tick}`

  return {
    timeRange: { from, to },
    intervalSecs,
    fromMs,
    toMs,
    tickKey,
  }
}
