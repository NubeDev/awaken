/**
 * Multi-series bar chart on recharts (DASHBOARDS-SCOPE §8 easy tier). Pure
 * renderer of a TrendWidget; used for cross-meter `quantity:<q>` comparison
 * panels on a site page (one bar per meter at each bucket). No fetching here.
 */
import {
  Bar,
  BarChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts'
import { seriesColor } from '../_shared/palette'
import { formatTick } from '../_shared/format-value'
import { formatLocalTime } from '../query/time-window'
import type { TrendWidget } from './types'
import { Empty } from './empty'
import { toChartRows } from './to-chart-rows'

export function BarWidget({ widget }: { widget: TrendWidget }) {
  const rows = toChartRows(widget.series)
  if (rows.length === 0) return <Empty />
  return (
    <ResponsiveContainer width='100%' height={240}>
      <BarChart data={rows} margin={{ top: 8, right: 12, bottom: 4, left: 0 }}>
        <CartesianGrid strokeDasharray='3 3' className='stroke-muted' />
        <XAxis
          dataKey='t'
          tickFormatter={(t: number) => formatLocalTime(new Date(t).toISOString(), widget.timezone)}
          fontSize={11}
          minTickGap={32}
        />
        <YAxis tickFormatter={formatTick} fontSize={11} width={44} />
        <Tooltip
          labelFormatter={(t) => formatLocalTime(new Date(Number(t)).toISOString(), widget.timezone, true)}
          formatter={(v) => `${formatTick(Number(v))}${widget.unit ? ` ${widget.unit}` : ''}`}
        />
        {widget.series.map((s, i) => (
          <Bar key={s.label} dataKey={s.label} fill={seriesColor(i)} isAnimationActive={false} />
        ))}
      </BarChart>
    </ResponsiveContainer>
  )
}
