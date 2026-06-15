// The responsive board grid (§2, LAMINAR-BORROW.md) — react-grid-layout v2 with
// drag/resize. Children are matched to layout items by `key === panel.chart_id`.
// Layout changes bubble up via `onLayoutChange`; the page debounces them into one
// gate write per settle (so every drag/resize is audited, never thrashed). A
// shared `syncId` ties tooltips/cursors across every panel — hover one chart,
// see the same instant on all (the cross-chart-sync payoff of a board).

import type { RefObject } from 'react'
import { GridLayout, type Layout, useContainerWidth } from 'react-grid-layout'
import type { SavedChart } from '../../api/charts'
import type { BoardPanel } from '../../api/boards'
import { ChartPanel } from './ChartPanel'

interface DashboardGridProps {
  tenant: string
  panels: BoardPanel[]
  charts: Map<string, SavedChart>
  onLayoutChange: (panels: BoardPanel[]) => void
  onRemovePanel: (chartId: string) => void
}

const COLS = 12

export function DashboardGrid({
  tenant,
  panels,
  charts,
  onLayoutChange,
  onRemovePanel,
}: DashboardGridProps) {
  const { width, containerRef } = useContainerWidth()

  const layout: Layout = panels.map((p) => ({ i: p.chart_id, x: p.x, y: p.y, w: p.w, h: p.h }))

  function handleLayoutChange(next: Layout) {
    onLayoutChange(
      next.map((l) => ({ chart_id: l.i, x: l.x, y: l.y, w: l.w, h: l.h })),
    )
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
                  tenant={tenant}
                  chart={chart}
                  syncId="board"
                  onRemove={() => onRemovePanel(p.chart_id)}
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
