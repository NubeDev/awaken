import { Link } from '@tanstack/react-router'
import { ChevronRight } from 'lucide-react'
import type { RunRecord } from '@/api/types'
import { relativeTime } from '@/lib/format'
import { RunStatusBadge } from '../status-badge'

const ORIGIN_LABEL: Record<RunRecord['origin'], string> = {
  chat: 'Chat',
  dispatch: 'Dispatch',
  mcp: 'MCP',
}

/** One row in the agent-runs list: links into the run detail by id. */
export function RunRow({ run }: { run: RunRecord }) {
  return (
    <li>
      <Link
        to='/runs/$runId'
        params={{ runId: run.id }}
        className='hover:bg-muted/40 flex items-center gap-3 rounded-md px-2.5 py-3 transition-colors'
      >
        <div className='min-w-0 flex-1'>
          <div className='truncate text-[13px] font-medium'>{run.response || run.id}</div>
          <div className='text-muted-foreground mt-0.5 flex items-center gap-2 text-[11px]'>
            <span>{ORIGIN_LABEL[run.origin]}</span>
            <span aria-hidden>·</span>
            <span className='font-mono'>{run.id}</span>
          </div>
        </div>
        <span className='text-muted-foreground text-[11px] whitespace-nowrap'>
          {relativeTime(run.created_at)}
        </span>
        <RunStatusBadge status={run.status} className='h-5 px-2 text-[10.5px]' />
        <ChevronRight className='text-muted-foreground size-4 shrink-0' />
      </Link>
    </li>
  )
}
