import { describe, expect, it } from 'vitest'
import type {
  MeterRecord,
  NetworkRecord,
  RegisterRec,
} from '@/api/records'
import { meterTag } from '@/enums/tags'
import type { HistorySample } from '@/features/dashboards/query/batch'
import { resolveWindow } from '@/features/dashboards/query/time-window'
import type { PortfolioIndex } from '../scope'
import { siteInventory } from './site-inventory'

const now = Date.now()
const iso = (hoursAgo: number) => new Date(now - hoursAgo * 3600_000).toISOString()
const W = resolveWindow('now-24h', now)

function meter(id: string, key: string, name: string, over: Partial<MeterRecord['content']> = {}): MeterRecord {
  return {
    id,
    namespace: 'acme',
    tags: [],
    created: '',
    updated: '',
    content: {
      kind: 'meter',
      key,
      name,
      network: 'net-1',
      meter_type: 'mt-1',
      meter_type_version: 1,
      address: 1,
      status: 'online',
      ...over,
    },
  } as MeterRecord
}

function network(id: string, protocol: string): NetworkRecord {
  return {
    id,
    namespace: 'acme',
    tags: [],
    created: '',
    updated: '',
    content: { kind: 'network', key: 'n', gateway: 'gw-1', net_type: 'lora', protocol, max_devices: 100 },
  } as NetworkRecord
}

function reg(id: string, meterKey: string, over: Partial<RegisterRec['content']> = {}): RegisterRec {
  return {
    id,
    namespace: 'acme',
    tags: [],
    created: '',
    updated: '',
    content: {
      key: `${meterKey}--r`,
      name: 'temp',
      address: 1,
      fn_code: 'lora_uplink',
      datatype: 'float32',
      word_count: 2,
      byte_order: 'big',
      scale: 1,
      offset: 0,
      signed: false,
      unit: '°C',
      quantity: 'temperature',
      history: true,
      chart_type: 'line',
      chart_group: 'temperature',
      precision: 1,
      meter: 'meter-rec-1',
      tags: [meterTag(meterKey)],
      ...over,
    },
  } as RegisterRec
}

function index(meterType = 'CO sensor'): PortfolioIndex {
  return {
    meterLocation: new Map([['m1', { gatewayKey: 'gw-a' }]]),
    meterTypeName: () => meterType,
  } as unknown as PortfolioIndex
}

describe('siteInventory', () => {
  it('builds one row per meter with protocol, type, status and headline metric', () => {
    const meters = [meter('m1', 'mk1', 'Carpark CO')]
    const networks = [network('net-1', 'lora')]
    const registers = [reg('r1', 'mk1', { unit: 'ppm', quantity: 'co' })]
    const history: HistorySample[] = [{ series: 'r1', at: iso(1), value: 12 }]

    const rows = siteInventory(index(), meters, networks, registers, history, W)

    expect(rows).toHaveLength(1)
    expect(rows[0]).toMatchObject({
      meterName: 'Carpark CO',
      gatewayKey: 'gw-a',
      protocol: 'lora',
      meterType: 'CO sensor',
      status: 'online',
    })
    expect(rows[0].metric.latest).toBe(12)
    expect(rows[0].metric.unit).toBe('ppm')
  })

  it('falls back when network/status are missing and sorts by name', () => {
    const meters = [
      meter('m1', 'b', 'Bravo', { network: 'missing', status: undefined }),
      meter('m1', 'a', 'Alpha'),
    ]
    const networks = [network('net-1', 'lora')]
    const rows = siteInventory(index(), meters, networks, [], [], W)

    expect(rows.map((r) => r.meterName)).toEqual(['Alpha', 'Bravo'])
    expect(rows[1].protocol).toBe('—')
    expect(rows[1].status).toBe('unknown')
  })
})
