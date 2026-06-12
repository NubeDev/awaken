import { useMemo } from 'react'
import { Building2, Check, CircuitBoard, Clock, Network, Sparkles, UserPlus } from 'lucide-react'
import { useNavigate } from '@tanstack/react-router'
import { toast } from 'sonner'
import { Area, AreaChart, CartesianGrid, ResponsiveContainer, XAxis, YAxis } from 'recharts'
import { useAgentChat, usePointHistory } from '@/api/hooks'
import type { Equip, Point, Site, Spark } from '@/api/types'
import { SeverityIcon } from '@/components/severity-icon'
import { Sparkline } from '@/components/charts/sparkline'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { relativeTime } from '@/lib/format'

const SEV_BADGE = { fault: 'fault', warning: 'warning', info: 'info' } as const

type SparkDetailProps = {
  spark: Spark
  site: Site | undefined
  points: Point[]
  equips: Equip[]
  onAck: () => void
  acking: boolean
}

/** Right pane: full finding context with the diagnose-with-awaken entry point. */
export function SparkDetail({ spark, site, points, equips, onAck, acking }: SparkDetailProps) {
  const navigate = useNavigate()
  const chat = useAgentChat()
  const implicated = spark.point_ids
    .map((id) => points.find((p) => p.id === id))
    .filter((p): p is Point => Boolean(p))
  const equip = equips.find((e) => e.id === implicated[0]?.equip_id)
  const { data: trendHis = [] } = usePointHistory(implicated[0]?.id)

  const trend = useMemo(
    () =>
      trendHis
        .filter((s) => typeof s.value === 'number')
        .slice(-48)
        .map((s) => ({
          t: new Date(s.ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
          value: s.value as number,
        })),
    [trendHis]
  )

  const diagnose = () =>
    chat.mutate(
      {
        thread_id: `spark-${spark.id}`,
        message: `Diagnose finding ${spark.rule}: ${spark.message}`,
      },
      {
        onSuccess: (res) => {
          if (res.status === 'awaiting_approval' && res.run_id) {
            const runId = res.run_id
            toast('Run awaiting approval', {
              description: res.response,
              action: {
                label: 'Review & approve',
                onClick: () => navigate({ to: '/runs/$runId', params: { runId } }),
              },
            })
            return
          }
          toast('awaken responded', { description: res.response })
        },
        onError: () => toast.error('Agent unavailable'),
      }
    )

  return (
    <div className='space-y-4'>
      {/* header */}
      <div className='flex items-start gap-3.5'>
        <span className='bg-sev-fault/10 grid size-11 shrink-0 place-items-center rounded-xl'>
          <SeverityIcon severity={spark.severity} className='size-5' />
        </span>
        <div className='min-w-0'>
          <div className='flex items-center gap-2'>
            <Badge variant={SEV_BADGE[spark.severity]} className='h-5 px-2 text-[10.5px] capitalize'>
              {spark.severity}
            </Badge>
            <span className='text-muted-foreground truncate font-mono text-[11.5px]'>
              {spark.rule}
            </span>
          </div>
          <h2 className='mt-1.5 text-[17px] leading-snug font-semibold tracking-tight'>
            {spark.message}
          </h2>
          <div className='text-muted-foreground mt-1.5 flex flex-wrap items-center gap-x-3 gap-y-1 text-[11.5px]'>
            <span className='flex items-center gap-1'>
              <Building2 className='size-3' /> {site?.display_name ?? '—'}
            </span>
            {equip ? (
              <span className='flex items-center gap-1'>
                <CircuitBoard className='size-3' /> {equip.display_name}
              </span>
            ) : null}
            <span className='flex items-center gap-1'>
              <Clock className='size-3' /> {relativeTime(spark.ts)}
            </span>
          </div>
        </div>
      </div>

      {/* actions */}
      <div className='flex flex-wrap items-center gap-2'>
        <Button size='sm' onClick={diagnose} disabled={chat.isPending}>
          <Sparkles className='size-3.5' /> Diagnose with awaken
        </Button>
        {!spark.acknowledged ? (
          <Button variant='outline' size='sm' onClick={onAck} disabled={acking}>
            <Check className='size-3.5' /> Acknowledge
          </Button>
        ) : null}
        {implicated[0] ? (
          <Button
            variant='outline'
            size='sm'
            onClick={() =>
              navigate({ to: '/points', search: { equip: implicated[0]!.equip_id } })
            }
          >
            <Network className='size-3.5' /> View points
          </Button>
        ) : null}
        <Button variant='outline' size='sm' disabled>
          <UserPlus className='size-3.5' /> Assign
        </Button>
      </div>

      {/* implicated points + rule context */}
      <div className='grid gap-3 lg:grid-cols-2'>
        <Card className='gap-2.5 p-4'>
          <span className='eyebrow text-[10px]'>Implicated points</span>
          {implicated.length === 0 ? (
            <p className='text-muted-foreground py-4 text-center text-[12px]'>
              No points recorded on this finding.
            </p>
          ) : (
            <div className='space-y-1.5'>
              {implicated.map((p) => (
                <ImplicatedPoint key={p.id} point={p} />
              ))}
            </div>
          )}
        </Card>

        <Card className='gap-2.5 p-4'>
          <span className='eyebrow text-[10px]'>Rule context</span>
          <dl className='space-y-1.5 text-[12px]'>
            <ContextRow label='Rule board' value={spark.rule} mono />
            <ContextRow label='Severity' value={spark.severity} />
            <ContextRow
              label='Keyexpr'
              value={`${site?.org ?? '*'}/*/spark/${spark.rule}/**`}
              mono
            />
            <ContextRow label='First seen' value={relativeTime(spark.ts)} />
            <ContextRow label='Status' value={spark.acknowledged ? 'Acknowledged' : 'Open'} />
          </dl>
        </Card>
      </div>

      {/* trend */}
      {trend.length > 2 ? (
        <Card>
          <CardHeader>
            <CardTitle className='eyebrow text-[10px] font-semibold'>
              Trend · {implicated[0]?.display_name}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <ResponsiveContainer width='100%' height={150}>
              <AreaChart data={trend} margin={{ top: 4, right: 4, left: -18, bottom: 0 }}>
                <defs>
                  <linearGradient id='sparkTrend' x1='0' y1='0' x2='0' y2='1'>
                    <stop offset='0%' stopColor='var(--sev-fault)' stopOpacity={0.2} />
                    <stop offset='100%' stopColor='var(--sev-fault)' stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid stroke='var(--grid-line)' vertical={false} />
                <XAxis
                  dataKey='t'
                  tickLine={false}
                  axisLine={false}
                  fontSize={10}
                  minTickGap={48}
                  tick={{ fill: 'var(--muted-foreground)' }}
                />
                <YAxis
                  tickLine={false}
                  axisLine={false}
                  fontSize={10}
                  width={44}
                  tick={{ fill: 'var(--muted-foreground)' }}
                  domain={['auto', 'auto']}
                />
                <Area
                  type='monotone'
                  dataKey='value'
                  stroke='var(--sev-fault)'
                  strokeWidth={1.8}
                  fill='url(#sparkTrend)'
                  isAnimationActive={false}
                />
              </AreaChart>
            </ResponsiveContainer>
          </CardContent>
        </Card>
      ) : null}
    </div>
  )
}

function ImplicatedPoint({ point }: { point: Point }) {
  const { data: his = [] } = usePointHistory(point.id)
  const spark = his
    .filter((s) => typeof s.value === 'number')
    .slice(-16)
    .map((s) => s.value as number)
  return (
    <div className='border-border bg-muted/30 flex items-center gap-2.5 rounded-md border px-2.5 py-1.5'>
      <span className='text-muted-foreground font-mono text-[11.5px]'>{point.slug}</span>
      <span className='ms-auto'>
        {spark.length > 1 ? <Sparkline data={spark} width={72} height={22} /> : null}
      </span>
    </div>
  )
}

function ContextRow({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className='flex items-baseline justify-between gap-3'>
      <dt className='text-muted-foreground'>{label}</dt>
      <dd className={mono ? 'truncate font-mono text-[11px] font-medium' : 'font-medium capitalize'}>
        {value}
      </dd>
    </div>
  )
}
