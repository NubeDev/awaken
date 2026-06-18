/**
 * Device-inventory assembly for the Site Overview report (pure). A flat per-meter
 * list under one site: meter, gateway, network protocol (modbus/lora), meter-type,
 * a device-agnostic headline reading (primaryMetric value+unit), and status. This
 * is the part of a site not already on any single dashboard page, so it lives here
 * (testable, no React) and the report just renders the rows.
 *
 * Protocol is reached meter → network → `network.protocol` (a meter only knows its
 * network id); gateway name via the resolved `meterLocation`. Honest: every value
 * is the meter's own latest sample over the window already fetched.
 */
import type { MeterRecord, NetworkRecord } from '@/api/records'
import type { HistorySample } from '@/features/dashboards/query/batch'
import { primaryMetric, type PrimaryMetric } from '@/features/dashboards/auto-build/primary-metric'
import type { ResolvedWindow } from '@/features/dashboards/query/time-window'
import type { PortfolioIndex } from '../scope'

export interface InventoryRow {
  meterId: string
  meterName: string
  gatewayKey?: string
  protocol: string
  meterType: string
  status: string
  metric: PrimaryMetric
}

/**
 * One row per meter at the site. `meters` is the site's meter set (already scoped);
 * `registers`/`history` are the whole windowed fetch — primaryMetric filters to each
 * meter by its `meter:<key>` tag. Sorted by meter name for a stable printed order.
 */
export function siteInventory(
  index: PortfolioIndex,
  meters: MeterRecord[],
  networks: NetworkRecord[],
  registers: { content: { tags?: string[] } }[],
  history: HistorySample[],
  resolved: ResolvedWindow
): InventoryRow[] {
  const protocolByNetwork = new Map(networks.map((n) => [n.id, n.content.protocol]))
  return meters
    .map((m) => {
      const loc = index.meterLocation.get(m.id)
      return {
        meterId: m.id,
        meterName: m.content.name,
        gatewayKey: loc?.gatewayKey,
        protocol: protocolByNetwork.get(m.content.network) ?? '—',
        meterType: index.meterTypeName(m.content.meter_type),
        status: m.content.status ?? 'unknown',
        metric: primaryMetric(
          m.content.key,
          registers as Parameters<typeof primaryMetric>[1],
          history,
          resolved
        ),
      }
    })
    .sort((a, b) => a.meterName.localeCompare(b.meterName))
}
