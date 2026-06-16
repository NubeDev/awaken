/**
 * A compact multi-series trend chart for the reporting surface — pure renderer of
 * pre-built series, reusing the dashboards' shared chart theme/axes so it matches
 * the app and prints crisp (recharts SVG). Pivots the series to one row per
 * instant internally (a gap stays a missing key → `connectNulls`).
 */
import {
  CartesianGrid,
  Legend,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts'
import {
  AXIS_LINE,
  AXIS_TICK,
  CHART_MARGIN,
  GRID_PROPS,
} from '@/features/dashboards/_shared/chart-axes'
import { formatTick } from '@/features/dashboards/_shared/format-value'

export interface ChartSeries {
  label: string
  color: string
  points: { t: number; v: number }[]
}

export function SeriesChart({
  series,
  height = 260,
}: {
  series: ChartSeries[]
  height?: number
}) {
  const byT = new Map<number, Record<string, number> & { t: number }>()
  for (const s of series) {
    for (const p of s.points) {
      let row = byT.get(p.t)
      if (!row) {
        row = { t: p.t }
        byT.set(p.t, row)
      }
      row[s.label] = p.v
    }
  }
  const rows = [...byT.values()].sort((a, b) => a.t - b.t)
  if (rows.length === 0) {
    return (
      <p className='text-muted-foreground py-10 text-center text-sm'>
        No readings in this window.
      </p>
    )
  }

  return (
    <ResponsiveContainer width='100%' height={height}>
      <LineChart data={rows} margin={CHART_MARGIN}>
        <CartesianGrid {...GRID_PROPS} />
        <XAxis
          dataKey='t'
          type='number'
          scale='time'
          domain={['dataMin', 'dataMax']}
          tickFormatter={(t: number) => new Date(t).toLocaleString()}
          tick={AXIS_TICK}
          axisLine={AXIS_LINE}
          tickLine={false}
          minTickGap={48}
        />
        <YAxis
          tickFormatter={formatTick}
          tick={AXIS_TICK}
          axisLine={false}
          tickLine={false}
          width={48}
        />
        <Tooltip
          labelFormatter={(t) => new Date(Number(t)).toLocaleString()}
          contentStyle={{ fontSize: 12 }}
        />
        {series.length > 1 ? (
          <Legend wrapperStyle={{ fontSize: 11, paddingTop: 4 }} />
        ) : null}
        {series.map((s) => (
          <Line
            key={s.label}
            type='monotone'
            dataKey={s.label}
            stroke={s.color}
            strokeWidth={2}
            dot={false}
            isAnimationActive={false}
            connectNulls
          />
        ))}
      </LineChart>
    </ResponsiveContainer>
  )
}
