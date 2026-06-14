import { Area, AreaChart, ResponsiveContainer } from 'recharts'

type SparklineProps = {
  data: number[]
  color?: string
  height?: number
  width?: number
}

/** Tiny inline trend line for KPI cards. Filled area, no axes. */
export function Sparkline({
  data,
  color = 'var(--chart-1)',
  height = 32,
  width = 88,
}: SparklineProps) {
  const id = `spark-${color.replace(/[^a-z0-9]/gi, '')}`
  const rows = data.map((value, i) => ({ i, value }))
  return (
    <div style={{ width, height }}>
      <ResponsiveContainer width='100%' height='100%'>
        <AreaChart data={rows} margin={{ top: 2, right: 0, left: 0, bottom: 0 }}>
          <defs>
            <linearGradient id={id} x1='0' y1='0' x2='0' y2='1'>
              <stop offset='0%' stopColor={color} stopOpacity={0.25} />
              <stop offset='100%' stopColor={color} stopOpacity={0} />
            </linearGradient>
          </defs>
          <Area
            type='monotone'
            dataKey='value'
            stroke={color}
            strokeWidth={1.6}
            fill={`url(#${id})`}
            isAnimationActive={false}
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  )
}
