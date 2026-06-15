// One dashboard panel: renders a single chart from a query result the board
// fetched in its batch (§3). The board issues ONE `POST /query/batch` keyed by
// chart id and hands each panel its slice, so a panel no longer runs its own
// request — it is a pure renderer of `{ rows, columns, error, loading }`. The
// header is the drag handle (`.panel-drag`); the × removes the panel.

import { useMemo } from 'react'
import { GripVertical, X } from 'lucide-react'
import type { SavedChart } from '../../api/charts'
import type { QueryColumn } from '../../api/query'
import { ChartRendererCore } from '../chart-builder/charts'
import { transformDataToColumns, type DataRow } from '../chart-builder/utils'

/** The board-supplied state of this panel's query. */
export interface PanelResult {
  rows?: Record<string, unknown>[]
  /** Backend column types, when present (preferred over sniffing rows). */
  columns?: QueryColumn[]
  error?: string
  loading?: boolean
}

interface ChartPanelProps {
  chart: SavedChart
  syncId: string
  onRemove: () => void
  /** This panel's slice of the board's batch query. */
  result: PanelResult
}

export function ChartPanel({ chart, syncId, onRemove, result }: ChartPanelProps) {
  const rows = result.rows ?? []
  // Prefer the backend's column types; fall back to sniffing the first row when a
  // panel renders before the batch lands (or for an empty result).
  const columns = useMemo(() => {
    if (result.columns && result.columns.length > 0) {
      return result.columns.map((c) => ({ name: c.name, type: coarseType(c.type) }))
    }
    return transformDataToColumns(rows as DataRow[])
  }, [result.columns, rows])

  return (
    <div className="flex h-full flex-col overflow-hidden rounded-xl border border-border bg-card/40">
      <div className="panel-drag flex cursor-move items-center gap-1.5 border-b border-border px-3 py-2">
        <GripVertical size={14} className="text-muted-foreground" />
        <span className="truncate text-sm font-medium">{chart.name}</span>
        <button
          onClick={onRemove}
          // Excluded from drag via the grid's dragConfig.cancel selector.
          className="panel-no-drag ml-auto rounded p-0.5 text-muted-foreground hover:bg-muted hover:text-foreground"
          aria-label="Remove panel"
        >
          <X size={14} />
        </button>
      </div>
      <div className="min-h-0 flex-1 p-3">
        {result.loading ? (
          <div className="grid h-full place-items-center text-sm text-muted-foreground">Loading…</div>
        ) : result.error ? (
          <div className="grid h-full place-items-center px-3 text-center text-xs text-destructive">
            {result.error}
          </div>
        ) : rows.length === 0 ? (
          <div className="grid h-full place-items-center text-sm text-muted-foreground">No rows.</div>
        ) : (
          <ChartRendererCore config={chart.config} data={rows} columns={columns} syncId={syncId} />
        )}
      </div>
    </div>
  )
}

// Map the backend's coarse column type onto the chart layer's narrower set.
// `timestamp`/`other` render as strings on the axis (the instant is still the raw
// value); numbers and booleans keep their kind.
function coarseType(type: QueryColumn['type']): 'string' | 'number' | 'boolean' {
  if (type === 'number') return 'number'
  if (type === 'boolean') return 'boolean'
  return 'string'
}
