/**
 * Single-value tile (DASHBOARDS-SCOPE §8 — "status/table are DOM, not charts";
 * stat is the same family). Shows the latest value of a single register (one
 * with no useful trend) or a rollup count, coloured by its alarm severity when
 * a ramp applies. Pure renderer of a StatWidget. Empty value → an em-dash, never
 * a fabricated zero.
 */
import { Card } from '@/components/ui/card'
import { SEVERITY_COLORS } from '../_shared/palette'
import { formatValue } from '../_shared/format-value'
import { severityFor } from '../_shared/field-config'
import type { StatWidget } from './types'

export function StatTile({ widget }: { widget: StatWidget }) {
  const severity = widget.value !== null ? severityFor(widget.value, widget.alarm) : 'ok'
  const color = severity === 'ok' ? undefined : SEVERITY_COLORS[severity]
  return (
    <Card className='p-4'>
      <div className='text-muted-foreground truncate text-xs'>{widget.title}</div>
      <div className='mt-1 text-2xl font-semibold tabular-nums' style={color ? { color } : undefined}>
        {formatValue(widget.value, { precision: widget.precision, unit: widget.unit })}
      </div>
    </Card>
  )
}
