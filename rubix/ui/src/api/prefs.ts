// Preferences surface — GET/PATCH /prefs (§2). The user's display preferences:
// unit system (drives backend conversion of declared quantity columns), datetime
// strftime pattern, and IANA timezone (the client formats UTC instants in it).

import type { ApiClient } from './client'

export type UnitSystem = 'metric' | 'imperial'

export interface Preferences {
  units: UnitSystem
  /** A chrono/strftime pattern, e.g. "%Y-%m-%d %H:%M:%S". */
  datetime: string
  /** An IANA timezone name, e.g. "Australia/Sydney". Absent = UTC. */
  timezone?: string
}

/** A partial update — only the fields a client wants to change. */
export type PreferencesUpdate = Partial<Preferences>

export function getPreferences(client: ApiClient): Promise<Preferences> {
  return client.get<Preferences>('prefs')
}

export function updatePreferences(
  client: ApiClient,
  update: PreferencesUpdate,
): Promise<Preferences> {
  return client.patch<Preferences>('prefs', update)
}

/** The canonical defaults — metric, ISO-8601, UTC — used before prefs load. */
export const DEFAULT_PREFERENCES: Preferences = {
  units: 'metric',
  datetime: '%Y-%m-%d %H:%M:%S',
}
