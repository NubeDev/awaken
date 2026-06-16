// The board time-range picker: a compact trigger showing the active range that
// opens a popover with quick ranges, a custom absolute from/to, and the bucket
// interval. Replaces the old inline `datetime-local` strip (BoardTimeRange) — same
// `BoardTimeRange` value in/out, nicer surface. The window math still flows through
// `board-params.ts`; this is purely the control.

import { useState } from 'react'
import { CalendarClock, ChevronDown } from 'lucide-react'
import { format } from 'date-fns'

import {
  type BoardTimeRange as Range,
  type IntervalUnit,
  QUICK_RANGES,
} from './board-params'
import { Button } from '../ui/button'
import { Input } from '../ui/input'
import { Label } from '../ui/label'
import { Popover, PopoverContent, PopoverTrigger } from '../ui/popover'

const INTERVALS: IntervalUnit[] = ['minute', 'hour', 'day', 'week']

interface TimeRangePickerProps {
  value: Range
  onChange: (range: Range) => void
}

// A quick range is "active" when the window's duration matches its span (within a
// minute) — quick ranges recompute `now`, so compare durations, not instants.
function activeQuick(value: Range): string | null {
  const span = value.end.getTime() - value.start.getTime()
  for (const r of QUICK_RANGES) {
    const { start, end } = r.compute()
    if (Math.abs(span - (end.getTime() - start.getTime())) < 60_000) return r.label
  }
  return null
}

function triggerLabel(value: Range): string {
  return activeQuick(value) ?? `${format(value.start, 'd MMM HH:mm')} → ${format(value.end, 'd MMM HH:mm')}`
}

export function TimeRangePicker({ value, onChange }: TimeRangePickerProps) {
  const [open, setOpen] = useState(false)
  const active = activeQuick(value)

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button variant="outline" size="sm" className="h-8 gap-1.5">
          <CalendarClock size={14} className="text-muted-foreground" />
          <span className="text-[12px] font-medium">{triggerLabel(value)}</span>
          <ChevronDown size={13} className="text-muted-foreground" />
        </Button>
      </PopoverTrigger>
      <PopoverContent align="start" className="w-[320px]">
        <div className="space-y-3">
          <div>
            <div className="mb-1.5 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
              Quick ranges
            </div>
            <div className="grid grid-cols-2 gap-1">
              {QUICK_RANGES.map((r) => {
                const sel = active === r.label
                return (
                  <button
                    key={r.label}
                    onClick={() => {
                      const { start, end } = r.compute()
                      onChange({ start, end, interval: r.interval })
                      setOpen(false)
                    }}
                    className={
                      'rounded-md px-2 py-1.5 text-left text-[12px] transition-colors ' +
                      (sel
                        ? 'bg-primary/15 text-foreground ring-1 ring-primary/40'
                        : 'text-muted-foreground hover:bg-accent hover:text-foreground')
                    }
                  >
                    {r.label}
                  </button>
                )
              })}
            </div>
          </div>

          <div className="space-y-2 border-t border-border pt-3">
            <div className="text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
              Custom range
            </div>
            <div className="space-y-1">
              <Label className="text-[11px] text-muted-foreground">From</Label>
              <Input
                type="datetime-local"
                className="h-8 text-[12px]"
                value={format(value.start, "yyyy-MM-dd'T'HH:mm")}
                // Radix menus run typeahead on keydown; stop it reaching the menu.
                onKeyDown={(e) => e.stopPropagation()}
                onChange={(e) => e.target.value && onChange({ ...value, start: new Date(e.target.value) })}
              />
            </div>
            <div className="space-y-1">
              <Label className="text-[11px] text-muted-foreground">To</Label>
              <Input
                type="datetime-local"
                className="h-8 text-[12px]"
                value={format(value.end, "yyyy-MM-dd'T'HH:mm")}
                onKeyDown={(e) => e.stopPropagation()}
                onChange={(e) => e.target.value && onChange({ ...value, end: new Date(e.target.value) })}
              />
            </div>
          </div>

          <div className="space-y-1.5 border-t border-border pt-3">
            <Label className="text-[11px] text-muted-foreground">Bucket interval</Label>
            <div className="grid grid-cols-4 gap-1">
              {INTERVALS.map((u) => {
                const sel = value.interval === u
                return (
                  <button
                    key={u}
                    onClick={() => onChange({ ...value, interval: u })}
                    className={
                      'rounded-md px-2 py-1.5 text-[11px] capitalize transition-colors ' +
                      (sel
                        ? 'bg-primary/15 text-foreground ring-1 ring-primary/40'
                        : 'text-muted-foreground hover:bg-accent hover:text-foreground')
                    }
                  >
                    {u}
                  </button>
                )
              })}
            </div>
          </div>
        </div>
      </PopoverContent>
    </Popover>
  )
}
