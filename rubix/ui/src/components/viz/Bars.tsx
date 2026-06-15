// Horizontal labelled bar list — ported from viz.js `bars()`.

import { col } from './colors'

export interface BarRow {
  label: string
  kw: number
  pct: number
  color: string
}

export function Bars({ rows }: { rows: BarRow[] }) {
  const mx = Math.max(...rows.map((r) => r.kw))
  return (
    <div className="space-y-2.5">
      {rows.map((r) => (
        <div key={r.label} className="flex items-center gap-3">
          <div className="w-[92px] text-[13px] text-fg/80 shrink-0">{r.label}</div>
          <div className="flex-1 h-6 rounded-md bg-bg/50 overflow-hidden">
            <div
              className="h-full rounded-md"
              style={{
                width: `${((r.kw / mx) * 100).toFixed(0)}%`,
                background: col(r.color),
                transition: 'width .7s cubic-bezier(.2,.7,.2,1)',
              }}
            />
          </div>
          <div className="w-[92px] text-right mono text-[13px]">
            <b>{r.kw}</b>
            <span className="text-muted">
              {' '}
              kW·{r.pct}%
            </span>
          </div>
        </div>
      ))}
    </div>
  )
}
