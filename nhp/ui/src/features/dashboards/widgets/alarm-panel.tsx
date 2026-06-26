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
  /** The ramp limit the value crossed (e.g. `{ value: 100, direction: 'above' }`). */
  threshold?: { value: number; direction: 'above' | 'below' }
  /** ISO instant of the sample that tripped the alarm. */
  at?: string
}

const SEVERITY_LABELS: Record<Exclude<AlarmSeverity, 'ok'>, string> = {
  warning: 'Warning',
  critical: 'Critical',
}

function ago(at: string | undefined): string | null {
  if (!at) return null
  const ms = Date.now() - Date.parse(at)
  if (!Number.isFinite(ms)) return null
  const m = Math.round(ms / 60_000)
  if (m < 1) return 'just now'
  if (m < 60) return `${m}m ago`
  const h = Math.round(m / 60)
  if (h < 24) return `${h}h ago`
  return `${Math.round(h / 24)}d ago`
}

export function AlarmPanel({ alarms }: { alarms: AlarmRow[] }) {
  return (
    <Card className='p-4'>
      <div className='mb-2 text-sm font-medium'>Alarms</div>
      {alarms.length === 0 ? (
        <div className='text-muted-foreground text-sm'>No active alarms</div>
      ) : (
        <ul className='space-y-2'>
          {alarms.map((a) => {
            const age = ago(a.at)
            const limit = a.threshold
              ? `${a.threshold.direction === 'below' ? '≤' : '≥'} ${formatValue(a.threshold.value, { precision: a.precision, unit: a.unit })}`
              : null
            return (
              <li key={a.register} className='flex items-start justify-between gap-3 text-sm'>
                <span className='flex items-start gap-2'>
                  <span
                    className='mt-1.5 inline-block h-2 w-2 shrink-0 rounded-full'
                    style={{ backgroundColor: SEVERITY_COLORS[a.severity] }}
                  />
                  <span className='flex flex-col'>
                    <span className='font-medium'>{a.name}</span>
                    <span className='text-muted-foreground text-xs'>
                      <span style={{ color: SEVERITY_COLORS[a.severity] }}>
                        {SEVERITY_LABELS[a.severity]}
                      </span>
                      {limit && <> · threshold {limit}</>}
                      {age && <> · {age}</>}
                    </span>
                  </span>
                </span>
                <span
                  className='shrink-0 tabular-nums'
                  style={{ color: SEVERITY_COLORS[a.severity] }}
                >
                  {formatValue(a.value, { precision: a.precision, unit: a.unit })}
                </span>
              </li>
            )
          })}
        </ul>
      )}
    </Card>
  )
}
