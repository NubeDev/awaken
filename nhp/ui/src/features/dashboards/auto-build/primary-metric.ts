/**
 * A meter's "headline" reading for a list row — device-agnostic. A power meter's
 * headline is energy (kWh), but a LoRa temp sensor's is °C, a CO sensor's is ppm,
 * a coil's is its on/off state. Rather than hard-code an energy column (which
 * shows "—" for every non-power device), each meter row shows the value + UNIT of
 * its primary register, coloured by that register's alarm severity.
 *
 * Picking the primary register, in order of preference:
 *  1. a register currently IN ALARM (so the row surfaces the problem), else
 *  2. an energy register (the power-meter headline), else
 *  3. the first history-bearing register (something with a trend), else
 *  4. the first register of any kind (a coil/gauge still has a latest value).
 *
 * PURE. Returns the latest value, its unit/precision, the severity, and the
 * windowed sparkline points (empty for a no-trend register).
 */
import type { Alarm, AlarmSeverity, RegisterRec } from '@/api/records'
import { meterTag } from '@/enums/tags'
import { severityFor } from '../_shared/field-config'
import type { HistorySample } from '../query/batch'
import { withinWindow, type ResolvedWindow } from '../query/time-window'

export interface PrimaryMetric {
  /** Latest sampled value, or null when the register has no reading yet. */
  latest: number | null
  unit?: string
  precision?: number
  /** Alarm severity of the latest value against the register's ramp. */
  severity: AlarmSeverity
  /** Windowed trend points for the row sparkline (empty ⇒ no trend to draw). */
  points: { t: number; v: number | null }[]
}

const EMPTY: PrimaryMetric = { latest: null, severity: 'ok', points: [] }

/** Latest sample (by `at`) for a register id, across the history fan-out. */
function latestSample(regId: string, history: HistorySample[]): HistorySample | null {
  let best: HistorySample | null = null
  for (const h of history) {
    if (h.series !== regId) continue
    if (!best || Date.parse(h.at) > Date.parse(best.at)) best = h
  }
  return best
}

/** Choose the register whose reading represents the meter in a list row. */
function pickRegister(
  meterRegisters: RegisterRec[],
  history: HistorySample[]
): RegisterRec | null {
  if (meterRegisters.length === 0) return null
  // 1. anything currently in alarm
  const alarming = meterRegisters.find((r) => {
    const s = latestSample(r.id, history)
    return s ? severityFor(s.value, r.content.alarm as Alarm | undefined) !== 'ok' : false
  })
  if (alarming) return alarming
  // 2. an energy register (the classic power-meter headline)
  const energy = meterRegisters.find((r) => r.content.quantity === 'energy')
  if (energy) return energy
  // 3. the first register that keeps a trend
  const trended = meterRegisters.find((r) => r.content.history !== false)
  if (trended) return trended
  // 4. anything at all
  return meterRegisters[0]
}

/**
 * The headline metric for one meter: pick a representative register, read its
 * latest value + window trend. `registers` is the WHOLE register set; we filter to
 * this meter by its `meter:<key>` tag (the same tag the energy rollup uses).
 */
export function primaryMetric(
  meterKey: string,
  registers: RegisterRec[],
  history: HistorySample[],
  resolved: ResolvedWindow
): PrimaryMetric {
  const mTag = meterTag(meterKey)
  const mine = registers.filter((r) => (r.content.tags ?? []).includes(mTag))
  const reg = pickRegister(mine, history)
  if (!reg) return EMPTY

  const rows = history
    .filter((h) => h.series === reg.id)
    .map((h) => ({ at: h.at, value: h.value }))
  const within = withinWindow(rows, resolved)
  const points = within.map((s) => ({ t: Date.parse(s.at), v: s.value }))
  const sample = latestSample(reg.id, history)
  const latest = sample ? sample.value : null
  const severity =
    latest !== null ? severityFor(latest, reg.content.alarm as Alarm | undefined) : 'ok'

  return {
    latest,
    unit: reg.content.unit,
    precision: reg.content.precision,
    severity,
    points,
  }
}
