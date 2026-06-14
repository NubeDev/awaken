import { GitFork } from 'lucide-react'
import type { RuleView } from '@/api/types'

/**
 * The change-impact / blast-radius surface: the rules that compose this one via
 * `rule(name, …)`. Editing or deleting this rule changes them on the next tick,
 * so the list is shown prominently before a destructive or behaviour-changing
 * action — the design's safety feature. Rendered only when the list is non-empty.
 */
export function ChangeImpact({
  referencing,
  action,
}: {
  referencing: RuleView[]
  action: 'editing' | 'deleting'
}) {
  return (
    <div className='border-sev-warning/40 bg-sev-warning/10 rounded-md border p-3'>
      <div className='flex items-center gap-2'>
        <GitFork className='text-sev-warning size-4' />
        <p className='text-[12px] font-medium'>
          {referencing.length} rule{referencing.length === 1 ? '' : 's'} compose this
          one — {action} it changes them on the next tick.
        </p>
      </div>
      <div className='mt-2 flex flex-wrap gap-1.5'>
        {referencing.map((r) => (
          <span
            key={r.id}
            className='border-border bg-card text-muted-foreground rounded border px-1.5 py-0.5 font-mono text-[10.5px]'
          >
            {r.name}
          </span>
        ))}
      </div>
    </div>
  )
}
