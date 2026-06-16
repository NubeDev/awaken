/**
 * Shared energy-rollup helper: sum a scope's `quantity:energy` registers into ONE
 * kWh trend by bucketing every sample by its instant and adding the per-meter
 * values at each bucket. Used for the per-site row sparkline (tenant-board) and
 * the per-gateway row sparkline (site-board) — same maths, different scope tag.
 * PURE. Returns ascending-time points clipped to the window, plus the latest
 * summed reading (the headline kWh value) — null when the scope logs no energy.
 */
import type { RegisterRec } from '@/api/records'
import { quantityTag } from '@/enums/tags'
import type { HistorySample } from '../query/batch'
import { withinWindow, type ResolvedWindow } from '../query/time-window'

export interface EnergyTrend {
  points: { t: number; v: number | null }[]
  /** Latest summed reading, the headline kWh value, or null. */
  latest: number | null
  unit?: string
}

export function energyTrend(
  scopeTag: string,
  registers: RegisterRec[],
  history: HistorySample[],
  resolved: ResolvedWindow
): EnergyTrend {
  const eTag = quantityTag('energy')
  const regs = registers.filter(
    (r) => (r.content.tags ?? []).includes(eTag) && (r.content.tags ?? []).includes(scopeTag)
  )
  if (regs.length === 0) return { points: [], latest: null }
  const regIds = new Set(regs.map((r) => r.id))
  const byInstant = new Map<number, number>()
  for (const h of history) {
    if (!regIds.has(h.series)) continue
    const t = Date.parse(h.at)
    byInstant.set(t, (byInstant.get(t) ?? 0) + h.value)
  }
  const all = [...byInstant.entries()]
    .map(([t, v]) => ({ at: new Date(t).toISOString(), value: v }))
    .sort((a, b) => Date.parse(a.at) - Date.parse(b.at))
  const within = withinWindow(all, resolved)
  const points = within.map((s) => ({ t: Date.parse(s.at), v: s.value }))
  const latest = points.length ? points[points.length - 1].v : null
  return { points, latest, unit: regs[0].content.unit }
}
