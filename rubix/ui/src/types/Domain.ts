// Domain shapes the UI derives from generic records. These are NOT backend
// types — they are a read-side projection of `record.content` by `kind`
// (site / equip / point / reading), mirroring the seed in
// crates/rubix-server/src/seed/portfolio.rs.

export type Severity = 'crit' | 'amber' | 'green' | 'muted'

export interface Site {
  id: string
  key: string
  name: string
  /** Live point count derived from the point records under the site. */
  points: number
  /** Equipment count under the site. */
  equips: number
  /** Highest severity across the site's zones (for the alert badge). */
  severity: Severity
  /** Number of zones currently out of band. */
  alerts: number
}

export interface Equip {
  id: string
  key: string
  name: string
  domain: string
  type: string
  site: string
}

export interface Point {
  id: string
  key: string
  name: string
  domain: string
  measure: string
  unit: string
  equip: string
  site: string
  /** Latest reading value, if any reading record was found. */
  value: number | null
  /** ISO timestamp of the latest reading. */
  ts: string | null
}

// A "zone" in the building view = one piece of HVAC equipment with its derived
// temperature, setpoint and load. The demo's heat-mapped floor rows.
export interface Zone {
  id: string
  /** The owning equipment key, for joining back to its points. */
  key: string
  /** The site key this zone belongs to. */
  site: string
  name: string
  domain: string
  temp: number | null
  sp: number | null
  load: number
  severity: Severity
  note?: string
  unit: string
}
