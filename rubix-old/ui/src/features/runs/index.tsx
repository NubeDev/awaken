import { useMemo, useState } from 'react'
import { Sparkles } from 'lucide-react'
import { useAgentStatus, useRuns } from '@/api/hooks'
import type { RunRecord } from '@/api/types'
import { useAskAwaken } from '@/features/ask-awaken/use-ask-awaken'
import { Main } from '@/components/layout/main'
import { PageHeader } from '@/components/layout/page-header'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { Skeleton } from '@/components/ui/skeleton'
import { AgentStatusCard } from './components/agent-status-card'
import { RunComposer } from './components/run-composer'
import { RunRow } from './components/run-row'
import { RunsToolbar, type OriginFilter, type StatusFilter } from './components/runs-toolbar'

/** awaken agent run records, read live from `/api/v1/runs`. */
export function Runs() {
  const [status, setStatus] = useState<StatusFilter>(undefined)
  const [origin, setOrigin] = useState<OriginFilter>(undefined)
  const { data: runs = [], isLoading } = useRuns(status)
  const { data: agent } = useAgentStatus()

  // Origin is filtered client-side (no server param); status is server-side.
  const visible = useMemo(
    () => (origin ? runs.filter((r) => r.origin === origin) : runs),
    [runs, origin]
  )

  // Suspended runs are the only actionable state — pin them above the rest, but
  // only when the operator hasn't already narrowed the status filter.
  const { pending, settled } = useMemo(() => splitByApproval(visible, status), [visible, status])

  return (
    <>
      <PageHeader title='Agent Runs' sub='awaken activity & approvals' />
      <Main fluid>
        <div className='mb-4 space-y-3'>
          {agent ? <AgentStatusCard status={agent} /> : null}
          {agent?.enabled === false ? null : <RunComposer />}
          <RunsToolbar
            status={status}
            origin={origin}
            onStatus={setStatus}
            onOrigin={setOrigin}
          />
        </div>

        {isLoading ? (
          <Card>
            <CardContent className='space-y-2 p-2'>
              {Array.from({ length: 4 }).map((_, i) => (
                <Skeleton key={i} className='h-12 rounded-lg' />
              ))}
            </CardContent>
          </Card>
        ) : visible.length === 0 ? (
          <RunsEmpty
            filtered={Boolean(status || origin)}
            agentEnabled={agent?.enabled !== false}
          />
        ) : (
          <div className='space-y-4'>
            {pending.length > 0 ? (
              <Section
                label='Awaiting approval'
                count={pending.length}
                accent
                runs={pending}
              />
            ) : null}
            {settled.length > 0 ? (
              <Section
                label={pending.length > 0 ? 'Activity' : undefined}
                count={settled.length}
                runs={settled}
              />
            ) : null}
          </div>
        )}
      </Main>
    </>
  )
}

/** Pull suspended runs to the front unless the operator already filtered status. */
function splitByApproval(runs: RunRecord[], status: StatusFilter) {
  if (status) return { pending: [] as RunRecord[], settled: runs }
  const pending: RunRecord[] = []
  const settled: RunRecord[] = []
  for (const r of runs) (r.status === 'suspended' ? pending : settled).push(r)
  return { pending, settled }
}

function Section({
  label,
  count,
  runs,
  accent,
}: {
  label?: string
  count: number
  runs: RunRecord[]
  accent?: boolean
}) {
  return (
    <section className='space-y-1.5'>
      {label ? (
        <h2 className='flex items-center gap-2 px-1'>
          <span className={`eyebrow text-[9.5px] ${accent ? 'text-sev-warning' : ''}`}>
            {label}
          </span>
          <span className='text-muted-foreground text-[10px]'>{count}</span>
        </h2>
      ) : null}
      <Card className={accent ? 'border-sev-warning/40' : undefined}>
        <CardContent className='p-2'>
          <ul className='divide-border divide-y'>
            {runs.map((r) => (
              <RunRow key={r.id} run={r} />
            ))}
          </ul>
        </CardContent>
      </Card>
    </section>
  )
}

/** No runs to show. When unfiltered this is the genuine zero-state, with a call
 *  to action; when filtered it's a softer "nothing matches". With the agent
 *  off, the CTA is dropped — the disabled banner above already explains why. */
function RunsEmpty({ filtered, agentEnabled }: { filtered: boolean; agentEnabled: boolean }) {
  const { setOpen } = useAskAwaken()
  if (filtered) {
    return (
      <Card>
        <CardContent className='text-muted-foreground py-12 text-center text-sm'>
          No runs match this filter.
        </CardContent>
      </Card>
    )
  }
  return (
    <Card>
      <CardContent className='flex flex-col items-center gap-3 py-14 text-center'>
        <div className='bg-primary/10 text-primary flex size-11 items-center justify-center rounded-full'>
          <Sparkles className='size-5' />
        </div>
        <div className='space-y-1'>
          <p className='text-[14px] font-medium'>No agent runs yet</p>
          <p className='text-muted-foreground mx-auto max-w-sm text-[12.5px] leading-relaxed'>
            Ask awaken to investigate a finding, query data, or command a write — runs
            and their approvals show up here.
          </p>
        </div>
        {agentEnabled ? (
          <Button size='sm' onClick={() => setOpen(true)}>
            <Sparkles className='size-3.5' /> Ask awaken
          </Button>
        ) : null}
      </CardContent>
    </Card>
  )
}
