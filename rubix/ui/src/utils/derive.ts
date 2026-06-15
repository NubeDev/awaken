// Project generic records into the domain shapes the UI draws. The backend bakes
// in no domain type (SCOPE principle 4) — these readers interpret `content` by
// `kind`, mirroring the seed (crates/rubix-server/src/seed/portfolio.rs). Every
// field is read defensively so a half-shaped record never throws.

import type { Record } from '../types/Record'
import type { Equip, Point, Severity, Site, Zone } from '../types/Domain'

const str = (v: unknown): string | undefined => (typeof v === 'string' ? v : undefined)
const num = (v: unknown): number | undefined => (typeof v === 'number' ? v : undefined)

export const byKind = (records: Record[], kind: string): Record[] =>
  records.filter((r) => r.content?.kind === kind)

export function toEquips(records: Record[]): Equip[] {
  return byKind(records, 'equip').map((r) => ({
    id: r.id,
    key: str(r.content.key) ?? r.id,
    name: str(r.content.name) ?? str(r.content.key) ?? r.id,
    domain: str(r.content.domain) ?? 'other',
    type: str(r.content.type) ?? 'equip',
    site: str(r.content.site) ?? '',
  }))
}

// Points carry their latest reading value, joined from reading records by point id.
export function toPoints(records: Record[]): Point[] {
  const latest = latestReadings(records)
  return byKind(records, 'point').map((r) => {
    const reading = latest.get(r.id)
    return {
      id: r.id,
      key: str(r.content.key) ?? r.id,
      name: str(r.content.name) ?? str(r.content.key) ?? r.id,
      domain: str(r.content.domain) ?? 'other',
      measure: str(r.content.measure) ?? '',
      unit: str(r.content.unit) ?? '',
      equip: str(r.content.equip) ?? '',
      site: str(r.content.site) ?? '',
      value: reading?.value ?? null,
      ts: reading?.ts ?? null,
    }
  })
}

interface Reading {
  value: number
  ts: string
}

// Newest reading per point id (readings carry their own `ts` in content).
function latestReadings(records: Record[]): Map<string, Reading> {
  const map = new Map<string, Reading>()
  for (const r of byKind(records, 'reading')) {
    const point = str(r.content.point)
    const value = num(r.content.value)
    const ts = str(r.content.ts) ?? r.created
    if (point == null || value == null) continue
    const prev = map.get(point)
    if (!prev || ts > prev.ts) map.set(point, { value, ts })
  }
  return map
}

// Sites with derived equip/point counts and a roll-up severity from their zones.
export function toSites(records: Record[]): Site[] {
  const equips = toEquips(records)
  const zonesBySite = groupZonesBySite(records)
  return byKind(records, 'site').map((r) => {
    const key = str(r.content.key) ?? r.id
    const siteEquips = equips.filter((e) => e.site === key)
    const points = byKind(records, 'point').filter((p) => str(p.content.site) === key).length
    const zones = zonesBySite.get(key) ?? []
    const alerts = zones.filter((z) => z.severity === 'crit' || z.severity === 'amber').length
    return {
      id: r.id,
      key,
      name: str(r.content.name) ?? key,
      points,
      equips: siteEquips.length,
      severity: rollup(zones.map((z) => z.severity)),
      alerts,
    }
  })
}

// Zones = HVAC equipment with their derived temperature/setpoint/load. One row
// per hvac equip, joining its points (zone-temp, setpoint, power-ish load).
export function toZones(records: Record[], site?: string): Zone[] {
  const points = toPoints(records)
  const equips = toEquips(records).filter((e) => e.domain === 'hvac' && (!site || e.site === site))
  return equips.map((e) => {
    const own = points.filter((p) => p.equip === e.key && (!site || p.site === site))
    const tempP = own.find((p) => p.measure === 'temp')
    const spP = own.find((p) => p.measure === 'setpoint')
    const temp = tempP?.value ?? null
    const sp = spP?.value ?? null
    const offline = temp == null
    return {
      id: e.id,
      key: e.key,
      site: e.site,
      name: e.name,
      domain: e.domain,
      temp,
      sp,
      load: own.find((p) => p.measure === 'damper')?.value ?? 0,
      severity: zoneSeverity(temp, sp, offline),
      note: offline ? 'no reading' : undefined,
      unit: tempP?.unit ?? 'degC',
    }
  })
}

function groupZonesBySite(records: Record[]): Map<string, Zone[]> {
  const map = new Map<string, Zone[]>()
  const equips = toEquips(records).filter((e) => e.domain === 'hvac')
  const sites = new Set(equips.map((e) => e.site))
  for (const site of sites) map.set(site, toZones(records, site))
  return map
}

function zoneSeverity(temp: number | null, sp: number | null, offline: boolean): Severity {
  if (offline) return 'muted'
  if (sp == null || temp == null) return 'green'
  const dev = Math.abs(temp - sp)
  if (dev >= 4) return 'crit'
  if (dev >= 1.5) return 'amber'
  return 'green'
}

const RANK: Severity[] = ['crit', 'amber', 'muted', 'green']
function rollup(sevs: Severity[]): Severity {
  for (const s of RANK) if (sevs.includes(s)) return s
  return 'green'
}

// The trailing reading values for one point id, oldest→newest. Used to draw a
// real time-series chart for a zone or point.
export function readingSeries(records: Record[], pointId: string): number[] {
  return byKind(records, 'reading')
    .filter((r) => str(r.content.point) === pointId)
    .map((r) => ({ v: num(r.content.value), ts: str(r.content.ts) ?? r.created }))
    .filter((x): x is { v: number; ts: string } => x.v != null)
    .sort((a, b) => (a.ts < b.ts ? -1 : 1))
    .map((x) => x.v)
}

// Find the point id of a given measure under an equipment (e.g. an hvac equip's
// 'temp' point), so callers can pull its series.
export function pointIdFor(records: Record[], equipKey: string, site: string, measure: string): string | null {
  const point = byKind(records, 'point').find(
    (p) => str(p.content.equip) === equipKey && str(p.content.site) === site && str(p.content.measure) === measure,
  )
  return point?.id ?? null
}
