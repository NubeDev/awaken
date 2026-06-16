/**
 * Single-value tile (DASHBOARDS-SCOPE §8 — "status/table are DOM, not charts";
 * stat is the same family). Shows the latest value of a single register (one
 * with no useful trend) or a rollup count, with a metric accent (metric-style.ts:
 * a left rule + icon coloured by quantity), an optional inline sparkline + delta
 * pill, and — when a value crosses its ramp — the alarm severity colour, which
 * always WINS over the metric accent. Pure renderer of a StatWidget. Empty value
 * → an em-dash, never a fabricated zero.
 */
import { TrendingDown, TrendingUp } from 'lucide-react'
import { Card } from '@/components/ui/card'
import { cn } from '@/lib/utils'
import { SEVERITY_COLORS } from '../_shared/palette'
import { metricStyle } from '../_shared/metric-style'
import { formatValue } from '../_shared/format-value'
import { severityFor } from '../_shared/field-config'
import { Sparkline } from '@/components/charts/sparkline'
import type { StatWidget } from './types'

export function StatTile({ widget }: { widget: StatWidget }) {
  const severity = widget.value !== null ? severityFor(widget.value, widget.alarm) : 'ok'
  const metric = metricStyle(widget.quantity)
  const inAlarm = severity !== 'ok'
  const accent = inAlarm ? SEVERITY_COLORS[severity] : metric.color
  const Icon = metric.icon
  const delta = widget.trend?.delta ?? null

  return (
    <Card className='relative gap-0 overflow-hidden p-4'>
      {/* Metric accent rule down the left edge. */}
      <span
        className='absolute inset-y-0 left-0 w-1'
        style={{ backgroundColor: accent }}
        aria-hidden
      />
      <div className='flex items-start justify-between gap-2'>
        <div className='text-muted-foreground truncate text-xs'>{widget.title}</div>
        <span
          className='flex size-7 shrink-0 items-center justify-center rounded-md'
          style={{ backgroundColor: `color-mix(in oklab, ${accent} 15%, transparent)`, color: accent }}
        >
          <Icon className='size-4' />
        </span>
      </div>
      <div className='mt-1 flex items-end justify-between gap-2'>
        <div
          className='text-2xl font-semibold tabular-nums'
          style={inAlarm ? { color: accent } : undefined}
        >
          {formatValue(widget.value, { precision: widget.precision, unit: widget.unit })}
        </div>
        {delta !== null && Number.isFinite(delta) && (
          <span
            className={cn(
              'inline-flex items-center gap-0.5 rounded-full px-1.5 py-0.5 text-xs font-medium tabular-nums',
              delta >= 0
                ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
                : 'bg-rose-500/10 text-rose-600 dark:text-rose-400'
            )}
          >
            {delta >= 0 ? <TrendingUp className='size-3' /> : <TrendingDown className='size-3' />}
            {Math.abs(delta * 100).toFixed(1)}%
          </span>
        )}
      </div>
      {widget.trend && widget.trend.points.length > 1 && (
        <div className='mt-2 -mb-1 -ml-1'>
          <Sparkline points={widget.trend.points} color={accent} />
        </div>
      )}
    </Card>
  )
}
