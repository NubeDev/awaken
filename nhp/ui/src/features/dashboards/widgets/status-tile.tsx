/**
 * Online/offline status pill (DASHBOARDS.md §"Online / offline & stats"). Renders
 * a poller-written status — `online`/`offline`/`unknown`, plus the derived
 * `degraded` rollup (a site/tenant with any offline child) — as a coloured pill,
 * optionally with a `last_seen` relative age. READ-ONLY: NHP never writes status
 * (DOMAIN-MODEL "Status fields are poller-owned").
 */
import { STATUS_COLORS } from '../_shared/palette'

export type RollupStatus = 'online' | 'offline' | 'degraded' | 'unknown'

function ago(lastSeen: string | undefined): string | null {
  if (!lastSeen) return null
  const ms = Date.now() - Date.parse(lastSeen)
  if (!Number.isFinite(ms)) return null
  const m = Math.round(ms / 60_000)
  if (m < 1) return 'just now'
  if (m < 60) return `${m}m ago`
  const h = Math.round(m / 60)
  if (h < 24) return `${h}h ago`
  return `${Math.round(h / 24)}d ago`
}

export function StatusPill({
  status,
  lastSeen,
}: {
  status: RollupStatus
  lastSeen?: string
}) {
  const color = STATUS_COLORS[status] ?? STATUS_COLORS.unknown
  const age = ago(lastSeen)
  return (
    <span className='inline-flex items-center gap-1.5 text-xs'>
      <span className='inline-block h-2 w-2 rounded-full' style={{ backgroundColor: color }} />
      <span className='capitalize'>{status}</span>
      {age && <span className='text-muted-foreground'>· {age}</span>}
    </span>
  )
}
