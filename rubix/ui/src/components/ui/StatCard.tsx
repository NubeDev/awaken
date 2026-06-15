// A single vital stat tile and a row of them — ported from the demo's stat strip
// / statRow widgets.

import { col } from '../viz/colors'

export interface Stat {
  label: string
  value: string
  unit?: string
  tone?: string
  delta?: string
  deltaGood?: boolean
}

export function StatCard({ stat }: { stat: Stat }) {
  return (
    <div className="rounded-xl border border-border bg-panel2 px-4 py-3">
      <div className="text-[11.5px] text-muted">{stat.label}</div>
      <div className="mono text-[20px] font-semibold mt-0.5" style={stat.tone ? { color: col(stat.tone) } : undefined}>
        {stat.value}
        {stat.unit && <span className="text-[11px] text-muted ml-1">{stat.unit}</span>}
      </div>
      {stat.delta && (
        <div className={`text-[11px] mt-0.5 ${stat.deltaGood ? 'text-green' : 'text-amber'}`}>{stat.delta}</div>
      )}
    </div>
  )
}

export function StatRow({ stats }: { stats: Stat[] }) {
  return (
    <div className="grid gap-3" style={{ gridTemplateColumns: `repeat(${stats.length}, minmax(0, 1fr))` }}>
      {stats.map((s) => (
        <StatCard key={s.label} stat={s} />
      ))}
    </div>
  )
}
