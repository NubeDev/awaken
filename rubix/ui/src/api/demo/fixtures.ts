/**
 * Demo dataset — the BMS/EMS sample from the original UI demo, shaped to the
 * real wire DTOs. Served by the API client ONLY when demo mode is on
 * (`VITE_DEMO=1` or unset). Real mode never touches this file.
 *
 * Every site gets the full equipment blueprint so the UI is populated no
 * matter which site the operator has selected.
 */
import type { Equip, HisSample, Point, PriorityArray, Site, Spark, TagSet } from '../types'

/** Author tags as name lists, emit the wire marker map (`{name: true}`). */
const markers = (names: readonly string[]): TagSet =>
  Object.fromEntries(names.map((n) => [n, true]))

const NOW = Date.now()
const ISO = (minsAgo: number) => new Date(NOW - minsAgo * 60_000).toISOString()

/** Deterministic pseudo-random series so charts are stable across renders. */
function series(n: number, base: number, amp: number, seed: number, period = 48): number[] {
  let s = seed % 2147483647
  if (s <= 0) s += 2147483646
  const rng = () => (s = (s * 16807) % 2147483647) / 2147483647
  return Array.from({ length: n }, (_, i) => {
    const wave = Math.sin((i / period) * Math.PI * 2) * amp
    const wobble = (rng() - 0.5) * amp * 0.5
    return +(base + wave + wobble).toFixed(2)
  })
}

type Seed = { base: number; amp: number; seed: number }
const seeds = new Map<string, Seed>()

/** 7 days of 30-minute samples ending now — enough for 24h/48h/7d ranges. */
export function historyFor(pointId: string): HisSample[] {
  const s = seeds.get(pointId)
  if (!s) return []
  const n = 7 * 48
  const vals = series(n, s.base, s.amp, s.seed)
  return vals.map((value, i) => ({ ts: ISO((n - i) * 30), value }))
}

export const SITES: Site[] = [
  { id: 's1', org: 'acme', slug: 'hq-tower', display_name: 'HQ Tower', tags: markers(['site', 'commercial']), created_at: ISO(100000) },
  { id: 's2', org: 'acme', slug: 'distribution-w', display_name: 'Distribution West', tags: markers(['site', 'warehouse']), created_at: ISO(100000) },
  { id: 's3', org: 'acme', slug: 'lab-campus', display_name: 'Lab Campus', tags: markers(['site', 'lab']), created_at: ISO(100000) },
  { id: 's4', org: 'acme', slug: 'cold-store-3', display_name: 'Cold Store 3', tags: markers(['site', 'cold']), created_at: ISO(100000) },
]

const EQUIP_BLUEPRINT = [
  { key: 'ahu-1', name: 'AHU-1 · L1 East', tags: ['ahu', 'hvac'] },
  { key: 'ahu-3', name: 'AHU-3 · L4 West', tags: ['ahu', 'hvac'] },
  { key: 'chiller-1', name: 'Chiller-1', tags: ['chiller', 'plant'] },
  { key: 'chiller-2', name: 'Chiller-2', tags: ['chiller', 'plant'] },
  { key: 'boiler-1', name: 'Boiler-1', tags: ['boiler', 'plant'] },
  { key: 'meter-main', name: 'Main Incomer', tags: ['elec', 'meter', 'energy'] },
  { key: 'vav-4-12', name: 'VAV 4-12', tags: ['vav', 'hvac'] },
  { key: 'vav-4-13', name: 'VAV 4-13', tags: ['vav', 'hvac'] },
  { key: 'ct-1', name: 'Cooling Tower 1', tags: ['tower', 'plant'] },
] as const

const emptyPa: PriorityArray = { slots: Array(16).fill(null), relinquish_default: null }
function pa(entries: Array<[number, number]>, def: number | null = null): PriorityArray {
  const slots: PriorityArray['slots'] = Array(16).fill(null)
  for (const [level, v] of entries) slots[level - 1] = v
  return { slots, relinquish_default: def }
}

type PointSpec = {
  equip: string
  slug: string
  name: string
  kind: Point['kind']
  unit: string
  tags: string[]
  cur: number | string
  minsAgo?: number
  pa?: PriorityArray
  seed?: Seed
}

/** Point blueprint — AHU-3 carries the showcase command points; the main
 * incomer carries the demand meter + per-system submeters; zone sensors carry
 * the comfort index. */
const POINT_BLUEPRINT: PointSpec[] = [
  { equip: 'ahu-3', slug: 'discharge-temp', name: 'Discharge Air Temp', kind: 'sensor', unit: '°C', tags: ['discharge', 'air', 'temp', 'sensor'], cur: 13.4, seed: { base: 13.5, amp: 1.4, seed: 11 } },
  { equip: 'ahu-3', slug: 'return-temp', name: 'Return Air Temp', kind: 'sensor', unit: '°C', tags: ['return', 'air', 'temp', 'sensor'], cur: 22.8, seed: { base: 22.6, amp: 0.8, seed: 21 } },
  { equip: 'ahu-3', slug: 'supply-fan-cmd', name: 'Supply Fan Speed', kind: 'cmd', unit: '%', tags: ['supply', 'fan', 'cmd'], cur: 82, pa: pa([[8, 82], [13, 70], [16, 60]]), seed: { base: 78, amp: 8, seed: 31 } },
  { equip: 'ahu-3', slug: 'cooling-valve', name: 'Cooling Valve', kind: 'cmd', unit: '%', tags: ['cool', 'valve', 'cmd'], cur: 96, pa: pa([[13, 96], [16, 40]]), seed: { base: 60, amp: 30, seed: 41 } },
  { equip: 'ahu-3', slug: 'heating-valve', name: 'Heating Valve', kind: 'cmd', unit: '%', tags: ['heat', 'valve', 'cmd'], cur: 35, pa: pa([[16, 35]]), seed: { base: 20, amp: 18, seed: 51 } },
  { equip: 'ahu-3', slug: 'discharge-sp', name: 'Discharge Temp Setpoint', kind: 'sp', unit: '°C', tags: ['discharge', 'temp', 'sp'], cur: 13.0, minsAgo: 1, pa: pa([[10, 13.0], [16, 14.0]], 14.0), seed: { base: 13, amp: 0.3, seed: 61 } },
  { equip: 'ahu-3', slug: 'occupancy', name: 'Zone Occupancy', kind: 'sensor', unit: '', tags: ['zone', 'occ', 'sensor'], cur: 'Occupied', minsAgo: 0.5 },
  { equip: 'ahu-3', slug: 'static-press', name: 'Duct Static Pressure', kind: 'sensor', unit: 'Pa', tags: ['duct', 'pressure', 'sensor'], cur: 248, seed: { base: 250, amp: 14, seed: 81 } },
  { equip: 'ahu-1', slug: 'discharge-temp', name: 'Discharge Air Temp', kind: 'sensor', unit: '°C', tags: ['discharge', 'air', 'temp', 'sensor'], cur: 13.0, seed: { base: 13.1, amp: 1.0, seed: 13 } },
  { equip: 'ahu-1', slug: 'supply-fan-cmd', name: 'Supply Fan Speed', kind: 'cmd', unit: '%', tags: ['supply', 'fan', 'cmd'], cur: 64, pa: pa([[16, 64]]), seed: { base: 62, amp: 9, seed: 33 } },
  { equip: 'chiller-1', slug: 'chw-supply-temp', name: 'CHW Supply Temp', kind: 'sensor', unit: '°C', tags: ['chw', 'cool', 'temp', 'sensor'], cur: 6.8, seed: { base: 6.6, amp: 0.5, seed: 91 } },
  { equip: 'chiller-1', slug: 'load-pct', name: 'Chiller Load', kind: 'sensor', unit: '%', tags: ['cool', 'load', 'sensor'], cur: 72, seed: { base: 68, amp: 16, seed: 93 } },
  { equip: 'meter-main', slug: 'kw-total', name: 'Total Demand', kind: 'sensor', unit: 'kW', tags: ['elec', 'meter', 'energy', 'kw'], cur: 412, seed: { base: 360, amp: 120, seed: 7 } },
  { equip: 'meter-main', slug: 'kw-chillers', name: 'Chillers', kind: 'sensor', unit: 'kW', tags: ['elec', 'submeter', 'energy'], cur: 168, seed: { base: 150, amp: 50, seed: 101 } },
  { equip: 'meter-main', slug: 'kw-ahus', name: 'AHUs / Fans', kind: 'sensor', unit: 'kW', tags: ['elec', 'submeter', 'energy'], cur: 96, seed: { base: 90, amp: 24, seed: 103 } },
  { equip: 'meter-main', slug: 'kw-lighting', name: 'Lighting', kind: 'sensor', unit: 'kW', tags: ['elec', 'submeter', 'energy'], cur: 64, seed: { base: 60, amp: 18, seed: 105 } },
  { equip: 'meter-main', slug: 'kw-plug', name: 'Plug loads', kind: 'sensor', unit: 'kW', tags: ['elec', 'submeter', 'energy'], cur: 52, seed: { base: 50, amp: 10, seed: 107 } },
  { equip: 'meter-main', slug: 'kw-other', name: 'Other', kind: 'sensor', unit: 'kW', tags: ['elec', 'submeter', 'energy'], cur: 32, seed: { base: 30, amp: 6, seed: 109 } },
  { equip: 'vav-4-12', slug: 'comfort-index', name: 'Comfort Index', kind: 'sensor', unit: '%', tags: ['zone', 'comfort', 'sensor'], cur: 97.2, seed: { base: 96.4, amp: 1.6, seed: 55 } },
  { equip: 'vav-4-12', slug: 'damper-pos', name: 'Damper Position', kind: 'cmd', unit: '%', tags: ['zone', 'damper', 'cmd'], cur: 44, pa: pa([[16, 44]]), seed: { base: 45, amp: 12, seed: 57 } },
]

export const EQUIPS: Equip[] = []
export const POINTS: Point[] = []

for (const site of SITES) {
  for (const e of EQUIP_BLUEPRINT) {
    EQUIPS.push({
      id: `${site.id}-${e.key}`,
      site_id: site.id,
      path: e.key,
      display_name: e.name,
      tags: markers(e.tags),
      created_at: ISO(100000),
    })
  }
  for (const p of POINT_BLUEPRINT) {
    const id = `${site.id}-${p.equip}-${p.slug}`
    if (p.seed) {
      // offset the seed per site so each site's curves differ
      seeds.set(id, { ...p.seed, seed: p.seed.seed + SITES.indexOf(site) * 7 })
    }
    POINTS.push({
      id,
      equip_id: `${site.id}-${p.equip}`,
      slug: p.slug,
      display_name: p.name,
      kind: p.kind,
      unit: p.unit || null,
      tags: markers(p.tags),
      priority_array: p.pa ? { slots: [...p.pa.slots], relinquish_default: p.pa.relinquish_default } : { ...emptyPa, slots: Array(16).fill(null) },
      cur_value: p.cur,
      cur_ts: ISO(p.minsAgo ?? 0.2),
      created_at: ISO(100000),
    })
  }
}

const SPARK_BLUEPRINT = [
  { rule: 'simultaneous-heat-cool', severity: 'fault', message: 'Simultaneous heating and cooling — cooling valve 96% while heating valve 35%', points: ['ahu-3-cooling-valve', 'ahu-3-heating-valve'], minsAgo: 6, ack: false },
  { rule: 'rogue-zone', severity: 'fault', message: 'CHW supply temp 9.2°C above setpoint for 14 min — possible fouling', points: ['chiller-1-chw-supply-temp'], minsAgo: 22, ack: false },
  { rule: 'stuck-damper', severity: 'warning', message: 'Damper command changed 40% but airflow flat — possible stuck actuator', points: ['vav-4-12-damper-pos'], minsAgo: 38, ack: false },
  { rule: 'after-hours-runtime', severity: 'warning', message: 'Fan running 3.2h after scheduled off — 41 kWh waste', points: ['ahu-1-supply-fan-cmd'], minsAgo: 60, ack: false },
  { rule: 'sensor-drift', severity: 'info', message: 'Flue temp sensor drift detected vs sibling sensor (1.8°C)', points: [], minsAgo: 120, ack: true },
  { rule: 'demand-spike', severity: 'warning', message: 'Peak demand approaching 92% of contracted capacity', points: ['meter-main-kw-total'], minsAgo: 180, ack: false },
  { rule: 'low-delta-t', severity: 'info', message: 'Chilled water ΔT 3.1°C — below 5°C design (low-ΔT syndrome)', points: ['chiller-1-load-pct'], minsAgo: 300, ack: true },
] as const

export const SPARKS: Spark[] = SITES.flatMap((site, si) =>
  SPARK_BLUEPRINT.map((s, i) => ({
    id: `${site.id}-sp${i + 1}`,
    site_id: site.id,
    rule: s.rule,
    severity: s.severity,
    message: s.message,
    point_ids: s.points.map((p) => `${site.id}-${p}`),
    ts: ISO(s.minsAgo + si * 3),
    acknowledged: s.ack,
  }))
)
