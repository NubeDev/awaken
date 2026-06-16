import { Area, AreaChart, ResponsiveContainer } from 'recharts'

type SparklineProps = {
  /** Time-stamped points in ascending time; `v` may be null (a gap). */
  points: { t: number; v: number | null }[]
  color?: string
  height?: number
}

/**
 * Tiny inline trend for KPI tiles: a filled, axis-less, tooltip-less area sweep
 * so an operator reads the SHAPE of a metric without opening its chart. Tracks
 * the theme via the passed `color` (a `--chart-N` var). Fewer than two real
 * points → nothing (a single dot is noise, not a trend).
 */
export function Sparkline({
  points,
  color = 'var(--chart-1)',
  height = 36,
}: SparklineProps) {
  const real = points.filter((p) => p.v !== null)
  if (real.length < 2) return null
  const id = `spark-${color.replace(/[^a-z0-9]/gi, '')}`
  return (
    <ResponsiveContainer width='100%' height={height}>
      <AreaChart data={points} margin={{ top: 2, right: 0, left: 0, bottom: 0 }}>
        <defs>
          <linearGradient id={id} x1='0' y1='0' x2='0' y2='1'>
            <stop offset='0%' stopColor={color} stopOpacity={0.35} />
            <stop offset='100%' stopColor={color} stopOpacity={0.02} />
          </linearGradient>
        </defs>
        <Area
          type='monotone'
          dataKey='v'
          stroke={color}
          strokeWidth={1.75}
          fill={`url(#${id})`}
          isAnimationActive={false}
          connectNulls
          dot={false}
        />
      </AreaChart>
    </ResponsiveContainer>
  )
}
