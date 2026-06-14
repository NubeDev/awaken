import { describe, expect, it } from 'vitest'
import { parseTags, tagNames, tagsToInput } from './tags'

describe('parseTags', () => {
  it('splits on commas and whitespace into marker tags', () => {
    expect(parseTags('ahu, vav, submeter')).toEqual({
      ahu: true,
      vav: true,
      submeter: true,
    })
    expect(parseTags('ahu vav')).toEqual({ ahu: true, vav: true })
  })

  it('drops blanks and trims names', () => {
    expect(parseTags(' ahu ,  , vav ,')).toEqual({ ahu: true, vav: true })
    expect(parseTags('')).toEqual({})
    expect(parseTags('   ')).toEqual({})
  })
})

describe('tagsToInput', () => {
  it('renders a TagSet back to a sorted comma list (round-trips parseTags)', () => {
    expect(tagsToInput({ vav: true, ahu: true })).toBe('ahu, vav')
    expect(tagsToInput(parseTags('rooftop, ahu'))).toBe('ahu, rooftop')
    expect(tagsToInput({})).toBe('')
  })

  it('round-trips through parseTags preserving the marker name set', () => {
    const input = 'submeter, comfort, ahu'
    expect(tagNames(parseTags(input))).toEqual(['ahu', 'comfort', 'submeter'])
  })
})
