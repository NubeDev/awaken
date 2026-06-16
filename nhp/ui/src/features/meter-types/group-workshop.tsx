/**
 * Grouping workshop — a live preview of the dashboard a meter-type's registers will
 * produce, so grouping stops being guesswork. It mirrors the auto-build rules
 * (auto-build/meter-board.ts): history-bearing registers sharing a `chart_group`
 * stack into ONE multi-series chart (chart_type + unit taken from the first member);
 * the rest become live stat tiles. It also surfaces what's *wrong*: history points
 * with no group, and groups whose members disagree on unit/chart type (the builder
 * silently uses the first, so mismatches are easy to miss).
 *
 * Pure preview — every "fix" routes back through `onChange`, never mutates directly.
 */
import { useMemo } from 'react'
import {
  AlertTriangle,
  BarChart3,
  Bell,
  Gauge,
  LineChart,
  Sparkles,
} from 'lucide-react'
import type { RegisterDef } from '@/api/records'
import { Badge } from '@/components/ui/badge'
import { Card } from '@/components/ui/card'
import { seriesColor } from '@/features/dashboards/_shared/palette'
import { GroupCombobox } from './group-combobox'

type GroupWorkshopProps = {
  registers: RegisterDef[]
  /** Patch one register by index (used by the inline "assign group" fixers). */
  onUpdate: (index: number, reg: RegisterDef) => void
}

type Member = { reg: RegisterDef; index: number }
type ChartGroup = {
  /** The group key as the builder sees it (chart_group, or key when blank). */
  label: string
  /** True when this is an ungrouped register charting on its own key. */
  solo: boolean
  members: Member[]
  unit: string
  chartType: string
  unitMismatch: boolean
  chartTypeMismatch: boolean
}

function titleCase(s: string): string {
  return s
    .replace(/[_-]+/g, ' ')
    .replace(/\b\w/g, (c) => c.toUpperCase())
    .trim()
}

function ChartIcon({ type }: { type: string }) {
  if (type === 'bar') return <BarChart3 className='size-4' />
  return <LineChart className='size-4' />
}

export function GroupWorkshop({ registers, onUpdate }: GroupWorkshopProps) {
  const groupOptions = useMemo(
    () =>
      [...new Set(registers.map((r) => r.chart_group).filter(Boolean))].sort(),
    [registers]
  )

  const { charts, tiles } = useMemo(() => {
    const indexed: Member[] = registers.map((reg, index) => ({ reg, index }))
    const tiles = indexed.filter((m) => !m.reg.history)
    const historied = indexed.filter((m) => m.reg.history)

    // Mirror the builder: group key = chart_group || key (ungrouped → own chart).
    const map = new Map<string, Member[]>()
    for (const m of historied) {
      const key = m.reg.chart_group || m.reg.key
      const list = map.get(key) ?? []
      list.push(m)
      map.set(key, list)
    }

    const charts: ChartGroup[] = [...map.entries()].map(([label, members]) => {
      const first = members[0].reg
      return {
        label,
        solo: !first.chart_group,
        members,
        unit: first.unit,
        chartType: first.chart_type,
        unitMismatch: members.some((m) => m.reg.unit !== first.unit),
        chartTypeMismatch: members.some(
          (m) => m.reg.chart_type !== first.chart_type
        ),
      }
    })
    // Real groups first (most members first), solo "ungrouped" charts last.
    charts.sort((a, b) => {
      if (a.solo !== b.solo) return a.solo ? 1 : -1
      return b.members.length - a.members.length
    })
    return { charts, tiles }
  }, [registers])

  const ungrouped = charts.filter((c) => c.solo)
  const problems =
    ungrouped.length +
    charts.filter((c) => c.unitMismatch || c.chartTypeMismatch).length

  return (
    <div className='space-y-4'>
      <div className='flex flex-wrap items-center justify-between gap-2'>
        <div className='flex items-center gap-2 text-sm font-medium'>
          <Sparkles className='size-4' /> Dashboard preview
        </div>
        <div className='flex items-center gap-2 text-xs'>
          <Badge variant='muted'>
            {charts.filter((c) => !c.solo).length} chart
            {charts.filter((c) => !c.solo).length === 1 ? '' : 's'}
          </Badge>
          <Badge variant='muted'>
            {tiles.length} stat tile{tiles.length === 1 ? '' : 's'}
          </Badge>
          {problems > 0 ? (
            <Badge variant='warning'>
              <AlertTriangle className='size-3' /> {problems} to review
            </Badge>
          ) : (
            <Badge variant='positive'>All grouped</Badge>
          )}
        </div>
      </div>

      <p className='text-muted-foreground text-xs'>
        Registers that keep history and share a group stack into one chart. The
        first member sets the chart type, unit and precision for the whole chart.
      </p>

      {/* Ungrouped history points — the thing that's easy to miss */}
      {ungrouped.length > 0 ? (
        <Card className='border-warning/40 bg-warning/5 p-3'>
          <div className='mb-2 flex items-center gap-2 text-sm font-medium'>
            <AlertTriangle className='text-warning size-4' />
            {ungrouped.length} point{ungrouped.length === 1 ? '' : 's'} not in a
            group
          </div>
          <p className='text-muted-foreground mb-3 text-xs'>
            Each draws its own single-series chart. Assign a group to stack them
            with related points.
          </p>
          <div className='space-y-2'>
            {ungrouped.map((c) => {
              const { reg, index } = c.members[0]
              return (
                <div
                  key={c.label}
                  className='flex flex-wrap items-center gap-2 text-sm'
                >
                  <span className='font-mono text-xs'>{reg.key}</span>
                  <span className='text-muted-foreground'>{reg.name}</span>
                  {reg.unit ? (
                    <Badge variant='outline'>{reg.unit}</Badge>
                  ) : null}
                  <div className='ms-auto'>
                    <GroupCombobox
                      value={reg.chart_group}
                      options={groupOptions}
                      onChange={(g) => onUpdate(index, { ...reg, chart_group: g })}
                      className='w-40'
                    />
                  </div>
                </div>
              )
            })}
          </div>
        </Card>
      ) : null}

      {/* Chart cards: what each group will actually render */}
      <div className='grid gap-3 sm:grid-cols-2'>
        {charts
          .filter((c) => !c.solo)
          .map((c) => (
            <ChartPreviewCard key={c.label} group={c} />
          ))}
      </div>

      {/* Live stat tiles (no history) */}
      {tiles.length > 0 ? (
        <div className='space-y-2'>
          <div className='flex items-center gap-2 text-sm font-medium'>
            <Gauge className='size-4' /> Live tiles
            <span className='text-muted-foreground text-xs font-normal'>
              (history off — shown as single values, not charts)
            </span>
          </div>
          <div className='flex flex-wrap gap-2'>
            {tiles.map((m) => (
              <Badge key={m.index} variant='outline'>
                {m.reg.name}
                {m.reg.unit ? ` · ${m.reg.unit}` : ''}
              </Badge>
            ))}
          </div>
        </div>
      ) : null}
    </div>
  )
}

function ChartPreviewCard({ group }: { group: ChartGroup }) {
  const hasAlarm = group.members.some(
    (m) => (m.reg.alarm?.thresholds?.length ?? 0) > 0
  )
  return (
    <Card className='p-3'>
      <div className='mb-2 flex items-center gap-2'>
        <ChartIcon type={group.chartType} />
        <span className='text-sm font-medium'>{titleCase(group.label)}</span>
        {group.unit ? <Badge variant='muted'>{group.unit}</Badge> : null}
        {hasAlarm ? (
          <Bell className='text-warning size-3.5' aria-label='has alarms' />
        ) : null}
        <span className='text-muted-foreground ms-auto text-xs'>
          {group.members.length} series
        </span>
      </div>

      {/* A faux sparkline so the user sees a chart, not just a list */}
      <FauxChart type={group.chartType} count={group.members.length} />

      <ul className='mt-2 space-y-1'>
        {group.members.map((m, i) => (
          <li key={m.index} className='flex items-center gap-2 text-xs'>
            <span
              className='inline-block size-2.5 rounded-full'
              style={{ backgroundColor: seriesColor(i) }}
            />
            <span className='truncate'>{m.reg.name}</span>
            {m.reg.unit !== group.unit ? (
              <Badge variant='warning' className='ms-auto'>
                {m.reg.unit || 'no unit'} ≠ {group.unit || 'no unit'}
              </Badge>
            ) : m.reg.chart_type !== group.chartType ? (
              <Badge variant='warning' className='ms-auto'>
                {m.reg.chart_type} ≠ {group.chartType}
              </Badge>
            ) : null}
          </li>
        ))}
      </ul>

      {group.unitMismatch ? (
        <p className='text-warning mt-2 flex items-center gap-1 text-xs'>
          <AlertTriangle className='size-3' />
          Mixed units share one Y-axis — the chart shows “{group.unit}” for all.
        </p>
      ) : null}
    </Card>
  )
}

/** Decorative preview of the chart shape — not real data, just the silhouette. */
function FauxChart({ type, count }: { type: string; count: number }) {
  const series = Array.from({ length: Math.min(count, 5) })
  if (type === 'bar') {
    const heights = [40, 70, 55, 85, 60, 45, 75]
    return (
      <div className='flex h-16 items-end gap-1'>
        {heights.map((h, i) => (
          <div
            key={i}
            className='flex-1 rounded-sm'
            style={{
              height: `${h}%`,
              backgroundColor: seriesColor(i % count),
              opacity: 0.7,
            }}
          />
        ))}
      </div>
    )
  }
  return (
    <div className='relative h-16 w-full overflow-hidden rounded-sm bg-muted/30'>
      {series.map((_, i) => (
        <svg
          key={i}
          className='absolute inset-0 size-full'
          preserveAspectRatio='none'
          viewBox='0 0 100 40'
        >
          <polyline
            fill='none'
            stroke={seriesColor(i)}
            strokeWidth={1.5}
            points={`0,${30 - i * 4} 20,${18 - i * 3} 40,${24 - i * 2} 60,${10 + i * 3} 80,${20 - i * 2} 100,${14 + i * 2}`}
          />
        </svg>
      ))}
    </div>
  )
}
