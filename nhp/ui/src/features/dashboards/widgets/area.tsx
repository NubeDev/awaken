/**
 * Multi-series area chart on recharts (DASHBOARDS-SCOPE §8 easy tier). Pure
 * renderer of a TrendWidget; the default for `chart_type: "area"` registers (e.g. a
 * power trend). Each series gets a vertical gradient fill (reads far better than a
 * flat fill) and the shared themed chrome from _shared/chart-theme. No fetching.
 */
import {
  Area,
  AreaChart,
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

export function AreaWidget({ widget }: { widget: TrendWidget }) {
  const rows = toChartRows(widget.series)
  if (rows.length === 0) return <Empty />
  const multi = widget.series.length > 1
  // Gradient <linearGradient> ids must be unique in the document; the panel title
  // is the widget's identity here (TrendWidget has no id), so derive a safe slug.
  const gid = widget.title.replace(/[^a-z0-9]/gi, '')
  return (
    <ResponsiveContainer width='100%' height={240}>
      <AreaChart data={rows} margin={CHART_MARGIN}>
        <defs>
          {widget.series.map((s, i) => (
            <linearGradient
              key={s.label}
              id={`area-fill-${gid}-${i}`}
              x1='0'
              y1='0'
              x2='0'
              y2='1'
            >
              <stop offset='0%' stopColor={seriesColor(i)} stopOpacity={0.3} />
              <stop offset='100%' stopColor={seriesColor(i)} stopOpacity={0.02} />
            </linearGradient>
          ))}
        </defs>
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
          cursor={{ stroke: 'var(--border)' }}
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
          <Legend iconType='plainline' wrapperStyle={{ fontSize: 11, paddingTop: 4 }} />
        ) : null}
        {widget.series.map((s, i) => (
          <Area
            key={s.label}
            type='monotone'
            dataKey={s.label}
            stroke={seriesColor(i)}
            strokeWidth={2}
            fill={`url(#area-fill-${gid}-${i})`}
            isAnimationActive={false}
            connectNulls
          />
        ))}
      </AreaChart>
    </ResponsiveContainer>
  )
}
