/**
 * Pivot a list of named series into the row-per-timestamp shape recharts wants:
 * `[{ t, "Voltage L1": 230, "Voltage L2": 231, … }]`. Pure; shared by the
 * line/bar/area trend widgets so the pivot lives in ONE place. Timestamps are
 * unioned across series (a gap stays a missing key → recharts `connectNulls`).
 */
import type { Series } from './types'

export interface ChartRow {
  t: number
  [series: string]: number | null
}

export function toChartRows(series: Series[]): ChartRow[] {
  const byTime = new Map<number, ChartRow>()
  for (const s of series) {
    for (const p of s.points) {
      let row = byTime.get(p.t)
      if (!row) {
        row = { t: p.t }
        byTime.set(p.t, row)
      }
      row[s.label] = p.v
    }
  }
  return [...byTime.values()].sort((a, b) => a.t - b.t)
}
