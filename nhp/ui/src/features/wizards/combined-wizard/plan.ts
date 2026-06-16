/**
 * Combined "add everything" plan builder (WS-06 task 5): one ordered plan for a
 * greenfield tree — tenant → site → gateway(+N networks) → meters(on the first
 * network) → registers — threading parent ids as `parentRef`s so the batch writer
 * resolves each child from its just-written parent (WIZARDS.md §"Add everything").
 *
 * Because every record is brand-new, ALL parent relations are late-bound here
 * (unlike the standalone wizards where the parent already exists). Tags are built
 * from the KEYS the operator entered (known up front) via the shared tag module
 * (enums/tags.ts), so the whole new tree carries the same tags the seed applies
 * and WS-07 auto-builds from. The optional first user is a principal, written
 * outside this plan (admin API, not the records batch) — see combined-wizard.tsx.
 */
import type { MeterTypeRecord, NetParams } from '@/api/records'
import type { NetType, Protocol } from '@/enums/options'
import {
  gatewayTags,
  meterTags,
  networkTags,
  registerTags,
  siteTags,
} from '@/enums/tags'
import type { PlannedRecord } from '../_shared/batch-write'
import { addressRange } from '../meters-wizard/plan'
import { networkKeys } from '../gateway-wizard/plan'

export interface CombinedInput {
  tenant: { key: string; name: string }
  site: { key: string; name: string; address: string; timezone: string }
  gateway: { key: string; name: string; model: string; host: string }
  networks: {
    count: number
    netType: NetType
    protocol: Protocol
    maxDevices: number
    namePattern: string
    params: NetParams
  }
  /** Meters land on the FIRST generated network. */
  meters: {
    type: MeterTypeRecord | undefined
    addressFrom: number
    addressTo: number
    keyPattern: string
    namePattern: string
  }
}

const TENANT = 'tenant'
const SITE = 'site'
const GATEWAY = 'gateway'

export function buildCombinedPlan(input: CombinedInput): PlannedRecord[] {
  const { tenant, site, gateway, networks, meters } = input
  const tKey = tenant.key
  const sKey = site.key
  const gKey = gateway.key

  const plan: PlannedRecord[] = []

  // Tenant (root, no ancestor tags — same as seed).
  plan.push({
    id: TENANT,
    label: `tenant ${tKey}`,
    kind: 'tenant',
    content: { kind: 'tenant', key: tKey, name: tenant.name, namespace: 'acme', tags: [] },
  })

  // Site → tenant.
  plan.push({
    id: SITE,
    label: `site ${sKey}`,
    kind: 'site',
    content: {
      kind: 'site',
      key: sKey,
      name: site.name,
      tenant: '',
      address: site.address,
      timezone: site.timezone,
      tags: siteTags({ tenant: tKey }),
    },
    parentRefs: [{ field: 'tenant', planId: TENANT }],
  })

  // Gateway → site.
  plan.push({
    id: GATEWAY,
    label: `gateway ${gKey}`,
    kind: 'gateway',
    content: {
      kind: 'gateway',
      key: gKey,
      name: gateway.name,
      site: '',
      model: gateway.model,
      host: gateway.host,
      tags: gatewayTags({ tenant: tKey, site: sKey }),
    },
    parentRefs: [{ field: 'site', planId: SITE }],
  })

  // N networks → gateway.
  const netKeys = networkKeys(networks)
  for (const nKey of netKeys) {
    plan.push({
      id: `net-${nKey}`,
      label: `network ${nKey}`,
      kind: 'network',
      content: {
        kind: 'network',
        key: nKey,
        name: nKey,
        gateway: '',
        net_type: networks.netType,
        protocol: networks.protocol,
        max_devices: networks.maxDevices,
        params: networks.params,
        tags: networkTags({ tenant: tKey, site: sKey, gateway: gKey }),
      },
      parentRefs: [{ field: 'gateway', planId: GATEWAY }],
    })
  }

  // Meters on the FIRST network (if a type + range were given), with stamped registers.
  const firstNetKey = netKeys[0]
  if (meters.type && firstNetKey) {
    const type = meters.type
    const firstNetPlanId = `net-${firstNetKey}`
    for (const addr of addressRange(meters.addressFrom, meters.addressTo)) {
      const mKey = meters.keyPattern.replace(/\{n\}/g, String(addr))
      const mName = meters.namePattern.replace(/\{n\}/g, String(addr))
      const mPlanId = `meter-${mKey}`
      plan.push({
        id: mPlanId,
        label: `meter ${mKey} (unit ${addr})`,
        kind: 'meter',
        content: {
          kind: 'meter',
          key: mKey,
          name: mName,
          network: '',
          meter_type: type.id,
          meter_type_version: type.content.version,
          address: addr,
          tags: meterTags({
            tenant: tKey,
            site: sKey,
            gateway: gKey,
            network: firstNetKey,
            meterType: type.content.key,
          }),
        },
        parentRefs: [{ field: 'network', planId: firstNetPlanId }],
      })
      for (const def of type.content.registers) {
        const regKey = `${mKey}--${def.key}`
        plan.push({
          id: `reg-${regKey}`,
          label: `register ${regKey}`,
          kind: 'register',
          content: {
            ...def,
            key: regKey,
            meter: '',
            tags: registerTags(
              {
                tenant: tKey,
                site: sKey,
                gateway: gKey,
                network: firstNetKey,
                meter: mKey,
              },
              def
            ),
          },
          parentRefs: [{ field: 'meter', planId: mPlanId }],
        })
      }
    }
  }

  return plan
}
