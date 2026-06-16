/**
 * Meter board builder (DASHBOARDS.md: "one chart per chart_group, stat tiles for
 * single registers, alarm panel, status+last_seen"). Deterministic: same tags ⇒
 * same page. PURE — takes the fetched records + history and returns widget specs;
 * the page component renders them.
 *
 * Grouping is by `chart_group` (mirrored as the `group:<g>` tag, DASHBOARDS.md
 * §"Chart grouping"): registers sharing a group render as ONE multi-series trend
 * (all voltages together). A register that keeps no `history` renders as a stat
 * tile from its latest live value. The alarm panel evaluates each register's ramp
 * against its latest sample (see alarm-panel.tsx for the POC rule).
 *
 * Join note (seed shape): a register record's `content.meter` is the meter RECORD
 * ID and its `content.key` is `<meterKey>--<defKey>`; a history sample's `register`
 * is the bare `<defKey>`. So a series matches on `meter` id + def-key suffix.
 */
import type { Alarm, RegisterRec } from '@/api/records'
import { severityFor } from '../_shared/field-config'
import type { HistorySample } from '../query/batch'
import { resolveWindow, withinWindow, type WindowToken } from '../query/time-window'
import type { AlarmRow } from '../widgets/alarm-panel'
import type { Series, StatWidget, TrendWidget } from '../widgets/types'

export interface MeterBoard {
  trends: TrendWidget[]
  stats: StatWidget[]
  alarms: AlarmRow[]
}

/** The bare def-key a register history series joins on. */
function defKey(register: RegisterRec): string {
  const k = register.content.key
  const i = k.indexOf('--')
  return i >= 0 ? k.slice(i + 2) : k
}

/** Latest sample value for a register def under a meter, or null. */
function latest(history: HistorySample[], def: string): { value: number; ts: string } | null {
  let best: HistorySample | null = null
  for (const h of history) {
    if (h.register !== def) continue
    if (!best || Date.parse(h.ts) > Date.parse(best.ts)) best = h
  }
  return best ? { value: best.value, ts: best.ts } : null
}

export function buildMeterBoard(
  registers: RegisterRec[],
  history: HistorySample[],
  window: WindowToken,
  timezone: string | undefined
): MeterBoard {
  const resolved = resolveWindow(window)

  // Group history-bearing registers by chart_group; ungrouped use their own key.
  const groups = new Map<string, RegisterRec[]>()
  const stats: StatWidget[] = []
  const alarms: AlarmRow[] = []

  for (const reg of registers) {
    const c = reg.content
    const def = defKey(reg)
    const last = latest(history, def)

    // Alarm evaluation over the latest value (shared ramp; POC client-side rule).
    if (last) {
      const sev = severityFor(last.value, c.alarm as Alarm | undefined)
      if (sev !== 'ok') {
        alarms.push({
          register: c.key,
          name: c.name,
          value: last.value,
          unit: c.unit,
          precision: c.precision,
          severity: sev,
        })
      }
    }

    if (!c.history) {
      // No trend kept → a live stat tile (never a fabricated zero; null → em-dash).
      stats.push({
        type: 'stat',
        title: c.name,
        value: last?.value ?? null,
        unit: c.unit,
        precision: c.precision,
        alarm: c.alarm as Alarm | undefined,
      })
      continue
    }
    const groupKey = c.chart_group || c.key
    const list = groups.get(groupKey) ?? []
    list.push(reg)
    groups.set(groupKey, list)
  }

  const trends: TrendWidget[] = []
  for (const [groupKey, regs] of groups) {
    const series: Series[] = regs.map((reg) => {
      const def = defKey(reg)
      const samples = withinWindow(
        history.filter((h) => h.register === def),
        resolved
      )
      return {
        label: reg.content.name,
        points: samples.map((s) => ({ t: Date.parse(s.ts), v: s.value })),
        alarm: reg.content.alarm as Alarm | undefined,
      }
    })
    // chart_type is shared within a group; first register decides the render.
    const ct = regs[0].content.chart_type
    const type: TrendWidget['type'] = ct === 'bar' ? 'bar' : ct === 'area' ? 'area' : 'line'
    trends.push({
      type,
      title: titleCase(groupKey),
      unit: regs[0].content.unit,
      precision: regs[0].content.precision,
      series,
      timezone,
    })
  }

  // Deterministic ordering: groups alphabetical, stats alphabetical.
  trends.sort((a, b) => a.title.localeCompare(b.title))
  stats.sort((a, b) => a.title.localeCompare(b.title))
  alarms.sort((a, b) => a.name.localeCompare(b.name))
  return { trends, stats, alarms }
}

function titleCase(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1)
}
