/**
 * The headline guarantee: the gateway wizard generates N networks correctly at
 * N=30 (WS-06 task 2). Asserts the plan shape, the naming pattern, the parentRef
 * wiring (every network's gateway relation is late-bound to the one gateway), and
 * that the standard tags are applied so dashboards auto-build.
 */
import { describe, expect, it } from 'vitest'
import {
  buildGatewayPlan,
  expandPattern,
  networkKeys,
  type GatewayInput,
  type NetworksInput,
} from './plan'

const gw: GatewayInput = {
  key: 'gw-01',
  name: 'Gateway 01',
  model: 'NHP-GW',
  host: '10.0.0.1',
  siteId: 'site-rec-id',
  siteKey: 'hq',
  tenantKey: 'acme',
}

const net: NetworksInput = {
  count: 30,
  netType: '485',
  protocol: 'modbus',
  maxDevices: 16,
  namePattern: 'gw-01-net-{n}',
  params: { baud: 9600, parity: 'none', stop_bits: 1, data_bits: 8 },
}

describe('gateway + N networks plan', () => {
  it('expands {n} to a 1-based index', () => {
    expect(expandPattern('gw-01-net-{n}', 3)).toBe('gw-01-net-3')
  })

  it('generates N=30 distinct network keys', () => {
    const keys = networkKeys(net)
    expect(keys).toHaveLength(30)
    expect(new Set(keys).size).toBe(30)
    expect(keys[0]).toBe('gw-01-net-1')
    expect(keys[29]).toBe('gw-01-net-30')
  })

  it('builds 1 gateway + 30 networks', () => {
    const plan = buildGatewayPlan(gw, net)
    expect(plan).toHaveLength(31)
    expect(plan[0].kind).toBe('gateway')
    expect(plan.filter((p) => p.kind === 'network')).toHaveLength(30)
  })

  it('late-binds every network gateway relation to the gateway plan id', () => {
    const plan = buildGatewayPlan(gw, net)
    for (const p of plan.filter((r) => r.kind === 'network')) {
      expect(p.parentRefs).toEqual([{ field: 'gateway', planId: 'gateway' }])
    }
  })

  it('applies the shared hierarchy tags to gateway and networks', () => {
    const plan = buildGatewayPlan(gw, net)
    expect(plan[0].content.tags).toEqual(['tenant:acme', 'site:hq'])
    expect(plan[1].content.tags).toEqual([
      'tenant:acme',
      'site:hq',
      'gateway:gw-01',
    ])
  })

  it('carries net_type/protocol/max_devices/params onto every network', () => {
    const plan = buildGatewayPlan(gw, net)
    const first = plan[1].content
    expect(first.net_type).toBe('485')
    expect(first.protocol).toBe('modbus')
    expect(first.max_devices).toBe(16)
    expect(first.params).toEqual(net.params)
  })
})
