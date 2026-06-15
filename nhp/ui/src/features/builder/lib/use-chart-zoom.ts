/**
 * Drag-to-zoom selection state for a history chart
 * (docs/design/time-range-and-refresh.md §6). Tracks the in-progress brush as
 * the user drags across the plot and, on release, reports the selected
 * `{fromMs, toMs}` (clamped/ordered) so the caller sets the global time range.
 * Recharts hands category-axis (`t`) values on its mouse events; the rows carry
 * the matching epoch `ms`, so the handler maps the label back to an instant.
 */
import { useCallback, useState } from 'react'
import type { ChartRow } from './charts'

interface ChartMouseState {
  activeLabel?: string | number
}

export interface ChartZoom {
  /** The active selection's left label, for the `ReferenceArea` x1 (or undefined). */
  refLeft?: string
  /** The active selection's right label, for the `ReferenceArea` x2 (or undefined). */
  refRight?: string
  onMouseDown: (state: ChartMouseState | null) => void
  onMouseMove: (state: ChartMouseState | null) => void
  onMouseUp: () => void
}

function labelMs(
  rows: ChartRow[],
  label: string | undefined
): number | undefined {
  if (label === undefined) return undefined
  return rows.find((r) => r.t === label)?.ms
}

/**
 * The ordered `{fromMs, toMs}` a drag from `left` to `right` selects, or
 * `undefined` if it is not a real drag (missing/equal bounds or unmapped
 * labels). Pure, so the commit rule is unit-tested without a React harness; the
 * hook calls it on mouse-up.
 */
export function zoomSelection(
  rows: ChartRow[],
  left: string | undefined,
  right: string | undefined
): { fromMs: number; toMs: number } | undefined {
  if (left === undefined || right === undefined || left === right)
    return undefined
  const a = labelMs(rows, left)
  const b = labelMs(rows, right)
  if (a === undefined || b === undefined) return undefined
  return { fromMs: Math.min(a, b), toMs: Math.max(a, b) }
}

/**
 * Wire a chart's mouse drag to `onZoom`. Returns the ReferenceArea bounds for
 * the live selection and the three handlers to spread onto the recharts chart.
 * A click with no drag (same start/end label) is ignored so a stray click does
 * not collapse the range.
 */
export function useChartZoom(
  rows: ChartRow[],
  onZoom: (fromMs: number, toMs: number) => void
): ChartZoom {
  const [left, setLeft] = useState<string | undefined>()
  const [right, setRight] = useState<string | undefined>()

  const onMouseDown = useCallback((state: ChartMouseState | null) => {
    const label = state?.activeLabel
    setLeft(label === undefined ? undefined : String(label))
    setRight(undefined)
  }, [])

  const onMouseMove = useCallback(
    (state: ChartMouseState | null) => {
      if (left === undefined) return
      const label = state?.activeLabel
      setRight(label === undefined ? undefined : String(label))
    },
    [left]
  )

  const onMouseUp = useCallback(() => {
    const span = zoomSelection(rows, left, right)
    if (span) onZoom(span.fromMs, span.toMs)
    setLeft(undefined)
    setRight(undefined)
  }, [left, right, rows, onZoom])

  return { refLeft: left, refRight: right, onMouseDown, onMouseMove, onMouseUp }
}
