// One dashboard panel: runs its chart's SQL and renders it via the vendored
// chart layer. Self-contained — given a `kind:"chart"` record it fetches and
// draws independently, so a board is just a grid of these. The header is the
// drag handle (`.panel-drag`); the × removes the panel from the board.

import { useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { GripVertical, X } from 'lucide-react'
import { useApi } from '../../api/ConnectionContext'
import { runQuery } from '../../api/query'
import type { SavedChart } from '../../api/charts'
import { ChartRendererCore } from '../chart-builder/charts'
import { transformDataToColumns, type DataRow } from '../chart-builder/utils'

interface ChartPanelProps {
  tenant: string
  chart: SavedChart
  syncId: string
  onRemove: () => void
}

export function ChartPanel({ tenant, chart, syncId, onRemove }: ChartPanelProps) {
  const api = useApi(tenant)

  const q = useQuery({
    queryKey: ['chart-panel', tenant, chart.id, chart.sql],
    queryFn: () => runQuery(api, chart.sql),
  })

  const rows = q.data?.rows ?? []
  const columns = useMemo(() => transformDataToColumns(rows as DataRow[]), [rows])

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
        {q.isPending ? (
          <div className="grid h-full place-items-center text-sm text-muted-foreground">Loading…</div>
        ) : q.error ? (
          <div className="grid h-full place-items-center px-3 text-center text-xs text-destructive">
            {(q.error as Error).message}
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
