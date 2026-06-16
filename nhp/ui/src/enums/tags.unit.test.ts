/**
 * Drift guard: the UI tag mirror (`tags.ts`) MUST produce byte-identical tags to
 * the WS-03 seed source of truth (`nhp/seed/tags.mjs`). The seed PRODUCES tagged
 * records, the wizards (WS-06) must produce the SAME tags, and WS-07 reads them —
 * a drift silently breaks dashboard auto-build. The `.mjs` lives outside `src` so
 * it can't enter the tsc build, but Vite resolves it for this test. See tags.ts.
 */
import { describe, expect, it } from 'vitest'
import * as node from '../../../seed/tags.mjs'
import * as ui from './tags'

const hierarchy = {
  tenant: 'acme',
  site: 'hq',
  gateway: 'gw-01',
  network: 'gw-01-net-1',
  meter: 'm-01',
}

describe('NHP tag mirror matches nhp/seed/tags.mjs', () => {
  it('atomic tag builders match', () => {
    expect(ui.tenantTag('acme')).toBe(node.tenantTag('acme'))
    expect(ui.siteTag('hq')).toBe(node.siteTag('hq'))
    expect(ui.gatewayTag('gw-01')).toBe(node.gatewayTag('gw-01'))
    expect(ui.networkTag('gw-01-net-1')).toBe(node.networkTag('gw-01-net-1'))
    expect(ui.meterTag('m-01')).toBe(node.meterTag('m-01'))
    expect(ui.groupTag('voltage')).toBe(node.groupTag('voltage'))
    expect(ui.quantityTag('power')).toBe(node.quantityTag('power'))
    expect(ui.meterTypeTag('pm5560')).toBe(node.meterTypeTag('pm5560'))
  })

  it('siteTags match', () => {
    expect(ui.siteTags(hierarchy)).toEqual(node.siteTags(hierarchy))
  })

  it('gatewayTags match', () => {
    expect(ui.gatewayTags(hierarchy)).toEqual(node.gatewayTags(hierarchy))
  })

  it('networkTags match', () => {
    expect(ui.networkTags(hierarchy)).toEqual(node.networkTags(hierarchy))
  })

  it('meterTags match', () => {
    const ctx = { ...hierarchy, meterType: 'pm5560' }
    expect(ui.meterTags(ctx)).toEqual(node.meterTags(ctx))
  })

  it('registerTags match (with group + quantity)', () => {
    const reg = { chart_group: 'voltage', quantity: 'Voltage' }
    expect(ui.registerTags(hierarchy, reg)).toEqual(
      node.registerTags(hierarchy, reg)
    )
  })

  it('registerTags match (no group / quantity → omitted)', () => {
    const reg = {}
    expect(ui.registerTags(hierarchy, reg)).toEqual(
      node.registerTags(hierarchy, reg)
    )
  })
})
