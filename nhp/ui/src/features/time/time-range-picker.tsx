/**
 * The dashboard time-range picker (docs/design/time-range-and-refresh.md §2): a
 * trigger showing the current range, opening a popover with quick ranges, an
 * absolute from/to calendar, and a relative-token input. Selecting any of them
 * sets the store range (which re-freezes `now` and bumps the refresh tick). The
 * refresh-interval dropdown + manual refresh sit alongside via `RefreshControl`.
 */
import { useState } from 'react'
import { CalendarClock } from 'lucide-react'
import { useTimeStore } from '@/stores/time-store'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover'
import { isValidToken } from './absolute'
import { rangeLabel } from './label'
import { QUICK_RANGES } from './presets'
import { RefreshControl } from './refresh-control'

export function TimeRangePicker() {
  const from = useTimeStore((s) => s.from)
  const to = useTimeStore((s) => s.to)
  const setRange = useTimeStore((s) => s.setRange)
  const [open, setOpen] = useState(false)

  return (
    <div className='flex items-center gap-1.5'>
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <Button size='sm' variant='outline' className='gap-1.5'>
            <CalendarClock className='size-3.5' />
            <span className='text-[12px]'>{rangeLabel(from, to)}</span>
          </Button>
        </PopoverTrigger>
        <PopoverContent align='start' className='w-[320px] p-3'>
          <PickerBody
            from={from}
            to={to}
            onApply={(f, t) => {
              setRange(f, t)
              setOpen(false)
            }}
          />
        </PopoverContent>
      </Popover>
      <RefreshControl />
    </div>
  )
}

function PickerBody({
  from,
  to,
  onApply,
}: {
  from: string
  to: string
  onApply: (from: string, to: string) => void
}) {
  const [fromInput, setFromInput] = useState(from)
  const [toInput, setToInput] = useState(to)
  const valid = isValidToken(fromInput) && isValidToken(toInput)

  return (
    <div className='space-y-3'>
      <div>
        <p className='eyebrow text-[10px] text-muted-foreground'>
          Quick ranges
        </p>
        <div className='mt-1.5 grid grid-cols-2 gap-1'>
          {QUICK_RANGES.map((q) => (
            <Button
              key={q.label}
              size='sm'
              variant='ghost'
              className='h-7 justify-start text-[12px]'
              onClick={() => onApply(q.from, q.to)}
            >
              {q.label}
            </Button>
          ))}
        </div>
      </div>

      <div className='border-t border-border pt-3'>
        <p className='eyebrow text-[10px] text-muted-foreground'>
          Custom range
        </p>
        <p className='mt-1 text-[10.5px] text-muted-foreground'>
          Relative (<span className='font-mono'>now-6h</span>,{' '}
          <span className='font-mono'>now/d</span>) or an absolute instant.
        </p>
        <div className='mt-2 space-y-2'>
          <Field label='From'>
            <Input
              className='h-8 text-[12px]'
              value={fromInput}
              onChange={(e) => setFromInput(e.target.value)}
              placeholder='now-6h'
            />
          </Field>
          <Field label='To'>
            <Input
              className='h-8 text-[12px]'
              value={toInput}
              onChange={(e) => setToInput(e.target.value)}
              placeholder='now'
            />
          </Field>
        </div>
        {!valid ? (
          <p className='mt-1.5 text-[11px] text-sev-fault'>
            One of the bounds is not a valid time token.
          </p>
        ) : null}
        <Button
          size='sm'
          className='mt-2 w-full'
          disabled={!valid}
          onClick={() => onApply(fromInput, toInput)}
        >
          Apply
        </Button>
      </div>
    </div>
  )
}

function Field({
  label,
  children,
}: {
  label: string
  children: React.ReactNode
}) {
  return (
    <div className='space-y-1'>
      <Label className='text-[11px] text-muted-foreground'>{label}</Label>
      {children}
    </div>
  )
}
