import { useState } from 'react'
import { ArrowLeft, Check, Copy } from 'lucide-react'
import { Link } from '@tanstack/react-router'
import { useScope } from '@/context/scope-provider'
import { useRun } from '@/api/hooks'
import { Main } from '@/components/layout/main'
import { PageHeader } from '@/components/layout/page-header'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { Skeleton } from '@/components/ui/skeleton'
import { relativeTime } from '@/lib/format'
import { RunStatusBadge } from './status-badge'
import { ORIGIN_META } from './origin'
import { ApprovalCard } from './components/approval-card'

/** One agent run: lifecycle, the assistant response, and — when suspended — the
 *  held write with its approve/reject controls. */
export function RunDetail({ runId }: { runId: string }) {
  const { org, site } = useScope()
  const { data: run, isLoading, isError } = useRun(runId)

  return (
    <>
      <PageHeader title='Agent Run' sub='awaken activity & approvals' />
      <Main fluid>
        {org && site ? (
          <Button asChild variant='ghost' size='sm' className='mb-3 -ms-2'>
            <Link
              to='/o/$org/s/$siteSlug/runs'
              params={{ org, siteSlug: site.slug }}
            >
              <ArrowLeft className='size-3.5' /> All runs
            </Link>
          </Button>
        ) : null}

        {isLoading ? (
          <Skeleton className='h-48 rounded-lg' />
        ) : isError || !run ? (
          <Card>
            <CardContent className='text-muted-foreground py-12 text-center text-sm'>
              Run not found.
            </CardContent>
          </Card>
        ) : (
          <div className='space-y-4'>
            <Card className='gap-3 p-5'>
              <div className='flex flex-wrap items-center gap-2'>
                <RunStatusBadge status={run.status} />
                {(() => {
                  const { label, icon: Icon } = ORIGIN_META[run.origin]
                  return (
                    <span className='text-muted-foreground inline-flex items-center gap-1 text-[11.5px]'>
                      <Icon className='size-3' /> {label}
                    </span>
                  )
                })()}
                <CopyId id={run.id} />
                <span className='text-muted-foreground ms-auto text-[11.5px]'>
                  updated {relativeTime(run.updated_at)}
                </span>
              </div>
              <p className='text-[13.5px] leading-relaxed whitespace-pre-wrap'>
                {run.response || 'No response recorded.'}
              </p>
              <dl className='text-muted-foreground grid grid-cols-2 gap-x-6 gap-y-1 text-[11.5px] sm:grid-cols-3'>
                <Meta label='Thread' value={run.thread_id} mono />
                <Meta label='Steps' value={String(run.steps)} />
                <Meta label='Started' value={relativeTime(run.created_at)} />
              </dl>
            </Card>

            {run.status === 'suspended' && run.pending_write ? (
              <ApprovalCard runId={run.id} write={run.pending_write} />
            ) : null}
          </div>
        )}
      </Main>
    </>
  )
}

/** The run id with a one-click copy — ids are long and operators paste them
 *  into `curl`/logs when chasing a run. */
function CopyId({ id }: { id: string }) {
  const [copied, setCopied] = useState(false)
  const copy = () => {
    navigator.clipboard?.writeText(id).then(() => {
      setCopied(true)
      setTimeout(() => setCopied(false), 1200)
    })
  }
  return (
    <Button
      variant='ghost'
      size='sm'
      onClick={copy}
      className='text-muted-foreground h-6 gap-1 px-1.5 font-mono text-[11px]'
      title='Copy run id'
    >
      {copied ? <Check className='size-3' /> : <Copy className='size-3' />}
      <span className='max-w-[14ch] truncate sm:max-w-[24ch]'>{id}</span>
    </Button>
  )
}

function Meta({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className='flex flex-col gap-0.5'>
      <dt className='eyebrow text-[9.5px]'>{label}</dt>
      <dd className={mono ? 'text-foreground truncate font-mono text-[11px]' : 'text-foreground'}>
        {value}
      </dd>
    </div>
  )
}
