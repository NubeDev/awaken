/**
 * The datetime formatting seam (WS-11). Resolves the viewer's
 * `PreferencesContext` (backend-resolved timezone / date / time format) and
 * returns formatters built against it, so every time axis and `time` table
 * cell renders in the viewer's zone + format with no per-call-site choice.
 *
 * String formatting stays client-side (locale/`Intl`-heavy, cheap on the
 * client); the *policy* (which tz, which format) is backend-resolved. The local
 * device falls back to the `DEFAULT_PREFERENCES` baseline until prefs load.
 */
import { useMemo } from 'react'
import { usePreferences } from '@/context/preferences-provider'
import type { ResolvedPreferences } from '@/api/types'

export interface DateTimeFormatters {
  /** Date + time, in the viewer's zone/format (e.g. axis tick, table cell). */
  dateTime: (value: number | string | Date) => string
  /** Date only. */
  date: (value: number | string | Date) => string
  /** Time only. */
  time: (value: number | string | Date) => string
}

function toDate(value: number | string | Date): Date {
  return value instanceof Date ? value : new Date(value)
}

/** `Intl` date-part options. All supported formats render the same numeric
 * parts; ordering is the locale's job (matches the backend format policy). */
function dateOptions(): Intl.DateTimeFormatOptions {
  return { year: 'numeric', month: '2-digit', day: '2-digit' }
}

/** Map the resolved `time_format` token to `Intl` time-part options. */
function timeOptions(prefs: ResolvedPreferences): Intl.DateTimeFormatOptions {
  const hour12 = prefs.time_format === '12h'
  return { hour: '2-digit', minute: '2-digit', second: '2-digit', hour12 }
}

/** Build the formatters from a concrete set of preferences. Pure — the hook is
 * a thin `useMemo` over this, and tests exercise it directly. */
export function makeFormatters(prefs: ResolvedPreferences): DateTimeFormatters {
  const tz = prefs.timezone || 'UTC'
  const locale = prefs.locale || 'en-US'
  const base: Intl.DateTimeFormatOptions = { timeZone: tz }
  const dateFmt = new Intl.DateTimeFormat(locale, { ...base, ...dateOptions() })
  const timeFmt = new Intl.DateTimeFormat(locale, {
    ...base,
    ...timeOptions(prefs),
  })
  const dateTimeFmt = new Intl.DateTimeFormat(locale, {
    ...base,
    ...dateOptions(),
    ...timeOptions(prefs),
  })
  return {
    dateTime: (v) => dateTimeFmt.format(toDate(v)),
    date: (v) => dateFmt.format(toDate(v)),
    time: (v) => timeFmt.format(toDate(v)),
  }
}

export function useDateTime(): DateTimeFormatters {
  const { prefs } = usePreferences()
  return useMemo(() => makeFormatters(prefs), [prefs])
}
