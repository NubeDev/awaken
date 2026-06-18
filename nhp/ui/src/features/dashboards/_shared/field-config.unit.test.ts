import { describe, expect, it } from 'vitest'
import type { Alarm } from '@/api/records'
import { hasAlarm, severityFor, thresholdLines } from './field-config'

// The voltage ramp from the seed (DOMAIN-MODEL §Alarms): warn ≥250, critical ≥253.
const ramp: Alarm = {
  for: '5m',
  thresholds: [
    { value: null, severity: 'ok' },
    { value: 250, severity: 'warning' },
    { value: 253, severity: 'critical' },
  ],
}

describe('severityFor', () => {
  it('returns the highest step at or below the value', () => {
    expect(severityFor(230, ramp)).toBe('ok')
    expect(severityFor(250, ramp)).toBe('warning')
    expect(severityFor(252.9, ramp)).toBe('warning')
    expect(severityFor(253, ramp)).toBe('critical')
    expect(severityFor(300, ramp)).toBe('critical')
  })
  it('is ok when there is no ramp', () => {
    expect(severityFor(999, undefined)).toBe('ok')
  })
  it('sorts unordered steps deterministically', () => {
    const shuffled: Alarm = { thresholds: [ramp.thresholds[2], ramp.thresholds[0], ramp.thresholds[1]] }
    expect(severityFor(253, shuffled)).toBe('critical')
  })

  // A LoRa low-battery ramp (DOMAIN-MODEL §Alarms): warn ≤30%, critical ≤15% — it
  // fires as the value FALLS, so direction is 'below'.
  const battery: Alarm = {
    direction: 'below',
    thresholds: [
      { value: null, severity: 'ok' },
      { value: 30, severity: 'warning' },
      { value: 15, severity: 'critical' },
    ],
  }
  it("trips downward for a 'below' ramp", () => {
    expect(severityFor(80, battery)).toBe('ok')
    expect(severityFor(30, battery)).toBe('warning')
    expect(severityFor(15.1, battery)).toBe('warning')
    expect(severityFor(15, battery)).toBe('critical')
    expect(severityFor(2, battery)).toBe('critical')
  })
})

describe('hasAlarm / thresholdLines', () => {
  it('detects a real ramp and lists its crossing lines', () => {
    expect(hasAlarm(ramp)).toBe(true)
    expect(hasAlarm({ thresholds: [{ value: null, severity: 'ok' }] })).toBe(false)
    expect(thresholdLines(ramp)).toEqual([
      { value: 250, severity: 'warning' },
      { value: 253, severity: 'critical' },
    ])
  })
})
