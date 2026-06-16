/**
 * Plan builder for the bulk-meters wizard (WS-06 task 3). For an address range on
 * one network, with a chosen meter-type, it generates: per address a `kind:"meter"`
 * record (stamped with `meter_type` + `meter_type_version`, DOMAIN-MODEL
 * §versioning), then that meter's `kind:"register"` records stamped from the type's
 * `registers[]` — the SAME stamping the seed and WS-04 re-apply do
 * (key `${meterKey}--${defKey}`). Registers late-bind their `meter` relation to the
 * meter's written id (parentRef), so the batch writer creates the meter first.
 *
 * Tags come from the shared module (enums/tags.ts): meterTags on the meter,
 * registerTags (adds group:/quantity:) on each register. The over-cap BLOCK lives
 * in the UI (capacity.ts); this builder assumes the count already fits.
 */
import type { Meter, MeterTypeRecord, RegisterRecord } from '@/api/records'
import { meterTags, registerTags } from '@/enums/tags'
import type { PlannedRecord } from '../_shared/batch-write'

export interface MetersInput {
  /** Parent network RECORD id (relation) + its key (for tags). */
  networkId: string
  networkKey: string
  /** Hierarchy KEYS for the standard tags (resolved from the network's ancestry). */
  tenantKey: string
  siteKey: string
  gatewayKey: string
  /** The meter-type to stamp from. */
  type: MeterTypeRecord
  /** Inclusive bus-address range; one meter per address. */
  addressFrom: number
  addressTo: number
  /** Meter key/name pattern, `{n}` → the bus address. */
  keyPattern: string
  namePattern: string
}

/** The bus addresses the range expands to (inclusive). */
export function addressRange(from: number, to: number): number[] {
  if (!Number.isFinite(from) || !Number.isFinite(to) || to < from) return []
  return Array.from({ length: to - from + 1 }, (_, i) => from + i)
}

export function buildMetersPlan(input: MetersInput): PlannedRecord[] {
  const plan: PlannedRecord[] = []
  const typeKey = input.type.content.key
  const typeVersion = input.type.content.version

  for (const addr of addressRange(input.addressFrom, input.addressTo)) {
    const meterKey = input.keyPattern.replace(/\{n\}/g, String(addr))
    const meterName = input.namePattern.replace(/\{n\}/g, String(addr))
    const meterPlanId = `meter-${meterKey}`

    const meterContent: Meter = {
      kind: 'meter',
      key: meterKey,
      name: meterName,
      network: input.networkId, // relation by record id
      meter_type: input.type.id,
      meter_type_version: typeVersion, // stamp-on-create (DOMAIN-MODEL §versioning)
      address: addr,
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
      label: `meter ${meterKey} (unit ${addr})`,
      kind: 'meter',
      content: meterContent as unknown as Record<string, unknown>,
    })

    // Stamp the type's register-defs onto the meter (same as portfolio.mjs / WS-04).
    for (const def of input.type.content.registers) {
      const regKey = `${meterKey}--${def.key}`
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
            meter: meterKey,
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
  }

  return plan
}
