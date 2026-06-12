import { Sparkles } from 'lucide-react'
import type { Spark } from '@/api/types'
import { SeverityIcon } from '@/components/severity-icon'
import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'
import { relativeTime } from '@/lib/format'

type SparkRowProps = {
  spark: Spark
  onClick?: () => void
  /** Render the rule as an `agent`-attributed finding. */
  agentAttributed?: boolean
}

/** One spark finding as a clickable row — shared by the dashboard and list. */
export function SparkRow({ spark, onClick, agentAttributed }: SparkRowProps) {
  return (
    <button
      onClick={onClick}
      className={cn(
        'hover:bg-accent flex w-full items-start gap-3 rounded-lg px-2.5 py-2.5 text-left transition-colors',
        spark.acknowledged && 'opacity-60'
      )}
    >
      <span className='mt-0.5'>
        <SeverityIcon severity={spark.severity} />
      </span>
      <div className='min-w-0 flex-1'>
        <div className='line-clamp-2 text-[12.5px] leading-snug font-medium'>{spark.message}</div>
        <div className='text-muted-foreground mt-1 flex items-center gap-2 text-[11px]'>
          <span className='font-mono font-medium'>{spark.rule}</span>
          <span>·</span>
          <span>{relativeTime(spark.ts)}</span>
          {agentAttributed ? (
            <Badge variant='secondary' className='h-4 gap-1 px-1.5 text-[9.5px]'>
              <Sparkles className='size-2.5' /> agent
            </Badge>
          ) : null}
          {spark.acknowledged ? (
            <Badge variant='outline' className='h-4 px-1.5 text-[9.5px]'>
              ack
            </Badge>
          ) : null}
        </div>
      </div>
    </button>
  )
}
