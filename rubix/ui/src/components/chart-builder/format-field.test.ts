import { describe, expect, it } from 'vitest'

import { formatFieldValue } from './format-field'

describe('formatFieldValue', () => {
  it('renders noValue for null / non-finite', () => {
    expect(formatFieldValue(null).text).toBe('—')
    expect(formatFieldValue(undefined, { noValue: 'n/a' }).text).toBe('n/a')
    expect(formatFieldValue(NaN).text).toBe('—')
  })

  it('applies fixed decimals and a trailing unit symbol', () => {
    expect(formatFieldValue(21.456, { unit: 'celsius', decimals: 1 }).text).toBe('21.5 °C')
  })

  it('prepends a currency symbol with no space', () => {
    expect(formatFieldValue(12, { unit: 'usd', decimals: 2 }).text).toBe('$12.00')
  })

  it('scales percentunit from 0–1 to 0–100', () => {
    expect(formatFieldValue(0.5, { unit: 'percentunit' }).text).toBe('50%')
  })

  it('auto-trims fractional precision when decimals is unset', () => {
    expect(formatFieldValue(3, {}).text).toBe('3')
    expect(formatFieldValue(3.14159, {}).text).toBe('3.14')
  })

  it('lets a text value-mapping win outright and carry its colour', () => {
    const r = formatFieldValue(0, {
      mappings: [{ type: 'value', match: '0', text: 'OFF', color: '0 0% 50%' }],
    })
    expect(r.text).toBe('OFF')
    expect(r.color).toBe('0 0% 50%')
  })

  it('applies a colour-only range mapping but still formats the number', () => {
    const r = formatFieldValue(95, {
      unit: 'percent',
      mappings: [{ type: 'range', from: 90, color: '0 90% 50%' }],
    })
    expect(r.text).toBe('95%')
    expect(r.color).toBe('0 90% 50%')
  })
})
