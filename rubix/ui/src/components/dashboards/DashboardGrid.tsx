// The responsive board grid (§2, LAMINAR-BORROW.md) — react-grid-layout v2 with
// drag/resize. Children are matched to layout items by `key === panel.chart_id`.
// Layout changes bubble up via `onLayoutChange`; the page debounces them into one
// gate write per settle (so every drag/resize is audited, never thrashed). A
// shared `syncId` ties tooltips/cursors across every panel.
//
// The board fetches every panel in ONE `POST /query/batch` keyed by chart id
// (§3) — one round trip and one consistent snapshot — and hands each panel its
// result. A single bad panel shows its error while the others render.

import { useEffect, useMemo, useState, type RefObject } from 'react'
import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
// RGL ships the positioning/transition + resize-handle rules in this stylesheet.
// Without it grid items never get `position: absolute`, so a drag transform moves
// the tile relative to document flow (it "flies off screen") and the resize
// handles are unpositioned/invisible. Importing it is required, not optional.
import 'react-grid-layout/css/styles.css'
import { GridLayout, type Layout, useContainerWidth } from 'react-grid-layout'
import { useApi } from '../../api/ConnectionContext'
import { updateChart, type SavedChart } from '../../api/charts'
import type { BoardPanel } from '../../api/boards'
import { runBatchQuery, type BatchQueryItem, type TimeScope } from '../../api/query'
import { applyCosmeticTransforms, splitTransforms } from '../chart-builder/transforms'
import { snapInstant, useRefreshTick, type RefreshInterval } from './board-refresh'
import { decodeDrag, PALETTE_DND_TYPE, type PaletteDrag } from './board-palette'
import { ChartPanel, type PanelResult } from './ChartPanel'
import { ChartSettingsDialog } from './ChartSettingsDialog'
import type { ChartConfig } from '../chart-builder/types'

interface DashboardGridProps {
  tenant: string
  panels: BoardPanel[]
  charts: Map<string, SavedChart>
  onLayoutChange: (panels: BoardPanel[]) => void
  onRemovePanel: (chartId: string) => void
  /** Add a panel from a palette drop (preset or saved chart). */
  onAddDrag?: (drag: PaletteDrag) => void
  /** The board's structured, UTC time scope, sent with the batch (§5). */
  time?: TimeScope
  /** Auto-refresh interval; `null` is Off (§6). */
  refresh?: RefreshInterval
  /** Reports whether a batch refetch is in flight, for the refresh control's spinner. */
  onFetchingChange?: (fetching: boolean) => void
}

const COLS = 12

export function DashboardGrid({
  tenant,
  panels,
  charts,
  onLayoutChange,
  onRemovePanel,
  onAddDrag,
  time,
  refresh = null,
  onFetchingChange,
}: DashboardGridProps) {
  const api = useApi(tenant)
  const qc = useQueryClient()
  const { width, containerRef } = useContainerWidth()
  const [dropActive, setDropActive] = useState(false)
  // Which chart's settings dialog is open (chart id), or null.
  const [editingChartId, setEditingChartId] = useState<string | null>(null)

  // Visibility-aware tick: advances once per interval while visible, pauses on a
  // hidden tab, catches up on return (§6). Folded into the snapped scope below so
  // a refresh re-resolves the window; folded into the query key so a tick triggers
  // a refetch even when `time` is otherwise stable.
  const tick = useRefreshTick(refresh)

  // Snap absolute window bounds DOWN to the refresh tick before they enter the
  // query key (§6): a relative "now-1h" resolved each render would otherwise mint
  // a fresh from/to and bust the cache (and miss the backend's time-snapshot
  // cache, §4a). Within a tick every render sees identical bounds → a guaranteed
  // hit. Relative-token bounds (strings) are resolved server-side, so pass through.
  const snappedTime = useMemo<TimeScope | undefined>(() => {
    if (!time) return undefined
    return {
      ...time,
      from: typeof time.from === 'number' ? snapInstant(time.from, refresh) : time.from,
      to: typeof time.to === 'number' ? snapInstant(time.to, refresh) : time.to,
    }
  }, [time, refresh, tick])

  // One batch per board, keyed by chart id. Only panels whose chart resolved are
  // queried; a missing-chart panel renders its own placeholder below.
  const items = useMemo<BatchQueryItem[]>(
    () =>
      panels
        .map((p) => charts.get(p.chart_id))
        .filter((c): c is SavedChart => Boolean(c))
        // Thread the chart's per-column quantity map (§2/§7) and the AGGREGATE
        // transform tier (§1) into the batch. Only filter/groupBy/reduce go to the
        // backend; the cosmetic tier runs client-side after the rows return. A
        // chart referencing a saved query sends its id instead of SQL (§4b).
        .map((c) => ({
          key: c.id,
          sql: c.sql,
          query_id: c.config?.query_id,
          time: snappedTime,
          quantities: c.config?.quantities,
          transforms: splitTransforms(c.config?.transforms).aggregate,
        })),
    [panels, charts, snappedTime],
  )

  // The cosmetic transform tier per chart id — applied to each panel's rows after
  // the batch returns (the aggregate tier already ran server-side, §1).
  const cosmeticByChart = useMemo(() => {
    const m = new Map<string, ReturnType<typeof splitTransforms>['cosmetic']>()
    for (const p of panels) {
      const c = charts.get(p.chart_id)
      if (c) m.set(c.id, splitTransforms(c.config?.transforms).cosmetic)
    }
    return m
  }, [panels, charts])

  const batch = useQuery({
    queryKey: [
      'board-batch',
      tenant,
      items.map(
        (i) =>
          `${i.key}:${i.query_id ?? i.sql}:${JSON.stringify(i.quantities ?? null)}:${JSON.stringify(
            i.transforms ?? null,
          )}`,
      ),
      snappedTime,
    ],
    queryFn: () => runBatchQuery(api, items),
    enabled: items.length > 0,
    // refetchInterval also serves as the live driver; the visibility pause is
    // handled by our tick (and TanStack's own refetchIntervalInBackground default
    // of false), keepPreviousData stops a refresh flashing a spinner per panel.
    refetchInterval: refresh ?? false,
    placeholderData: keepPreviousData,
  })

  // Surface fetch state to the page so the refresh control can spin its icon.
  useEffect(() => {
    onFetchingChange?.(batch.isFetching)
  }, [batch.isFetching, onFetchingChange])

  // Index the batch results by key (chart id) for O(1) per-panel lookup.
  const byKey = useMemo(() => {
    const m = new Map<string, PanelResult>()
    for (const r of batch.data?.results ?? []) {
      // Run the cosmetic transform tier client-side on this panel's rows (§1).
      const cosmetic = cosmeticByChart.get(r.key)
      const rows =
        r.rows && cosmetic && cosmetic.length > 0
          ? applyCosmeticTransforms(r.rows, cosmetic)
          : r.rows
      m.set(r.key, { rows, columns: r.columns, error: r.error })
    }
    return m
  }, [batch.data, cosmeticByChart])

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

  // Persist an edited chart (name + config). The chart is a shared record, so
  // invalidating ['charts'] re-renders every panel referencing it; ['board-batch']
  // re-runs the query (a changed type/quantity/transform can change the rows).
  const editingChart = editingChartId ? charts.get(editingChartId) : undefined
  const saveChart = useMutation({
    mutationFn: (input: { name: string; config: ChartConfig }) =>
      updateChart(api, editingChartId!, { name: input.name, sql: editingChart!.sql, config: input.config }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['charts', tenant] })
      void qc.invalidateQueries({ queryKey: ['board-batch'] })
      setEditingChartId(null)
    },
  })

  // Accept palette drops anywhere on the grid surface. RGL handles the in-grid
  // moves; this is purely "add a new panel from the rail". We don't compute a drop
  // cell — the page appends in reading order (RGL compacts), matching click-to-add.
  function onDrop(e: React.DragEvent) {
    e.preventDefault()
    setDropActive(false)
    const raw = e.dataTransfer.getData(PALETTE_DND_TYPE)
    const drag = decodeDrag(raw)
    if (drag) onAddDrag?.(drag)
  }

  return (
    <div
      ref={containerRef as RefObject<HTMLDivElement>}
      onDragOver={(e) => {
        if (!onAddDrag) return
        e.preventDefault()
        e.dataTransfer.dropEffect = 'copy'
        if (!dropActive) setDropActive(true)
      }}
      onDragLeave={(e) => {
        // Only clear when the cursor actually leaves the container, not on every
        // child boundary crossing.
        if (e.currentTarget === e.target) setDropActive(false)
      }}
      onDrop={onDrop}
      className={
        'rounded-xl transition-colors ' +
        (dropActive ? 'bg-primary/[0.06] outline-2 outline-dashed outline-primary/40' : '')
      }
    >
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
                  onEdit={() => setEditingChartId(p.chart_id)}
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

      {editingChart && (
        <ChartSettingsDialog
          open={editingChartId !== null}
          onOpenChange={(o) => !o && setEditingChartId(null)}
          chart={editingChart}
          rows={panelResult(editingChart.id).rows ?? []}
          columns={panelResult(editingChart.id).columns}
          onSave={(input) => saveChart.mutate(input)}
          saving={saveChart.isPending}
        />
      )}
    </div>
  )
}
