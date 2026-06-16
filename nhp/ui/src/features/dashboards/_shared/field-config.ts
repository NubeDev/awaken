/**
 * Threshold resolution (DASHBOARDS-SCOPE §7 FieldConfig; DOMAIN-MODEL §Alarms).
 * A register's `alarm.thresholds` is an ordered ramp of `{ value, severity }` —
 * the SAME ramp that fires alarms and colours a chart. This is the pure function
 * that maps a value to its severity, used by both the chart colouring and the
 * alarm panel so "what you see is what alarms".
 */
import type { Alarm, AlarmSeverity } from '@/api/records'

/**
 * The severity a value lands in: the highest step whose `value` the sample is at
 * or above. The baseline step (`value: null`) is the floor (`ok`). Steps need not
 * be pre-sorted — we sort by ascending `value` (nulls first) so ramp order is
 * deterministic regardless of how the editor stored them.
 */
export function severityFor(value: number, alarm: Alarm | undefined): AlarmSeverity {
  if (!alarm?.thresholds?.length) return 'ok'
  const steps = [...alarm.thresholds].sort((a, b) => {
    if (a.value === null) return -1
    if (b.value === null) return 1
    return a.value - b.value
  })
  let current: AlarmSeverity = 'ok'
  for (const step of steps) {
    if (step.value === null || value >= step.value) current = step.severity
    else break
  }
  return current
}

/** True when the ramp defines anything beyond the baseline `ok`. */
export function hasAlarm(alarm: Alarm | undefined): boolean {
  return !!alarm?.thresholds?.some((t) => t.severity !== 'ok')
}

/** The numeric reference lines a chart draws for warning/critical crossings. */
export function thresholdLines(
  alarm: Alarm | undefined
): { value: number; severity: AlarmSeverity }[] {
  if (!alarm?.thresholds) return []
  return alarm.thresholds
    .filter((t): t is { value: number; severity: AlarmSeverity } => t.value !== null && t.severity !== 'ok')
    .sort((a, b) => a.value - b.value)
}
