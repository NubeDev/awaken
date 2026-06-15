// Multi-series line chart — ported from viz.js `line()`. Renders grid lines, an
// optional setpoint/limit guide, and one or more series with an end-cap dot and
// optional area fill. `null` points break the line (offline gaps).

import { col } from './colors'

export interface Series {
  data: (number | null)[]
  color?: string
  dash?: string
  fill?: boolean
  dot?: boolean
  width?: number
  draw?: boolean
}

interface LineProps {
  series: Series[]
  height?: number
  min?: number
  max?: number
  setpoint?: number
  setpointLabel?: string
  limit?: number
  limitLabel?: string
}

const W = 620
const P = 14

export function Line({
  series,
  height = 140,
  min,
  max,
  setpoint,
  setpointLabel = 'setpoint',
  limit,
  limitLabel = 'limit',
}: LineProps) {
  const all = series.flatMap((s) => s.data).filter((v): v is number => v != null)
  const mn = min != null ? min : Math.min(...all)
  const mx = max != null ? max : Math.max(...all)
  const X = (i: number, n: number) => P + (i / (n - 1)) * (W - P * 2)
  const Y = (v: number) => height - P - ((v - mn) / (mx - mn || 1)) * (height - P * 2)

  return (
    <svg
      viewBox={`0 0 ${W} ${height}`}
      preserveAspectRatio="none"
      className="w-full"
      style={{ height }}
    >
      {[0, 1, 2, 3].map((g) => {
        const y = P + g * ((height - P * 2) / 3)
        return <line key={g} x1={P} y1={y} x2={W - P} y2={y} stroke={col('grid')} strokeWidth={1} />
      })}
      {setpoint != null && (
        <>
          <line
            x1={P}
            y1={Y(setpoint)}
            x2={W - P}
            y2={Y(setpoint)}
            stroke={col('axis')}
            strokeWidth={1}
            strokeDasharray="5 4"
          />
          <text x={W - P - 3} y={Y(setpoint) - 5} fill={col('axis')} fontSize={10} className="mono" textAnchor="end">
            {setpointLabel}
          </text>
        </>
      )}
      {limit != null && (
        <>
          <line
            x1={P}
            y1={Y(limit)}
            x2={W - P}
            y2={Y(limit)}
            stroke={col('crit')}
            strokeWidth={1.2}
            strokeDasharray="6 5"
            opacity={0.7}
          />
          <text x={W - P - 3} y={Y(limit) - 5} fill={col('crit')} fontSize={10} className="mono" textAnchor="end" opacity={0.85}>
            {limitLabel}
          </text>
        </>
      )}
      {series.map((se, si) => {
        const n = se.data.length
        let d = ''
        se.data.forEach((v, i) => {
          if (v == null) return
          d += (d ? 'L' : 'M') + X(i, n).toFixed(1) + ' ' + Y(v).toFixed(1) + ' '
        })
        const co = col(se.color || 'r1')
        const li = n - 1
        const lastVal = se.data[li]
        return (
          <g key={si}>
            {se.fill && (
              <path d={`${d} L${X(li, n)} ${height - P} L${X(0, n)} ${height - P} Z`} fill={co} opacity={0.12} />
            )}
            <path
              d={d}
              fill="none"
              stroke={co}
              strokeWidth={se.width || 2.2}
              strokeDasharray={se.dash}
              strokeLinecap="round"
              style={
                se.draw
                  ? { strokeDasharray: 1400, strokeDashoffset: 1400, animation: 'draw 1.1s ease forwards' }
                  : undefined
              }
            />
            {se.dot !== false && lastVal != null && (
              <circle cx={X(li, n)} cy={Y(lastVal)} r={3.5} fill={co} />
            )}
          </g>
        )
      })}
    </svg>
  )
}
