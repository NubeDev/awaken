// Board time-range control (§3) — quick ranges + a bucket-interval select. Emits
// a BoardTimeRange the page formats into query params for every panel. The
// custom datetime inputs let you pin an exact window; the quick buttons cover
// the common cases. Faithful to Laminar's dashboard time picker, trimmed to the
// primitives Rubix already ships (no calendar/popover dependency).

import { Clock } from 'lucide-react'
import { format } from 'date-fns'

import {
  type BoardTimeRange as Range,
  type IntervalUnit,
  QUICK_RANGES,
} from './board-params'
import { Input } from '../ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../ui/select'

const INTERVALS: IntervalUnit[] = ['minute', 'hour', 'day', 'week']

interface BoardTimeRangeProps {
  value: Range
  onChange: (range: Range) => void
}

export function BoardTimeRange({ value, onChange }: BoardTimeRangeProps) {
  return (
    <div className="flex flex-wrap items-center gap-2">
      <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
        <Clock size={14} /> Range
      </span>
      <div className="flex items-center gap-1 rounded-lg border border-border p-0.5">
        {QUICK_RANGES.map((r) => {
          const { start, end } = r.compute()
          // Active when the span roughly matches (within a minute) — quick
          // ranges recompute "now", so compare durations, not exact instants.
          const active =
            Math.abs(value.end.getTime() - value.start.getTime() - (end.getTime() - start.getTime())) < 60_000
          return (
            <button
              key={r.label}
              onClick={() => onChange({ start, end, interval: r.interval })}
              className={
                'rounded-md px-2 py-1 text-xs ' +
                (active ? 'bg-accent text-accent-foreground' : 'text-muted-foreground hover:bg-muted')
              }
            >
              {r.label}
            </button>
          )
        })}
      </div>

      <Input
        type="datetime-local"
        className="h-8 w-[200px]"
        value={format(value.start, "yyyy-MM-dd'T'HH:mm")}
        onChange={(e) => e.target.value && onChange({ ...value, start: new Date(e.target.value) })}
      />
      <span className="text-xs text-muted-foreground">→</span>
      <Input
        type="datetime-local"
        className="h-8 w-[200px]"
        value={format(value.end, "yyyy-MM-dd'T'HH:mm")}
        onChange={(e) => e.target.value && onChange({ ...value, end: new Date(e.target.value) })}
      />

      <Select value={value.interval} onValueChange={(v) => onChange({ ...value, interval: v as IntervalUnit })}>
        <SelectTrigger className="h-8 w-[120px]">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {INTERVALS.map((u) => (
            <SelectItem key={u} value={u}>
              {u}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  )
}
