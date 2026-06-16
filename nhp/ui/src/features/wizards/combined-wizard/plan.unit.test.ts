/**
 * Combined "add everything" plan: the whole greenfield tree in one ordered plan
 * with every parent relation late-bound (WS-06 task 5). Asserts ordering (parents
 * before children), the parentRef threading, and that the shared tags are applied
 * across every level so the new tree's dashboards auto-build.
 */
import { describe, expect, it } from 'vitest'
import type { MeterTypeRecord } from '@/api/records'
import { buildCombinedPlan, type CombinedInput } from './plan'

const type = {
  id: 'type-rec-id',
  content: {
    kind: 'meter-type',
    key: 'pm5560',
    name: 'PM5560',
    version: 1,
    registers: [
      {
        key: 'v_l1',
        name: 'V L1',
        address: 1,
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

const input: CombinedInput = {
  tenant: { key: 'acme', name: 'Acme' },
  site: { key: 'hq', name: 'HQ', address: '1 Main', timezone: 'UTC' },
  gateway: { key: 'gw-01', name: 'GW', model: 'm', host: 'h' },
  networks: {
    count: 2,
    netType: '485',
    protocol: 'modbus',
    maxDevices: 16,
    namePattern: 'gw-01-net-{n}',
    params: { baud: 9600, parity: 'none', stop_bits: 1, data_bits: 8 },
  },
  meters: {
    type,
    addressFrom: 1,
    addressTo: 2,
    keyPattern: 'm-{n}',
    namePattern: 'Meter {n}',
  },
}

describe('combined add-everything plan', () => {
  it('builds the full tree: 1 tenant, 1 site, 1 gateway, 2 networks, 2 meters, 2 registers', () => {
    const plan = buildCombinedPlan(input)
    const byKind = (k: string) => plan.filter((p) => p.kind === k).length
    expect(byKind('tenant')).toBe(1)
    expect(byKind('site')).toBe(1)
    expect(byKind('gateway')).toBe(1)
    expect(byKind('network')).toBe(2)
    expect(byKind('meter')).toBe(2)
    expect(byKind('register')).toBe(2)
  })

  it('orders parents before children', () => {
    const plan = buildCombinedPlan(input)
    const idx = (id: string) => plan.findIndex((p) => p.id === id)
    expect(idx('tenant')).toBeLessThan(idx('site'))
    expect(idx('site')).toBeLessThan(idx('gateway'))
    expect(idx('gateway')).toBeLessThan(idx('net-gw-01-net-1'))
    expect(idx('net-gw-01-net-1')).toBeLessThan(idx('meter-m-1'))
    expect(idx('meter-m-1')).toBeLessThan(idx('reg-m-1--v_l1'))
  })

  it('threads parent ids via late-bound parentRefs', () => {
    const plan = buildCombinedPlan(input)
    const ref = (id: string) =>
      plan.find((p) => p.id === id)?.parentRefs?.[0]
    expect(plan.find((p) => p.id === 'tenant')?.parentRefs).toBeUndefined()
    expect(ref('site')).toEqual({ field: 'tenant', planId: 'tenant' })
    expect(ref('gateway')).toEqual({ field: 'site', planId: 'site' })
    expect(ref('net-gw-01-net-1')).toEqual({ field: 'gateway', planId: 'gateway' })
    // meters land on the FIRST network
    expect(ref('meter-m-1')).toEqual({
      field: 'network',
      planId: 'net-gw-01-net-1',
    })
    expect(ref('reg-m-1--v_l1')).toEqual({
      field: 'meter',
      planId: 'meter-m-1',
    })
  })

  it('applies the shared hierarchy tags across every level', () => {
    const plan = buildCombinedPlan(input)
    const tags = (id: string) =>
      plan.find((p) => p.id === id)?.content.tags
    expect(tags('tenant')).toEqual([])
    expect(tags('site')).toEqual(['tenant:acme'])
    expect(tags('gateway')).toEqual(['tenant:acme', 'site:hq'])
    expect(tags('net-gw-01-net-1')).toEqual([
      'tenant:acme',
      'site:hq',
      'gateway:gw-01',
    ])
    expect(tags('meter-m-1')).toEqual([
      'tenant:acme',
      'site:hq',
      'gateway:gw-01',
      'network:gw-01-net-1',
      'meter-type:pm5560',
    ])
  })

  it('omits meters when no type is chosen', () => {
    const plan = buildCombinedPlan({
      ...input,
      meters: { ...input.meters, type: undefined },
    })
    expect(plan.filter((p) => p.kind === 'meter')).toHaveLength(0)
    expect(plan.filter((p) => p.kind === 'register')).toHaveLength(0)
  })
})
