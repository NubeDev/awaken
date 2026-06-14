import { describe, expect, it } from 'vitest'
import { DEFAULT_PREFERENCES } from '@/context/preferences-provider'
import type { ResolvedPreferences } from '@/api/types'
import { makeFormatters } from './use-date-time'

// A fixed instant: 2026-06-13T15:30:00Z.
const TS = Date.parse('2026-06-13T15:30:00Z')

function prefs(over: Partial<ResolvedPreferences>): ResolvedPreferences {
  return { ...DEFAULT_PREFERENCES, ...over }
}

describe('makeFormatters', () => {
  it('renders time in the resolved timezone', () => {
    // UTC 15:30 is 17:30 in Europe/Paris (CEST, +2 in June).
    const paris = makeFormatters(prefs({ timezone: 'Europe/Paris' }))
    expect(paris.time(TS)).toContain('17:30')
    const utc = makeFormatters(prefs({ timezone: 'UTC' }))
    expect(utc.time(TS)).toContain('15:30')
  })

  it('honours the 12h/24h time-format preference', () => {
    const h24 = makeFormatters(prefs({ time_format: '24h', timezone: 'UTC' }))
    expect(h24.time(TS)).toContain('15:30')
    const h12 = makeFormatters(
      prefs({ time_format: '12h', timezone: 'UTC', locale: 'en-US' })
    )
    // 12h form renders an AM/PM marker and a 3:30 hour.
    expect(h12.time(TS)).toMatch(/3:30/)
    expect(h12.time(TS).toLowerCase()).toMatch(/pm/)
  })

  it('orders the date by locale', () => {
    const us = makeFormatters(prefs({ locale: 'en-US', timezone: 'UTC' }))
    // en-US is month-first.
    expect(us.date(TS)).toMatch(/^06\/13\/2026/)
    const gb = makeFormatters(prefs({ locale: 'en-GB', timezone: 'UTC' }))
    // en-GB is day-first.
    expect(gb.date(TS)).toMatch(/^13\/06\/2026/)
  })
})
