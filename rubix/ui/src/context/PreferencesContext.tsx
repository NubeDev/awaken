// Per-user preferences context (§2). Fetches GET /prefs once and exposes it via
// usePreferences(), so chart formatters render datetimes in the user's timezone +
// pattern and numeric axes carry the converted unit. A PATCH refreshes the cache.
//
// Lives inside the connection gate (an active connection is required for the API
// client). Before prefs load — or if the fetch fails — the canonical defaults
// (metric, ISO-8601, UTC) are used, so a chart always has something to format with.

import { createContext, useContext, useMemo, type ReactNode } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useApi, useConnection } from '../api/ConnectionContext'
import { DEFAULT_PREFERENCES, getPreferences, type Preferences } from '../api/prefs'

interface PreferencesCtx {
  prefs: Preferences
  /** Whether the initial fetch is still in flight (defaults are in use until then). */
  loading: boolean
}

const Ctx = createContext<PreferencesCtx | null>(null)

export function PreferencesProvider({ children }: { children: ReactNode }) {
  const { connection } = useConnection()
  const api = useApi()

  const q = useQuery({
    queryKey: ['prefs', connection?.subject],
    queryFn: () => getPreferences(api),
    enabled: Boolean(connection),
    // Prefs change rarely; don't refetch on every focus.
    staleTime: 5 * 60 * 1000,
  })

  const value = useMemo<PreferencesCtx>(
    () => ({ prefs: q.data ?? DEFAULT_PREFERENCES, loading: q.isPending }),
    [q.data, q.isPending],
  )

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>
}

/** The current user's preferences (defaults until the fetch lands). */
export function usePreferences(): Preferences {
  const ctx = useContext(Ctx)
  // Fall back to defaults outside a provider so a stray formatter never throws.
  return ctx?.prefs ?? DEFAULT_PREFERENCES
}
