import { useQueries } from '@tanstack/react-query'
import {
  Area,
  AreaChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts'
import * as api from '@/api/endpoints'
import { usePoints } from '@/api/hooks'
import { qk } from '@/api/keys'
import type { HisSample, Point, Uuid } from '@/api/types'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import { Skeleton } from '@/components/ui/skeleton'

/** Points whose tags mark them as an electrical/energy demand signal. */
function isDemandPoint(p: Point): boolean {
  const tags = new Set(p.tags)
  return (
    (tags.has('elec') || tags.has('energy') || tags.has('power')) &&
    (tags.has('meter') || tags.has('kw') || p.unit === 'kW')
  )
}

type SeriesRow = { t: string; value: number }

function toSeries(samples: HisSample[]): SeriesRow[] {
  return samples
    .filter((s) => typeof s.value === 'number')
    .map((s) => ({
      t: new Date(s.ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
      value: s.value as number,
    }))
}

/**
 * Whole-building demand over time, plotted from the real history of the site's
 * demand-tagged points. No synthetic data: if the site has no such point, the
 * card says so rather than inventing a curve.
 */
export function DemandChart({ siteId }: { siteId: Uuid | undefined }) {
  const { data: points = [], isLoading: pointsLoading } = usePoints({ siteId })
  const demandPoints = points.filter(isDemandPoint)

  const histories = useQueries({
    queries: demandPoints.map((p) => ({
      queryKey: qk.pointHistory(p.id),
      queryFn: ({ signal }: { signal: AbortSignal }) => api.points.history(p.id, signal),
    })),
  })

  const loading = pointsLoading || histories.some((h) => h.isLoading)
  const primary = histories[0]?.data ?? []
  const data = toSeries(primary)

  return (
    <Card className='col-span-1 lg:col-span-2'>
      <CardHeader>
        <CardTitle>Demand</CardTitle>
        <CardDescription>
          {demandPoints[0]?.display_name ?? 'Whole-building electrical load'}
        </CardDescription>
      </CardHeader>
      <CardContent>
        {loading ? (
          <Skeleton className='h-[240px] w-full rounded-lg' />
        ) : data.length === 0 ? (
          <div className='text-muted-foreground grid h-[240px] place-items-center text-sm'>
            No demand history for this site yet.
          </div>
        ) : (
          <ResponsiveContainer width='100%' height={240}>
            <AreaChart data={data} margin={{ top: 8, right: 8, left: -16, bottom: 0 }}>
              <defs>
                <linearGradient id='demandFill' x1='0' y1='0' x2='0' y2='1'>
                  <stop offset='0%' stopColor='var(--chart-1)' stopOpacity={0.25} />
                  <stop offset='100%' stopColor='var(--chart-1)' stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid stroke='var(--grid-line)' vertical={false} />
              <XAxis dataKey='t' tickLine={false} axisLine={false} fontSize={10} minTickGap={32} />
              <YAxis tickLine={false} axisLine={false} fontSize={10} width={40} />
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
                fill='url(#demandFill)'
              />
            </AreaChart>
          </ResponsiveContainer>
        )}
      </CardContent>
    </Card>
  )
}
