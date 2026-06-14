/**
 * Auto-refresh interval dropdown + manual refresh button
 * (docs/design/time-range-and-refresh.md §2-3). The dropdown sets the store
 * `refresh`; the button bumps the store `tick` for an immediate coordinated
 * refetch across every widget at one frozen `now`.
 */
import { RefreshCw } from 'lucide-react'
import { useTimeStore } from '@/stores/time-store'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { isRefreshSecs, REFRESH_OPTIONS } from './presets'

export function RefreshControl() {
  const refresh = useTimeStore((s) => s.refresh)
  const setRefresh = useTimeStore((s) => s.setRefresh)
  const bumpTick = useTimeStore((s) => s.bumpTick)

  return (
    <div className='flex items-center gap-1.5'>
      <Button
        size='icon'
        variant='outline'
        className='size-8'
        title='Refresh now'
        aria-label='Refresh now'
        onClick={() => bumpTick()}
      >
        <RefreshCw className='size-3.5' />
      </Button>
      <Select
        value={String(refresh)}
        onValueChange={(v) => {
          const n = Number(v)
          if (isRefreshSecs(n)) setRefresh(n)
        }}
      >
        <SelectTrigger
          size='sm'
          className='w-20'
          aria-label='Auto-refresh interval'
        >
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {REFRESH_OPTIONS.map((o) => (
            <SelectItem key={o.secs} value={String(o.secs)}>
              {o.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  )
}
