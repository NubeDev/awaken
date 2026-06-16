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
 * Join note (readings plane): a sample's `series` IS the register RECORD ID, so a
 * register's series is matched by a direct `sample.series === register.id`. Chart
 * x-axis and recency read the measurement instant `at`.
 */
import type { Alarm, RegisterRec } from '@/api/records'
import { severityFor } from '../_shared/field-config'
import type { HistorySample } from '../query/batch'
import { resolveWindow, withinWindow, type WindowToken } from '../query/time-window'
import type { AlarmRow } from '../widgets/alarm-panel'
import type { Series, StatWidget, TrendWidget } from '../widgets/types'

export interface MeterBoard {
  /** Headline KPI tiles: one per history-bearing group, latest value + sparkline + delta. */
  kpis: StatWidget[]
  trends: TrendWidget[]
  stats: StatWidget[]
  alarms: AlarmRow[]
}

/** Fractional change first→last over a point list (null if not computable). */
function windowDelta(points: { v: number | null }[]): number | null {
  const vals = points.filter((p): p is { v: number } => p.v !== null)
  if (vals.length < 2) return null
  const first = vals[0].v
  const last = vals[vals.length - 1].v
  if (first === 0) return null
  return (last - first) / Math.abs(first)
}

/** Latest sample value for a register's series (by register id), or null. */
function latest(history: HistorySample[], seriesId: string): { value: number; at: string } | null {
  let best: HistorySample | null = null
  for (const h of history) {
    if (h.series !== seriesId) continue
    if (!best || Date.parse(h.at) > Date.parse(best.at)) best = h
  }
  return best ? { value: best.value, at: best.at } : null
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
    const last = latest(history, reg.id)

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
        quantity: c.quantity,
      })
      continue
    }
    const groupKey = c.chart_group || c.key
    const list = groups.get(groupKey) ?? []
    list.push(reg)
    groups.set(groupKey, list)
  }

  const trends: TrendWidget[] = []
  const kpis: StatWidget[] = []
  for (const [groupKey, regs] of groups) {
    const series: Series[] = regs.map((reg) => {
      const samples = withinWindow(
        history.filter((h) => h.series === reg.id),
        resolved
      )
      return {
        label: reg.content.name,
        points: samples.map((s) => ({ t: Date.parse(s.at), v: s.value })),
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

    // One headline KPI per group: the group's primary series (first register),
    // showing its latest value, a sparkline of the window and the window delta.
    // Multi-series groups (e.g. the three voltages) collapse to their first leg —
    // the full set still renders in the trend chart below.
    const lead = regs[0]
    const leadLast = latest(history, lead.id)
    const leadPoints = series[0]?.points ?? []
    kpis.push({
      type: 'stat',
      title: titleCase(groupKey),
      value: leadLast?.value ?? null,
      unit: lead.content.unit,
      precision: lead.content.precision,
      alarm: lead.content.alarm as Alarm | undefined,
      quantity: lead.content.quantity,
      trend: { points: leadPoints, delta: windowDelta(leadPoints) },
    })
  }

  // Deterministic ordering: groups alphabetical, stats alphabetical.
  kpis.sort((a, b) => a.title.localeCompare(b.title))
  trends.sort((a, b) => a.title.localeCompare(b.title))
  stats.sort((a, b) => a.title.localeCompare(b.title))
  alarms.sort((a, b) => a.name.localeCompare(b.name))
  return { kpis, trends, stats, alarms }
}

function titleCase(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1)
}
