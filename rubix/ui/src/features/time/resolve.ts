/**
 * Relative-range resolver (docs/design/time-range-and-refresh.md §1): turn a
 * `{from, to}` where each bound is an absolute RFC 3339 instant **or** a relative
 * token (`now`, `now-6h`, `now/d`) into concrete millisecond bounds against one
 * frozen `now`. The server resolves `now` authoritatively per query (WS-03); this
 * client mirror powers the picker preview and the `point_history` `start`/`end`
 * wiring, and snaps the cache key. A bound that cannot be parsed falls through to
 * an error the caller surfaces — never a silent wrong range.
 */

/** Relative token units, mapping to milliseconds. */
const UNIT_MS: Record<string, number> = {
  s: 1_000,
  m: 60_000,
  h: 3_600_000,
  d: 86_400_000,
  w: 604_800_000,
}

/** Units `now/<unit>` rounding (floor) supports, longest first for matching. */
const ROUND_MS: Record<string, number> = {
  d: 86_400_000,
  h: 3_600_000,
  m: 60_000,
  w: 604_800_000,
}

export class TimeRangeError extends Error {
  constructor(token: string) {
    super(`Unparseable time token: "${token}"`)
    this.name = 'TimeRangeError'
  }
}

/**
 * Resolve one bound to an epoch-ms instant against `nowMs`. Accepts:
 * - `now` → `nowMs`
 * - `now-<n><unit>` / `now+<n><unit>` → offset by n units (s/m/h/d/w)
 * - `now/<unit>` → floor `nowMs` to the start of the unit (d/h/m/w)
 * - `now-<n><unit>/<unit>` → offset then floor
 * - an absolute RFC 3339 / ISO instant → its epoch ms
 * Throws `TimeRangeError` on anything else (no silent fallback).
 */
export function resolveBound(token: string, nowMs: number): number {
  const t = token.trim()
  if (!t.startsWith('now')) {
    const parsed = Date.parse(t)
    if (Number.isNaN(parsed)) throw new TimeRangeError(token)
    return parsed
  }

  // Split an optional trailing `/unit` rounding off the offset expression.
  const slash = t.indexOf('/')
  const offsetPart = slash === -1 ? t : t.slice(0, slash)
  const roundPart = slash === -1 ? '' : t.slice(slash + 1)

  let ms = nowMs
  if (offsetPart !== 'now') {
    const m = /^now([+-])(\d+)([smhdw])$/.exec(offsetPart)
    if (!m) throw new TimeRangeError(token)
    const [, sign, count, unit] = m
    const delta = Number(count) * UNIT_MS[unit]
    ms += sign === '-' ? -delta : delta
  }

  if (roundPart !== '') {
    const unitMs = ROUND_MS[roundPart]
    if (unitMs === undefined) throw new TimeRangeError(token)
    // Floor to the unit boundary in UTC. For day/week this snaps to UTC
    // midnight; the picker labels remain user-tz, but bounds bind as UTC
    // instants so server and client agree (no per-widget skew).
    ms = Math.floor(ms / unitMs) * unitMs
  }

  return ms
}

export interface ResolvedRange {
  fromMs: number
  toMs: number
}

/**
 * Resolve a `{from, to}` pair against one frozen `now`. Both bounds share the
 * single `nowMs` so a fan-out of widgets in one refresh agree on the instant
 * (docs/design/time-range-and-refresh.md "Freeze one `now` per refresh").
 */
export function resolveRange(
  from: string,
  to: string,
  nowMs: number
): ResolvedRange {
  return {
    fromMs: resolveBound(from, nowMs),
    toMs: resolveBound(to, nowMs),
  }
}

/**
 * Snap an epoch-ms instant down to a tick boundary so the resolved range folds
 * into a query cache key without busting every render. Raw `now` ms changes each
 * render; snapping to the refresh tick (or a coarse default when paused) keeps a
 * stable key between refreshes (docs/design/time-range-and-refresh.md
 * "Cache-key snapping").
 */
export function snapToTick(ms: number, tickMs: number): number {
  if (tickMs <= 0) return ms
  return Math.floor(ms / tickMs) * tickMs
}
