/**
 * Bulk-meters plan: address-range expansion, per-meter register stamping from the
 * type, late-bound meter relation on registers, and the standard tags (WS-06 task
 * 3). The over-cap BLOCK itself is enforced in the UI via capacity.ts; here we
 * assert the plan the wizard writes once the count fits.
 */
import { describe, expect, it } from 'vitest'
import type { MeterTypeRecord } from '@/api/records'
import { addressRange, buildMetersPlan, type MetersInput } from './plan'

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

const input: MetersInput = {
  networkId: 'net-rec-id',
  networkKey: 'gw-01-net-1',
  tenantKey: 'acme',
  siteKey: 'hq',
  gatewayKey: 'gw-01',
  type,
  addressFrom: 1,
  addressTo: 3,
  keyPattern: 'gw-01-net-1-m{n}',
  namePattern: 'Meter {n}',
}

describe('bulk-meters plan', () => {
  it('expands an inclusive address range', () => {
    expect(addressRange(1, 3)).toEqual([1, 2, 3])
    expect(addressRange(5, 5)).toEqual([5])
    expect(addressRange(3, 1)).toEqual([])
  })

  it('creates one meter + its stamped registers per address', () => {
    const plan = buildMetersPlan(input)
    const meters = plan.filter((p) => p.kind === 'meter')
    const regs = plan.filter((p) => p.kind === 'register')
    expect(meters).toHaveLength(3)
    expect(regs).toHaveLength(3) // 1 reg per meter × 3
  })

  it('stamps meter_type + version and the unit address', () => {
    const plan = buildMetersPlan(input)
    const m = plan[0].content
    expect(m.meter_type).toBe('type-rec-id')
    expect(m.meter_type_version).toBe(2)
    expect(m.address).toBe(1)
    expect(m.network).toBe('net-rec-id')
  })

  it('keys registers ${meterKey}--${defKey} and late-binds the meter relation', () => {
    const plan = buildMetersPlan(input)
    const reg = plan.find((p) => p.kind === 'register')!
    expect(reg.content.key).toBe('gw-01-net-1-m1--v_l1')
    expect(reg.parentRefs).toEqual([
      { field: 'meter', planId: 'meter-gw-01-net-1-m1' },
    ])
  })

  it('applies meter + register tags from the shared module', () => {
    const plan = buildMetersPlan(input)
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
      'meter:gw-01-net-1-m1',
      'group:voltage',
      'quantity:Voltage',
    ])
  })
})
