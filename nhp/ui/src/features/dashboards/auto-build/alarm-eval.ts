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
 * Join (seed shape): register `content.meter` is the meter id; its `content.key`
 * is `<meterKey>--<defKey>`; a history sample's `register` is the bare `<defKey>`.
 */
import type { Alarm, RegisterRec } from '@/api/records'
import { severityFor } from '../_shared/field-config'
import type { HistorySample } from '../query/batch'

function defKey(key: string): string {
  const i = key.indexOf('--')
  return i >= 0 ? key.slice(i + 2) : key
}

/** Map of meter RECORD ID → count of active alarms among its registers. */
export function alarmCountsByMeter(
  registers: RegisterRec[],
  history: HistorySample[]
): Map<string, number> {
  // Latest sample per (meter id, def key).
  const latest = new Map<string, HistorySample>()
  for (const h of history) {
    const k = `${h.meter}::${h.register}`
    const prev = latest.get(k)
    if (!prev || Date.parse(h.ts) > Date.parse(prev.ts)) latest.set(k, h)
  }
  const counts = new Map<string, number>()
  for (const reg of registers) {
    const alarm = reg.content.alarm as Alarm | undefined
    if (!alarm) continue
    const sample = latest.get(`${reg.content.meter}::${defKey(reg.content.key)}`)
    if (!sample) continue
    if (severityFor(sample.value, alarm) !== 'ok') {
      counts.set(reg.content.meter, (counts.get(reg.content.meter) ?? 0) + 1)
    }
  }
  return counts
}
