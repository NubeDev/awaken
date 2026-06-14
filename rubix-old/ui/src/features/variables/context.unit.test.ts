import { describe, expect, it } from 'vitest'
import type { NavNode } from '@/api/types'
import {
  assemblePageContext,
  builtinContextSeeds,
  readBareParams,
  resolveContextValue,
  type NavMount,
} from './context'

function dashNode(
  id: string,
  title: string,
  context?: NavNode['context']
): NavNode {
  return {
    id,
    org: 'kfc',
    parent_id: null,
    title,
    sort_order: 0,
    target: { kind: 'dashboard', dashboard_id: 'd1' },
    context,
  }
}

function mount(node: NavNode, slug = 'energy-overview', path = ['Buildings', node.title]): NavMount {
  return { node, slug, path }
}

describe('readBareParams', () => {
  it('keeps bare params and drops var-/scope/nav reserved keys', () => {
    const p = new URLSearchParams(
      '?building=b1&var-site=s1&from=now-1h&to=now&refresh=30&nav=n1&zone=z2'
    )
    expect(readBareParams(p)).toEqual({ building: 'b1', zone: 'z2' })
  })

  it('collapses a single value but keeps repeats as an array', () => {
    const p = new URLSearchParams('?a=1&b=2&b=3')
    expect(readBareParams(p)).toEqual({ a: '1', b: ['2', '3'] })
  })
})

describe('assemblePageContext precedence (design §1)', () => {
  it('merges node tags over board tags and copies node values', () => {
    const node = dashNode('n1', 'Building-1', {
      values: { site: 's1' },
      tags: { building: 'b1' },
    })
    const ctx = assemblePageContext({
      org: 'kfc',
      siteId: undefined,
      bareParams: { region: 'west' },
      boardTags: { building: 'BOARD', floor: '3' },
      mount: mount(node),
    })
    // Node tag pin wins over the board's own tag; board-only tags survive.
    expect(ctx.tags).toEqual({ building: 'b1', floor: '3' })
    expect(ctx.values).toEqual({ site: 's1' })
    expect(ctx.url).toEqual({ region: 'west' })
    expect(ctx.nav).toEqual({
      node_id: 'n1',
      slug: 'energy-overview',
      name: 'Building-1',
      path: ['Buildings', 'Building-1'],
    })
  })

  it('one board at two nodes yields two different values', () => {
    const a = mount(dashNode('a', 'Building-1', { values: { site: 's1' } }))
    const b = mount(dashNode('b', 'Building-2', { values: { site: 's2' } }))
    const ca = assemblePageContext({ org: 'kfc', bareParams: {}, boardTags: {}, mount: a })
    const cb = assemblePageContext({ org: 'kfc', bareParams: {}, boardTags: {}, mount: b })
    expect(resolveContextValue(ca, 'values', 'site')).toBe('s1')
    expect(resolveContextValue(cb, 'values', 'site')).toBe('s2')
  })
})

describe('resolveContextValue per source', () => {
  const ctx = assemblePageContext({
    org: 'kfc',
    bareParams: { building: 'b9' },
    boardTags: { zone: 'north' },
    mount: mount(dashNode('n1', 'L1', { values: { site: 's7' } })),
  })

  it('nav slug/name/path', () => {
    expect(resolveContextValue(ctx, 'nav', 'slug')).toBe('energy-overview')
    expect(resolveContextValue(ctx, 'nav', 'name')).toBe('L1')
    expect(resolveContextValue(ctx, 'nav', 'path[0]')).toBe('Buildings')
    expect(resolveContextValue(ctx, 'nav', 'path[1]')).toBe('L1')
    expect(resolveContextValue(ctx, 'nav', 'path[9]')).toBeUndefined()
  })

  it('url / tag / values', () => {
    expect(resolveContextValue(ctx, 'url', 'building')).toBe('b9')
    expect(resolveContextValue(ctx, 'tag', 'zone')).toBe('north')
    expect(resolveContextValue(ctx, 'values', 'site')).toBe('s7')
    expect(resolveContextValue(ctx, 'url', 'missing')).toBeUndefined()
  })
})

describe('builtinContextSeeds', () => {
  it('seeds nav tokens and one __tag(key) per present tag', () => {
    const ctx = assemblePageContext({
      org: 'kfc',
      bareParams: {},
      boardTags: { building: 'b1', empty: null },
      mount: mount(dashNode('n1', 'B1')),
    })
    expect(builtinContextSeeds(ctx)).toEqual({
      __nav_slug: 'energy-overview',
      __nav_name: 'B1',
      '__tag(building)': 'b1',
    })
  })
})
