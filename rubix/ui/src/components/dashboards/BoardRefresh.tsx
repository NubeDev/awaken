// Board refresh control (§6) — Off · 5s · 10s · 30s · 1m · 5m. Sits beside the
// time range; drives TanStack's `refetchInterval` on the board batch query. The
// 5s floor matches the backend scoped-context cache TTL (§4a) — polling faster
// turns over no new data. A spinning icon while a refetch is in flight gives the
// board a heartbeat without flashing each panel (panels use keepPreviousData).

import { RefreshCw } from 'lucide-react'

import { REFRESH_OPTIONS, type RefreshInterval } from './board-refresh'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../ui/select'

interface BoardRefreshProps {
  value: RefreshInterval
  onChange: (interval: RefreshInterval) => void
  /** True while a refetch is in flight — spins the icon. */
  refreshing?: boolean
}

// Encode the nullable interval as a string for the Select. Radix forbids an
// empty-string SelectItem value, so "Off" uses an explicit sentinel.
const OFF = 'off'
function encode(v: RefreshInterval): string {
  return v === null ? OFF : String(v)
}
function decode(v: string): RefreshInterval {
  return v === OFF ? null : Number(v)
}

export function BoardRefresh({ value, onChange, refreshing }: BoardRefreshProps) {
  return (
    <div className="flex items-center gap-1.5">
      <RefreshCw
        size={14}
        className={
          'text-muted-foreground ' + (refreshing ? 'animate-spin' : '')
        }
      />
      <Select value={encode(value)} onValueChange={(v) => onChange(decode(v))}>
        <SelectTrigger className="h-8 w-[88px]">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {REFRESH_OPTIONS.map((o) => (
            <SelectItem key={o.label} value={encode(o.value)}>
              {o.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  )
}
