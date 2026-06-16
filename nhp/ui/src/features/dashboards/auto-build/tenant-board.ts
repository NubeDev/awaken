/**
 * Tenant board builder (DASHBOARDS.md: tenant page shows "site cards: online/
 * offline rollup, alarm count, total meters"). Built from `tenant:<key>` tags.
 * PURE — returns the card specs; the page renders them.
 *
 * Each site card rolls up the statuses of the gateways beneath it (rollup.ts: a
 * site is "degraded" if any gateway is offline), counts the meters under the site,
 * and sums the active alarms across those meters (alarm-eval.ts).
 */
import type { GatewayRecord, MeterRecord, RegisterRec, SiteRecord } from '@/api/records'
import { tenantTag, siteTag } from '@/enums/tags'
import type { HistorySample } from '../query/batch'
import { resolveWindow, type WindowToken } from '../query/time-window'
import { alarmCountsByMeter } from './alarm-eval'
import { energyTrend, type EnergyTrend } from './energy-trend'
import { rollupStatus } from './rollup'
import type { RollupStatus } from '../widgets/status-tile'

export interface SiteCard {
  key: string
  name: string
  status: RollupStatus
  meterCount: number
  gatewayCount: number
  alarmCount: number
  /** Site-wide energy trend over the window (summed across meters), for a row
   *  sparkline. `unit` is the energy register unit (kWh). Empty when no energy. */
  energy: EnergyTrend
}

export function buildTenantBoard(
  tenantKey: string,
  sites: SiteRecord[],
  gateways: GatewayRecord[],
  meters: MeterRecord[],
  registers: RegisterRec[],
  history: HistorySample[],
  window: WindowToken
): SiteCard[] {
  const tTag = tenantTag(tenantKey)
  const tenantSites = sites.filter((s) => (s.content.tags ?? []).includes(tTag))
  const alarmByMeter = alarmCountsByMeter(registers, history)
  const resolved = resolveWindow(window)

  return tenantSites
    .map((site) => {
      const sTag = siteTag(site.content.key)
      const siteGateways = gateways.filter((g) => (g.content.tags ?? []).includes(sTag))
      const siteMeters = meters.filter((m) => (m.content.tags ?? []).includes(sTag))
      const alarmCount = siteMeters.reduce((sum, m) => sum + (alarmByMeter.get(m.id) ?? 0), 0)
      return {
        key: site.content.key,
        name: site.content.name,
        status: rollupStatus(siteGateways.map((g) => g.content)),
        meterCount: siteMeters.length,
        gatewayCount: siteGateways.length,
        alarmCount,
        energy: energyTrend(sTag, registers, history, resolved),
      }
    })
    .sort((a, b) => a.name.localeCompare(b.name))
}
