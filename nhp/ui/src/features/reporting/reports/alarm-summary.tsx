/**
 * Alarm-summary report — a printable snapshot of the active alarms across the
 * scope: the severity counts plus the full list, derived the same way the live
 * console derives them (latest value vs the register's ramp). The print companion
 * to the alarm console.
 */
import { useMemo } from 'react'
import { Card } from '@/components/ui/card'
import { activeAlarms } from '../alarms'
import { selectRegisters, type PortfolioIndex, type ScopeFilter } from '../scope'
import { useLatestReadings } from '../use-portfolio'
import { AlarmCounts, AlarmTable } from '../alarm-view'

export function AlarmSummaryReport({
  index,
  filter,
}: {
  index: PortfolioIndex
  filter: ScopeFilter
}) {
  const registers = useMemo(
    () => selectRegisters(index, filter, { alarmsOnly: true }),
    [index, filter]
  )
  const { latest, isLoading } = useLatestReadings(
    registers.map((r) => ({ id: r.id }))
  )
  const alarms = useMemo(
    () => activeAlarms(index, registers, latest),
    [index, registers, latest]
  )

  if (isLoading) {
    return <Card className='text-muted-foreground p-8 text-center text-sm'>Loading…</Card>
  }

  return (
    <div className='report-avoid-break space-y-3'>
      <AlarmCounts alarms={alarms} />
      <AlarmTable alarms={alarms} />
    </div>
  )
}
