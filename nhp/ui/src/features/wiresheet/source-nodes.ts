/**
 * Turn the LIVE portfolio (reused via reporting's usePortfolio → PortfolioIndex)
 * into wiresheet "source" blocks: one draggable point per meter register, grouped
 * by site → meter so the palette mirrors the real hierarchy. A source block has no
 * inputs and one typed output (its quantity), and carries the register's unit +
 * latest-known label so the canvas reads like real instrumentation.
 *
 * This is the one place the demo touches real data — everything downstream of a
 * source is illustrative logic (palette.ts). Keeping the wiring here means the
 * canvas component stays agnostic about where points come from.
 */
import type { PortfolioIndex } from '@/features/reporting/scope'

export interface SourcePoint {
  /** Stable id used as the dnd payload + node seed. */
  type: string
  registerId: string
  name: string
  meterName: string
  siteName: string
  unit: string
  quantity: string
}

export interface SourceGroup {
  siteName: string
  meters: { meterName: string; points: SourcePoint[] }[]
}

/** Walk the index into grouped source points, history-bearing registers first. */
export function buildSourceGroups(index: PortfolioIndex): SourceGroup[] {
  const bySite = new Map<string, Map<string, SourcePoint[]>>()

  for (const meter of index.data.meters) {
    const loc = index.meterLocation.get(meter.id)
    const siteName = loc?.siteName ?? 'Unassigned'
    const meterName = meter.content.name
    const regs = index.registersByMeter.get(meter.id) ?? []

    for (const reg of regs) {
      const point: SourcePoint = {
        type: `src:${reg.id}`,
        registerId: reg.id,
        name: reg.content.name,
        meterName,
        siteName,
        unit: reg.content.unit,
        quantity: reg.content.quantity,
      }
      if (!bySite.has(siteName)) bySite.set(siteName, new Map())
      const meters = bySite.get(siteName)!
      if (!meters.has(meterName)) meters.set(meterName, [])
      meters.get(meterName)!.push(point)
    }
  }

  return [...bySite.entries()]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([siteName, meters]) => ({
      siteName,
      meters: [...meters.entries()].map(([meterName, points]) => ({
        meterName,
        points,
      })),
    }))
}
