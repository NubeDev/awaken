import { describe, expect, it } from 'vitest'

import {
  applyCosmeticTransforms,
  isAggregate,
  splitTransforms,
  type Transform,
} from './transforms'

describe('splitTransforms', () => {
  it('partitions aggregate from cosmetic, preserving order', () => {
    const spec: Transform[] = [
      { kind: 'rename', from: 'a', to: 'b' },
      { kind: 'filter', field: 'x', op: '>', value: '1' },
      { kind: 'organize', order: ['b'] },
      { kind: 'groupBy', by: 'b', field: 'x', agg: 'sum', as: 's' },
    ]
    const { aggregate, cosmetic } = splitTransforms(spec)
    expect(aggregate.map((t) => t.kind)).toEqual(['filter', 'groupBy'])
    expect(cosmetic.map((t) => t.kind)).toEqual(['rename', 'organize'])
  })

  it('classifies the cardinality-changing ops as aggregate', () => {
    expect(isAggregate({ kind: 'filter', field: 'x', op: '=', value: '1' })).toBe(true)
    expect(isAggregate({ kind: 'reduce', field: 'x', calc: 'sum', as: 's' })).toBe(true)
    expect(isAggregate({ kind: 'rename', from: 'a', to: 'b' })).toBe(false)
  })
})

describe('applyCosmeticTransforms', () => {
  it('renames a column across rows', () => {
    const out = applyCosmeticTransforms(
      [{ a: 1 }, { a: 2 }],
      [{ kind: 'rename', from: 'a', to: 'b' }],
    )
    expect(out).toEqual([{ b: 1 }, { b: 2 }])
  })

  it('adds a calculated column, null on non-numeric or /0', () => {
    const out = applyCosmeticTransforms(
      [
        { a: 10, b: 2 },
        { a: 3, b: 0 },
        { a: 'x', b: 1 },
      ],
      [{ kind: 'calculated', field: 'c', left: 'a', op: '/', right: 'b' }],
    )
    expect(out.map((r) => r.c)).toEqual([5, null, null])
  })

  it('reorders columns with organize, keeping unlisted keys', () => {
    const out = applyCosmeticTransforms([{ a: 1, b: 2, c: 3 }], [
      { kind: 'organize', order: ['c', 'a'] },
    ])
    expect(Object.keys(out[0])).toEqual(['c', 'a', 'b'])
  })

  it('skips aggregate ops (the backend ran them)', () => {
    const out = applyCosmeticTransforms(
      [{ a: 1 }, { a: 9 }],
      [{ kind: 'filter', field: 'a', op: '>', value: '5' }],
    )
    expect(out).toHaveLength(2) // filter NOT applied client-side
  })
})
