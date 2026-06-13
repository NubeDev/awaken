/**
 * Recharts wrappers for a `point_history` tile, keyed by `ChartType`. A small
 * registry so a tile can pick its rendering from `settings.config.type` without
 * the card branching on every kind — adapted in spirit from lmnr's
 * `chart-builder/charts/` (Apache-2.0), rebuilt against our point-history rows
 * and theme tokens.
 */
import {
  Area,
  AreaChart,
  Bar,
  BarChart,
  CartesianGrid,
  Line,
  LineChart,
  ReferenceArea,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts'
import type { ChartType } from '@/api/types'
import { useChartZoom } from './use-chart-zoom'

/** A chart point: the axis label `t`, the numeric `value`, and the source epoch
 *  `ms` the drag-zoom maps a selected label back to. */
export type ChartRow = { t: string; value: number; ms: number }

/** Height accepted by recharts' ResponsiveContainer. */
type ChartHeight = number | `${number}%`

const AXIS = {
  tickLine: false,
  axisLine: false,
  fontSize: 10,
  tick: { fill: 'var(--muted-foreground)' },
} as const

const TOOLTIP = {
  contentStyle: {
    background: 'var(--popover)',
    border: '1px solid var(--border)',
    borderRadius: 8,
    fontSize: 12,
  },
} as const

const NOOP_ZOOM = () => {}

/**
 * Render history rows as the requested chart type (default: area). When `onZoom`
 * is given, dragging across the plot selects a sub-range and reports its
 * `{fromMs, toMs}` so the caller narrows the global time range
 * (docs/design/time-range-and-refresh.md §6).
 */
export function HistoryChart({
  rows,
  type = 'area',
  gradientId,
  height = 120,
  onZoom,
}: {
  rows: ChartRow[]
  type?: ChartType
  gradientId: string
  height?: ChartHeight
  /** Called with the dragged sub-range on release; omit to disable drag-zoom. */
  onZoom?: (fromMs: number, toMs: number) => void
}) {
  const zoom = useChartZoom(rows, onZoom ?? NOOP_ZOOM)
  if (type === 'table') {
    return <HistoryTable rows={rows} height={height} />
  }

  // Shared drag handlers (only active when a zoom callback is supplied) and the
  // live-selection overlay, so every chart type gets the same brush behaviour.
  const dragProps = onZoom
    ? {
        onMouseDown: zoom.onMouseDown,
        onMouseMove: zoom.onMouseMove,
        onMouseUp: zoom.onMouseUp,
      }
    : {}
  const selection =
    onZoom && zoom.refLeft && zoom.refRight ? (
      <ReferenceArea
        x1={zoom.refLeft}
        x2={zoom.refRight}
        fill='var(--chart-1)'
        fillOpacity={0.12}
      />
    ) : null
  const margin = { top: 6, right: 4, left: -18, bottom: 0 }

  return (
    <ResponsiveContainer width='100%' height={height}>
      {type === 'bar' ? (
        <BarChart data={rows} margin={margin} {...dragProps}>
          <CartesianGrid stroke='var(--grid-line)' vertical={false} />
          <XAxis dataKey='t' minTickGap={42} {...AXIS} />
          <YAxis width={46} domain={['auto', 'auto']} {...AXIS} />
          <Tooltip {...TOOLTIP} />
          <Bar
            dataKey='value'
            fill='var(--chart-1)'
            radius={[2, 2, 0, 0]}
            isAnimationActive={false}
          />
          {selection}
        </BarChart>
      ) : type === 'line' ? (
        <LineChart data={rows} margin={margin} {...dragProps}>
          <CartesianGrid stroke='var(--grid-line)' vertical={false} />
          <XAxis dataKey='t' minTickGap={42} {...AXIS} />
          <YAxis width={46} domain={['auto', 'auto']} {...AXIS} />
          <Tooltip {...TOOLTIP} />
          <Line
            type='monotone'
            dataKey='value'
            stroke='var(--chart-1)'
            strokeWidth={2}
            dot={false}
            isAnimationActive={false}
          />
          {selection}
        </LineChart>
      ) : (
        <AreaChart data={rows} margin={margin} {...dragProps}>
          <defs>
            <linearGradient id={gradientId} x1='0' y1='0' x2='0' y2='1'>
              <stop offset='0%' stopColor='var(--chart-1)' stopOpacity={0.25} />
              <stop offset='100%' stopColor='var(--chart-1)' stopOpacity={0} />
            </linearGradient>
          </defs>
          <CartesianGrid stroke='var(--grid-line)' vertical={false} />
          <XAxis dataKey='t' minTickGap={42} {...AXIS} />
          <YAxis width={46} domain={['auto', 'auto']} {...AXIS} />
          <Tooltip {...TOOLTIP} />
          <Area
            type='monotone'
            dataKey='value'
            stroke='var(--chart-1)'
            strokeWidth={2}
            fill={`url(#${gradientId})`}
            isAnimationActive={false}
          />
          {selection}
        </AreaChart>
      )}
    </ResponsiveContainer>
  )
}

/** Compact tabular view of the same rows, newest first. */
function HistoryTable({
  rows,
  height,
}: {
  rows: ChartRow[]
  height: ChartHeight
}) {
  return (
    <div className='scroll overflow-y-auto' style={{ height }}>
      <table className='w-full text-[11.5px]'>
        <tbody>
          {[...rows].reverse().map((r, i) => (
            <tr key={i} className='border-b border-border/60 last:border-0'>
              <td className='py-1 pe-2 text-muted-foreground'>{r.t}</td>
              <td className='tabular py-1 text-end font-medium'>
                {r.value.toFixed(1)}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
