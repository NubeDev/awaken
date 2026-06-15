// The responsive board grid (§2, LAMINAR-BORROW.md) — react-grid-layout v2 with
// drag/resize. Children are matched to layout items by `key === panel.chart_id`.
// Layout changes bubble up via `onLayoutChange`; the page debounces them into one
// gate write per settle (so every drag/resize is audited, never thrashed). A
// shared `syncId` ties tooltips/cursors across every panel.
//
// The board fetches every panel in ONE `POST /query/batch` keyed by chart id
// (§3) — one round trip and one consistent snapshot — and hands each panel its
// result. A single bad panel shows its error while the others render.

import { useMemo, type RefObject } from 'react'
import { useQuery } from '@tanstack/react-query'
import { GridLayout, type Layout, useContainerWidth } from 'react-grid-layout'
import { useApi } from '../../api/ConnectionContext'
import type { SavedChart } from '../../api/charts'
import type { BoardPanel } from '../../api/boards'
import { runBatchQuery, type BatchQueryItem, type TimeScope } from '../../api/query'
import { ChartPanel, type PanelResult } from './ChartPanel'

interface DashboardGridProps {
  tenant: string
  panels: BoardPanel[]
  charts: Map<string, SavedChart>
  onLayoutChange: (panels: BoardPanel[]) => void
  onRemovePanel: (chartId: string) => void
  /** The board's structured, UTC time scope, sent with the batch (§5). */
  time?: TimeScope
}

const COLS = 12

export function DashboardGrid({
  tenant,
  panels,
  charts,
  onLayoutChange,
  onRemovePanel,
  time,
}: DashboardGridProps) {
  const api = useApi(tenant)
  const { width, containerRef } = useContainerWidth()

  // One batch per board, keyed by chart id. Only panels whose chart resolved are
  // queried; a missing-chart panel renders its own placeholder below.
  const items = useMemo<BatchQueryItem[]>(
    () =>
      panels
        .map((p) => charts.get(p.chart_id))
        .filter((c): c is SavedChart => Boolean(c))
        .map((c) => ({ key: c.id, sql: c.sql, time })),
    [panels, charts, time],
  )

  const batch = useQuery({
    queryKey: ['board-batch', tenant, items.map((i) => `${i.key}:${i.sql}`), time],
    queryFn: () => runBatchQuery(api, items),
    enabled: items.length > 0,
  })

  // Index the batch results by key (chart id) for O(1) per-panel lookup.
  const byKey = useMemo(() => {
    const m = new Map<string, PanelResult>()
    for (const r of batch.data?.results ?? []) {
      m.set(r.key, { rows: r.rows, columns: r.columns, error: r.error })
    }
    return m
  }, [batch.data])

  const layout: Layout = panels.map((p) => ({ i: p.chart_id, x: p.x, y: p.y, w: p.w, h: p.h }))

  function handleLayoutChange(next: Layout) {
    onLayoutChange(
      next.map((l) => ({ chart_id: l.i, x: l.x, y: l.y, w: l.w, h: l.h })),
    )
  }

  function panelResult(chartId: string): PanelResult {
    if (batch.isPending) return { loading: true }
    if (batch.error) return { error: (batch.error as Error).message }
    return byKey.get(chartId) ?? { error: 'no result for this panel' }
  }

  return (
    <div ref={containerRef as RefObject<HTMLDivElement>}>
      <GridLayout
        width={width}
        layout={layout}
        onLayoutChange={handleLayoutChange}
        gridConfig={{ cols: COLS, rowHeight: 80, margin: [12, 12] }}
        dragConfig={{ handle: '.panel-drag', cancel: '.panel-no-drag' }}
      >
        {panels.map((p) => {
          const chart = charts.get(p.chart_id)
          return (
            <div key={p.chart_id}>
              {chart ? (
                <ChartPanel
                  chart={chart}
                  syncId="board"
                  onRemove={() => onRemovePanel(p.chart_id)}
                  result={panelResult(p.chart_id)}
                />
              ) : (
                <div className="flex h-full items-center justify-center rounded-xl border border-dashed border-border text-xs text-muted-foreground">
                  <span className="panel-drag cursor-move px-2">Missing chart</span>
                  <button onClick={() => onRemovePanel(p.chart_id)} className="panel-no-drag underline">
                    remove
                  </button>
                </div>
              )}
            </div>
          )
        })}
      </GridLayout>
    </div>
  )
}
