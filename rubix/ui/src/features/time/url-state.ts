/**
 * URL round-trip for the dashboard time range + refresh interval
 * (docs/design/time-range-and-refresh.md §5): `?from=now-6h&to=now&refresh=30`.
 * Shares the mechanism the variable URL state introduced
 * (features/variables/url-state.ts): pure functions over a `URLSearchParams`,
 * preserving every unrelated param. `from`/`to` are relative tokens or absolute
 * instants; `refresh` is the interval in seconds (`0` = off). Absent params mean
 * "use the defaults", so a clean link carries no time noise.
 */
import {
  DEFAULT_RANGE,
  DEFAULT_REFRESH,
  isRefreshSecs,
  type RefreshSecs,
} from './presets'

export interface TimeUrlState {
  from: string
  to: string
  refresh: RefreshSecs
}

const FROM = 'from'
const TO = 'to'
const REFRESH = 'refresh'

/**
 * Read `?from/to/refresh` into a fully-populated state, falling back to the
 * defaults for any absent or malformed param. `from`/`to` must travel together —
 * a lone bound is ignored so a half-written link does not yield a nonsense range.
 */
export function readTimeParams(params: URLSearchParams): TimeUrlState {
  const from = params.get(FROM)
  const to = params.get(TO)
  const refreshRaw = params.get(REFRESH)

  const refreshNum = refreshRaw === null ? NaN : Number(refreshRaw)
  const refresh =
    refreshRaw !== null && isRefreshSecs(refreshNum)
      ? refreshNum
      : DEFAULT_REFRESH

  if (from && to) {
    return { from, to, refresh }
  }
  return { from: DEFAULT_RANGE.from, to: DEFAULT_RANGE.to, refresh }
}

/**
 * Write the time state back as `from/to/refresh` params on a copy of `base`,
 * preserving unrelated params (including the variable `var-*` ones). A state that
 * equals the defaults clears its params so shared links stay clean.
 */
export function writeTimeParams(
  base: URLSearchParams,
  state: TimeUrlState
): URLSearchParams {
  const next = new URLSearchParams(base)
  next.delete(FROM)
  next.delete(TO)
  next.delete(REFRESH)

  const isDefaultRange =
    state.from === DEFAULT_RANGE.from && state.to === DEFAULT_RANGE.to
  if (!isDefaultRange) {
    next.set(FROM, state.from)
    next.set(TO, state.to)
  }
  if (state.refresh !== DEFAULT_REFRESH) {
    next.set(REFRESH, String(state.refresh))
  }
  return next
}
