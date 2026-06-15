// Donut/ring chart — ported from viz.js `donut()`.

import { col } from './colors'

export interface DonutSegment {
  pct: number
  color: string
}

export function Donut({ segments, size = 104 }: { segments: DonutSegment[]; size?: number }) {
  const r = 15.9
  let off = 0
  return (
    <svg viewBox="0 0 42 42" style={{ width: size, height: size }} className="-rotate-90">
      <circle cx={21} cy={21} r={r} fill="none" stroke="hsl(230 10% 18%)" strokeWidth={5} />
      {segments.map((seg, i) => {
        const node = (
          <circle
            key={i}
            cx={21}
            cy={21}
            r={r}
            fill="none"
            stroke={col(seg.color)}
            strokeWidth={5}
            strokeDasharray={`${seg.pct} ${100 - seg.pct}`}
            strokeDashoffset={-off}
            strokeLinecap="round"
          />
        )
        off += seg.pct
        return node
      })}
    </svg>
  )
}
