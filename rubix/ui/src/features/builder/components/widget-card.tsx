import { useState } from 'react'
import { GripVertical, X } from 'lucide-react'
import { useDeleteWidget, usePointHistory } from '@/api/hooks'
import type { Point, Widget } from '@/api/types'
import { ageShort, formatValue } from '@/lib/format'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { ConfirmDialog } from '@/components/confirm-dialog'
import { HistoryChart, type ChartRow } from '../lib/charts'
import { BoardOutputCard } from './board-output-card'

type WidgetCardProps = {
  widget: Widget
  /** Resolved point for `point_*` kinds; undefined until the index loads or if the keyexpr no longer maps. */
  point: Point | undefined
}

/**
 * One dashboard tile, sized to fill its grid cell. `point_*` kinds render live
 * point data resolved from the widget target keyexpr; `board_output` runs its
 * stored board. A `.drag-handle` in the header drives `react-grid-layout`
 * dragging; a hover Remove control deletes the pin (`DELETE /widgets/{id}`).
 */
export function WidgetCard({ widget, point }: WidgetCardProps) {
  const inner =
    widget.kind === 'board_output' ? (
      <BoardOutputCard widget={widget} />
    ) : !point ? (
      <Card className='h-full gap-2 p-3.5'>
        <TileHeader widget={widget} />
        <p className='text-[11.5px] text-muted-foreground'>
          Point not found for <span className='font-mono'>{widget.target}</span>
          .
        </p>
      </Card>
    ) : widget.kind === 'point_value' ? (
      <PointValueCard widget={widget} point={point} />
    ) : (
      <PointHistoryCard widget={widget} point={point} />
    )

  return (
    <div className='group relative h-full'>
      {inner}
      <RemoveButton widget={widget} />
    </div>
  )
}

function RemoveButton({ widget }: { widget: Widget }) {
  const del = useDeleteWidget()
  const [confirmOpen, setConfirmOpen] = useState(false)
  return (
    <>
      <Button
        size='icon'
        variant='ghost'
        title='Remove widget'
        className='absolute end-1.5 top-1.5 size-6 bg-background/80 opacity-0 backdrop-blur transition-opacity group-hover:opacity-100'
        onClick={() => setConfirmOpen(true)}
      >
        <X className='size-3.5' />
      </Button>
      <ConfirmDialog
        open={confirmOpen}
        onOpenChange={setConfirmOpen}
        destructive
        title={`Remove "${widget.title}"?`}
        desc='This unpins the tile from the dashboard. The underlying point or board is untouched.'
        confirmText='Remove'
        isLoading={del.isPending}
        handleConfirm={() =>
          del.mutate(widget.id, { onSuccess: () => setConfirmOpen(false) })
        }
      />
    </>
  )
}

/**
 * Tile header: a drag handle (the `.drag-handle` class `react-grid-layout`'s
 * `dragConfig` targets) plus the title. Dragging is confined to this strip so
 * clicks inside the tile (tooltips, the Remove control) still work.
 */
function TileHeader({ widget }: { widget: Widget }) {
  return (
    <div className='flex items-center gap-1'>
      <span className='drag-handle text-muted-foreground/50 hover:text-muted-foreground -ms-1 cursor-grab active:cursor-grabbing'>
        <GripVertical className='size-3.5' />
      </span>
      <span className='eyebrow text-[10px]'>{widget.title}</span>
    </div>
  )
}

function PointValueCard({ widget, point }: { widget: Widget; point: Point }) {
  return (
    <Card className='h-full gap-2 p-3.5'>
      <TileHeader widget={widget} />
      <div className='flex items-baseline gap-1'>
        <span className='tabular text-2xl leading-none font-semibold tracking-tight'>
          {formatValue(point.cur_value)}
        </span>
        <span className='text-[12px] text-muted-foreground'>
          {point.unit ?? ''}
        </span>
      </div>
      <div className='flex items-center gap-1.5 text-[11px] text-muted-foreground'>
        <span className='size-1.5 rounded-full bg-positive' />
        updated {ageShort(point.cur_ts)} ago
      </div>
    </Card>
  )
}

function PointHistoryCard({ widget, point }: { widget: Widget; point: Point }) {
  const { data: history = [] } = usePointHistory(point.id)
  const rows: ChartRow[] = history
    .filter((s) => typeof s.value === 'number')
    .slice(-48)
    .map((s) => ({
      t: new Date(s.ts).toLocaleTimeString([], {
        hour: '2-digit',
        minute: '2-digit',
      }),
      value: s.value as number,
    }))

  return (
    <Card className='h-full gap-2 overflow-hidden p-3.5'>
      <TileHeader widget={widget} />
      <div className='min-h-0 flex-1'>
        {rows.length < 2 ? (
          <div className='grid h-full place-items-center text-[12px] text-muted-foreground'>
            No numeric history yet.
          </div>
        ) : (
          <HistoryChart
            rows={rows}
            type={widget.settings?.config?.type ?? 'area'}
            gradientId={`wFill-${widget.id}`}
            height='100%'
          />
        )}
      </div>
    </Card>
  )
}
