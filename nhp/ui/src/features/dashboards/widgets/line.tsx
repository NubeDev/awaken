/**
 * Multi-series line chart on recharts (DASHBOARDS-SCOPE §7: recharts is the base;
 * §8 line is the "easy" tier). Pure renderer of a TrendWidget — one <Line> per
 * series (e.g. V L1/L2/L3 of a `group:voltage`), threshold reference lines drawn
 * from the register alarm ramp so a crossing is visible (the §4 alarm colouring).
 * Shared chrome (themed grid/axes from _shared/chart-axes, the tooltip from
 * _shared/chart-theme) so every widget matches the app theme. No fetching here.
 */
import {
  CartesianGrid,
  Legend,
  Line,
  LineChart,
  ReferenceLine,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts'
import { SEVERITY_COLORS, seriesColor } from '../_shared/palette'
import { formatTick } from '../_shared/format-value'
import {
  AXIS_LINE,
  AXIS_TICK,
  CHART_MARGIN,
  GRID_PROPS,
} from '../_shared/chart-axes'
import { ChartTooltip } from '../_shared/chart-theme'
import { formatLocalTime } from '../query/time-window'
import { thresholdLines } from '../_shared/field-config'
import type { TrendWidget } from './types'
import { Empty } from './empty'
import { toChartRows } from './to-chart-rows'

export function LineWidget({ widget }: { widget: TrendWidget }) {
  const rows = toChartRows(widget.series)
  if (rows.length === 0) return <Empty />

  // Reference lines come from any series that carries an alarm ramp (a group
  // shares thresholds; first wins for the shared axis lines).
  const alarm = widget.series.find((s) => s.alarm)?.alarm
  const lines = thresholdLines(alarm)
  const multi = widget.series.length > 1

  return (
    <ResponsiveContainer width='100%' height={240}>
      <LineChart data={rows} margin={CHART_MARGIN}>
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
          <Legend
            iconType='plainline'
            wrapperStyle={{ fontSize: 11, paddingTop: 4 }}
          />
        ) : null}
        {lines.map((l) => (
          <ReferenceLine
            key={`${l.severity}-${l.value}`}
            y={l.value}
            stroke={SEVERITY_COLORS[l.severity]}
            strokeDasharray='4 2'
            strokeOpacity={0.7}
          />
        ))}
        {widget.series.map((s, i) => (
          <Line
            key={s.label}
            type='monotone'
            dataKey={s.label}
            stroke={seriesColor(i)}
            strokeWidth={2}
            dot={false}
            activeDot={{ r: 3, strokeWidth: 0 }}
            isAnimationActive={false}
            connectNulls
          />
        ))}
      </LineChart>
    </ResponsiveContainer>
  )
}
