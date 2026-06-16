/**
 * Cross-hierarchy alarm evaluation (DASHBOARDS.md §Alarms: alarm count rolls up
 * onto tenant/site cards). Pure: given every register and every history sample,
 * compute the count of active (warning/critical) alarms per meter RECORD ID, so a
 * site/tenant card can sum the alarms beneath it.
 *
 * POC rule (documented in alarm-panel.tsx): evaluate the register's threshold ramp
 * against its LATEST sampled value — the same severityFor() the charts use, since
 * the rubix rule engine that writes alarm records is not seeded.
 *
 * Join (readings plane): a sample's `series` IS the register RECORD ID, so history
 * joins to a register by a direct `sample.series === register.id` match — no
 * meter-id + def-key string splitting. Recency reads the measurement instant `at`.
 */
import type { Alarm, RegisterRec } from '@/api/records'
import { severityFor } from '../_shared/field-config'
import type { HistorySample } from '../query/batch'

/** Map of meter RECORD ID → count of active alarms among its registers. */
export function alarmCountsByMeter(
  registers: RegisterRec[],
  history: HistorySample[]
): Map<string, number> {
  // Latest sample per series (= register record id).
  const latest = new Map<string, HistorySample>()
  for (const h of history) {
    const prev = latest.get(h.series)
    if (!prev || Date.parse(h.at) > Date.parse(prev.at)) latest.set(h.series, h)
  }
  const counts = new Map<string, number>()
  for (const reg of registers) {
    const alarm = reg.content.alarm as Alarm | undefined
    if (!alarm) continue
    const sample = latest.get(reg.id)
    if (!sample) continue
    if (severityFor(sample.value, alarm) !== 'ok') {
      counts.set(reg.content.meter, (counts.get(reg.content.meter) ?? 0) + 1)
    }
  }
  return counts
}
