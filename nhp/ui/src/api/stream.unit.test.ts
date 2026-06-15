import { describe, expect, it } from 'vitest'

import { splitSseEvents, sseEventData } from './stream'

describe('splitSseEvents', () => {
  it('returns complete events and keeps the trailing partial as rest', () => {
    const { events, rest } = splitSseEvents('data: a\n\ndata: b\n\ndata: par')
    expect(events).toEqual(['data: a', 'data: b'])
    expect(rest).toBe('data: par')
  })

  it('treats a buffer with no separator as all-rest', () => {
    const { events, rest } = splitSseEvents('data: incomplete')
    expect(events).toEqual([])
    expect(rest).toBe('data: incomplete')
  })
})

describe('sseEventData', () => {
  it('extracts the data payload, tolerating the optional leading space', () => {
    expect(sseEventData('data: [1,2]')).toBe('[1,2]')
    expect(sseEventData('data:[1,2]')).toBe('[1,2]')
  })

  it('joins multi-line data and ignores comment/keep-alive lines', () => {
    expect(sseEventData(': keep-alive')).toBeNull()
    expect(sseEventData('event: ping')).toBeNull()
    expect(sseEventData('data: line1\ndata: line2')).toBe('line1\nline2')
  })
})
