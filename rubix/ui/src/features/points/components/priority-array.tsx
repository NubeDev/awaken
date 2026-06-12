import { ListOrdered, X } from 'lucide-react'
import type { Point } from '@/api/types'
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { formatValue } from '@/lib/format'
import { SLOT_LABELS, winningSlotIndex } from '../lib/priority'
import { WriteForm } from './write-form'

type PriorityArrayCardProps = {
  point: Point
  onRelinquish: (priority: number) => void
  relinquishing: boolean
}

/**
 * The 16-level BACnet priority array as a two-column labelled grid. Lower
 * level wins; the winning slot is highlighted, filled slots can be
 * relinquished, and the Command form writes through the real API.
 */
export function PriorityArrayCard({ point, onRelinquish, relinquishing }: PriorityArrayCardProps) {
  const slots = point.priority_array.slots
  const winning = winningSlotIndex(point.priority_array)

  return (
    <Card>
      <CardHeader>
        <CardTitle className='flex items-center gap-2 text-[13.5px]'>
          <ListOrdered className='text-muted-foreground size-4' />
          Priority Array
        </CardTitle>
        <CardDescription className='text-[11.5px]'>
          BACnet 16-level command arbitration · level 1 wins, operator always beats agent
        </CardDescription>
        <CardAction>
          <WriteForm point={point} />
        </CardAction>
      </CardHeader>
      <CardContent>
        <div className='grid gap-x-6 gap-y-0.5 sm:grid-cols-2'>
          {slots.map((slot, i) => {
            const level = i + 1
            const filled = slot !== null
            const isWinning = i === winning
            return (
              <div
                key={level}
                className={cn(
                  'flex h-8 items-center gap-2.5 rounded-md px-2 text-[12px]',
                  isWinning && 'bg-primary/10 ring-primary/30 ring-1'
                )}
              >
                <span className='tabular text-muted-foreground w-5 text-end font-mono text-[11px] font-semibold'>
                  {level}
                </span>
                <span
                  className={cn(
                    'flex-1 truncate',
                    filled ? 'text-foreground font-medium' : 'text-muted-foreground/60'
                  )}
                >
                  {SLOT_LABELS[level] ?? '—'}
                </span>
                {isWinning ? (
                  <Badge variant='primary' className='h-4 px-1.5 text-[9px] uppercase'>
                    effective
                  </Badge>
                ) : null}
                <span
                  className={cn(
                    'tabular font-mono text-[11.5px]',
                    filled ? 'font-semibold' : 'text-muted-foreground/50'
                  )}
                >
                  {filled ? formatValue(slot, point.unit) : 'null'}
                </span>
                {filled ? (
                  <Button
                    variant='ghost'
                    size='icon-sm'
                    className='size-5'
                    disabled={relinquishing}
                    title={`Relinquish level ${level}`}
                    onClick={() => onRelinquish(level)}
                  >
                    <X className='size-3' />
                  </Button>
                ) : (
                  <span className='size-5' />
                )}
              </div>
            )
          })}
        </div>
        {point.priority_array.relinquish_default !== null ? (
          <div className='text-muted-foreground mt-2 flex items-center gap-2 border-t border-border px-2 pt-2 text-[11.5px]'>
            <span className='font-mono'>relinquish default</span>
            <span className='tabular ms-auto font-mono font-semibold'>
              {formatValue(point.priority_array.relinquish_default, point.unit)}
            </span>
          </div>
        ) : null}
      </CardContent>
    </Card>
  )
}
