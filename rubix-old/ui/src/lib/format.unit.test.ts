import { describe, expect, it } from 'vitest'
import { formatValue, relativeTime } from './format'

describe('formatValue', () => {
  it('renders an em dash for null/undefined', () => {
    expect(formatValue(null)).toBe('—')
    expect(formatValue(undefined)).toBe('—')
  })

  it('renders booleans as On/Off', () => {
    expect(formatValue(true)).toBe('On')
    expect(formatValue(false)).toBe('Off')
  })

  it('rounds numbers and attaches units', () => {
    expect(formatValue(13.44, '°C')).toBe('13.4°C')
    expect(formatValue(412.7, 'kW')).toBe('413 kW')
    expect(formatValue(96, '%')).toBe('96%')
  })

  it('passes strings through', () => {
    expect(formatValue('Occupied')).toBe('Occupied')
  })
})

describe('relativeTime', () => {
  const now = Date.parse('2026-06-12T12:00:00Z')

  it('reports seconds, minutes, hours, days', () => {
    expect(relativeTime('2026-06-12T11:59:50Z', now)).toBe('10s ago')
    expect(relativeTime('2026-06-12T11:54:00Z', now)).toBe('6m ago')
    expect(relativeTime('2026-06-12T09:00:00Z', now)).toBe('3h ago')
    expect(relativeTime('2026-06-10T12:00:00Z', now)).toBe('2d ago')
  })

  it('handles missing/invalid input', () => {
    expect(relativeTime(null, now)).toBe('—')
    expect(relativeTime('not-a-date', now)).toBe('—')
  })
})
