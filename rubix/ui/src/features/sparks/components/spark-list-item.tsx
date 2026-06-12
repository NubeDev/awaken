import { Check, Sparkles } from 'lucide-react'
import type { Spark } from '@/api/types'
import { SeverityIcon } from '@/components/severity-icon'
import { Badge } from '@/components/ui/badge'
import { relativeTime } from '@/lib/format'
import { cn } from '@/lib/utils'

type SparkListItemProps = {
  spark: Spark
  equipName?: string
  active: boolean
  agentAttributed?: boolean
  onClick: () => void
}

/** One finding in the master list — rule, message, equip, freshness. */
export function SparkListItem({ spark, equipName, active, agentAttributed, onClick }: SparkListItemProps) {
  return (
    <button
      onClick={onClick}
      className={cn(
        'w-full rounded-lg border border-transparent px-3 py-2.5 text-left transition-colors',
        active ? 'bg-accent border-border' : 'hover:bg-accent/60',
        spark.acknowledged && 'opacity-55'
      )}
    >
      <div className='flex items-center gap-2'>
        <SeverityIcon severity={spark.severity} className='size-3.5 shrink-0' />
        <span className='text-muted-foreground truncate font-mono text-[10.5px]'>{spark.rule}</span>
        {spark.acknowledged ? <Check className='text-positive ms-auto size-3 shrink-0' /> : null}
      </div>
      <div className='mt-1 line-clamp-2 text-[12.5px] leading-snug font-medium'>{spark.message}</div>
      <div className='text-muted-foreground mt-1.5 flex items-center gap-1.5 text-[10.5px]'>
        {equipName ? <span className='font-medium'>{equipName}</span> : null}
        {equipName ? <span>·</span> : null}
        <span>{relativeTime(spark.ts)}</span>
        {agentAttributed ? (
          <Badge variant='primary' className='ms-1 h-4 gap-1 px-1.5 text-[9px]'>
            <Sparkles className='size-2.5' /> agent
          </Badge>
        ) : null}
      </div>
    </button>
  )
}
