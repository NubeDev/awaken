import { describe, expect, it } from 'vitest'
import { addRow, removeRow, setRow, toRows, toTags } from './tag-edits'

describe('tag-edits', () => {
  it('rows round-trip a set, sorted by key, null shown as empty', () => {
    const rows = toRows({ zone: 'north', building: 'b1', unset: null })
    expect(rows).toEqual([
      { key: 'building', value: 'b1' },
      { key: 'unset', value: '' },
      { key: 'zone', value: 'north' },
    ])
  })

  it('serialises rows to wire map, dropping blank keys, empty value as null', () => {
    const rows = [
      { key: 'building', value: 'b1' },
      { key: '  ', value: 'x' },
      { key: 'unset', value: '' },
    ]
    expect(toTags(rows)).toEqual({ building: 'b1', unset: null })
  })

  it('last write wins on a duplicate key', () => {
    const rows = [
      { key: 'k', value: 'first' },
      { key: 'k', value: 'second' },
    ]
    expect(toTags(rows)).toEqual({ k: 'second' })
  })

  it('add / set / remove operate immutably', () => {
    let rows = addRow([])
    expect(rows).toEqual([{ key: '', value: '' }])
    rows = setRow(rows, 0, { key: 'a', value: '1' })
    expect(rows).toEqual([{ key: 'a', value: '1' }])
    expect(removeRow(rows, 0)).toEqual([])
  })

  it('a tag value with SQL metacharacters survives serialisation verbatim', () => {
    // It binds as a parameter downstream; the editor never quotes/escapes it.
    const rows = [{ key: 'building', value: "'); DROP TABLE points; --" }]
    expect(toTags(rows)).toEqual({ building: "'); DROP TABLE points; --" })
  })
})
