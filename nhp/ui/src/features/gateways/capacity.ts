/**
 * Per-network device-limit math (DOMAIN-MODEL "Device limit"). The rubix gate
 * cannot express a count-based writeRule (WS-02 verified: it enforces `required`
 * + field TYPE only — see nhp/collections/enforce.mjs `deviceLimitViolation`,
 * logged RUBIX-TEAM), so the cap is enforced CLIENT-SIDE: the add-meter / network
 * UI shows remaining capacity and blocks an over-cap add.
 *
 * This is the TS mirror of enforce.mjs's `deviceLimitViolation` concept for the
 * browser build (the `.mjs` is Node-only, outside `src`). Pure functions, reused
 * by WS-06's wizards.
 */
import type { MeterRecord, NetworkRecord } from '@/api/records'

export interface Capacity {
  /** The network's `max_devices` cap. */
  cap: number
  /** Meters currently on the network. */
  used: number
  /** `cap - used`, floored at 0. */
  remaining: number
  /** True when `used >= cap` — an add must be blocked. */
  full: boolean
}

/** Meters whose `network` relation points at `networkId`. */
export function metersOnNetwork(
  networkId: string,
  meters: MeterRecord[]
): MeterRecord[] {
  return meters.filter((m) => m.content.network === networkId)
}

/** Remaining-capacity summary for one network given the full meter set. */
export function capacityFor(
  network: NetworkRecord,
  meters: MeterRecord[]
): Capacity {
  const cap = network.content.max_devices
  const used = metersOnNetwork(network.id, meters).length
  const remaining = Math.max(0, cap - used)
  return { cap, used, remaining, full: used >= cap }
}
