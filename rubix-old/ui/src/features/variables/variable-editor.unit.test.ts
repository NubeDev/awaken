import { describe, expect, it } from 'vitest'
import {
  defaultConfig,
  deleteVariable,
  moveVariable,
  newVariable,
  updateVariable,
} from './variable-editor'

describe('defaultConfig', () => {
  it('produces a config whose tag matches the kind', () => {
    for (const kind of [
      'constant',
      'custom',
      'query',
      'datasource',
      'site',
      'interval',
      'textbox',
    ] as const) {
      expect(defaultConfig(kind).kind).toBe(kind)
    }
  })
})

describe('newVariable', () => {
  it('names the first of a kind after the kind', () => {
    expect(newVariable('site', []).name).toBe('site')
  })

  it('disambiguates against existing names', () => {
    const existing = [newVariable('site', [])]
    expect(newVariable('site', existing).name).toBe('site2')
  })
})

describe('updateVariable', () => {
  it('patches a field at the index', () => {
    const vars = [newVariable('site', [])]
    expect(updateVariable(vars, 0, { name: 'renamed' })[0].name).toBe('renamed')
  })

  it('resets config when the kind changes', () => {
    const vars = [newVariable('site', [])]
    const next = updateVariable(vars, 0, { kind: 'query' })
    expect(next[0].config.kind).toBe('query')
  })

  it('leaves other indices untouched', () => {
    const vars = [newVariable('site', []), newVariable('custom', [])]
    const next = updateVariable(vars, 0, { name: 'x' })
    expect(next[1]).toBe(vars[1])
  })
})

describe('deleteVariable', () => {
  it('removes the indexed variable', () => {
    const vars = [newVariable('site', []), newVariable('custom', [])]
    const next = deleteVariable(vars, 0)
    expect(next).toHaveLength(1)
    expect(next[0].kind).toBe('custom')
  })
})

describe('moveVariable', () => {
  it('moves an element forward', () => {
    const vars = [
      newVariable('site', []),
      { ...newVariable('custom', []), name: 'b' },
      { ...newVariable('custom', []), name: 'c' },
    ]
    expect(moveVariable(vars, 0, 2).map((v) => v.name)).toEqual([
      'b',
      'c',
      'site',
    ])
  })

  it('is a no-op for an out-of-range source', () => {
    const vars = [newVariable('site', [])]
    expect(moveVariable(vars, 5, 0)).toBe(vars)
  })
})
