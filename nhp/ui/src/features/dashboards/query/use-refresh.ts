/**
 * Visibility-aware auto-refresh (DASHBOARDS-SCOPE §6: "visibility-aware timer;
 * pause on hidden tabs; catch up on return"). A board picks a refresh interval
 * (Off · 30s · 1m · 5m); this hook returns the value to feed React Query's
 * `refetchInterval`, going `false` (paused) whenever the tab is hidden so a
 * backgrounded board stops polling, and resuming + refetching on return.
 *
 * POC choice (documented per WS-07): refresh is a TIMER, not a `/ws/records`
 * live subscription. The spec lists live updates as a nice-to-have and explicitly
 * blesses a visibility-aware refetch as "sufficient for the POC if WS is fiddly".
 * rubix's `/ws/records` exists but wiring a WS client + per-principal row filter +
 * debounced invalidation is out of the POC timebox; a timer over the small seed
 * gives the same observable result (a status flip / new sample appears within a
 * tick) for far less surface. Swap to WS later behind this same hook.
 */
import { useEffect, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'

export const REFRESH_OPTIONS = [
  { label: 'Off', ms: 0 },
  { label: '30s', ms: 30_000 },
  { label: '1m', ms: 60_000 },
  { label: '5m', ms: 300_000 },
] as const

export type RefreshMs = (typeof REFRESH_OPTIONS)[number]['ms']

/** Track tab visibility so a hidden board pauses its timer. */
function useTabVisible(): boolean {
  const [visible, setVisible] = useState(() =>
    typeof document === 'undefined' ? true : document.visibilityState === 'visible'
  )
  useEffect(() => {
    const onChange = () => setVisible(document.visibilityState === 'visible')
    document.addEventListener('visibilitychange', onChange)
    return () => document.removeEventListener('visibilitychange', onChange)
  }, [])
  return visible
}

/**
 * Returns the `refetchInterval` value (ms, or `false` to pause) for the chosen
 * interval, paused while hidden. On becoming visible again it triggers one
 * immediate refetch of the dashboard queries so the board catches up.
 */
export function useRefreshInterval(intervalMs: RefreshMs): number | false {
  const visible = useTabVisible()
  const qc = useQueryClient()
  useEffect(() => {
    if (visible && intervalMs > 0) qc.invalidateQueries({ queryKey: ['dash'] })
  }, [visible, intervalMs, qc])
  if (!visible || intervalMs === 0) return false
  return intervalMs
}
