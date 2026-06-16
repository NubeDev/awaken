/**
 * Drive the dashboard's auto-refresh tick (DASHBOARDS-SCOPE §6: "one refresh per
 * tick", visibility-aware). The container resolves an interval (use-refresh.ts,
 * `false` when paused/off); this hook re-runs every `['dash']` query on that
 * interval. Kept separate from use-refresh.ts so the visibility/interval logic and
 * the invalidation effect are one-responsibility files.
 */
import { useEffect } from 'react'
import { useQueryClient } from '@tanstack/react-query'

export function useDashboardRefetch(intervalMs: number | false): void {
  const qc = useQueryClient()
  useEffect(() => {
    if (intervalMs === false || intervalMs <= 0) return
    const id = window.setInterval(() => {
      qc.invalidateQueries({ queryKey: ['dash'] })
    }, intervalMs)
    return () => window.clearInterval(id)
  }, [intervalMs, qc])
}
