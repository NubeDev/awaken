import { Check, X } from 'lucide-react'
import { useNavigate } from '@tanstack/react-router'
import { toast } from 'sonner'
import { useScope } from '@/context/scope-provider'
import { useCancelRun, useResumeRun } from '@/api/hooks'
import type { PendingWrite } from '@/api/types'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { formatValue } from '@/lib/format'

type ApprovalCardProps = {
  runId: string
  write: PendingWrite
}

/**
 * The held `write_point` command a suspended run awaits approval on. Approve
 * re-applies it through the priority array (the agent's write lands at its
 * slot); reject discards it. Both re-check the escalation floor server-side, so
 * a server error here is surfaced rather than retried.
 */
export function ApprovalCard({ runId, write }: ApprovalCardProps) {
  const navigate = useNavigate()
  const { org, site } = useScope()
  const resume = useResumeRun()
  const cancel = useCancelRun()
  const busy = resume.isPending || cancel.isPending

  const approve = () =>
    resume.mutate(runId, {
      onSuccess: (res) => {
        toast.success('Write approved', {
          description: `${res.point} → ${formatValue(res.effective ?? null)} at priority ${res.priority}`,
        })
        if (org && site)
          navigate({
            to: '/o/$org/s/$siteSlug/points',
            params: { org, siteSlug: site.slug },
          })
      },
      onError: (e) => toast.error('Approval failed', { description: (e as Error).message }),
    })

  const reject = () =>
    cancel.mutate(runId, {
      onSuccess: () => toast('Run cancelled', { description: 'The held write was discarded.' }),
      onError: (e) => toast.error('Cancel failed', { description: (e as Error).message }),
    })

  return (
    <Card className='border-sev-warning/40'>
      <CardHeader>
        <CardTitle className='eyebrow text-sev-warning text-[10px] font-semibold'>
          Awaiting approval
        </CardTitle>
      </CardHeader>
      <CardContent className='space-y-3'>
        <dl className='space-y-1.5 text-[12px]'>
          <WriteRow label='Point' value={write.point} mono />
          <WriteRow label='Value' value={formatValue(write.value)} />
          <WriteRow label='Priority' value={String(write.priority)} />
          <WriteRow label='Agent ceiling' value={String(write.agent_min_priority)} />
        </dl>
        <div className='flex flex-wrap gap-2'>
          <Button size='sm' onClick={approve} disabled={busy}>
            <Check className='size-3.5' /> Approve &amp; write
          </Button>
          <Button variant='outline' size='sm' onClick={reject} disabled={busy}>
            <X className='size-3.5' /> Reject
          </Button>
        </div>
      </CardContent>
    </Card>
  )
}

function WriteRow({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className='flex items-baseline justify-between gap-3'>
      <dt className='text-muted-foreground'>{label}</dt>
      <dd className={mono ? 'truncate font-mono text-[11px] font-medium' : 'font-medium'}>
        {value}
      </dd>
    </div>
  )
}
