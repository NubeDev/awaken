/**
 * Backend-resolved user/org preferences, made available app-wide (WS-11).
 *
 * The provider reads `GET /api/v1/me/preferences` once (cached generously) and
 * exposes the resolved view — timezone, locale, unit choices, date/time format
 * — so any component formats values consistently against the *server's*
 * resolution (user → org → system default), not its own guess. `useDateTime`
 * below is the formatting seam: time axes and table cells route through it, so
 * mounting this provider makes them obey the viewer's prefs with no call-site
 * changes.
 *
 * While the request is in flight (or on an auth-off edge node that returns the
 * system defaults) the context falls back to a metric/UTC/ISO baseline that
 * mirrors the backend `SystemDefaults`, so rendering never blocks on prefs.
 */
import { createContext, use, type ReactNode } from 'react'
import { useMyPreferences } from '@/api/hooks'
import type { ResolvedPreferences } from '@/api/types'

/** The backend `SystemDefaults::starter()` baseline, mirrored for the loading
 * / no-backend case so the UI always has a concrete set of prefs. */
export const DEFAULT_PREFERENCES: ResolvedPreferences = {
  timezone: 'UTC',
  locale: 'en-US',
  language: 'en',
  unit_system: 'metric',
  temperature_unit: 'celsius',
  pressure_unit: 'kilopascal',
  speed_unit: 'meter_per_second',
  length_unit: 'meter',
  mass_unit: 'kilogram',
  date_format: 'YYYY-MM-DD',
  time_format: '24h',
  week_start: 'monday',
  number_format: '1,234.56',
  currency: 'USD',
  theme: 'system',
}

interface PreferencesContextValue {
  prefs: ResolvedPreferences
  /** True once the server's resolved prefs have loaded (else the baseline). */
  isResolved: boolean
}

const PreferencesContext = createContext<PreferencesContextValue | null>(null)

export function PreferencesProvider({ children }: { children: ReactNode }) {
  const { data, isSuccess } = useMyPreferences()
  const value: PreferencesContextValue = {
    prefs: data ?? DEFAULT_PREFERENCES,
    isResolved: isSuccess,
  }
  return <PreferencesContext value={value}>{children}</PreferencesContext>
}

/** Read the resolved preferences. Falls back to the baseline when no provider
 * is mounted (keeps the hook safe in isolated tests / storybook). */
export function usePreferences(): PreferencesContextValue {
  return (
    use(PreferencesContext) ?? {
      prefs: DEFAULT_PREFERENCES,
      isResolved: false,
    }
  )
}
