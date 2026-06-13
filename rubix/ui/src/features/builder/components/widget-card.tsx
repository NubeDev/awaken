import { useState } from 'react'
import { X } from 'lucide-react'
import {
  Area,
  AreaChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts'
import { useDeleteWidget, usePointHistory } from '@/api/hooks'
import type { Point, Widget } from '@/api/types'
import { ageShort, formatValue } from '@/lib/format'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { ConfirmDialog } from '@/components/confirm-dialog'
import { BoardOutputCard } from './board-output-card'

type WidgetCardProps = {
  widget: Widget
  /** Resolved point for `point_*` kinds; undefined until the index loads or if the keyexpr no longer maps. */
  point: Point | undefined
}

/**
 * One dashboard tile. `point_*` kinds render live point data resolved from the
 * widget target keyexpr; `board_output` runs its stored board. Chrome matches
 * the point-detail stat cards so the look stays frozen. A hover Remove control
 * deletes the pin (`DELETE /widgets/{id}`).
 */
export function WidgetCard({ widget, point }: WidgetCardProps) {
  const inner =
    widget.kind === 'board_output' ? (
      <BoardOutputCard widget={widget} />
    ) : !point ? (
      <Card className='gap-2 p-3.5'>
        <Eyebrow widget={widget} />
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
    <div className='group relative'>
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

function Eyebrow({ widget }: { widget: Widget }) {
  return <span className='eyebrow text-[10px]'>{widget.title}</span>
}

function PointValueCard({ widget, point }: { widget: Widget; point: Point }) {
  return (
    <Card className='gap-2 p-3.5'>
      <Eyebrow widget={widget} />
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
  const rows = history
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
    <Card className='gap-2 p-3.5'>
      <Eyebrow widget={widget} />
      {rows.length < 2 ? (
        <div className='grid h-[120px] place-items-center text-[12px] text-muted-foreground'>
          No numeric history yet.
        </div>
      ) : (
        <ResponsiveContainer width='100%' height={120}>
          <AreaChart
            data={rows}
            margin={{ top: 6, right: 4, left: -18, bottom: 0 }}
          >
            <defs>
              <linearGradient
                id={`wFill-${widget.id}`}
                x1='0'
                y1='0'
                x2='0'
                y2='1'
              >
                <stop
                  offset='0%'
                  stopColor='var(--chart-1)'
                  stopOpacity={0.25}
                />
                <stop
                  offset='100%'
                  stopColor='var(--chart-1)'
                  stopOpacity={0}
                />
              </linearGradient>
            </defs>
            <CartesianGrid stroke='var(--grid-line)' vertical={false} />
            <XAxis
              dataKey='t'
              tickLine={false}
              axisLine={false}
              fontSize={10}
              minTickGap={42}
              tick={{ fill: 'var(--muted-foreground)' }}
            />
            <YAxis
              tickLine={false}
              axisLine={false}
              fontSize={10}
              width={46}
              tick={{ fill: 'var(--muted-foreground)' }}
              domain={['auto', 'auto']}
            />
            <Tooltip
              contentStyle={{
                background: 'var(--popover)',
                border: '1px solid var(--border)',
                borderRadius: 8,
                fontSize: 12,
              }}
            />
            <Area
              type='monotone'
              dataKey='value'
              stroke='var(--chart-1)'
              strokeWidth={2}
              fill={`url(#wFill-${widget.id})`}
              isAnimationActive={false}
            />
          </AreaChart>
        </ResponsiveContainer>
      )}
    </Card>
  )
}
