/**
 * Meter dashboard page — the richest auto-built view (DASHBOARDS.md meter row):
 * ONE chart per chart_group (all voltages together), stat tiles for single
 * registers, an alarm panel, and the meter's status + last_seen. Pure builder
 * (meter-board.ts) over the meter's registers + its history series.
 *
 * The meter's `register` records are matched by `content.meter` === meter id; each
 * such register IS a series, so the trend is read windowed and series-scoped by
 * fanning a /readings query out over the meter's register ids (useRegistersHistory).
 */
import { Card } from '@/components/ui/card'
import { useMeters, useRegisters, useRegistersHistory } from '../query/batch'
import { buildMeterBoard } from '../auto-build/meter-board'
import type { WindowToken } from '../query/time-window'
import { AlarmPanel } from '../widgets/alarm-panel'
import { StatTile } from '../widgets/stat'
import { StatusPill } from '../widgets/status-tile'
import { TrendPanel } from '../widgets/widget-host'
import { Empty } from '../widgets/empty'

export function MeterPage({ meterId, window }: { meterId: string; window: WindowToken }) {
  const meters = useMeters()
  const registers = useRegisters()
  const meterRegisters = (registers.data ?? []).filter((r) => r.content.meter === meterId)
  const history = useRegistersHistory(meterRegisters)

  const meter = (meters.data ?? []).find((m) => m.id === meterId)
  const timezone = undefined // site tz is resolved at the site level; meter page uses browser-local

  if (registers.isLoading || history.isLoading) return <Empty message='Loading…' />

  const board = buildMeterBoard(meterRegisters, history.data ?? [], window, timezone)

  return (
    <div className='space-y-4'>
      <Card className='flex items-center justify-between p-4'>
        <div className='text-sm font-medium'>{meter?.content.name ?? 'Meter'}</div>
        <StatusPill
          status={(meter?.content.status as never) ?? 'unknown'}
          lastSeen={meter?.content.last_seen}
        />
      </Card>

      {board.stats.length > 0 && (
        <div className='grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4'>
          {board.stats.map((s) => (
            <StatTile key={s.title} widget={s} />
          ))}
        </div>
      )}

      {board.trends.length === 0 ? (
        <Empty message='No history for this meter' />
      ) : (
        <div className='grid grid-cols-1 gap-3 xl:grid-cols-2'>
          {board.trends.map((t) => (
            <TrendPanel key={t.title} widget={t} />
          ))}
        </div>
      )}

      <AlarmPanel alarms={board.alarms} />
    </div>
  )
}
