/**
 * Demo dataset — the BMS/EMS sample from the original UI demo, shaped to the
 * real wire DTOs. Served by the API client ONLY when demo mode is on
 * (`VITE_DEMO=1` or no backend). Real mode never touches this file.
 */
import type { Equip, HisSample, Point, PriorityArray, Site, Spark } from '../types'

const ISO = (minsAgo: number) => new Date(Date.UTC(2026, 5, 12, 12, 0, 0) - minsAgo * 60_000).toISOString()

/** Deterministic pseudo-random series so charts are stable across renders. */
function series(n: number, base: number, amp: number, seed: number, period = 24): number[] {
  let s = seed % 2147483647
  if (s <= 0) s += 2147483646
  const rng = () => (s = (s * 16807) % 2147483647) / 2147483647
  return Array.from({ length: n }, (_, i) => {
    const wave = Math.sin((i / period) * Math.PI * 2) * amp
    const wobble = (rng() - 0.5) * amp * 0.5
    return +(base + wave + wobble).toFixed(2)
  })
}

/** Build a 48-sample hourly history ending now. */
export function demoHistory(base: number, amp: number, seed: number): HisSample[] {
  const vals = series(48, base, amp, seed)
  return vals.map((value, i) => ({ ts: ISO((48 - i) * 30), value }))
}

export const SITES: Site[] = [
  { id: 's1', org: 'acme', slug: 'hq-tower', display_name: 'HQ Tower', tags: ['site', 'commercial'], created_at: ISO(100000) },
  { id: 's2', org: 'acme', slug: 'distribution-w', display_name: 'Distribution West', tags: ['site', 'warehouse'], created_at: ISO(100000) },
  { id: 's3', org: 'acme', slug: 'lab-campus', display_name: 'Lab Campus', tags: ['site', 'lab'], created_at: ISO(100000) },
  { id: 's4', org: 'acme', slug: 'cold-store-3', display_name: 'Cold Store 3', tags: ['site', 'cold'], created_at: ISO(100000) },
]

export const EQUIPS: Equip[] = [
  { id: 'e1', site_id: 's1', path: 'ahu-1', display_name: 'AHU-1 · L1 East', tags: ['ahu', 'hvac'], created_at: ISO(100000) },
  { id: 'e2', site_id: 's1', path: 'ahu-3', display_name: 'AHU-3 · L4 West', tags: ['ahu', 'hvac'], created_at: ISO(100000) },
  { id: 'e3', site_id: 's1', path: 'chiller-1', display_name: 'Chiller-1', tags: ['chiller', 'plant'], created_at: ISO(100000) },
  { id: 'e4', site_id: 's1', path: 'chiller-2', display_name: 'Chiller-2', tags: ['chiller', 'plant'], created_at: ISO(100000) },
  { id: 'e5', site_id: 's1', path: 'boiler-1', display_name: 'Boiler-1', tags: ['boiler', 'plant'], created_at: ISO(100000) },
  { id: 'e6', site_id: 's1', path: 'meter-main', display_name: 'Main Incomer', tags: ['elec', 'meter', 'energy'], created_at: ISO(100000) },
  { id: 'e7', site_id: 's1', path: 'vav-4-12', display_name: 'VAV 4-12', tags: ['vav', 'hvac'], created_at: ISO(100000) },
  { id: 'e8', site_id: 's1', path: 'vav-4-13', display_name: 'VAV 4-13', tags: ['vav', 'hvac'], created_at: ISO(100000) },
  { id: 'e9', site_id: 's1', path: 'ct-1', display_name: 'Cooling Tower 1', tags: ['tower', 'plant'], created_at: ISO(100000) },
]

const emptyPa: PriorityArray = { slots: Array(16).fill(null), relinquish_default: null }
function pa(entries: Array<[number, number]>, def: number | null = null): PriorityArray {
  const slots: PriorityArray['slots'] = Array(16).fill(null)
  for (const [level, v] of entries) slots[level - 1] = v
  return { slots, relinquish_default: def }
}

type Seed = { base: number; amp: number; seed: number }
const seeds: Record<string, Seed> = {}

function point(p: Omit<Point, 'created_at'> & { _seed?: Seed }): Point {
  if (p._seed) seeds[p.id] = p._seed
  const { _seed, ...rest } = p
  void _seed
  return { ...rest, created_at: ISO(100000) }
}

export const POINTS: Point[] = [
  point({ id: 'p1', equip_id: 'e2', slug: 'discharge-temp', display_name: 'Discharge Air Temp', kind: 'sensor', unit: '°C', tags: ['discharge', 'air', 'temp', 'sensor'], priority_array: emptyPa, cur_value: 13.4, cur_ts: ISO(0.2), _seed: { base: 13.5, amp: 1.4, seed: 11 } }),
  point({ id: 'p2', equip_id: 'e2', slug: 'return-temp', display_name: 'Return Air Temp', kind: 'sensor', unit: '°C', tags: ['return', 'air', 'temp', 'sensor'], priority_array: emptyPa, cur_value: 22.8, cur_ts: ISO(0.2), _seed: { base: 22.6, amp: 0.8, seed: 21 } }),
  point({ id: 'p3', equip_id: 'e2', slug: 'supply-fan-cmd', display_name: 'Supply Fan Speed', kind: 'cmd', unit: '%', tags: ['supply', 'fan', 'cmd'], priority_array: pa([[8, 82], [13, 70], [16, 60]]), cur_value: 82, cur_ts: ISO(0.1), _seed: { base: 78, amp: 8, seed: 31 } }),
  point({ id: 'p4', equip_id: 'e2', slug: 'cooling-valve', display_name: 'Cooling Valve', kind: 'cmd', unit: '%', tags: ['cool', 'valve', 'cmd'], priority_array: pa([[13, 96], [16, 40]]), cur_value: 96, cur_ts: ISO(0.1), _seed: { base: 60, amp: 30, seed: 41 } }),
  point({ id: 'p5', equip_id: 'e2', slug: 'heating-valve', display_name: 'Heating Valve', kind: 'cmd', unit: '%', tags: ['heat', 'valve', 'cmd'], priority_array: pa([[16, 35]]), cur_value: 35, cur_ts: ISO(0.1), _seed: { base: 20, amp: 18, seed: 51 } }),
  point({ id: 'p6', equip_id: 'e2', slug: 'discharge-sp', display_name: 'Discharge Temp Setpoint', kind: 'sp', unit: '°C', tags: ['discharge', 'temp', 'sp'], priority_array: pa([[10, 13.0], [16, 14.0]], 14.0), cur_value: 13.0, cur_ts: ISO(1), _seed: { base: 13, amp: 0.3, seed: 61 } }),
  point({ id: 'p7', equip_id: 'e2', slug: 'occupancy', display_name: 'Zone Occupancy', kind: 'sensor', unit: '', tags: ['zone', 'occ', 'sensor'], priority_array: emptyPa, cur_value: 'Occupied', cur_ts: ISO(0.5) }),
  point({ id: 'p8', equip_id: 'e2', slug: 'static-press', display_name: 'Duct Static Pressure', kind: 'sensor', unit: 'Pa', tags: ['duct', 'pressure', 'sensor'], priority_array: emptyPa, cur_value: 248, cur_ts: ISO(0.1), _seed: { base: 250, amp: 14, seed: 81 } }),
  // Main incomer demand point so the dashboard chart has a real source.
  point({ id: 'p9', equip_id: 'e6', slug: 'kw-total', display_name: 'Total Demand', kind: 'sensor', unit: 'kW', tags: ['elec', 'meter', 'energy', 'kw'], priority_array: emptyPa, cur_value: 412, cur_ts: ISO(0.1), _seed: { base: 360, amp: 120, seed: 7 } }),
]

export function historyFor(pointId: string): HisSample[] {
  const s = seeds[pointId]
  return s ? demoHistory(s.base, s.amp, s.seed) : []
}

export const SPARKS: Spark[] = [
  { id: 'sp1', site_id: 's1', rule: 'simultaneous-heat-cool', severity: 'fault', message: 'Simultaneous heating and cooling — cooling valve 96% while heating valve 35%', point_ids: ['p4', 'p5'], ts: ISO(6), acknowledged: false },
  { id: 'sp2', site_id: 's1', rule: 'rogue-zone', severity: 'fault', message: 'CHW supply temp 9.2°C above setpoint for 14 min — possible fouling', point_ids: [], ts: ISO(22), acknowledged: false },
  { id: 'sp3', site_id: 's1', rule: 'stuck-damper', severity: 'warning', message: 'Damper command changed 40% but airflow flat — possible stuck actuator', point_ids: [], ts: ISO(38), acknowledged: false },
  { id: 'sp4', site_id: 's1', rule: 'after-hours-runtime', severity: 'warning', message: 'Fan running 3.2h after scheduled off — 41 kWh waste', point_ids: ['p3'], ts: ISO(60), acknowledged: false },
  { id: 'sp5', site_id: 's1', rule: 'sensor-drift', severity: 'info', message: 'Flue temp sensor drift detected vs sibling sensor (1.8°C)', point_ids: [], ts: ISO(120), acknowledged: true },
  { id: 'sp6', site_id: 's1', rule: 'demand-spike', severity: 'warning', message: 'Peak demand approaching 92% of contracted capacity', point_ids: ['p9'], ts: ISO(180), acknowledged: false },
  { id: 'sp7', site_id: 's1', rule: 'low-delta-t', severity: 'info', message: 'Chilled water ΔT 3.1°C — below 5°C design (low-ΔT syndrome)', point_ids: [], ts: ISO(300), acknowledged: true },
]
