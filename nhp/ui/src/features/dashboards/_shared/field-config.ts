/**
 * Threshold resolution (DASHBOARDS-SCOPE §7 FieldConfig; DOMAIN-MODEL §Alarms).
 * A register's `alarm.thresholds` is an ordered ramp of `{ value, severity }` —
 * the SAME ramp that fires alarms and colours a chart. This is the pure function
 * that maps a value to its severity, used by both the chart colouring and the
 * alarm panel so "what you see is what alarms".
 */
import type { Alarm, AlarmSeverity } from '@/api/records'

/**
 * The severity a value lands in: walk the ramp from the baseline outward and take
 * the last step the value has crossed. The baseline step (`value: null`) is the
 * floor (`ok`). Steps need not be pre-sorted.
 *
 * Direction (`alarm.direction`, default `'above'`):
 *  - `'above'` — sort ascending (nulls first); a step trips when `value >= step`.
 *    The default, so every existing electrical alarm is unchanged.
 *  - `'below'` — sort descending (nulls first); a step trips when `value <= step`.
 *    Used by the LoRa low-battery alarm (fires as the battery DROPS into a step).
 */
export function severityFor(value: number, alarm: Alarm | undefined): AlarmSeverity {
  if (!alarm?.thresholds?.length) return 'ok'
  const below = alarm.direction === 'below'
  const steps = [...alarm.thresholds].sort((a, b) => {
    if (a.value === null) return -1
    if (b.value === null) return 1
    return below ? b.value - a.value : a.value - b.value
  })
  let current: AlarmSeverity = 'ok'
  for (const step of steps) {
    const crossed =
      step.value === null || (below ? value <= step.value : value >= step.value)
    if (crossed) current = step.severity
    else break
  }
  return current
}

/**
 * The threshold step a value has tripped — the numeric ramp value for the
 * severity returned by {@link severityFor}, or null at baseline `ok`. Used by the
 * alarm panel to show WHICH limit was crossed ("≥ 100 ppm").
 */
export function crossedThreshold(
  value: number,
  alarm: Alarm | undefined
): { value: number; severity: Exclude<AlarmSeverity, 'ok'>; direction: 'above' | 'below' } | null {
  const sev = severityFor(value, alarm)
  if (sev === 'ok' || !alarm) return null
  const direction = alarm.direction === 'below' ? 'below' : 'above'
  const step = alarm.thresholds.find((t) => t.severity === sev && t.value !== null)
  if (!step || step.value === null) return null
  return { value: step.value, severity: sev, direction }
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
