/**
 * Multi-series bar chart on recharts (DASHBOARDS-SCOPE §8 easy tier). Pure
 * renderer of a TrendWidget; used for cross-meter `quantity:<q>` comparison panels
 * on a site page (one bar per meter at each bucket). Shared themed chrome from
 * _shared/chart-theme; rounded bar tops. No fetching here.
 */
import {
  Bar,
  BarChart,
  CartesianGrid,
  Legend,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts'
import { seriesColor } from '../_shared/palette'
import { formatTick } from '../_shared/format-value'
import {
  AXIS_LINE,
  AXIS_TICK,
  CHART_MARGIN,
  GRID_PROPS,
} from '../_shared/chart-axes'
import { ChartTooltip } from '../_shared/chart-theme'
import { formatLocalTime } from '../query/time-window'
import type { TrendWidget } from './types'
import { Empty } from './empty'
import { toChartRows } from './to-chart-rows'

export function BarWidget({ widget }: { widget: TrendWidget }) {
  const rows = toChartRows(widget.series)
  if (rows.length === 0) return <Empty />
  const multi = widget.series.length > 1
  return (
    <ResponsiveContainer width='100%' height={240}>
      <BarChart data={rows} margin={CHART_MARGIN}>
        <CartesianGrid {...GRID_PROPS} />
        <XAxis
          dataKey='t'
          tickFormatter={(t: number) => formatLocalTime(new Date(t).toISOString(), widget.timezone)}
          tick={AXIS_TICK}
          axisLine={AXIS_LINE}
          tickLine={false}
          minTickGap={32}
        />
        <YAxis
          tickFormatter={formatTick}
          tick={AXIS_TICK}
          axisLine={false}
          tickLine={false}
          width={44}
        />
        <Tooltip
          cursor={{ fill: 'var(--border)', fillOpacity: 0.3 }}
          content={
            <ChartTooltip
              unit={widget.unit}
              labelFormat={(t) =>
                formatLocalTime(new Date(Number(t)).toISOString(), widget.timezone, true)
              }
            />
          }
        />
        {multi ? (
          <Legend wrapperStyle={{ fontSize: 11, paddingTop: 4 }} />
        ) : null}
        {widget.series.map((s, i) => (
          <Bar
            key={s.label}
            dataKey={s.label}
            fill={seriesColor(i)}
            radius={[3, 3, 0, 0]}
            maxBarSize={28}
            isAnimationActive={false}
          />
        ))}
      </BarChart>
    </ResponsiveContainer>
  )
}
