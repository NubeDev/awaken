import { describe, expect, it } from 'vitest'
import { pointIdFor, readingSeries, toSites, toZones } from './derive'
import type { Record } from '../types/Record'

// Fake records shaped like the seed (crates/rubix-server/src/seed/portfolio.rs):
// a site, an hvac equip, its temp + setpoint points, and a few readings.
function rec(id: string, content: Record['content']): Record {
  return { id, namespace: 'acme', content, created: '2026-06-15T00:00:00Z', updated: '2026-06-15T00:00:00Z' }
}

const records: Record[] = [
  rec('acme--hq', { kind: 'site', key: 'hq', name: 'Acme HQ' }),
  rec('acme--hq--ahu-1', { kind: 'equip', key: 'ahu-1', name: 'AHU 1', domain: 'hvac', type: 'ahu', site: 'hq' }),
  rec('acme--hq--ahu-1--zone-temp', {
    kind: 'point',
    key: 'zone-temp',
    name: 'Zone Temp',
    domain: 'hvac',
    measure: 'temp',
    unit: 'degC',
    equip: 'ahu-1',
    site: 'hq',
  }),
  rec('acme--hq--ahu-1--setpoint', {
    kind: 'point',
    key: 'setpoint',
    name: 'Setpoint',
    domain: 'hvac',
    measure: 'setpoint',
    unit: 'degC',
    equip: 'ahu-1',
    site: 'hq',
  }),
  rec('r0', { kind: 'reading', point: 'acme--hq--ahu-1--zone-temp', measure: 'temp', value: 22.0, ts: '2026-06-15T10:00:00Z' }),
  rec('r1', { kind: 'reading', point: 'acme--hq--ahu-1--zone-temp', measure: 'temp', value: 27.5, ts: '2026-06-15T11:00:00Z' }),
  rec('sp0', { kind: 'reading', point: 'acme--hq--ahu-1--setpoint', measure: 'setpoint', value: 22.0, ts: '2026-06-15T11:00:00Z' }),
]

describe('derive', () => {
  it('rolls a site up from its records', () => {
    const sites = toSites(records)
    expect(sites).toHaveLength(1)
    expect(sites[0].name).toBe('Acme HQ')
    expect(sites[0].equips).toBe(1)
    expect(sites[0].points).toBe(2)
  })

  it('derives a zone using the latest reading and flags deviation', () => {
    const zones = toZones(records, 'hq')
    expect(zones).toHaveLength(1)
    // Latest temp reading is 27.5 against a 22.0 setpoint → 5.5° off → crit.
    expect(zones[0].temp).toBe(27.5)
    expect(zones[0].sp).toBe(22.0)
    expect(zones[0].severity).toBe('crit')
  })

  it('returns a reading series oldest→newest', () => {
    const pid = pointIdFor(records, 'ahu-1', 'hq', 'temp')
    expect(pid).toBe('acme--hq--ahu-1--zone-temp')
    expect(readingSeries(records, pid!)).toEqual([22.0, 27.5])
  })
})
