import { describe, expect, it } from 'vitest'

import type { FieldConfig } from '../field-config'
import { fieldValueFormatter, pointRampColor, seriesColor } from './field-format'

describe('fieldValueFormatter', () => {
  it('falls back to compact numbers with no FieldConfig', () => {
    const fmt = fieldValueFormatter('n', undefined)
    expect(fmt(1500)).toBe('1.5K')
  })

  it('applies the resolved unit + decimals from the FieldConfig', () => {
    const cfg: FieldConfig = { defaults: { unit: 'celsius', decimals: 1 } }
    const fmt = fieldValueFormatter('temp', cfg)
    expect(fmt(21.46)).toBe('21.5 °C')
  })

  it('passes non-numbers through as strings', () => {
    expect(fieldValueFormatter('x', undefined)('hi')).toBe('hi')
  })
})

describe('pointRampColor', () => {
  it('colours a point by the series threshold ramp', () => {
    const cfg: FieldConfig = {
      defaults: {
        thresholds: [
          { value: null, color: '120 50% 50%' },
          { value: 80, color: '0 90% 50%' },
        ],
      },
    }
    expect(pointRampColor('temp', 90, cfg)).toBe('hsl(0 90% 50%)')
    expect(pointRampColor('temp', 10, cfg)).toBe('hsl(120 50% 50%)')
  })

  it('returns undefined with no ramp', () => {
    expect(pointRampColor('temp', 5, undefined)).toBeUndefined()
  })
})

describe('seriesColor', () => {
  it('keeps the palette colour when no override sets one', () => {
    expect(seriesColor('a', 'hsl(var(--chart-1))', undefined)).toBe('hsl(var(--chart-1))')
  })

  it('honours a byName override colour, wrapped as hsl()', () => {
    const cfg: FieldConfig = {
      overrides: [{ matcher: { type: 'byName', value: 'a' }, display: { color: '200 80% 50%' } }],
    }
    expect(seriesColor('a', 'hsl(var(--chart-1))', cfg)).toBe('hsl(200 80% 50%)')
  })
})
