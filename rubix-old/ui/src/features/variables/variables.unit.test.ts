import { describe, expect, it } from 'vitest'
import type { Variable } from '@/api/types'
import { referencedVariables } from './refs'
import { resolutionOrder, VariableCycleError } from './order'
import { varRevision } from './revision'
import { queryVariablesFor } from './query-variables'
import { readVarParams, writeVarParams } from './url-state'

function siteVar(name: string): Variable {
  return { name, kind: 'site', config: { kind: 'site' }, current: undefined }
}

function queryVar(name: string, sql: string): Variable {
  return { name, kind: 'query', config: { kind: 'query', sql } }
}

describe('referencedVariables', () => {
  it('finds bare, brace, format, and sqlIn tokens', () => {
    const sql =
      "SELECT * FROM points WHERE a = $site AND b = ${equip} AND c IN (${ds:csv}) AND d $__sqlIn(zone)"
    expect(referencedVariables(sql).sort()).toEqual([
      'ds',
      'equip',
      'site',
      'zone',
    ])
  })

  it('does not treat a positional $1 placeholder as a variable', () => {
    expect(referencedVariables('WHERE x = $1 AND y = $site')).toEqual(['site'])
  })

  it('returns each name once', () => {
    expect(referencedVariables('$site $site ${site}')).toEqual(['site'])
  })
})

describe('resolutionOrder', () => {
  it('orders a parent before its dependent child', () => {
    const vars = [
      queryVar('equip', "SELECT path FROM equips WHERE site_id = '$site'"),
      siteVar('site'),
    ]
    const order = resolutionOrder(vars).map((v) => v.name)
    expect(order.indexOf('site')).toBeLessThan(order.indexOf('equip'))
  })

  it('keeps independent variables in input order', () => {
    const vars = [siteVar('a'), siteVar('b'), siteVar('c')]
    expect(resolutionOrder(vars).map((v) => v.name)).toEqual(['a', 'b', 'c'])
  })

  it('rejects a dependency cycle with a clear error', () => {
    const vars = [
      queryVar('a', "WHERE x = '$b'"),
      queryVar('b', "WHERE x = '$a'"),
    ]
    expect(() => resolutionOrder(vars)).toThrow(VariableCycleError)
  })

  it('ignores self-references (not a cycle)', () => {
    const vars = [queryVar('a', "WHERE x = '$a'")]
    expect(resolutionOrder(vars).map((v) => v.name)).toEqual(['a'])
  })
})

describe('varRevision', () => {
  it('is stable regardless of name order', () => {
    const vars = [
      { ...siteVar('site'), current: 'A' },
      { ...siteVar('zone'), current: 'Z' },
    ]
    expect(varRevision(vars, ['zone', 'site'])).toEqual(
      varRevision(vars, ['site', 'zone'])
    )
  })

  it('changes when a referenced value changes', () => {
    const before = [{ ...siteVar('site'), current: 'A' }]
    const after = [{ ...siteVar('site'), current: 'B' }]
    expect(varRevision(before, ['site'])).not.toEqual(
      varRevision(after, ['site'])
    )
  })

  it('is constant for a widget that references nothing', () => {
    expect(varRevision([], [])).toEqual('none')
  })

  it('is unaffected by an unrelated variable change', () => {
    const a = [
      { ...siteVar('site'), current: 'A' },
      { ...siteVar('other'), current: '1' },
    ]
    const b = [
      { ...siteVar('site'), current: 'A' },
      { ...siteVar('other'), current: '2' },
    ]
    // A widget that only references `site` sees the same revision.
    expect(varRevision(a, ['site'])).toEqual(varRevision(b, ['site']))
  })
})

describe('queryVariablesFor', () => {
  it('sends only referenced variables, carrying their current value', () => {
    const vars = [
      { ...siteVar('site'), current: 'Site-A' },
      { ...siteVar('unused'), current: 'x' },
    ]
    expect(queryVariablesFor('WHERE site_id = $site', vars)).toEqual([
      { name: 'site', value: 'Site-A' },
    ])
  })

  it('sends a multi-select value as an array (for IN expansion)', () => {
    const vars = [{ ...siteVar('site'), current: ['A', 'B'], multi: true }]
    expect(queryVariablesFor('WHERE $__sqlIn(site)', vars)).toEqual([
      { name: 'site', value: ['A', 'B'] },
    ])
  })

  it('omits a referenced-but-undeclared variable (server raises the error)', () => {
    expect(queryVariablesFor('WHERE x = $missing', [])).toEqual([])
  })

  it('carries an injection payload verbatim as data (server binds it)', () => {
    const vars = [{ ...siteVar('site'), current: "'); DROP TABLE points; --" }]
    expect(queryVariablesFor('WHERE site_id = $site', vars)).toEqual([
      { name: 'site', value: "'); DROP TABLE points; --" },
    ])
  })
})

describe('url-state round-trip', () => {
  it('reads a single var param as a scalar', () => {
    expect(readVarParams(new URLSearchParams('var-site=Site-A'))).toEqual({
      site: 'Site-A',
    })
  })

  it('reads repeated params as an array (multi-select)', () => {
    expect(
      readVarParams(new URLSearchParams('var-site=A&var-site=B'))
    ).toEqual({ site: ['A', 'B'] })
  })

  it('ignores non-var params', () => {
    expect(readVarParams(new URLSearchParams('from=now-1h&var-site=A'))).toEqual(
      { site: 'A' }
    )
  })

  it('writes a multi selection as repeated params and preserves others', () => {
    const base = new URLSearchParams('from=now-1h')
    const next = writeVarParams(base, { site: ['A', 'B'] })
    expect(next.getAll('var-site')).toEqual(['A', 'B'])
    expect(next.get('from')).toBe('now-1h')
  })

  it('round-trips a selection', () => {
    const selection = { site: ['A', 'B'], zone: 'Z' }
    const written = writeVarParams(new URLSearchParams(), selection)
    expect(readVarParams(written)).toEqual(selection)
  })

  it('clears a variable whose selection is empty', () => {
    const base = new URLSearchParams('var-site=A')
    const next = writeVarParams(base, { site: '' })
    expect(next.getAll('var-site')).toEqual([])
  })
})
