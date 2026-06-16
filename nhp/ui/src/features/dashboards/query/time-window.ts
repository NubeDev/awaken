/**
 * Time windows for trend charts (DASHBOARDS-SCOPE §5: UTC windows + relative
 * tokens; charts show site-local time using the site `timezone`).
 *
 * History is read windowed and series-scoped through the historian (see batch.ts:
 * useSeriesHistory / useRegistersHistory). The board-state token is still applied
 * CLIENT-SIDE to the returned samples: a relative token like `now-24h` resolves to
 * an absolute UTC `[from,to)` at read time, and samples whose MEASUREMENT instant
 * `at` falls in the window are kept. Tokens (not absolute instants) live in board
 * state so "last 24h" stays fresh across refreshes — the §5 relative-token rule.
 */

/** A relative-window token; the value is the trailing hours. */
export const WINDOW_TOKENS = {
  'now-6h': 6,
  'now-24h': 24,
  'now-7d': 24 * 7,
} as const

export type WindowToken = keyof typeof WINDOW_TOKENS

export interface ResolvedWindow {
  /** Inclusive UTC start, epoch ms. */
  from: number
  /** Exclusive UTC end, epoch ms (resolves to "now"). */
  to: number
}

/** Resolve a relative token to an absolute UTC window at call time. */
export function resolveWindow(token: WindowToken, now = Date.now()): ResolvedWindow {
  return { from: now - WINDOW_TOKENS[token] * 3600_000, to: now }
}

/** Keep only the samples whose measurement instant `at` falls in `[from,to)`,
 *  ascending by `at`. */
export function withinWindow<T extends { at: string }>(
  rows: T[],
  window: ResolvedWindow
): T[] {
  return rows
    .filter((r) => {
      const t = Date.parse(r.at)
      return t >= window.from && t < window.to
    })
    .sort((a, b) => Date.parse(a.at) - Date.parse(b.at))
}

/**
 * Format a UTC instant for a chart tick in the SITE's local time (DASHBOARDS-SCOPE
 * §5: charts show site-local time using the site `timezone`). Falls back to the
 * browser zone when the site has no IANA `timezone` set.
 */
export function formatLocalTime(
  iso: string,
  timezone: string | undefined,
  withDate = false
): string {
  const d = new Date(iso)
  const opts: Intl.DateTimeFormatOptions = {
    hour: '2-digit',
    minute: '2-digit',
    ...(withDate ? { month: 'short', day: 'numeric' } : {}),
    ...(timezone ? { timeZone: timezone } : {}),
  }
  try {
    return new Intl.DateTimeFormat(undefined, opts).format(d)
  } catch {
    // An invalid IANA string (bad seed data) must not crash the axis.
    return new Intl.DateTimeFormat(undefined, { hour: '2-digit', minute: '2-digit' }).format(d)
  }
}
