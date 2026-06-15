// Tiny inline sparkline — ported from viz.js `spark()`.

import { col } from './colors'

interface SparkProps {
  data: number[]
  color?: string
  width?: number
  height?: number
}

export function Spark({ data, color = 'r1', width = 120, height = 34 }: SparkProps) {
  const P = 3
  const mn = Math.min(...data)
  const mx = Math.max(...data)
  const X = (i: number) => (i / (data.length - 1)) * width
  const Y = (v: number) => height - P - ((v - mn) / (mx - mn || 1)) * (height - P * 2)
  const d = data.map((v, i) => (i ? 'L' : 'M') + X(i).toFixed(1) + ' ' + Y(v).toFixed(1)).join(' ')
  return (
    <svg viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="none" style={{ width: '100%', height }}>
      <path d={d} fill="none" stroke={col(color)} strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  )
}
