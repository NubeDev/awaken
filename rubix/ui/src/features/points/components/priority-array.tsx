import { X } from 'lucide-react'
import type { Point } from '@/api/types'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { formatValue } from '@/lib/format'
import { winningSlotIndex } from '../lib/priority'

type PriorityArrayProps = {
  point: Point
  onRelinquish?: (priority: number) => void
  relinquishing?: boolean
}

/**
 * The 16-level BACnet priority array. Lower level wins; the first non-null slot
 * is the effective command. Operators can relinquish a slot to fall through to
 * the next. Read straight from `point.priority_array` — no derived state.
 */
export function PriorityArray({ point, onRelinquish, relinquishing }: PriorityArrayProps) {
  const slots = point.priority_array.slots
  const winningIndex = winningSlotIndex(point.priority_array)

  return (
    <div className='space-y-1'>
      {slots.map((slot, i) => {
        const level = i + 1
        const winning = i === winningIndex
        const filled = slot !== null
        return (
          <div
            key={level}
            className={cn(
              'flex items-center gap-3 rounded-md px-2.5 py-1.5 text-[12.5px]',
              winning && 'bg-primary/10',
              !filled && 'opacity-50'
            )}
          >
            <span className='text-muted-foreground tabular w-6 font-mono text-[11px]'>
              {String(level).padStart(2, '0')}
            </span>
            <span className='tabular flex-1 font-medium'>
              {filled ? formatValue(slot, point.unit) : '—'}
            </span>
            {winning && (
              <Badge variant='primary' className='h-4 px-1.5 text-[9.5px]'>
                active
              </Badge>
            )}
            {filled && onRelinquish && (
              <Button
                variant='ghost'
                size='icon-sm'
                className='size-6'
                disabled={relinquishing}
                title={`Relinquish level ${level}`}
                onClick={() => onRelinquish(level)}
              >
                <X className='size-3.5' />
              </Button>
            )}
          </div>
        )
      })}
      {point.priority_array.relinquish_default !== null && (
        <div className='text-muted-foreground flex items-center gap-3 rounded-md px-2.5 py-1.5 text-[12.5px]'>
          <span className='w-6 font-mono text-[11px]'>def</span>
          <span className='tabular flex-1'>
            {formatValue(point.priority_array.relinquish_default, point.unit)}
          </span>
          <Badge variant='outline' className='h-4 px-1.5 text-[9.5px]'>
            fallback
          </Badge>
        </div>
      )}
    </div>
  )
}
