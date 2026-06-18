/**
 * Alarm console (sidebar → Report → Alarms). The live view of active alarms across
 * a scope — a tenant (all its sites) or one site — derived from each register's
 * latest value against its own threshold ramp (alarms.ts, the same definition the
 * dashboard alarm panel and the alarm-summary report use). Filter by tenant /
 * site / meter-type / quantity and by severity; export the current list to PDF.
 *
 * "Active" is derived, not stored: the POC has no rubix alarm-record surface
 * (DASHBOARDS.md §Alarms), so there is no acknowledge/clear — an alarm is present
 * exactly while a register's latest sample crosses its ramp.
 */
import { useMemo, useState } from 'react'
import { Download, RefreshCw } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Main } from '@/components/layout/main'
import { useQueryClient } from '@tanstack/react-query'
import { usePortfolio, useLatestReadings } from './use-portfolio'
import { FilterBar } from './filter-bar'
import { selectRegisters, type ScopeFilter } from './scope'
import { activeAlarms, type ActiveSeverity } from './alarms'
import { AlarmCounts, AlarmTable } from './alarm-view'
import {
  PrintStyles,
  REPORT_ID,
  printDocument,
  scopeSummary,
} from './report-chrome'

type SeverityFilter = 'all' | ActiveSeverity

export function AlarmsConsole() {
  const { index, isLoading } = usePortfolio()
  const qc = useQueryClient()
  const [filter, setFilter] = useState<ScopeFilter>({})
  const [severity, setSeverity] = useState<SeverityFilter>('all')

  const registers = useMemo(
    // includeNoHistory: a register can be in alarm on its LATEST value even when
    // it keeps no trend (a LoRa battery / a gauge are history=false but still
    // alarm — e.g. low-battery). The seed stands in a single latest point for
    // those, so the console must consider them too.
    () => selectRegisters(index, filter, { alarmsOnly: true, includeNoHistory: true }),
    [index, filter]
  )
  const { latest, isLoading: loadingReadings } = useLatestReadings(
    registers.map((r) => ({ id: r.id }))
  )
  const all = useMemo(
    () => activeAlarms(index, registers, latest),
    [index, registers, latest]
  )
  const alarms = severity === 'all' ? all : all.filter((a) => a.severity === severity)

  const labels = scopeSummary(index, filter)
  const exportPdf = () =>
    printDocument(
      `alarms-${labels.tenant.toLowerCase().replace(/\s+/g, '-')}-${new Date()
        .toISOString()
        .slice(0, 10)}`
    )

  return (
    <Main>
      <PrintStyles />

      <div className='flex items-start justify-between gap-4 print:hidden'>
        <div>
          <h2 className='text-xl font-semibold'>Alarm console</h2>
          <p className='text-muted-foreground text-sm'>
            Active alarms across a tenant or site — registers whose latest reading
            crosses a warning or critical threshold.
          </p>
        </div>
        <div className='flex gap-2'>
          <Button
            variant='outline'
            onClick={() => qc.invalidateQueries({ queryKey: ['dash', 'readings'] })}
            title='Re-read latest values'
          >
            <RefreshCw className='mr-1 size-4' /> Refresh
          </Button>
          <Button onClick={exportPdf} disabled={isLoading}>
            <Download className='mr-1 size-4' /> Export PDF
          </Button>
        </div>
      </div>

      <Card className='my-4 space-y-4 p-4 print:hidden'>
        <FilterBar index={index} filter={filter} onChange={setFilter} />
        <div className='grid gap-1 sm:max-w-xs'>
          <Label className='text-xs'>Severity</Label>
          <Select value={severity} onValueChange={(v) => setSeverity(v as SeverityFilter)}>
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value='all'>All severities</SelectItem>
              <SelectItem value='critical'>Critical only</SelectItem>
              <SelectItem value='warning'>Warning only</SelectItem>
            </SelectContent>
          </Select>
        </div>
      </Card>

      <div id={REPORT_ID} className='space-y-4'>
        <div className='report-avoid-break flex flex-wrap items-center justify-between gap-3'>
          <div className='space-y-0.5'>
            <h1 className='text-2xl font-semibold'>Alarm console</h1>
            <p className='text-muted-foreground text-sm'>
              {labels.tenant} · {labels.site} · {labels.meterType} ·{' '}
              {new Date().toLocaleString()}
            </p>
          </div>
          <AlarmCounts alarms={alarms} />
        </div>

        {isLoading || loadingReadings ? (
          <Card className='text-muted-foreground p-8 text-center text-sm'>
            Evaluating alarms…
          </Card>
        ) : (
          <AlarmTable alarms={alarms} />
        )}
      </div>
    </Main>
  )
}
