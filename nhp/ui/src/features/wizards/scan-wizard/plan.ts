/**
 * Plan builder for the scan-to-add-a-device wizard (WS-09 Part B). A scanned/typed
 * barcode resolves to a meter-type (enums/barcode.ts), and this builds the write
 * plan for ONE meter on the chosen network: a `kind:"meter"` record stamped with
 * `meter_type` + `meter_type_version` (DOMAIN-MODEL §versioning), then its
 * `kind:"register"` records stamped from the type's `registers[]` — the SAME
 * stamping the seed, WS-04 re-apply, and the bulk-meters wizard do
 * (key `${meterKey}--${defKey}`; registers late-bind their `meter` relation).
 *
 * Tags come from the shared module (enums/tags.ts) so a scan-added meter carries the
 * EXACT tags the seed/other wizards apply — a drift silently breaks the dashboards
 * (DASHBOARDS.md). The over-cap BLOCK lives in the UI (capacity.ts); this builder
 * assumes the meter fits. One meter per scan: scan-to-add is a single-device flow.
 */
import type { Meter, MeterTypeRecord, RegisterRecord } from '@/api/records'
import { meterTags, registerTags } from '@/enums/tags'
import type { PlannedRecord } from '../_shared/batch-write'

export interface ScanMeterInput {
  /** Parent network RECORD id (relation) + its key (for tags). */
  networkId: string
  networkKey: string
  /** Hierarchy KEYS for the standard tags (resolved from the network's ancestry). */
  tenantKey: string
  siteKey: string
  gatewayKey: string
  /** The meter-type the scanned barcode resolved to. */
  type: MeterTypeRecord
  /** The meter's bus (unit/slave) address on the network. */
  address: number
  /** The new meter's stable key + display name. */
  meterKey: string
  meterName: string
}

export function buildScanMeterPlan(input: ScanMeterInput): PlannedRecord[] {
  const plan: PlannedRecord[] = []
  const typeKey = input.type.content.key
  const typeVersion = input.type.content.version
  const meterPlanId = `meter-${input.meterKey}`

  const meterContent: Meter = {
    kind: 'meter',
    key: input.meterKey,
    name: input.meterName,
    network: input.networkId, // relation by record id
    meter_type: input.type.id,
    meter_type_version: typeVersion, // stamp-on-create (DOMAIN-MODEL §versioning)
    address: input.address,
    // status/last_seen are POLLER-OWNED — never set by the wizard.
    tags: meterTags({
      tenant: input.tenantKey,
      site: input.siteKey,
      gateway: input.gatewayKey,
      network: input.networkKey,
      meterType: typeKey,
    }),
  }
  plan.push({
    id: meterPlanId,
    label: `meter ${input.meterKey} (unit ${input.address})`,
    kind: 'meter',
    content: meterContent as unknown as Record<string, unknown>,
  })

  // Stamp the type's register-defs onto the meter (same as the bulk-meters wizard).
  for (const def of input.type.content.registers) {
    const regKey = `${input.meterKey}--${def.key}`
    const regContent: RegisterRecord = {
      ...def,
      key: regKey,
      meter: '', // late-bound from the meter's written id (parentRef)
      tags: registerTags(
        {
          tenant: input.tenantKey,
          site: input.siteKey,
          gateway: input.gatewayKey,
          network: input.networkKey,
          meter: input.meterKey,
        },
        def
      ),
    }
    plan.push({
      id: `reg-${regKey}`,
      label: `register ${regKey}`,
      kind: 'register',
      content: regContent as unknown as Record<string, unknown>,
      parentRefs: [{ field: 'meter', planId: meterPlanId }],
    })
  }

  return plan
}
