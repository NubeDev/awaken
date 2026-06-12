import { keyexprIndex } from '@/api/keyexpr'
import type { Equip, Point, Site, Widget } from '@/api/types'
import { Card } from '@/components/ui/card'
import { WidgetCard } from './widget-card'

type WidgetCanvasProps = {
  site: Site
  widgets: Widget[]
  equips: Equip[]
  points: Point[]
}

/**
 * Grid of pinned tiles in creation order (the server returns newest-first; no
 * layout field exists on the wire — see `docs/sessions/TODOs.md`). `point_*`
 * tiles resolve their target keyexpr to a live `Point` via the site index.
 */
export function WidgetCanvas({ site, widgets, equips, points }: WidgetCanvasProps) {
  if (widgets.length === 0) {
    return (
      <Card className='grid h-full place-items-center'>
        <div className='max-w-xs text-center'>
          <p className='text-[13px] font-medium'>No widgets pinned yet</p>
          <p className='text-muted-foreground mt-1 text-[12px]'>
            Pick a widget from the rail to bind it to a live point or board.
          </p>
        </div>
      </Card>
    )
  }

  const index = keyexprIndex(site, equips, points)

  return (
    <div className='grid auto-rows-min grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3'>
      {widgets.map((w) => (
        <WidgetCard key={w.id} widget={w} point={index.get(w.target)} />
      ))}
    </div>
  )
}
