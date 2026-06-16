/**
 * Plan builder for the gateway + N-networks wizard (WS-06 task 2, the headline).
 * Turns the collected input into an ordered `PlannedRecord[]`: ONE gateway, then
 * N networks whose `gateway` relation is a `parentRef` to the gateway's
 * yet-to-be-written id (the batch writer resolves it after the gateway is created).
 *
 * The N networks are generated from a count + net_type/protocol/max_devices + a
 * naming pattern (`gw-01-net-{n}`) + per-type params defaults — verified to work
 * at N=30 (gateway-networks.plan.test.ts). Tags come from the SHARED tag module
 * (enums/tags.ts) so they match the seed + dashboard auto-build exactly.
 */
import type { Gateway, NetParams, Network } from '@/api/records'
import type { NetType, Protocol } from '@/enums/options'
import { gatewayTags, networkTags } from '@/enums/tags'
import type { PlannedRecord } from '../_shared/batch-write'

/** The gateway half of the wizard input. */
export interface GatewayInput {
  key: string
  name: string
  model: string
  host: string
  /** Parent site RECORD ID (relation) — already exists. */
  siteId: string
  /** Parent site KEY + tenant KEY — for the standard tags. */
  siteKey: string
  tenantKey: string
}

/** The bulk-networks half. `{n}` in `namePattern` is replaced with the index. */
export interface NetworksInput {
  count: number
  netType: NetType
  protocol: Protocol
  maxDevices: number
  /** Naming pattern, `{n}` → 1..count, e.g. `gw-01-net-{n}`. */
  namePattern: string
  /** Per-type params defaults applied to every generated network. */
  params: NetParams
}

/** Expand `{n}` in a pattern to a 1-based index. */
export function expandPattern(pattern: string, n: number): string {
  return pattern.replace(/\{n\}/g, String(n))
}

const GATEWAY_PLAN_ID = 'gateway'

/** The N network keys the wizard will generate (for preview + cross-checks). */
export function networkKeys(net: NetworksInput): string[] {
  return Array.from({ length: net.count }, (_, i) =>
    expandPattern(net.namePattern, i + 1)
  )
}

export function buildGatewayPlan(
  gw: GatewayInput,
  net: NetworksInput
): PlannedRecord[] {
  const gatewayContent: Gateway = {
    kind: 'gateway',
    key: gw.key,
    name: gw.name,
    site: gw.siteId, // relation by record id (matches WS-03 portfolio.mjs)
    model: gw.model,
    host: gw.host,
    // status/last_seen are POLLER-OWNED — the wizard never sets them.
    tags: gatewayTags({ tenant: gw.tenantKey, site: gw.siteKey }),
  }

  const plan: PlannedRecord[] = [
    {
      id: GATEWAY_PLAN_ID,
      label: `gateway ${gw.key}`,
      kind: 'gateway',
      content: gatewayContent as unknown as Record<string, unknown>,
    },
  ]

  for (const key of networkKeys(net)) {
    const content: Network = {
      kind: 'network',
      key,
      name: key,
      gateway: '', // late-bound: filled from the gateway's written id (parentRef)
      net_type: net.netType,
      protocol: net.protocol,
      max_devices: net.maxDevices,
      params: net.params,
      tags: networkTags({
        tenant: gw.tenantKey,
        site: gw.siteKey,
        gateway: gw.key,
      }),
    }
    plan.push({
      id: `net-${key}`,
      label: `network ${key}`,
      kind: 'network',
      content: content as unknown as Record<string, unknown>,
      parentRefs: [{ field: 'gateway', planId: GATEWAY_PLAN_ID }],
    })
  }

  return plan
}
