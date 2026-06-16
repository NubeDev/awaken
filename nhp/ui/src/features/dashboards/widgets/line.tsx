/**
 * Multi-series line chart on recharts (DASHBOARDS-SCOPE §7: recharts is the base;
 * §8 line is the "easy" tier). Pure renderer of a TrendWidget — one <Line> per
 * series (e.g. V L1/L2/L3 of a `group:voltage`), threshold reference lines drawn
 * from the register alarm ramp so a crossing is visible (the §4 alarm colouring).
 * No fetching here; the data is handed in.
 */
import {
  CartesianGrid,
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

  return (
    <ResponsiveContainer width='100%' height={240}>
      <LineChart data={rows} margin={{ top: 8, right: 12, bottom: 4, left: 0 }}>
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
        {lines.map((l) => (
          <ReferenceLine
            key={`${l.severity}-${l.value}`}
            y={l.value}
            stroke={SEVERITY_COLORS[l.severity]}
            strokeDasharray='4 2'
          />
        ))}
        {widget.series.map((s, i) => (
          <Line
            key={s.label}
            type='monotone'
            dataKey={s.label}
            stroke={seriesColor(i)}
            dot={false}
            isAnimationActive={false}
            connectNulls
          />
        ))}
      </LineChart>
    </ResponsiveContainer>
  )
}
