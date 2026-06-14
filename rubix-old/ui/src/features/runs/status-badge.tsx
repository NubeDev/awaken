import type { RunStatus } from '@/api/types'
import { Badge } from '@/components/ui/badge'

/**
 * Run lifecycle → badge variant. `suspended` is the awaiting-approval state and
 * reads as a warning so the approval queue stands out; the rest are terminal.
 */
const STATUS_VARIANT = {
  suspended: 'warning',
  resumed: 'positive',
  completed: 'muted',
  cancelled: 'muted',
} as const satisfies Record<RunStatus, 'warning' | 'positive' | 'muted'>

const STATUS_LABEL = {
  suspended: 'Awaiting approval',
  resumed: 'Resumed',
  completed: 'Completed',
  cancelled: 'Cancelled',
} as const satisfies Record<RunStatus, string>

export function RunStatusBadge({ status, className }: { status: RunStatus; className?: string }) {
  return (
    <Badge variant={STATUS_VARIANT[status]} className={className}>
      {STATUS_LABEL[status]}
    </Badge>
  )
}
