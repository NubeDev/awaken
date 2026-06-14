import { useCallback, useMemo, useRef } from 'react'
import 'react-grid-layout/css/styles.css'
import {
  GridLayout,
  type Layout,
  type LayoutItem,
  useContainerWidth,
} from 'react-grid-layout'
import { usePatchWidget } from '@/api/hooks'
import type { GridLayout as GridCell, Point, Widget } from '@/api/types'
import { Card } from '@/components/ui/card'
import { WidgetCard } from './widget-card'

type WidgetCanvasProps = {
  widgets: Widget[]
  /** Keyexpr → live point, for resolving `point_*` tiles (site or multi-site). */
  index: Map<string, Point>
}

/** 12-column grid; an unplaced tile defaults to this footprint. */
const COLS = 12
const ROW_H = 96
const DEFAULT_W = 4
const DEFAULT_H = 3

/**
 * Place tiles that have no stored layout in reading order, two-up by default,
 * so a freshly pinned or agent-pinned tile lands somewhere sensible until the
 * operator drags it. Stored layouts win; only the gaps are auto-filled.
 */
function resolveLayout(widgets: Widget[]): LayoutItem[] {
  let cursorX = 0
  let cursorY = 0
  return widgets.map((w) => {
    const stored = w.settings?.layout
    if (stored) {
      return { i: w.id, ...stored }
    }
    if (cursorX + DEFAULT_W > COLS) {
      cursorX = 0
      cursorY += DEFAULT_H
    }
    const item: LayoutItem = {
      i: w.id,
      x: cursorX,
      y: cursorY,
      w: DEFAULT_W,
      h: DEFAULT_H,
    }
    cursorX += DEFAULT_W
    return item
  })
}

const sameCell = (a: GridCell | undefined, b: LayoutItem): boolean =>
  !!a && a.x === b.x && a.y === b.y && a.w === b.w && a.h === b.h

/**
 * Draggable, resizable grid of pinned tiles backed by `react-grid-layout`. Each
 * tile's cell persists to its widget `settings.layout` (`PATCH /widgets/{id}`);
 * tiles without a stored layout auto-flow until moved. `point_*` tiles resolve
 * their target keyexpr to a live `Point` via the index the parent builds — a
 * single site's for a site board, the org's union for an overview.
 */
export function WidgetCanvas({ widgets, index }: WidgetCanvasProps) {
  const patch = usePatchWidget()
  const { width, containerRef, mounted } = useContainerWidth()

  const layout = useMemo(() => resolveLayout(widgets), [widgets])
  // Lookup from id → widget so the change handler can read current settings.
  const byId = useMemo(
    () => new Map(widgets.map((w) => [w.id, w])),
    [widgets]
  )

  // Persist only the cells that actually moved, preserving each tile's chart
  // config. A ref guards the first synthetic onLayoutChange RGL fires on mount.
  const settled = useRef(false)
  const onLayoutChange = useCallback(
    (next: Layout) => {
      if (!settled.current) {
        settled.current = true
        return
      }
      for (const item of next) {
        const w = byId.get(item.i)
        if (!w || sameCell(w.settings?.layout, item)) continue
        const cell: GridCell = { x: item.x, y: item.y, w: item.w, h: item.h }
        patch.mutate({
          id: w.id,
          body: { settings: { ...w.settings, layout: cell } },
        })
      }
    },
    [byId, patch]
  )

  if (widgets.length === 0) {
    return (
      <Card className='grid h-full place-items-center'>
        <div className='max-w-xs text-center'>
          <p className='text-[13px] font-medium'>No widgets pinned yet</p>
          <p className='mt-1 text-[12px] text-muted-foreground'>
            Pick a widget from the rail to bind it to a live point or board.
          </p>
        </div>
      </Card>
    )
  }

  return (
    <div ref={containerRef}>
      {mounted ? (
        <GridLayout
          width={width}
          layout={layout}
          gridConfig={{ cols: COLS, rowHeight: ROW_H, margin: [12, 12] }}
          dragConfig={{ handle: '.drag-handle' }}
          onLayoutChange={onLayoutChange}
        >
          {widgets.map((w) => (
            <div key={w.id}>
              <WidgetCard widget={w} point={index.get(w.target)} />
            </div>
          ))}
        </GridLayout>
      ) : null}
    </div>
  )
}
