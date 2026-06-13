/**
 * A vertical timeline of change rows (docs/design/audit-and-undo.md "UI"): each
 * entry shows who/what/when (actor, op, time) with an expandable before→after
 * diff. Shared by the per-resource History tab and the admin Audit screen so both
 * render the ledger identically. The rows arrive newest-first from the server.
 */
import { useState } from 'react'
import { ChevronDown, ChevronRight } from 'lucide-react'
import type { Change, Op } from '@/api/types'
import { useDateTime } from '@/datetime/use-date-time'
import { Badge } from '@/components/ui/badge'
import { Card } from '@/components/ui/card'
import { actorKind, actorLabel } from '../actor'
import { ChangeDiff } from './change-diff'

const OP_VARIANT: Record<Op, 'positive' | 'warning' | 'fault'> = {
  create: 'positive',
  update: 'warning',
  delete: 'fault',
}

const ACTOR_VARIANT = {
  user: 'muted',
  agent: 'info',
  system: 'secondary',
} as const

export function ChangeTimeline({
  changes,
  showKind = false,
  emptyLabel = 'No history yet.',
}: {
  changes: Change[]
  /** Show the resource kind per row (the admin screen spans many kinds). */
  showKind?: boolean
  emptyLabel?: string
}) {
  if (changes.length === 0) {
    return (
      <Card className='grid h-40 place-items-center'>
        <p className='text-sm text-muted-foreground'>{emptyLabel}</p>
      </Card>
    )
  }

  return (
    <div className='space-y-1.5'>
      {changes.map((c) => (
        <ChangeRow key={c.id} change={c} showKind={showKind} />
      ))}
    </div>
  )
}

function ChangeRow({ change, showKind }: { change: Change; showKind: boolean }) {
  const { dateTime } = useDateTime()
  const [open, setOpen] = useState(false)

  return (
    <div className='rounded-md bg-muted/40 px-3 py-2'>
      <button
        type='button'
        className='flex w-full items-center gap-2 text-left'
        onClick={() => setOpen((o) => !o)}
      >
        {open ? (
          <ChevronDown className='size-3.5 shrink-0 text-muted-foreground' />
        ) : (
          <ChevronRight className='size-3.5 shrink-0 text-muted-foreground' />
        )}
        <Badge
          variant={OP_VARIANT[change.op]}
          className='h-4 px-1.5 text-[10px] uppercase'
        >
          {change.op}
        </Badge>
        {showKind ? (
          <code className='text-[11px] text-muted-foreground'>{change.kind}</code>
        ) : null}
        <span className='truncate text-[12px]'>
          {actorLabel(change.actor)}
        </span>
        <Badge
          variant={ACTOR_VARIANT[actorKind(change.actor)]}
          className='h-4 px-1.5 text-[10px]'
        >
          {change.actor.kind}
        </Badge>
        <span className='ms-auto shrink-0 text-[11px] text-muted-foreground'>
          {dateTime(change.at)}
        </span>
      </button>
      {open ? (
        <div className='mt-2 ps-5'>
          <ChangeDiff change={change} />
        </div>
      ) : null}
    </div>
  )
}
