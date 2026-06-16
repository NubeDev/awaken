import { describe, expect, it } from 'vitest'
import type {
  GatewayRecord,
  MeterRecord,
  NetworkRecord,
  RegisterRec,
  SiteRecord,
  TenantRecord,
} from '@/api/records'
import {
  buildIndex,
  quantitiesInScope,
  selectMeters,
  selectRegisters,
  type PortfolioData,
} from './scope'

// A tiny 1-tenant / 2-site portfolio: each site has a gateway → network → meter,
// and each meter one history register. Site A's register has an alarm ramp.
const rec = <C>(id: string, content: C) =>
  ({ id, namespace: 'acme', content, tags: [], created: '', updated: '' }) as never

const data: PortfolioData = {
  tenants: [rec<TenantRecord['content']>('t1', { kind: 'tenant', key: 'acme', name: 'Acme' })],
  sites: [
    rec<SiteRecord['content']>('sA', { kind: 'site', key: 'a', name: 'Site A', tenant: 't1' }),
    rec<SiteRecord['content']>('sB', { kind: 'site', key: 'b', name: 'Site B', tenant: 't1' }),
  ],
  gateways: [
    rec<GatewayRecord['content']>('gA', { kind: 'gateway', key: 'gA', name: 'GW A', site: 'sA' }),
    rec<GatewayRecord['content']>('gB', { kind: 'gateway', key: 'gB', name: 'GW B', site: 'sB' }),
  ],
  networks: [
    rec<NetworkRecord['content']>('nA', { kind: 'network', key: 'nA', gateway: 'gA', net_type: '485', protocol: 'modbus', max_devices: 32 }),
    rec<NetworkRecord['content']>('nB', { kind: 'network', key: 'nB', gateway: 'gB', net_type: '485', protocol: 'modbus', max_devices: 32 }),
  ],
  meters: [
    rec<MeterRecord['content']>('mA', { kind: 'meter', key: 'mA', name: 'Meter A', network: 'nA', meter_type: 'pm', meter_type_version: 1, address: 1 }),
    rec<MeterRecord['content']>('mB', { kind: 'meter', key: 'mB', name: 'Meter B', network: 'nB', meter_type: 'em', meter_type_version: 1, address: 1 }),
  ],
  registers: [
    rec<RegisterRec['content']>('rA', baseReg('rA', 'mA', 'power', { thresholds: [{ value: null, severity: 'ok' }, { value: 100, severity: 'warning' }] })),
    rec<RegisterRec['content']>('rB', baseReg('rB', 'mB', 'voltage')),
  ],
  meterTypes: [],
}

function baseReg(
  key: string,
  meter: string,
  quantity: string,
  alarm?: unknown
): RegisterRec['content'] {
  return {
    key,
    name: key,
    meter,
    address: 0,
    fn_code: 'read_holding',
    datatype: 'float32',
    word_count: 2,
    byte_order: 'big',
    scale: 1,
    offset: 0,
    signed: false,
    unit: 'x',
    quantity,
    history: true,
    chart_type: 'line',
    chart_group: quantity,
    precision: 1,
    ...(alarm ? { alarm } : {}),
  } as RegisterRec['content']
}

describe('buildIndex / scope selection', () => {
  const index = buildIndex(data)

  it('resolves a meter to its site and tenant through the hierarchy', () => {
    expect(index.meterLocation.get('mA')).toMatchObject({
      siteId: 'sA',
      siteName: 'Site A',
      tenantId: 't1',
      tenantName: 'Acme',
    })
  })

  it('selectMeters filters by site', () => {
    expect(selectMeters(index, { siteId: 'sB' }).map((m) => m.id)).toEqual(['mB'])
  })

  it('selectMeters filters by meter-type', () => {
    expect(selectMeters(index, { meterTypeId: 'pm' }).map((m) => m.id)).toEqual(['mA'])
  })

  it('selectRegisters scopes registers and filters by quantity', () => {
    expect(selectRegisters(index, { tenantId: 't1' }).length).toBe(2)
    expect(selectRegisters(index, { quantity: 'voltage' }).map((r) => r.id)).toEqual(['rB'])
  })

  it('selectRegisters alarmsOnly keeps only registers with a ramp', () => {
    expect(selectRegisters(index, {}, { alarmsOnly: true }).map((r) => r.id)).toEqual(['rA'])
  })

  it('quantitiesInScope lists distinct quantities for the scope', () => {
    expect(quantitiesInScope(index, { siteId: 'sA' })).toEqual(['power'])
    expect(quantitiesInScope(index, {})).toEqual(['power', 'voltage'])
  })
})
