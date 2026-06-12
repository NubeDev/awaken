import {
  Area,
  AreaChart,
  ResponsiveContainer,
  Tooltip,
  YAxis,
} from 'recharts'
import { useRelinquishPoint } from '@/api/hooks'
import { usePointHistory } from '@/api/hooks'
import type { Point } from '@/api/types'
import { Badge } from '@/components/ui/badge'
import { Separator } from '@/components/ui/separator'
import { formatValue, relativeTime } from '@/lib/format'
import { PriorityArray } from './priority-array'
import { WriteForm } from './write-form'

/** Detail panel for a selected point: live value, trend, and command path. */
export function PointDetail({ point }: { point: Point }) {
  const relinquish = useRelinquishPoint()
  const { data: history = [] } = usePointHistory(point.id)
  const writable = point.kind !== 'sensor'

  const trend = history
    .filter((s) => typeof s.value === 'number')
    .map((s) => ({ t: s.ts, value: s.value as number }))

  return (
    <div className='space-y-5'>
      <div>
        <div className='flex items-center gap-2'>
          <h3 className='text-base font-semibold'>{point.display_name}</h3>
          <Badge variant='outline' className='font-mono text-[10px] uppercase'>
            {point.kind}
          </Badge>
        </div>
        <p className='text-muted-foreground font-mono text-xs'>{point.slug}</p>
      </div>

      <div className='flex items-baseline gap-2'>
        <span className='tabular text-3xl font-semibold tracking-tight'>
          {formatValue(point.cur_value, point.unit)}
        </span>
        <span className='text-muted-foreground text-xs'>updated {relativeTime(point.cur_ts)}</span>
      </div>

      {trend.length > 1 && (
        <ResponsiveContainer width='100%' height={96}>
          <AreaChart data={trend} margin={{ top: 4, right: 0, left: 0, bottom: 0 }}>
            <defs>
              <linearGradient id='pdFill' x1='0' y1='0' x2='0' y2='1'>
                <stop offset='0%' stopColor='var(--chart-1)' stopOpacity={0.25} />
                <stop offset='100%' stopColor='var(--chart-1)' stopOpacity={0} />
              </linearGradient>
            </defs>
            <YAxis hide domain={['dataMin', 'dataMax']} />
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
              fill='url(#pdFill)'
            />
          </AreaChart>
        </ResponsiveContainer>
      )}

      {point.tags.length > 0 && (
        <div className='flex flex-wrap gap-1.5'>
          {point.tags.map((t) => (
            <Badge key={t} variant='muted' className='text-[10px]'>
              {t}
            </Badge>
          ))}
        </div>
      )}

      {writable && (
        <>
          <Separator />
          <div className='space-y-3'>
            <h4 className='text-[13px] font-semibold'>Priority Array</h4>
            <PriorityArray
              point={point}
              relinquishing={relinquish.isPending}
              onRelinquish={(priority) => relinquish.mutate({ id: point.id, priority })}
            />
            <WriteForm point={point} />
          </div>
        </>
      )}
    </div>
  )
}
