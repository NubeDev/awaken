import type { RunOrigin, RunStatus } from '@/api/types'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { ORIGIN_META } from '../origin'

/** `undefined` = no filter (all). The status filter is server-side. */
export type StatusFilter = RunStatus | undefined
export type OriginFilter = RunOrigin | undefined

const STATUS_TABS: { value: StatusFilter; label: string }[] = [
  { value: undefined, label: 'All' },
  { value: 'suspended', label: 'Awaiting approval' },
  { value: 'completed', label: 'Completed' },
  { value: 'resumed', label: 'Resumed' },
  { value: 'cancelled', label: 'Cancelled' },
]

const ORIGINS: RunOrigin[] = ['chat', 'dispatch', 'mcp']

type RunsToolbarProps = {
  status: StatusFilter
  origin: OriginFilter
  onStatus: (s: StatusFilter) => void
  onOrigin: (o: OriginFilter) => void
}

/** Status (server-side) + origin (client-side) filters for the runs list. */
export function RunsToolbar({ status, origin, onStatus, onOrigin }: RunsToolbarProps) {
  return (
    <div className='flex flex-wrap items-center gap-x-4 gap-y-2'>
      <div className='bg-muted text-muted-foreground inline-flex items-center rounded-lg p-0.75'>
        {STATUS_TABS.map((t) => (
          <button
            key={t.label}
            type='button'
            onClick={() => onStatus(t.value)}
            className={cn(
              'rounded-md px-2.5 py-1 text-[12px] font-medium whitespace-nowrap transition-colors',
              status === t.value
                ? 'bg-background text-foreground shadow-sm'
                : 'hover:text-foreground'
            )}
          >
            {t.label}
          </button>
        ))}
      </div>

      <div className='flex items-center gap-1'>
        <span className='eyebrow text-[9.5px]'>Origin</span>
        <Chip active={origin === undefined} onClick={() => onOrigin(undefined)}>
          Any
        </Chip>
        {ORIGINS.map((o) => {
          const { label, icon: Icon } = ORIGIN_META[o]
          return (
            <Chip
              key={o}
              active={origin === o}
              onClick={() => onOrigin(origin === o ? undefined : o)}
            >
              <Icon className='size-3' /> {label}
            </Chip>
          )
        })}
      </div>
    </div>
  )
}

function Chip({
  active,
  onClick,
  children,
}: {
  active: boolean
  onClick: () => void
  children: React.ReactNode
}) {
  return (
    <Button
      type='button'
      size='sm'
      variant={active ? 'secondary' : 'ghost'}
      onClick={onClick}
      className='h-6 gap-1 px-2 text-[11px]'
    >
      {children}
    </Button>
  )
}
