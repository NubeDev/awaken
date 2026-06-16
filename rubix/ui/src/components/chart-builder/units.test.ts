import { describe, expect, it } from 'vitest'

import { PHYSICAL_QUANTITIES, quantityOf, unitDef } from './units'

describe('unit registry', () => {
  it('resolves a known unit id', () => {
    expect(unitDef('celsius')?.symbol).toBe('°C')
  })

  it('returns undefined for an unknown id', () => {
    expect(unitDef('not_a_unit')).toBeUndefined()
    expect(unitDef(undefined)).toBeUndefined()
  })

  it('maps physical units to the backend quantity they convert under (§2/§7)', () => {
    // The quantity strings must be exactly what rubix-prefs::Quantity::parse takes.
    expect(quantityOf('celsius')).toBe('temperature')
    expect(quantityOf('foot')).toBe('length')
    expect(quantityOf('pound')).toBe('mass')
    expect(quantityOf('mph')).toBe('speed')
  })

  it('leaves non-physical units unconvertible', () => {
    expect(quantityOf('usd')).toBeUndefined()
    expect(quantityOf('percent')).toBeUndefined()
  })

  it('offers exactly the four convertible quantities in the dropdown', () => {
    expect(PHYSICAL_QUANTITIES.map((q) => q.value)).toEqual([
      'temperature',
      'length',
      'mass',
      'speed',
    ])
  })
})
