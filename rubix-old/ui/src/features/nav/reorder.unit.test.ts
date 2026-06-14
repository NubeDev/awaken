import { describe, expect, it } from 'vitest'
import type { NavNode } from '@/api/types'
import { movePatches, siblings } from './reorder'

function node(id: string, parent: string | null, order: number): NavNode {
  return {
    id,
    org: 'kfc',
    parent_id: parent,
    title: id,
    sort_order: order,
    target: { kind: 'group' },
  }
}

describe('siblings', () => {
  it('filters by parent and orders by sort_order', () => {
    const nodes = [
      node('a', null, 1),
      node('b', null, 0),
      node('c', 'a', 0),
    ]
    expect(siblings(nodes, null).map((n) => n.id)).toEqual(['b', 'a'])
    expect(siblings(nodes, 'a').map((n) => n.id)).toEqual(['c'])
  })
})

describe('movePatches', () => {
  const nodes = [node('a', null, 0), node('b', null, 1), node('c', null, 2)]

  it('moves a node down by swapping with its next sibling', () => {
    // Move 'a' down: order becomes b, a, c → patches re-number to indices.
    expect(movePatches(nodes, 'a', 1)).toEqual([
      { id: 'b', sort_order: 0 },
      { id: 'a', sort_order: 1 },
    ])
  })

  it('moves a node up', () => {
    expect(movePatches(nodes, 'c', -1)).toEqual([
      { id: 'c', sort_order: 1 },
      { id: 'b', sort_order: 2 },
    ])
  })

  it('is a no-op at the edges or for an unknown node', () => {
    expect(movePatches(nodes, 'a', -1)).toEqual([])
    expect(movePatches(nodes, 'c', 1)).toEqual([])
    expect(movePatches(nodes, 'missing', 1)).toEqual([])
  })

  it('only re-numbers nodes whose order actually changed', () => {
    const gapped = [node('a', null, 5), node('b', null, 9)]
    // Swapping yields b(0), a(1); both changed from 5/9.
    expect(movePatches(gapped, 'a', 1)).toEqual([
      { id: 'b', sort_order: 0 },
      { id: 'a', sort_order: 1 },
    ])
  })
})
