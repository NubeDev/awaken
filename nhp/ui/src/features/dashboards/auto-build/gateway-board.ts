/**
 * Gateway board builder (DASHBOARDS.md: gateway page shows "network list with
 * device counts vs max_devices, gateway online/offline + last_seen"). Built from
 * `gateway:<key>` tags. PURE — returns the gateway's status + a network table.
 *
 * A network's device count is the meters tagged into it (`network:<key>`), shown
 * against the network's `max_devices` cap (DOMAIN-MODEL §network "Device limit").
 */
import type {
  GatewayRecord,
  MeterRecord,
  Net485Params,
  NetEthernetParams,
  NetworkRecord,
  RegisterRec,
} from '@/api/records'
import { gatewayTag, meterTag, networkTag } from '@/enums/tags'
import type { HistorySample } from '../query/batch'
import { resolveWindow, type WindowToken } from '../query/time-window'
import { energyTrend, type EnergyTrend } from './energy-trend'
import type { RollupStatus } from '../widgets/status-tile'

/** One network's device utilisation against its cap, plus a human params hint. */
export interface NetworkRow {
  key: string
  name: string
  type: string
  protocol: string
  /** Devices currently tagged into the network. */
  count: number
  /** Per-network cap (DOMAIN-MODEL "Device limit"). */
  max: number
  /** A short params summary, e.g. "9600 8N1" or "10.0.0.4:502". */
  detail?: string
}

/** Headline counts for the gateway KPI strip — all honest rollups, no fabrication. */
export interface GatewayKpis {
  networks: number
  /** Devices used across all networks, and the summed capacity. */
  devices: number
  capacity: number
  /** Meters under the gateway, and how many report `online`. */
  meters: number
  metersOnline: number
}

/** One meter under the gateway: identity + status + its energy (kWh) trend. */
export interface MeterRow {
  id: string
  name: string
  status: string
  lastSeen?: string
  energy: EnergyTrend
}

export interface GatewayBoard {
  status: RollupStatus
  lastSeen?: string
  model?: string
  host?: string
  kpis: GatewayKpis
  networks: NetworkRow[]
  meters: MeterRow[]
}

/** Build the params hint shown under a network's name (serial vs tcp shapes). */
function paramsDetail(net: NetworkRecord['content']): string | undefined {
  if (!net.params) return undefined
  if (net.net_type === '485') {
    const p = net.params as Net485Params
    const parity = p.parity?.[0]?.toUpperCase() ?? 'N'
    return `${p.baud} ${p.data_bits}${parity}${p.stop_bits}`
  }
  const p = net.params as NetEthernetParams
  return p.ip ? `${p.ip}:${p.port}` : undefined
}

export function buildGatewayBoard(
  gatewayKey: string,
  gateways: GatewayRecord[],
  networks: NetworkRecord[],
  meters: MeterRecord[],
  registers: RegisterRec[],
  history: HistorySample[],
  window: WindowToken
): GatewayBoard | null {
  const gw = gateways.find((g) => g.content.key === gatewayKey)
  if (!gw) return null

  const gTag = gatewayTag(gatewayKey)
  const gwNetworks = networks.filter((n) => (n.content.tags ?? []).includes(gTag))
  const gwMeters = meters.filter((m) => (m.content.tags ?? []).includes(gTag))
  const resolved = resolveWindow(window)

  const rows: NetworkRow[] = gwNetworks
    .map((net) => {
      const nTag = networkTag(net.content.key)
      const count = meters.filter((m) => (m.content.tags ?? []).includes(nTag)).length
      return {
        key: net.content.key,
        name: net.content.name ?? net.content.key,
        type: net.content.net_type,
        protocol: net.content.protocol,
        count,
        max: net.content.max_devices,
        detail: paramsDetail(net.content),
      }
    })
    .sort((a, b) => a.name.localeCompare(b.name))

  const kpis: GatewayKpis = {
    networks: rows.length,
    devices: rows.reduce((s, r) => s + r.count, 0),
    capacity: rows.reduce((s, r) => s + r.max, 0),
    meters: gwMeters.length,
    metersOnline: gwMeters.filter((m) => m.content.status === 'online').length,
  }

  const meterRows: MeterRow[] = gwMeters
    .map((m) => ({
      id: m.id,
      name: m.content.name,
      status: m.content.status ?? 'unknown',
      lastSeen: m.content.last_seen,
      energy: energyTrend(meterTag(m.content.key), registers, history, resolved),
    }))
    .sort((a, b) => a.name.localeCompare(b.name))

  return {
    status: (gw.content.status as RollupStatus) ?? 'unknown',
    lastSeen: gw.content.last_seen,
    model: gw.content.model,
    host: gw.content.host,
    kpis,
    networks: rows,
    meters: meterRows,
  }
}
