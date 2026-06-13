import type { Point, Widget } from '@/api/types'
import { Card } from '@/components/ui/card'
import { WidgetCard } from './widget-card'

type WidgetCanvasProps = {
  widgets: Widget[]
  /** Keyexpr → live point, for resolving `point_*` tiles (site or multi-site). */
  index: Map<string, Point>
}

/**
 * Grid of pinned tiles in creation order (the server returns newest-first; no
 * layout field exists on the wire). `point_*` tiles resolve their target
 * keyexpr to a live `Point` via the index the parent builds — a single site's
 * for a site board, the org's union for an overview.
 */
export function WidgetCanvas({ widgets, index }: WidgetCanvasProps) {
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
    <div className='grid auto-rows-min grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3'>
      {widgets.map((w) => (
        <WidgetCard key={w.id} widget={w} point={index.get(w.target)} />
      ))}
    </div>
  )
}
