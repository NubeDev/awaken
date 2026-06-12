import { useMemo, useState } from 'react'
import { CircleAlert, Check, Pin } from 'lucide-react'
import {
  Area,
  AreaChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts'
import { useCreateWidget, usePointHistory, useRelinquishPoint } from '@/api/hooks'
import { pointKeyexpr } from '@/api/keyexpr'
import type { Point, Site, Equip } from '@/api/types'
import { tagNames } from '@/api/tags'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { ageShort, formatValue } from '@/lib/format'
import { SLOT_LABELS, winningSlotIndex } from '../lib/priority'
import { PointKindIcon } from './point-kind-icon'
import { PriorityArrayCard } from './priority-array'

type Range = '1h' | '24h' | '7d'
const RANGE_SAMPLES: Record<Range, number> = { '1h': 4, '24h': 48, '7d': 7 * 48 }

type PointDetailProps = {
  point: Point
  site: Site | undefined
  equip: Equip | undefined
  inFinding: boolean
}

/** Right pane: live value, command source, tags, history, and the priority array. */
export function PointDetail({ point, site, equip, inFinding }: PointDetailProps) {
  const relinquish = useRelinquishPoint()
  const pin = useCreateWidget(site?.id)
  const { data: history = [] } = usePointHistory(point.id)
  const [range, setRange] = useState<Range>('24h')

  const canPin = Boolean(site && equip)
  const pinPoint = () => {
    if (!site || !equip) return
    pin.mutate({
      site_id: site.id,
      kind: 'point_value',
      title: point.display_name,
      target: pointKeyexpr(site, equip, point),
    })
  }
  const writable = point.kind !== 'sensor'
  const winning = winningSlotIndex(point.priority_array)

  const rows = useMemo(
    () =>
      history
        .filter((s) => typeof s.value === 'number')
        .slice(-RANGE_SAMPLES[range])
        .map((s) => ({
          t: new Date(s.ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
          value: s.value as number,
        })),
    [history, range]
  )

  const keyexpr =
    site && equip ? `${site.org}/${site.slug}/${equip.path}/${point.slug}/cur` : point.slug

  return (
    <div className='space-y-4'>
      {/* header */}
      <div className='flex items-start justify-between gap-3'>
        <div className='flex items-center gap-3'>
          <PointKindIcon kind={point.kind} inFinding={inFinding} size='lg' />
          <div>
            <div className='flex items-center gap-2'>
              <h2 className='text-[16px] font-semibold tracking-tight'>{point.display_name}</h2>
              <Badge variant='muted' className='h-[18px] px-1.5 font-mono text-[9.5px] uppercase'>
                {point.kind}
              </Badge>
              {inFinding ? (
                <Badge variant='fault' className='h-[18px] gap-1 px-1.5 text-[9.5px]'>
                  <CircleAlert className='size-2.5' /> in finding
                </Badge>
              ) : null}
            </div>
            <p className='text-muted-foreground mt-0.5 font-mono text-[11px]'>{keyexpr}</p>
          </div>
        </div>
        {canPin ? (
          <Button
            variant='outline'
            size='sm'
            className='h-7 gap-1.5 text-[12px]'
            disabled={pin.isPending || pin.isSuccess}
            onClick={pinPoint}
          >
            {pin.isSuccess ? (
              <>
                <Check className='size-3.5' /> Pinned
              </>
            ) : (
              <>
                <Pin className='size-3.5' /> Pin to dashboard
              </>
            )}
          </Button>
        ) : null}
      </div>

      {/* stat cards */}
      <div className='grid gap-3 sm:grid-cols-3'>
        <Card className='gap-2 p-3.5'>
          <span className='eyebrow text-[10px]'>Live value</span>
          <div className='flex items-baseline gap-1'>
            <span className='tabular text-2xl leading-none font-semibold tracking-tight'>
              {formatValue(point.cur_value)}
            </span>
            <span className='text-muted-foreground text-[12px]'>{point.unit ?? ''}</span>
          </div>
          <div className='text-muted-foreground flex items-center gap-1.5 text-[11px]'>
            <span className='bg-positive size-1.5 rounded-full' />
            updated {ageShort(point.cur_ts)} ago
          </div>
        </Card>
        <Card className='gap-2 p-3.5'>
          <span className='eyebrow text-[10px]'>Source · effective</span>
          {writable && winning >= 0 ? (
            <>
              <div className='flex items-center gap-2'>
                <Badge variant='primary' className='h-5 px-2 text-[10.5px]'>
                  Level {winning + 1}
                </Badge>
                <span className='text-[12.5px] font-medium'>
                  {SLOT_LABELS[winning + 1] ?? `priority ${winning + 1}`}
                </span>
              </div>
              <p className='text-muted-foreground text-[11px]'>Lowest occupied level wins</p>
            </>
          ) : writable ? (
            <>
              <span className='text-[12.5px] font-medium'>Relinquish default</span>
              <p className='text-muted-foreground text-[11px]'>All 16 slots are null</p>
            </>
          ) : (
            <>
              <span className='text-[12.5px] font-medium'>Field sensor</span>
              <p className='text-muted-foreground text-[11px]'>Read-only · driver ingest</p>
            </>
          )}
        </Card>
        <Card className='gap-2 p-3.5'>
          <span className='eyebrow text-[10px]'>Tags</span>
          <div className='flex flex-wrap gap-1'>
            {tagNames(point.tags).map((t) => (
              <Badge key={t} variant='outline' className='h-[18px] px-1.5 font-mono text-[10px]'>
                #{t}
              </Badge>
            ))}
          </div>
        </Card>
      </div>

      {/* history */}
      <Card>
        <CardHeader>
          <CardTitle className='text-[13.5px]'>History</CardTitle>
          <CardDescription className='text-[11.5px]'>
            Served by local store · Parquet partitions
          </CardDescription>
          <CardAction>
            <Tabs value={range} onValueChange={(v) => setRange(v as Range)}>
              <TabsList className='h-7'>
                <TabsTrigger value='1h' className='px-2 text-xs'>1h</TabsTrigger>
                <TabsTrigger value='24h' className='px-2 text-xs'>24h</TabsTrigger>
                <TabsTrigger value='7d' className='px-2 text-xs'>7d</TabsTrigger>
              </TabsList>
            </Tabs>
          </CardAction>
        </CardHeader>
        <CardContent>
          {rows.length < 2 ? (
            <div className='text-muted-foreground grid h-[160px] place-items-center text-sm'>
              No numeric history for this point.
            </div>
          ) : (
            <ResponsiveContainer width='100%' height={170}>
              <AreaChart data={rows} margin={{ top: 6, right: 4, left: -18, bottom: 0 }}>
                <defs>
                  <linearGradient id='pdFill' x1='0' y1='0' x2='0' y2='1'>
                    <stop offset='0%' stopColor='var(--chart-1)' stopOpacity={0.25} />
                    <stop offset='100%' stopColor='var(--chart-1)' stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid stroke='var(--grid-line)' vertical={false} />
                <XAxis
                  dataKey='t'
                  tickLine={false}
                  axisLine={false}
                  fontSize={10}
                  minTickGap={42}
                  tick={{ fill: 'var(--muted-foreground)' }}
                />
                <YAxis
                  tickLine={false}
                  axisLine={false}
                  fontSize={10}
                  width={46}
                  tick={{ fill: 'var(--muted-foreground)' }}
                  domain={['auto', 'auto']}
                />
                <Tooltip
                  contentStyle={{
                    background: 'var(--popover)',
                    border: '1px solid var(--border)',
                    borderRadius: 8,
                    fontSize: 12,
                  }}
                />
                <Area
                  type='monotone'
                  dataKey='value'
                  stroke='var(--chart-1)'
                  strokeWidth={2}
                  fill='url(#pdFill)'
                  isAnimationActive={false}
                />
              </AreaChart>
            </ResponsiveContainer>
          )}
        </CardContent>
      </Card>

      {writable ? (
        <PriorityArrayCard
          point={point}
          relinquishing={relinquish.isPending}
          onRelinquish={(priority) => relinquish.mutate({ id: point.id, priority })}
        />
      ) : null}
    </div>
  )
}
