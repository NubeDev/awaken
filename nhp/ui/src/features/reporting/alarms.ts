/**
 * Active-alarm evaluation (pure). The POC has no rubix rule-engine alarm records
 * (DASHBOARDS.md §Alarms), so an "active alarm" is DERIVED the same way the
 * dashboard alarm panel derives it: take a register's LATEST sampled value and run
 * it through the register's own threshold ramp (`severityFor`). This module rolls
 * that evaluation over a whole scope (many registers) and returns the crossings,
 * so the live alarm console and the printable alarm summary share one definition
 * of "what is alarming".
 */
import type { AlarmSeverity, RegisterRec } from '@/api/records'
import { severityFor } from '@/features/dashboards/_shared/field-config'
import type { PortfolioIndex } from './scope'

export type ActiveSeverity = Exclude<AlarmSeverity, 'ok'>

export interface ActiveAlarm {
  registerId: string
  registerName: string
  meterId: string
  meterName: string
  siteName: string
  tenantName: string
  quantity?: string
  unit?: string
  precision?: number
  value: number
  /** Instant of the latest sample (RFC3339). */
  at: string
  severity: ActiveSeverity
}

/** Latest sample per series, keyed by register record id. */
export type LatestBySeries = Map<string, { at: string; value: number }>

const RANK: Record<ActiveSeverity, number> = { critical: 2, warning: 1 }

/**
 * Evaluate the active alarms for a set of registers against their latest values.
 * A register with no latest sample, or whose value lands on the `ok` baseline, is
 * not returned. Sorted critical-first, then by value-over-threshold recency.
 */
export function activeAlarms(
  index: PortfolioIndex,
  registers: RegisterRec[],
  latest: LatestBySeries
): ActiveAlarm[] {
  const out: ActiveAlarm[] = []
  for (const r of registers) {
    const sample = latest.get(r.id)
    if (!sample) continue
    const sev = severityFor(sample.value, r.content.alarm)
    if (sev === 'ok') continue
    const meter = index.meterById.get(r.content.meter)
    const loc = index.meterLocation.get(r.content.meter)
    out.push({
      registerId: r.id,
      registerName: r.content.name,
      meterId: r.content.meter,
      meterName: meter?.content.name ?? '—',
      siteName: loc?.siteName ?? '—',
      tenantName: loc?.tenantName ?? '—',
      quantity: r.content.quantity,
      unit: r.content.unit,
      precision: r.content.precision,
      value: sample.value,
      at: sample.at,
      severity: sev,
    })
  }
  return out.sort(
    (a, b) =>
      RANK[b.severity] - RANK[a.severity] || Date.parse(b.at) - Date.parse(a.at)
  )
}

/** Count alarms by severity (for the console/report summary chips). */
export function severityCounts(alarms: ActiveAlarm[]): {
  critical: number
  warning: number
} {
  let critical = 0
  let warning = 0
  for (const a of alarms) {
    if (a.severity === 'critical') critical++
    else warning++
  }
  return { critical, warning }
}
