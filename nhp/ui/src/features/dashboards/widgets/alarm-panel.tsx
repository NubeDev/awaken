/**
 * Alarm panel (DASHBOARDS.md §Alarms; DOMAIN-MODEL §Alarms). Lists the registers
 * on a meter whose LATEST sampled value crosses a warning/critical threshold,
 * with the severity colour from the same ramp that colours the chart ("what you
 * see is what alarms"). Pure renderer.
 *
 * POC scope: the spec's rubix RULE engine that writes `insight`/alarm records on
 * a cross is not seeded, so the POC EVALUATES the ramp client-side over the latest
 * history value — the same severityFor() the charts use. Operator acknowledge (a
 * gate-audited write) is the spec's nice-to-have and is NOT built (it would need
 * a real alarm-record surface); noted in WS-07. Empty → an explicit "no alarms".
 */
import { Card } from '@/components/ui/card'
import { SEVERITY_COLORS } from '../_shared/palette'
import { formatValue } from '../_shared/format-value'
import type { AlarmSeverity } from '@/api/records'

export interface AlarmRow {
  register: string
  name: string
  value: number
  unit?: string
  precision?: number
  severity: Exclude<AlarmSeverity, 'ok'>
}

export function AlarmPanel({ alarms }: { alarms: AlarmRow[] }) {
  return (
    <Card className='p-4'>
      <div className='mb-2 text-sm font-medium'>Alarms</div>
      {alarms.length === 0 ? (
        <div className='text-muted-foreground text-sm'>No active alarms</div>
      ) : (
        <ul className='space-y-1.5'>
          {alarms.map((a) => (
            <li key={a.register} className='flex items-center justify-between text-sm'>
              <span className='flex items-center gap-2'>
                <span
                  className='inline-block h-2 w-2 rounded-full'
                  style={{ backgroundColor: SEVERITY_COLORS[a.severity] }}
                />
                {a.name}
              </span>
              <span className='tabular-nums' style={{ color: SEVERITY_COLORS[a.severity] }}>
                {formatValue(a.value, { precision: a.precision, unit: a.unit })}
              </span>
            </li>
          ))}
        </ul>
      )}
    </Card>
  )
}
