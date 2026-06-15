import { Eye, SlidersHorizontal, SquarePen } from 'lucide-react'
import type { PointKind } from '@/api/types'
import { cn } from '@/lib/utils'

const ICONS = { sensor: Eye, cmd: SquarePen, sp: SlidersHorizontal } as const

/** Round point-kind glyph with an optional in-finding alarm dot. */
export function PointKindIcon({
  kind,
  inFinding,
  size = 'md',
}: {
  kind: PointKind
  inFinding?: boolean
  size?: 'md' | 'lg'
}) {
  const Icon = ICONS[kind]
  return (
    <span
      className={cn(
        'bg-accent text-muted-foreground relative grid shrink-0 place-items-center rounded-full',
        size === 'lg' ? 'size-10' : 'size-8'
      )}
    >
      <Icon className={size === 'lg' ? 'size-[18px]' : 'size-3.5'} />
      {inFinding ? (
        <span className='bg-sev-fault ring-card absolute -top-0.5 -right-0.5 size-2.5 rounded-full ring-2' />
      ) : null}
    </span>
  )
}
