// Drag-select-to-zoom for the recharts wrappers (§3, LAMINAR-BORROW.md). The
// vendored line/bar charts already accept a `drag` handler set and paint a
// `ReferenceArea` while a range is being dragged; this hook owns that state and
// turns a committed drag into a zoom window, then filters the rows to it. Zoom is
// client-side over the rows already fetched (no re-query) — the cheap, contained
// version of Laminar's "drag a time range" behaviour.

import { useCallback, useMemo, useState } from 'react'

import { type ChartDragHandlers } from './line-chart'

// recharts hands the mouse callbacks a state object carrying the active x label.
interface ActiveState {
  activeLabel?: string | number
}

interface UseChartZoom {
  /** Handlers to spread onto `ChartRendererCore`'s `drag` prop. */
  drag: ChartDragHandlers
  /** Whether a zoom window is currently applied. */
  zoomed: boolean
  /** Clear the zoom window. */
  reset: () => void
  /** Filter `rows` to the active zoom window over column `x` (identity if none). */
  apply: (rows: Record<string, any>[], x?: string) => Record<string, any>[]
}

export function useChartZoom(): UseChartZoom {
  const [refArea, setRefArea] = useState<{ left?: string; right?: string }>({})
  const [zoom, setZoom] = useState<{ left: string; right: string } | null>(null)

  const onMouseDown = useCallback((s: ActiveState) => {
    if (s?.activeLabel != null) setRefArea({ left: String(s.activeLabel) })
  }, [])

  const onMouseMove = useCallback(
    (s: ActiveState) => {
      if (refArea.left != null && s?.activeLabel != null) {
        setRefArea((r) => ({ ...r, right: String(s.activeLabel) }))
      }
    },
    [refArea.left],
  )

  const onMouseUp = useCallback(() => {
    const { left, right } = refArea
    if (left != null && right != null && left !== right) {
      setZoom({ left, right })
    }
    setRefArea({})
  }, [refArea])

  const reset = useCallback(() => setZoom(null), [])

  const apply = useCallback(
    (rows: Record<string, any>[], x?: string) => {
      if (!zoom || !x) return rows
      // x values are categorical; window by their first-seen order so the slice
      // is correct regardless of left/right drag direction.
      const order: string[] = []
      const seen = new Set<string>()
      for (const row of rows) {
        const v = String(row[x])
        if (!seen.has(v)) {
          seen.add(v)
          order.push(v)
        }
      }
      const a = order.indexOf(zoom.left)
      const b = order.indexOf(zoom.right)
      if (a < 0 || b < 0) return rows
      const [lo, hi] = a <= b ? [a, b] : [b, a]
      const window = new Set(order.slice(lo, hi + 1))
      return rows.filter((row) => window.has(String(row[x])))
    },
    [zoom],
  )

  const drag = useMemo<ChartDragHandlers>(
    () => ({ onMouseDown, onMouseMove, onMouseUp, refArea }),
    [onMouseDown, onMouseMove, onMouseUp, refArea],
  )

  return { drag, zoomed: zoom !== null, reset, apply }
}
