// Tests for the domain-agnostic backend-shape derivation. These deliberately use
// NON-domain kinds (`thing`, `widget`) to prove the inspector derives structure
// from whatever data it is given, never from baked-in domain knowledge.

import { describe, expect, it } from 'vitest'
import { jsonType, profileKinds, tagFrequencies } from './schema'
import type { Record } from '../types/Record'
import type { CollectionDef } from '../api/collections'

function rec(id: string, content: Record['content'], tags: string[] = []): Record {
  return { id, namespace: 'ns', content, tags, created: '2026-01-01T00:00:00Z', updated: '2026-01-01T00:00:00Z' }
}

describe('jsonType', () => {
  it('names JSON types the way a developer reads them', () => {
    expect(jsonType('x')).toBe('string')
    expect(jsonType(3)).toBe('number')
    expect(jsonType(true)).toBe('boolean')
    expect(jsonType(null)).toBe('null')
    expect(jsonType([1, 2])).toBe('array')
    expect(jsonType({ a: 1 })).toBe('object')
  })
})

describe('profileKinds', () => {
  it('groups by content.kind and counts each, most-populated first', () => {
    const records = [
      rec('1', { kind: 'thing', a: 1 }),
      rec('2', { kind: 'thing', a: 2 }),
      rec('3', { kind: 'widget', b: 'hi' }),
    ]
    const profiles = profileKinds(records)
    expect(profiles.map((p) => [p.kind, p.count])).toEqual([
      ['thing', 2],
      ['widget', 1],
    ])
  })

  it('infers field shape, presence and sample from the data', () => {
    const records = [
      rec('1', { kind: 'thing', a: 1, label: 'one' }),
      rec('2', { kind: 'thing', a: 2 }), // no `label`
    ]
    const thing = profileKinds(records)[0]
    const a = thing.fields.find((f) => f.name === 'a')!
    const label = thing.fields.find((f) => f.name === 'label')!

    expect(a.types).toEqual(['number'])
    expect(a.presence).toBe(1) // present on both
    expect(label.presence).toBe(0.5) // present on one of two
    expect(a.declared).toBe(false)
  })

  it('flags inconsistent field types across records', () => {
    const records = [rec('1', { kind: 'thing', v: 1 }), rec('2', { kind: 'thing', v: 'two' })]
    const v = profileKinds(records)[0].fields.find((f) => f.name === 'v')!
    expect(v.types).toEqual(['number', 'string'])
  })

  it('never drops the `kind` field as a profiled field', () => {
    const thing = profileKinds([rec('1', { kind: 'thing', a: 1 })])[0]
    expect(thing.fields.map((f) => f.name)).not.toContain('kind')
  })

  it('groups records with no kind under (unkinded) rather than dropping them', () => {
    const profiles = profileKinds([rec('1', { a: 1 })])
    expect(profiles[0].kind).toBe('(unkinded)')
    expect(profiles[0].count).toBe(1)
  })

  it('overlays a registered collection: declared fields are marked and typed', () => {
    const collections: CollectionDef[] = [
      {
        name: 'thing',
        recordId: 'collection:thing',
        schema: [
          { name: 'a', type: 'number', required: true, unique: false },
          { name: 'missing', type: 'text', required: false, unique: false },
        ],
      },
    ]
    const thing = profileKinds([rec('1', { kind: 'thing', a: 1 })], collections)[0]
    expect(thing.hasCollection).toBe(true)

    const a = thing.fields.find((f) => f.name === 'a')!
    expect(a.declared).toBe(true)
    expect(a.declaredType).toBe('number')

    // A declared field with no data still appears (so the schema is visible).
    const missing = thing.fields.find((f) => f.name === 'missing')!
    expect(missing.declared).toBe(true)
    expect(missing.presence).toBe(0)
  })
})

describe('tagFrequencies', () => {
  it('counts distinct tags across records, most frequent first', () => {
    const records = [
      rec('1', { kind: 'thing' }, ['floor-2', 'critical']),
      rec('2', { kind: 'thing' }, ['floor-2']),
      rec('3', { kind: 'widget' }, []),
    ]
    expect(tagFrequencies(records)).toEqual([
      { tag: 'floor-2', count: 2 },
      { tag: 'critical', count: 1 },
    ])
  })

  it('returns nothing when no record carries a tag', () => {
    expect(tagFrequencies([rec('1', { kind: 'thing' })])).toEqual([])
  })
})
