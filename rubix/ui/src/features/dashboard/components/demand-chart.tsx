import { useMemo, useState } from 'react'
import { TrendingDown, TrendingUp } from 'lucide-react'
import {
  Area,
  ComposedChart,
  CartesianGrid,
  Line,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts'
import { usePointHistory, usePoints } from '@/api/hooks'
import type { HisSample, Point, Uuid } from '@/api/types'
import { Badge } from '@/components/ui/badge'
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import { Skeleton } from '@/components/ui/skeleton'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'

/** Points whose tags mark them as an electrical/energy demand signal. */
function isDemandPoint(p: Point): boolean {
  const tags = new Set(p.tags)
  return (
    (tags.has('elec') || tags.has('energy') || tags.has('power')) &&
    (tags.has('meter') || tags.has('kw') || p.unit === 'kW')
  )
}

type Row = { t: string; demand: number; baseline: number }

/**
 * Shape history into chart rows. The baseline is a centred rolling mean of the
 * same series — a real derivation labelled as such, not a synthetic curve.
 */
function toRows(samples: HisSample[], window = 8): Row[] {
  const nums = samples.filter((s) => typeof s.value === 'number')
  return nums.map((s, i) => {
    const lo = Math.max(0, i - window)
    const hi = Math.min(nums.length, i + window + 1)
    const slice = nums.slice(lo, hi)
    const mean = slice.reduce((a, b) => a + (b.value as number), 0) / slice.length
    return {
      t: new Date(s.ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
      demand: s.value as number,
      baseline: +mean.toFixed(1),
    }
  })
}

function LegendChip({ color, label, value, dashed }: { color: string; label: string; value: string; dashed?: boolean }) {
  return (
    <div className='flex items-center gap-1.5'>
      <span
        className='w-3.5'
        style={{ borderTop: `2px ${dashed ? 'dashed' : 'solid'} ${color}` }}
      />
      <span className='text-muted-foreground text-xs'>{label}</span>
      <span className='tabular text-xs font-semibold'>{value}</span>
    </div>
  )
}

type Range = '24h' | '48h'

/** Whole-building demand vs its rolling average, from real point history. */
export function DemandChart({ siteId }: { siteId: Uuid | undefined }) {
  const { data: points = [], isLoading: pointsLoading } = usePoints({ siteId })
  const demandPoint = points.find(isDemandPoint)
  const { data: history = [], isLoading: hisLoading } = usePointHistory(demandPoint?.id)
  const [range, setRange] = useState<Range>('48h')

  const rows = useMemo(() => {
    const all = toRows(history)
    return range === '24h' ? all.slice(-Math.ceil(all.length / 2)) : all
  }, [history, range])

  const loading = pointsLoading || hisLoading
  const last = rows[rows.length - 1]
  const diff = last ? +(last.demand - last.baseline).toFixed(0) : 0
  const below = diff <= 0

  return (
    <Card className='col-span-1 lg:col-span-2'>
      <CardHeader>
        <CardTitle>Demand · {range}</CardTitle>
        <CardDescription>
          {demandPoint
            ? `${demandPoint.display_name} vs rolling average`
            : 'Whole-building electrical load'}
        </CardDescription>
        <CardAction>
          <Tabs value={range} onValueChange={(v) => setRange(v as Range)}>
            <TabsList className='h-7'>
              <TabsTrigger value='24h' className='px-2.5 text-xs'>24h</TabsTrigger>
              <TabsTrigger value='48h' className='px-2.5 text-xs'>48h</TabsTrigger>
            </TabsList>
          </Tabs>
        </CardAction>
      </CardHeader>
      <CardContent>
        {loading ? (
          <Skeleton className='h-[240px] w-full rounded-lg' />
        ) : rows.length === 0 ? (
          <div className='text-muted-foreground grid h-[240px] place-items-center text-sm'>
            No demand history for this site yet.
          </div>
        ) : (
          <>
            <div className='mb-2 flex items-center gap-4'>
              <LegendChip
                color='var(--chart-1)'
                label='Actual'
                value={last ? `${Math.round(last.demand)} kW` : '—'}
              />
              <LegendChip
                color='var(--muted-foreground)'
                label='Rolling avg'
                value={last ? `${Math.round(last.baseline)} kW` : '—'}
                dashed
              />
              <Badge variant={below ? 'info' : 'warning'} className='ms-auto gap-1'>
                {below ? <TrendingDown className='size-3' /> : <TrendingUp className='size-3' />}
                {Math.abs(diff)} kW {below ? 'below' : 'above'} average
              </Badge>
            </div>
            <ResponsiveContainer width='100%' height={224}>
              <ComposedChart data={rows} margin={{ top: 6, right: 8, left: -12, bottom: 0 }}>
                <defs>
                  <linearGradient id='demandFill' x1='0' y1='0' x2='0' y2='1'>
                    <stop offset='0%' stopColor='var(--chart-1)' stopOpacity={0.28} />
                    <stop offset='100%' stopColor='var(--chart-1)' stopOpacity={0} />
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
                  width={42}
                  tick={{ fill: 'var(--muted-foreground)' }}
                  domain={['dataMin - 20', 'dataMax + 20']}
                  tickFormatter={(v: number) => String(Math.round(v))}
                />
                <Tooltip
                  contentStyle={{
                    background: 'var(--popover)',
                    border: '1px solid var(--border)',
                    borderRadius: 8,
                    fontSize: 12,
                    boxShadow: 'var(--shadow-lg)',
                  }}
                  labelStyle={{ color: 'var(--muted-foreground)' }}
                />
                <Line
                  type='monotone'
                  dataKey='baseline'
                  stroke='var(--muted-foreground)'
                  strokeWidth={1.5}
                  strokeDasharray='4 4'
                  dot={false}
                  isAnimationActive={false}
                />
                <Area
                  type='monotone'
                  dataKey='demand'
                  stroke='var(--chart-1)'
                  strokeWidth={2}
                  fill='url(#demandFill)'
                  isAnimationActive={false}
                />
              </ComposedChart>
            </ResponsiveContainer>
          </>
        )}
      </CardContent>
    </Card>
  )
}
