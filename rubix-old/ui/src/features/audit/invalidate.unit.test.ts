import { describe, expect, it } from 'vitest'
import type { UndoResult } from '@/api/types'
import { invalidationKeys } from './invalidate'

describe('invalidationKeys', () => {
  it('returns no keys when nothing moved', () => {
    const result: UndoResult = { touched: [] }
    expect(invalidationKeys(result)).toEqual([])
  })

  it('returns the touchable config-entity roots when an undo touched a resource', () => {
    const result: UndoResult = { group: 'g1', touched: ['w1', 'd1'] }
    const keys = invalidationKeys(result)
    // Each key is a single-segment prefix (a root), so TanStack invalidates the
    // whole subtree (lists + detail queries) under it.
    expect(keys.every((k) => k.length === 1)).toBe(true)
    // The widget + dashboard roots — the canvas refresh path — are present.
    expect(keys).toContainEqual(['widgets'])
    expect(keys).toContainEqual(['widget-data'])
    expect(keys).toContainEqual(['dashboards'])
  })

  it('does not depend on the number of touched ids (roots are fixed)', () => {
    const one = invalidationKeys({ group: 'g', touched: ['a'] })
    const many = invalidationKeys({ group: 'g', touched: ['a', 'b', 'c'] })
    expect(one).toEqual(many)
  })
})
