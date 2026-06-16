/**
 * Read-only online/offline badge for a gateway's poller-written `status`
 * (DOMAIN-MODEL "Status fields are poller-owned"). NHP only displays this — it is
 * never an input. `last_seen` is rendered alongside as a relative-ish label.
 */
import type { Status } from '@/enums/options'
import { Badge } from '@/components/ui/badge'

const TONE: Record<Status, string> = {
  online: 'text-emerald-600',
  offline: 'text-red-600',
  unknown: 'text-muted-foreground',
}

export function StatusBadge({
  status,
  lastSeen,
}: {
  status?: string
  lastSeen?: string
}) {
  const s = (status ?? 'unknown') as Status
  const tone = TONE[s] ?? TONE.unknown
  return (
    <span className='inline-flex items-center gap-2'>
      <Badge variant='outline' className={tone}>
        {s}
      </Badge>
      {lastSeen ? (
        <span className='text-muted-foreground text-xs'>
          {new Date(lastSeen).toLocaleString()}
        </span>
      ) : null}
    </span>
  )
}
