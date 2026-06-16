/**
 * Site board builder (DASHBOARDS.md: site page shows "gateway cards + a site-wide
 * summary, e.g. total power across meters via quantity:power"). Built from
 * `site:<key>` tags. PURE — returns gateway cards + a cross-meter quantity panel.
 *
 * The cross-meter summary uses the `quantity:<q>` tag (DASHBOARDS.md §"Chart
 * grouping": cross-cut a quantity across meters). The POC draws the per-meter
 * `quantity:power` trend — one series per meter — so the site page compares power
 * across its meters in one panel.
 */
import type { GatewayRecord, MeterRecord, RegisterRec } from '@/api/records'
import { siteTag, gatewayTag, quantityTag } from '@/enums/tags'
import type { HistorySample } from '../query/batch'
import { resolveWindow, withinWindow, type WindowToken } from '../query/time-window'
import { alarmCountsByMeter } from './alarm-eval'
import { energyTrend, type EnergyTrend } from './energy-trend'
import type { RollupStatus } from '../widgets/status-tile'
import type { Series, TrendWidget } from '../widgets/types'

export interface GatewayCard {
  key: string
  name: string
  status: RollupStatus
  lastSeen?: string
  meterCount: number
  alarmCount: number
  /** Per-gateway energy trend over the window (summed across its meters). */
  energy: EnergyTrend
}

/** Site-wide rollups for the KPI strip — honest sums across the site's gateways. */
export interface SiteKpis {
  gateways: number
  meters: number
  alarms: number
  /** Latest summed energy across the site, and its unit (kWh). */
  energy: number | null
  energyUnit?: string
}

export interface SiteBoard {
  kpis: SiteKpis
  gateways: GatewayCard[]
  /** Cross-meter quantity:power comparison, or null if no power registers. */
  powerPanel: TrendWidget | null
}

export function buildSiteBoard(
  siteKey: string,
  gateways: GatewayRecord[],
  meters: MeterRecord[],
  registers: RegisterRec[],
  history: HistorySample[],
  window: WindowToken,
  timezone: string | undefined
): SiteBoard {
  const sTag = siteTag(siteKey)
  const siteGateways = gateways.filter((g) => (g.content.tags ?? []).includes(sTag))
  const siteMeters = meters.filter((m) => (m.content.tags ?? []).includes(sTag))
  const alarmByMeter = alarmCountsByMeter(registers, history)
  const resolved = resolveWindow(window)

  const gatewayCards: GatewayCard[] = siteGateways
    .map((gw) => {
      const gTag = gatewayTag(gw.content.key)
      const gwMeters = siteMeters.filter((m) => (m.content.tags ?? []).includes(gTag))
      return {
        key: gw.content.key,
        name: gw.content.name,
        status: (gw.content.status as RollupStatus) ?? 'unknown',
        lastSeen: gw.content.last_seen,
        meterCount: gwMeters.length,
        alarmCount: gwMeters.reduce((s, m) => s + (alarmByMeter.get(m.id) ?? 0), 0),
        energy: energyTrend(gTag, registers, history, resolved),
      }
    })
    .sort((a, b) => a.name.localeCompare(b.name))

  const kpis: SiteKpis = {
    gateways: gatewayCards.length,
    meters: siteMeters.length,
    alarms: gatewayCards.reduce((s, g) => s + g.alarmCount, 0),
    energy: energyTrend(sTag, registers, history, resolved).latest,
    energyUnit: gatewayCards.find((g) => g.energy.unit)?.energy.unit,
  }

  // Cross-meter power: each meter's quantity:power register becomes one series.
  const pTag = quantityTag('power')
  const meterById = new Map(siteMeters.map((m) => [m.id, m.content.name]))
  const powerRegs = registers.filter(
    (r) => (r.content.tags ?? []).includes(pTag) && (r.content.tags ?? []).includes(sTag)
  )
  const series: Series[] = powerRegs
    .map((reg) => {
      const samples = withinWindow(
        history.filter((h) => h.series === reg.id),
        resolved
      )
      return {
        label: meterById.get(reg.content.meter) ?? reg.content.name,
        points: samples.map((s) => ({ t: Date.parse(s.at), v: s.value })),
      }
    })
    .filter((s) => s.points.length > 0)
    .sort((a, b) => a.label.localeCompare(b.label))

  const powerPanel: TrendWidget | null =
    series.length > 0
      ? {
          type: 'line',
          title: 'Power by meter',
          unit: powerRegs[0]?.content.unit,
          precision: powerRegs[0]?.content.precision,
          series,
          timezone,
        }
      : null

  return { kpis, gateways: gatewayCards, powerPanel }
}
