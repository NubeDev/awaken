/**
 * The widget data contract (DASHBOARDS-SCOPE §7: "a widget is a pure function of
 * { widget, data }"). The auto-build layer produces these specs; the widgets in
 * this folder render them. No widget fetches — query/batch.ts is the one fetcher.
 */
import type { Alarm } from '@/api/records'

/** One named series of time-stamped points (a register's trend). */
export interface Series {
  /** Legend label, e.g. "Voltage L1". */
  label: string
  /** Points in ascending time. `t` is epoch ms; `v` the value (may be null gap). */
  points: { t: number; v: number | null }[]
  /** Threshold ramp, when the register defines alarms — colours the line. */
  alarm?: Alarm
}

/** A line/bar/area trend chart of one or more series sharing an axis (a group). */
export interface TrendWidget {
  type: 'line' | 'bar' | 'area'
  title: string
  unit?: string
  precision?: number
  series: Series[]
  /** IANA tz of the owning site, so the x-axis renders site-local (§5). */
  timezone?: string
}

/** A single-value tile (a register with no useful trend, or a rollup count). */
export interface StatWidget {
  type: 'stat'
  title: string
  value: number | null
  unit?: string
  precision?: number
  /** Alarm ramp to colour the value by its current severity. */
  alarm?: Alarm
}

/** A flat table of rows (gateway network list, etc). */
export interface TableWidget {
  type: 'table'
  title: string
  columns: { key: string; label: string }[]
  rows: Record<string, string | number>[]
}
