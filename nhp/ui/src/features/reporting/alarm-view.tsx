/**
 * Shared presentation of a set of active alarms: severity-count chips + a sortable
 * table (severity, register, value, meter, site, when). Pure renderer of an
 * `ActiveAlarm[]` — both the live alarm console and the printable alarm-summary
 * report feed it, so "what alarms" looks identical live and on paper.
 */
import { Card } from '@/components/ui/card'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { SEVERITY_COLORS } from '@/features/dashboards/_shared/palette'
import { formatValue } from '@/features/dashboards/_shared/format-value'
import { severityCounts, type ActiveAlarm } from './alarms'

export function AlarmCounts({ alarms }: { alarms: ActiveAlarm[] }) {
  const { critical, warning } = severityCounts(alarms)
  return (
    <div className='flex flex-wrap gap-2'>
      <CountChip label='Critical' count={critical} severity='critical' />
      <CountChip label='Warning' count={warning} severity='warning' />
      <span className='text-muted-foreground self-center text-sm'>
        {alarms.length} active
      </span>
    </div>
  )
}

function CountChip({
  label,
  count,
  severity,
}: {
  label: string
  count: number
  severity: 'critical' | 'warning'
}) {
  const color = SEVERITY_COLORS[severity]
  return (
    <span
      className='inline-flex items-center gap-2 rounded-md border px-2.5 py-1 text-sm font-medium'
      style={{ borderColor: color, color }}
    >
      <span
        className='size-2 rounded-full'
        style={{ backgroundColor: color }}
      />
      {label}: {count}
    </span>
  )
}

export function AlarmTable({ alarms }: { alarms: ActiveAlarm[] }) {
  return (
    <Card className='overflow-hidden p-0'>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Severity</TableHead>
            <TableHead>Register</TableHead>
            <TableHead className='text-right'>Value</TableHead>
            <TableHead>Meter</TableHead>
            <TableHead>Site</TableHead>
            <TableHead>When</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {alarms.length === 0 ? (
            <TableRow>
              <TableCell colSpan={6} className='text-muted-foreground'>
                No active alarms in this scope.
              </TableCell>
            </TableRow>
          ) : (
            alarms.map((a) => (
              <TableRow key={a.registerId}>
                <TableCell>
                  <span
                    className='inline-flex items-center gap-2 font-medium capitalize'
                    style={{ color: SEVERITY_COLORS[a.severity] }}
                  >
                    <span
                      className='size-2 rounded-full'
                      style={{ backgroundColor: SEVERITY_COLORS[a.severity] }}
                    />
                    {a.severity}
                  </span>
                </TableCell>
                <TableCell>{a.registerName}</TableCell>
                <TableCell
                  className='text-right font-mono tabular-nums'
                  style={{ color: SEVERITY_COLORS[a.severity] }}
                >
                  {formatValue(a.value, { precision: a.precision, unit: a.unit })}
                </TableCell>
                <TableCell>{a.meterName}</TableCell>
                <TableCell>{a.siteName}</TableCell>
                <TableCell className='text-muted-foreground whitespace-nowrap text-xs'>
                  {new Date(a.at).toLocaleString()}
                </TableCell>
              </TableRow>
            ))
          )}
        </TableBody>
      </Table>
    </Card>
  )
}
