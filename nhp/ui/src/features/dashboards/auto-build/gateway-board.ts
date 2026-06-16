/**
 * Gateway board builder (DASHBOARDS.md: gateway page shows "network list with
 * device counts vs max_devices, gateway online/offline + last_seen"). Built from
 * `gateway:<key>` tags. PURE — returns the gateway's status + a network table.
 *
 * A network's device count is the meters tagged into it (`network:<key>`), shown
 * against the network's `max_devices` cap (DOMAIN-MODEL §network "Device limit").
 */
import type { GatewayRecord, MeterRecord, NetworkRecord } from '@/api/records'
import { gatewayTag, networkTag } from '@/enums/tags'
import type { RollupStatus } from '../widgets/status-tile'
import type { TableWidget } from '../widgets/types'

export interface GatewayBoard {
  status: RollupStatus
  lastSeen?: string
  networkTable: TableWidget
}

export function buildGatewayBoard(
  gatewayKey: string,
  gateways: GatewayRecord[],
  networks: NetworkRecord[],
  meters: MeterRecord[]
): GatewayBoard | null {
  const gw = gateways.find((g) => g.content.key === gatewayKey)
  if (!gw) return null

  const gTag = gatewayTag(gatewayKey)
  const gwNetworks = networks.filter((n) => (n.content.tags ?? []).includes(gTag))

  const rows = gwNetworks
    .map((net) => {
      const nTag = networkTag(net.content.key)
      const count = meters.filter((m) => (m.content.tags ?? []).includes(nTag)).length
      const max = net.content.max_devices
      return {
        name: net.content.name ?? net.content.key,
        type: net.content.net_type,
        protocol: net.content.protocol,
        devices: `${count} / ${max}`,
      }
    })
    .sort((a, b) => a.name.localeCompare(b.name))

  return {
    status: (gw.content.status as RollupStatus) ?? 'unknown',
    lastSeen: gw.content.last_seen,
    networkTable: {
      type: 'table',
      title: 'Networks',
      columns: [
        { key: 'name', label: 'Network' },
        { key: 'type', label: 'Type' },
        { key: 'protocol', label: 'Protocol' },
        { key: 'devices', label: 'Devices' },
      ],
      rows,
    },
  }
}
