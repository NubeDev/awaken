import { useMemo, useState } from 'react'
import { Play, TriangleAlert } from 'lucide-react'
import {
  Area,
  AreaChart,
  CartesianGrid,
  ReferenceDot,
  ResponsiveContainer,
  XAxis,
  YAxis,
} from 'recharts'
import { ApiError } from '@/api/client'
import { useDryRunRule } from '@/api/hooks'
import type { DryRunResponse } from '@/api/types'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { SeverityIcon } from '@/components/severity-icon'

const SEV_BADGE = { fault: 'fault', warning: 'warning', info: 'info' } as const

type RuleDebuggerProps = {
  org: string | undefined
  /** The current source: an inline script (editor draft) or a stored rule name. */
  source: { script: string } | { rule: string }
  params: Record<string, unknown>
  /** Seeded point keyexpr (e.g. handed over from the SQL workbench). */
  keyexpr: string
  onKeyexprChange: (keyexpr: string) => void
}

/**
 * The debugger: pick a point + window, run the rule once via the dry-run
 * endpoint, and show the verdict (Spark-style severity badge + message + value)
 * over the exact frame the rule saw (charted with Recharts, anomalies marked),
 * plus a data-table fallback. Errors surface with their category so a compile
 * vs runtime vs resolve failure is legible. This is the tight edit→run loop.
 */
export function RuleDebugger({
  org,
  source,
  params,
  keyexpr,
  onKeyexprChange,
}: RuleDebuggerProps) {
  const [limit, setLimit] = useState(500)
  const dryRun = useDryRunRule(org)

  const run = () =>
    dryRun.mutate({
      ...source,
      params,
      point: keyexpr.trim() || undefined,
      limit,
    })

  return (
    <Card className='gap-3 p-4'>
      <div className='flex items-center justify-between'>
        <span className='eyebrow text-[10px]'>Debugger · dry-run</span>
        {dryRun.data && <Verdict data={dryRun.data} />}
      </div>

      <div className='flex flex-wrap items-end gap-2'>
        <div className='flex min-w-[240px] flex-1 flex-col gap-1'>
          <Label htmlFor='dbg-keyexpr' className='text-[11px]'>
            Input point (keyexpr)
          </Label>
          <Input
            id='dbg-keyexpr'
            value={keyexpr}
            onChange={(e) => onKeyexprChange(e.target.value)}
            placeholder='org/site/equip/point'
            className='h-8 font-mono text-[12px]'
            spellCheck={false}
          />
        </div>
        <div className='flex w-24 flex-col gap-1'>
          <Label htmlFor='dbg-limit' className='text-[11px]'>
            Rows
          </Label>
          <Input
            id='dbg-limit'
            type='number'
            min={1}
            max={10000}
            value={limit}
            onChange={(e) => setLimit(Math.max(1, Number(e.target.value) || 1))}
            className='h-8 text-[12px]'
          />
        </div>
        <Button
          size='sm'
          onClick={run}
          disabled={dryRun.isPending || !org}
          className='h-8 gap-1.5'
        >
          <Play className='size-3.5' /> Run
        </Button>
      </div>

      {dryRun.isError && (
        <DryRunError error={dryRun.error} />
      )}

      {dryRun.data && <FrameView data={dryRun.data} />}

      {!dryRun.data && !dryRun.isError && (
        <p className='text-muted-foreground py-6 text-center text-[12px]'>
          Pick a point and run to see the verdict and the frame the rule saw.
        </p>
      )}
    </Card>
  )
}

function Verdict({ data }: { data: DryRunResponse }) {
  const { result } = data
  if (!result.flagged) {
    return (
      <Badge variant='muted' className='h-5 gap-1 px-2 text-[10.5px]'>
        Clear — no finding
      </Badge>
    )
  }
  return (
    <div className='flex items-center gap-2'>
      <SeverityIcon severity={result.severity} className='size-4' />
      <Badge
        variant={SEV_BADGE[result.severity]}
        className='h-5 px-2 text-[10.5px] capitalize'
      >
        {result.severity}
      </Badge>
      <span className='text-[12.5px] font-medium'>{result.message}</span>
      {result.value !== null && (
        <span className='text-muted-foreground font-mono text-[11px]'>
          = {result.value}
        </span>
      )}
    </div>
  )
}

/**
 * The category-tagged dry-run error. The server maps compile/runtime/limit/
 * resolve failures to a 400 with a prefixed message; surface the message
 * verbatim (it is engine diagnostics, not user-controlled markup) with a label.
 */
function DryRunError({ error }: { error: unknown }) {
  const message =
    error instanceof ApiError ? error.message : 'Dry-run failed.'
  const category = /compile/i.test(message)
    ? 'compile'
    : /limit|deadline|exceeded/i.test(message)
      ? 'limit'
      : /resolve|not found|compose/i.test(message)
        ? 'resolve'
        : 'runtime'
  return (
    <div
      role='alert'
      className='border-destructive/40 bg-destructive/10 flex items-start gap-2 rounded-md border p-2.5'
    >
      <TriangleAlert className='text-destructive mt-0.5 size-4 shrink-0' />
      <div className='min-w-0'>
        <Badge variant='fault' className='h-4 px-1.5 text-[9.5px] capitalize'>
          {category}
        </Badge>
        <p className='text-destructive mt-1 font-mono text-[11px] break-words'>
          {message}
        </p>
      </div>
    </div>
  )
}

function FrameView({ data }: { data: DryRunResponse }) {
  const points = useMemo(
    () =>
      data.frame.rows
        .map((r, i) => ({
          i,
          t: new Date(r.ts).toLocaleTimeString([], {
            hour: '2-digit',
            minute: '2-digit',
          }),
          value: r.value,
        }))
        .filter((r): r is { i: number; t: string; value: number } => r.value !== null),
    [data.frame.rows]
  )

  // Mark the row that carries the verdict value, when it appears in the series —
  // a cheap "this is what tripped the rule" cue without re-deriving anomalies.
  const flaggedIndex =
    data.result.flagged && data.result.value !== null
      ? points.findIndex((p) => p.value === data.result.value)
      : -1

  if (points.length < 2) {
    return (
      <div className='text-muted-foreground rounded-md border border-border p-3 text-center text-[12px]'>
        {data.frame.row_count === 0
          ? 'Empty frame — no history rows in the selected window.'
          : `${data.frame.row_count} row(s); not enough numeric points to chart.`}
      </div>
    )
  }

  return (
    <div className='space-y-2'>
      <ResponsiveContainer width='100%' height={200}>
        <AreaChart data={points} margin={{ top: 6, right: 8, left: -14, bottom: 0 }}>
          <defs>
            <linearGradient id='frameFill' x1='0' y1='0' x2='0' y2='1'>
              <stop offset='0%' stopColor='var(--chart-1)' stopOpacity={0.28} />
              <stop offset='100%' stopColor='var(--chart-1)' stopOpacity={0} />
            </linearGradient>
          </defs>
          <CartesianGrid stroke='var(--grid-line)' vertical={false} />
          <XAxis
            dataKey='t'
            tickLine={false}
            axisLine={false}
            fontSize={10}
            minTickGap={44}
            tick={{ fill: 'var(--muted-foreground)' }}
          />
          <YAxis
            tickLine={false}
            axisLine={false}
            fontSize={10}
            width={48}
            tick={{ fill: 'var(--muted-foreground)' }}
            domain={['auto', 'auto']}
          />
          <Area
            type='monotone'
            dataKey='value'
            stroke='var(--chart-1)'
            strokeWidth={1.8}
            fill='url(#frameFill)'
            isAnimationActive={false}
          />
          {flaggedIndex >= 0 && (
            <ReferenceDot
              x={points[flaggedIndex]!.t}
              y={points[flaggedIndex]!.value}
              r={4}
              fill='var(--sev-fault)'
              stroke='var(--background)'
            />
          )}
        </AreaChart>
      </ResponsiveContainer>
      <p className='text-muted-foreground text-[10.5px]'>
        {data.frame.row_count} row(s) · columns: ts, value
      </p>
    </div>
  )
}
