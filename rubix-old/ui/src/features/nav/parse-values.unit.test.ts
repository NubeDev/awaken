import { describe, expect, it } from 'vitest'
import { parseValues } from './parse-values'

describe('parseValues', () => {
  it('parses key=value lines into a map', () => {
    expect(parseValues('site=s1\nfloor=2')).toEqual({ site: 's1', floor: '2' })
  })

  it('trims keys and values', () => {
    expect(parseValues('  site = s1  ')).toEqual({ site: 's1' })
  })

  it('ignores blank lines and lines without =', () => {
    expect(parseValues('\nsite=s1\nnope\n')).toEqual({ site: 's1' })
  })

  it('ignores a leading = (empty key)', () => {
    expect(parseValues('=value')).toEqual({})
  })

  it('keeps everything after the first = so values may contain =', () => {
    expect(parseValues('q=a=b=c')).toEqual({ q: 'a=b=c' })
  })

  it('keeps SQL metacharacters verbatim — they bind, never execute', () => {
    expect(parseValues("site='); DROP TABLE x; --")).toEqual({
      site: "'); DROP TABLE x; --",
    })
  })
})
