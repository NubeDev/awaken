import { describe, expect, it } from 'vitest'
import { diffSnapshots, formatDiffValue } from './diff'

describe('diffSnapshots', () => {
  it('a create (no before) lists every field as added', () => {
    const d = diffSnapshots(undefined, { title: 'A', enabled: true })
    expect(d).toEqual([
      { field: 'enabled', status: 'added', after: true },
      { field: 'title', status: 'added', after: 'A' },
    ])
  })

  it('a delete (no after) lists every field as removed', () => {
    const d = diffSnapshots({ title: 'A' }, undefined)
    expect(d).toEqual([{ field: 'title', status: 'removed', before: 'A' }])
  })

  it('an update lists only the changed fields', () => {
    const d = diffSnapshots(
      { title: 'A', color: 'red', x: 1 },
      { title: 'B', color: 'red', x: 1 }
    )
    expect(d).toEqual([
      { field: 'title', status: 'changed', before: 'A', after: 'B' },
    ])
  })

  it('detects added and removed keys within an update', () => {
    const d = diffSnapshots({ a: 1, b: 2 }, { a: 1, c: 3 })
    expect(d).toEqual([
      { field: 'b', status: 'removed', before: 2 },
      { field: 'c', status: 'added', after: 3 },
    ])
  })

  it('no change yields an empty diff', () => {
    expect(diffSnapshots({ a: 1 }, { a: 1 })).toEqual([])
  })

  it('compares nested structures structurally', () => {
    const d = diffSnapshots(
      { tags: { unit: 'kW' } },
      { tags: { unit: 'kWh' } }
    )
    expect(d).toHaveLength(1)
    expect(d[0].status).toBe('changed')
  })
})

describe('formatDiffValue', () => {
  it('renders absent / null sentinels and scalars', () => {
    expect(formatDiffValue(undefined)).toBe('∅')
    expect(formatDiffValue(null)).toBe('null')
    expect(formatDiffValue('hi')).toBe('hi')
    expect(formatDiffValue(42)).toBe('42')
    expect(formatDiffValue({ a: 1 })).toBe('{"a":1}')
  })
})
