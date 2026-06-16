import { describe, expect, it } from 'vitest'

import {
  matchOverride,
  rampColor,
  resolveField,
  type FieldConfig,
  type SeriesField,
} from './field-config'

const series: SeriesField = { value: 'temp', label: 'Temperature', color: '0 0% 50%' }

describe('resolveField', () => {
  it('returns the series defaults when no config is set', () => {
    const r = resolveField(series, undefined)
    expect(r.displayName).toBe('Temperature')
    expect(r.color).toBe('0 0% 50%')
    expect(r.unit).toBeUndefined()
  })

  it('layers fieldConfig.defaults under the series', () => {
    const config: FieldConfig = { defaults: { unit: 'celsius', decimals: 1 } }
    const r = resolveField(series, config)
    expect(r.unit).toBe('celsius')
    expect(r.decimals).toBe(1)
  })

  it('lays a matching override on top of the defaults', () => {
    const config: FieldConfig = {
      defaults: { unit: 'celsius', decimals: 1 },
      overrides: [
        { matcher: { type: 'byName', value: 'temp' }, display: { decimals: 2, displayName: 'Temp °C' } },
      ],
    }
    const r = resolveField(series, config)
    expect(r.unit).toBe('celsius') // from defaults
    expect(r.decimals).toBe(2) // overridden
    expect(r.displayName).toBe('Temp °C') // override display name
  })

  it('does not let an undefined override field clobber a default', () => {
    const config: FieldConfig = {
      defaults: { unit: 'celsius' },
      overrides: [{ matcher: { type: 'byName', value: 'temp' }, display: { decimals: 3 } }],
    }
    expect(resolveField(series, config).unit).toBe('celsius')
  })
})

describe('matchOverride', () => {
  it('matches byName against the value column or label', () => {
    const o = { matcher: { type: 'byName' as const, value: 'Temperature' }, display: {} }
    expect(matchOverride(series, [o])).toBe(o)
  })

  it('matches byRegex and tolerates an invalid pattern', () => {
    const ok = { matcher: { type: 'byRegex' as const, value: '^temp' }, display: {} }
    const bad = { matcher: { type: 'byRegex' as const, value: '[' }, display: {} }
    expect(matchOverride(series, [bad, ok])).toBe(ok)
  })
})

describe('rampColor', () => {
  const steps = [
    { value: null, color: '120 50% 50%' }, // base
    { value: 50, color: '40 90% 50%' },
    { value: 80, color: '0 90% 50%' },
  ]

  it('picks the highest step the value meets or exceeds', () => {
    expect(rampColor(10, steps)).toBe('hsl(120 50% 50%)')
    expect(rampColor(60, steps)).toBe('hsl(40 90% 50%)')
    expect(rampColor(90, steps)).toBe('hsl(0 90% 50%)')
  })

  it('is order-invariant', () => {
    const shuffled = [steps[2], steps[0], steps[1]]
    expect(rampColor(60, shuffled)).toBe('hsl(40 90% 50%)')
  })

  it('returns undefined for an empty ramp', () => {
    expect(rampColor(5, [])).toBeUndefined()
  })
})
