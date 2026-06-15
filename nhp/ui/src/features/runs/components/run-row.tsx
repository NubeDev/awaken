import { Link } from '@tanstack/react-router'
import { ChevronRight } from 'lucide-react'
import { useScope } from '@/context/scope-provider'
import type { RunRecord } from '@/api/types'
import { relativeTime } from '@/lib/format'
import { RunStatusBadge } from '../status-badge'
import { ORIGIN_META } from '../origin'

const ROW_CLASS =
  'flex items-center gap-3 rounded-md px-2.5 py-3 transition-colors'

/** One row in the agent-runs list: links into the run detail by id. */
export function RunRow({ run }: { run: RunRecord }) {
  const { org, site } = useScope()
  const { label: originLabel, icon: OriginIcon } = ORIGIN_META[run.origin]

  const body = (
    <>
      <div className='min-w-0 flex-1'>
        <div className='truncate text-[13px] font-medium'>{run.response || run.id}</div>
        <div className='text-muted-foreground mt-0.5 flex items-center gap-2 text-[11px]'>
          <span className='inline-flex items-center gap-1'>
            <OriginIcon className='size-3' /> {originLabel}
          </span>
          <span aria-hidden>·</span>
          <span>
            {run.steps} step{run.steps === 1 ? '' : 's'}
          </span>
          <span aria-hidden>·</span>
          <span className='truncate font-mono'>{run.id}</span>
        </div>
      </div>
      <span className='text-muted-foreground text-[11px] whitespace-nowrap'>
        {relativeTime(run.created_at)}
      </span>
      <RunStatusBadge status={run.status} className='h-5 px-2 text-[10.5px]' />
      <ChevronRight className='text-muted-foreground size-4 shrink-0' />
    </>
  )

  // Scope can lag the run list on first paint (sites still resolving). Only link
  // once org + site are known so we never dereference an undefined site; until
  // then the row renders as static content rather than crashing.
  return (
    <li>
      {org && site ? (
        <Link
          to='/o/$org/s/$siteSlug/runs/$runId'
          params={{ org, siteSlug: site.slug, runId: run.id }}
          className={`hover:bg-muted/40 ${ROW_CLASS}`}
        >
          {body}
        </Link>
      ) : (
        <div className={ROW_CLASS}>{body}</div>
      )}
    </li>
  )
}
