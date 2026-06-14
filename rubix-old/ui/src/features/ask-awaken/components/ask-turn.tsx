import { TriangleAlert } from 'lucide-react'
import type { ChatStatus } from '@/api/types'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

export type Turn =
  | { role: 'user'; text: string }
  | { role: 'agent'; text: string; runId?: string; status?: ChatStatus }
  | { role: 'error'; text: string }

/** One bubble in the Ask-awaken transcript. An awaiting-approval agent turn
 *  exposes a Review action that opens the suspended run. */
export function AskTurn({ turn, onReview }: { turn: Turn; onReview: (runId: string) => void }) {
  if (turn.role === 'user') {
    return (
      <div className='flex justify-end'>
        <div className='bg-primary text-primary-foreground max-w-[85%] rounded-lg rounded-ee-sm px-3 py-2 text-[13px]'>
          {turn.text}
        </div>
      </div>
    )
  }

  if (turn.role === 'error') {
    return (
      <div className='text-sev-fault flex items-center gap-2 text-[12px]'>
        <TriangleAlert className='size-3.5' /> {turn.text}
      </div>
    )
  }

  const reviewRunId = turn.status === 'awaiting_approval' ? turn.runId : undefined
  return (
    <div className='flex justify-start'>
      <div
        className={cn(
          'bg-muted max-w-[85%] space-y-2 rounded-lg rounded-es-sm px-3 py-2 text-[13px]',
          reviewRunId && 'ring-sev-warning/40 ring-1'
        )}
      >
        <p className='whitespace-pre-wrap'>{turn.text}</p>
        {reviewRunId ? (
          <Button size='sm' variant='outline' onClick={() => onReview(reviewRunId)}>
            Review &amp; approve
          </Button>
        ) : null}
      </div>
    </div>
  )
}
