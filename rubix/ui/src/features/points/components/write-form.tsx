import { useState } from 'react'
import { useWritePoint } from '@/api/hooks'
import type { Point, PointValue } from '@/api/types'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'

/** Coerce the text input to the wire value type the point expects. */
function coerce(point: Point, raw: string): PointValue {
  if (point.unit === '' || point.kind === 'sensor') return raw
  if (raw === 'true' || raw === 'false') return raw === 'true'
  const n = Number(raw)
  return Number.isFinite(n) && raw.trim() !== '' ? n : raw
}

/**
 * Operator write into a priority slot. Posts a real command through
 * `/points/{id}/write` at operator source; the priority array refetches on
 * success so the new winning value is reflected immediately.
 */
export function WriteForm({ point }: { point: Point }) {
  const write = useWritePoint()
  const [value, setValue] = useState('')
  const [priority, setPriority] = useState('8')

  const submit = (e: React.FormEvent) => {
    e.preventDefault()
    if (value.trim() === '') return
    write.mutate({
      id: point.id,
      body: { value: coerce(point, value), priority: Number(priority), source: 'operator' },
    })
    setValue('')
  }

  return (
    <form onSubmit={submit} className='flex items-end gap-2'>
      <div className='flex-1 space-y-1'>
        <Label htmlFor='wf-value' className='text-[11px]'>
          Value{point.unit ? ` (${point.unit})` : ''}
        </Label>
        <Input
          id='wf-value'
          value={value}
          onChange={(e) => setValue(e.target.value)}
          placeholder='command…'
          className='h-8'
        />
      </div>
      <div className='w-20 space-y-1'>
        <Label htmlFor='wf-prio' className='text-[11px]'>
          Priority
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
      <Button type='submit' size='sm' disabled={write.isPending}>
        Write
      </Button>
    </form>
  )
}
