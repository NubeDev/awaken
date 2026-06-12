import { type LucideIcon, TrendingDown, TrendingUp } from 'lucide-react'
import { Card } from '@/components/ui/card'
import { cn } from '@/lib/utils'

type KpiCardProps = {
  label: string
  value: string
  unit?: string
  icon?: LucideIcon
  delta?: string
  deltaDir?: 'up' | 'down'
  sub?: string
}

/** Compact KPI stat card used across the dashboard header row. */
export function KpiCard({ label, value, unit, icon: Icon, delta, deltaDir, sub }: KpiCardProps) {
  const up = deltaDir === 'up'
  return (
    <Card className='gap-3 p-4'>
      <div className='flex items-center justify-between gap-2'>
        <span className='text-muted-foreground flex items-center gap-1.5 text-[12.5px] font-medium'>
          {Icon ? <Icon className='size-3.5' /> : null}
          {label}
        </span>
        {delta ? (
          <span
            className={cn(
              'flex items-center gap-0.5 text-xs font-semibold',
              deltaDir ? (up ? 'text-positive' : 'text-sev-fault') : 'text-muted-foreground'
            )}
          >
            {up ? <TrendingUp className='size-3' /> : <TrendingDown className='size-3' />}
            {delta}
          </span>
        ) : null}
      </div>
      <div className='flex items-baseline gap-1'>
        <span className='tabular text-2xl leading-none font-semibold tracking-tight'>{value}</span>
        {unit ? <span className='text-muted-foreground text-[13px] font-medium'>{unit}</span> : null}
      </div>
      {sub ? <div className='text-muted-foreground text-[11.5px]'>{sub}</div> : null}
    </Card>
  )
}
