/**
 * A headline KPI tile (the count/rollup family that complements the value-bearing
 * StatTile): a metric-accented card with an icon chip, a big value, an optional
 * subtext, and an optional capacity bar. Shared by the tenant/site/gateway pages
 * so every level's KPI strip reads identically. Pure renderer — no fetching.
 */
import type { LucideIcon } from 'lucide-react'
import { Card } from '@/components/ui/card'
import { cn } from '@/lib/utils'

export function KpiTile({
  icon: Icon,
  color,
  label,
  value,
  sub,
  alarm,
  bar,
}: {
  icon: LucideIcon
  /** Accent colour (a `--chart-N` var or a literal); tints the rule + icon chip. */
  color: string
  label: string
  value: string | number
  sub?: string
  /** Render the value in the destructive colour (e.g. active alarms > 0). */
  alarm?: boolean
  /** Optional capacity bar (e.g. devices used vs cap). */
  bar?: { value: number; max: number }
}) {
  const pct = bar && bar.max > 0 ? Math.min(100, Math.round((bar.value / bar.max) * 100)) : 0
  return (
    <Card className='relative gap-0 overflow-hidden p-4'>
      <span className='absolute inset-y-0 left-0 w-1' style={{ backgroundColor: color }} aria-hidden />
      <div className='flex items-start justify-between gap-2'>
        <div className='text-muted-foreground text-xs'>{label}</div>
        <span
          className='flex size-7 shrink-0 items-center justify-center rounded-md'
          style={{ backgroundColor: `color-mix(in oklab, ${color} 15%, transparent)`, color }}
        >
          <Icon className='size-4' />
        </span>
      </div>
      <div className={cn('mt-1 text-2xl font-semibold tabular-nums', alarm && 'text-destructive')}>
        {value}
      </div>
      {sub && <div className='text-muted-foreground mt-0.5 text-xs'>{sub}</div>}
      {bar && (
        <div className='bg-muted mt-2 h-1.5 overflow-hidden rounded-full'>
          <div className='h-full rounded-full' style={{ width: `${pct}%`, backgroundColor: color }} />
        </div>
      )}
    </Card>
  )
}
