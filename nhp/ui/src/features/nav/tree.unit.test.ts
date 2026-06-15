import { describe, expect, it } from 'vitest'
import type { NavNode } from '@/api/types'
import { assembleNavTree, navPath } from './tree'

function node(
  id: string,
  parent_id: string | null,
  title: string,
  sort_order = 0
): NavNode {
  return {
    id,
    org: 'kfc',
    parent_id,
    title,
    sort_order,
    target: { kind: 'group' },
  }
}

describe('assembleNavTree', () => {
  it('nests children under parents, ordered by sort_order then title', () => {
    const flat = [
      node('a', null, 'Buildings', 0),
      node('b2', 'a', 'Building-2', 1),
      node('b1', 'a', 'Building-1', 0),
      node('root2', null, 'Admin', 1),
    ]
    const tree = assembleNavTree(flat)
    expect(tree.map((n) => n.title)).toEqual(['Buildings', 'Admin'])
    expect(tree[0].children.map((n) => n.title)).toEqual([
      'Building-1',
      'Building-2',
    ])
  })

  it('re-parents a node whose parent was filtered out to root', () => {
    // The grant view-filter dropped the parent `a`; `b1` must still appear.
    const flat = [node('b1', 'a', 'Building-1', 0)]
    const tree = assembleNavTree(flat)
    expect(tree.map((n) => n.title)).toEqual(['Building-1'])
  })
})

describe('navPath', () => {
  it('returns the root-first ancestor path including the node', () => {
    const flat = [
      node('a', null, 'Buildings'),
      node('b', 'a', 'Building-1'),
      node('c', 'b', 'Level-1'),
    ]
    expect(navPath(flat, 'c')).toEqual(['Buildings', 'Building-1', 'Level-1'])
    expect(navPath(flat, 'a')).toEqual(['Buildings'])
    expect(navPath(flat, 'missing')).toEqual([])
  })

  it('is bounded under a parent cycle', () => {
    const flat = [node('x', 'y', 'X'), node('y', 'x', 'Y')]
    expect(navPath(flat, 'x').length).toBeLessThanOrEqual(2)
  })
})
