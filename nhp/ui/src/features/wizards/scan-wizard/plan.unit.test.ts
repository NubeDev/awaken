/**
 * Scan-wizard plan: a scanned barcode resolves to a meter-type, and the builder
 * stamps ONE meter + its registers from that type with the standard tags and the
 * late-bound meter relation (WS-09 Part B). Mirrors meters-wizard/plan.unit.test.ts;
 * the over-cap BLOCK is enforced in the UI via capacity.ts. Also asserts the full
 * barcode → resolve → plan path so the round-trip is covered end-to-end.
 */
import { describe, expect, it } from 'vitest'
import type { MeterTypeRecord } from '@/api/records'
import { encodeBarcode, resolveBarcode } from '@/enums/barcode'
import { buildScanMeterPlan, type ScanMeterInput } from './plan'

const type = {
  id: 'type-rec-id',
  namespace: 'acme',
  created: '',
  updated: '',
  tags: [],
  content: {
    kind: 'meter-type',
    key: 'pm5560',
    name: 'PM5560',
    version: 2,
    registers: [
      {
        key: 'v_l1',
        name: 'Voltage L1',
        address: 3027,
        fn_code: 'read_holding',
        datatype: 'float32',
        word_count: 2,
        byte_order: 'big',
        scale: 1,
        offset: 0,
        signed: false,
        unit: 'V',
        quantity: 'Voltage',
        history: true,
        chart_type: 'line',
        chart_group: 'voltage',
        precision: 1,
      },
    ],
  },
} as unknown as MeterTypeRecord

const input: ScanMeterInput = {
  networkId: 'net-rec-id',
  networkKey: 'gw-01-net-1',
  tenantKey: 'acme',
  siteKey: 'hq',
  gatewayKey: 'gw-01',
  type,
  address: 5,
  meterKey: 'gw-01-net-1-m5',
  meterName: 'Scanned Meter 5',
}

describe('scan-wizard plan', () => {
  it('resolves a scanned barcode to the meter-type the plan stamps from', () => {
    const code = encodeBarcode('pm5560')
    const resolved = resolveBarcode(code, [type])
    expect(resolved?.id).toBe('type-rec-id')
  })

  it('builds one meter + its stamped registers', () => {
    const plan = buildScanMeterPlan(input)
    expect(plan.filter((p) => p.kind === 'meter')).toHaveLength(1)
    expect(plan.filter((p) => p.kind === 'register')).toHaveLength(1)
  })

  it('stamps meter_type + version, the unit address, and the network relation', () => {
    const m = buildScanMeterPlan(input)[0].content
    expect(m.meter_type).toBe('type-rec-id')
    expect(m.meter_type_version).toBe(2)
    expect(m.address).toBe(5)
    expect(m.network).toBe('net-rec-id')
    expect(m.key).toBe('gw-01-net-1-m5')
    expect(m.name).toBe('Scanned Meter 5')
  })

  it('keys registers ${meterKey}--${defKey} and late-binds the meter relation', () => {
    const reg = buildScanMeterPlan(input).find((p) => p.kind === 'register')!
    expect(reg.content.key).toBe('gw-01-net-1-m5--v_l1')
    expect(reg.parentRefs).toEqual([
      { field: 'meter', planId: 'meter-gw-01-net-1-m5' },
    ])
  })

  it('applies meter + register tags from the shared module', () => {
    const plan = buildScanMeterPlan(input)
    expect(plan[0].content.tags).toEqual([
      'tenant:acme',
      'site:hq',
      'gateway:gw-01',
      'network:gw-01-net-1',
      'meter-type:pm5560',
    ])
    const reg = plan.find((p) => p.kind === 'register')!
    expect(reg.content.tags).toEqual([
      'tenant:acme',
      'site:hq',
      'gateway:gw-01',
      'network:gw-01-net-1',
      'meter:gw-01-net-1-m5',
      'group:voltage',
      'quantity:Voltage',
    ])
  })
})
