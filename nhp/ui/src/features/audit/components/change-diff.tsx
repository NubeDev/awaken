/**
 * Before→after diff of one change row (docs/design/audit-and-undo.md). Renders the
 * field-level diff from `diffSnapshots` as a compact two-column before/after list;
 * a create shows additions, a delete removals, an update only the changed fields.
 */
import type { Change } from '@/api/types'
import { diffSnapshots, formatDiffValue, type DiffStatus } from '../diff'

const STATUS_STYLE: Record<DiffStatus, string> = {
  added: 'text-positive',
  removed: 'text-sev-fault',
  changed: 'text-sev-warning',
}

export function ChangeDiff({ change }: { change: Change }) {
  const diff = diffSnapshots(change.before, change.after)

  if (diff.length === 0) {
    return (
      <p className='text-[12px] text-muted-foreground'>No field changes.</p>
    )
  }

  return (
    <div className='space-y-1'>
      {diff.map((d) => (
        <div
          key={d.field}
          className='grid grid-cols-[8rem_1fr_1fr] items-start gap-2 text-[12px]'
        >
          <code className={`truncate ${STATUS_STYLE[d.status]}`}>{d.field}</code>
          <span className='break-all text-muted-foreground line-through'>
            {d.status === 'added' ? '' : formatDiffValue(d.before)}
          </span>
          <span className='break-all'>
            {d.status === 'removed' ? '' : formatDiffValue(d.after)}
          </span>
        </div>
      ))}
    </div>
  )
}
