import { Cpu, Power, Radio, ShieldCheck } from 'lucide-react'
import type { AgentStatus } from '@/api/types'
import { Card, CardContent } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'

/**
 * The embedded agent is process-global and configured by env vars read at boot
 * (`RUBIX_AI`, `RUBIX_AI_PROVIDER`/`_MODEL_ID`, the priority gate) — it is not
 * per-org and not editable from the UI. This card surfaces that config read-only
 * so an operator can see whether the agent is on and how its writes are gated,
 * and — when off — how to enable it.
 */
export function AgentStatusCard({ status }: { status: AgentStatus }) {
  if (!status.enabled) return <AgentDisabled />

  return (
    <Card className='gap-0 p-0'>
      <CardContent className='flex flex-wrap items-center gap-x-5 gap-y-2 p-3.5'>
        <span className='inline-flex items-center gap-1.5'>
          <span className='bg-positive size-2 rounded-full' />
          <span className='text-[13px] font-medium'>Agent online</span>
        </span>

        <Field icon={Cpu} label='Model'>
          <span className='font-mono text-[12px]'>
            {status.provider}/{status.model}
          </span>
        </Field>

        <Field icon={ShieldCheck} label='Write gate'>
          <span className='text-[12px]'>
            commit ≥ <b>{status.min_priority}</b> · approve {status.escalation_floor}–
            {status.min_priority - 1} · deny &lt; {status.escalation_floor}
          </span>
        </Field>

        <Field icon={Radio} label='Dispatch'>
          <Badge
            variant={status.dispatch_ready ? 'positive' : 'muted'}
            className='h-5 px-1.5 text-[10.5px]'
          >
            {status.dispatch_ready ? 'Ready' : 'Off'}
          </Badge>
        </Field>

        {status.max_rounds != null ? (
          <span className='text-muted-foreground ms-auto text-[11px]'>
            {status.max_rounds} rounds/run
          </span>
        ) : null}
      </CardContent>
    </Card>
  )
}

function Field({
  icon: Icon,
  label,
  children,
}: {
  icon: typeof Cpu
  label: string
  children: React.ReactNode
}) {
  return (
    <span className='inline-flex items-center gap-1.5'>
      <Icon className='text-muted-foreground size-3.5' />
      <span className='eyebrow text-muted-foreground text-[9.5px]'>{label}</span>
      {children}
    </span>
  )
}

/** Agent off (`RUBIX_AI != 1`): a quiet, honest banner — the agent is a server
 *  deploy concern, so there is no in-UI toggle, only the enable instruction. */
function AgentDisabled({ className }: { className?: string }) {
  return (
    <Card className={cn('border-dashed', className)}>
      <CardContent className='flex flex-wrap items-center gap-x-3 gap-y-1 p-3.5'>
        <Power className='text-muted-foreground size-4' />
        <span className='text-[13px] font-medium'>Agent disabled</span>
        <span className='text-muted-foreground text-[12px]'>
          The embedded agent is off, so chat and dispatch are unavailable.
        </span>
        <span className='text-muted-foreground/80 ms-auto text-[11.5px]'>
          Enable by starting the server with{' '}
          <code className='font-mono'>RUBIX_AI=1</code> and an{' '}
          <code className='font-mono'>OPENAI_API_KEY</code>.
        </span>
      </CardContent>
    </Card>
  )
}
