import { useState } from 'react'
import { SquarePen } from 'lucide-react'
import { useWritePoint } from '@/api/hooks'
import type { Point, PointValue } from '@/api/types'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'

/** Coerce the text input to the wire value type. */
function coerce(raw: string): PointValue {
  if (raw === 'true' || raw === 'false') return raw === 'true'
  const n = Number(raw)
  return Number.isFinite(n) && raw.trim() !== '' ? n : raw
}

/**
 * Operator command into a priority slot, as a popover off a primary "Command"
 * button. Posts through `/points/{id}/write` at operator source; the array
 * refetches on success so the new effective value shows immediately.
 */
export function WriteForm({ point }: { point: Point }) {
  const write = useWritePoint()
  const [open, setOpen] = useState(false)
  const [value, setValue] = useState('')
  const [priority, setPriority] = useState('8')

  const submit = (e: React.FormEvent) => {
    e.preventDefault()
    if (value.trim() === '') return
    write.mutate(
      {
        id: point.id,
        body: { value: coerce(value), priority: Number(priority), source: 'operator' },
      },
      { onSuccess: () => setOpen(false) }
    )
    setValue('')
  }

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button size='sm'>
          <SquarePen className='size-3.5' /> Command
        </Button>
      </PopoverTrigger>
      <PopoverContent align='end' className='w-64'>
        <form onSubmit={submit} className='space-y-3'>
          <div className='space-y-1'>
            <Label htmlFor='wf-value' className='text-[11px]'>
              Value{point.unit ? ` (${point.unit})` : ''}
            </Label>
            <Input
              id='wf-value'
              value={value}
              onChange={(e) => setValue(e.target.value)}
              placeholder='command…'
              className='h-8'
              autoFocus
            />
          </div>
          <div className='space-y-1'>
            <Label htmlFor='wf-prio' className='text-[11px]'>
              Priority (1–16, operator default 8)
            </Label>
            <Input
              id='wf-prio'
              type='number'
              min={1}
              max={16}
              value={priority}
              onChange={(e) => setPriority(e.target.value)}
              className='h-8'
            />
          </div>
          <Button type='submit' size='sm' className='w-full' disabled={write.isPending}>
            Write at operator priority
          </Button>
        </form>
      </PopoverContent>
    </Popover>
  )
}
