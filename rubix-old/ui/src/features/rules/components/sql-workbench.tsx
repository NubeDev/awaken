import { useMemo, useState } from 'react'
import { useMutation } from '@tanstack/react-query'
import { ArrowUpDown, ChartLine, Clock, Play, Table2 } from 'lucide-react'
import {
  Area,
  AreaChart,
  CartesianGrid,
  ResponsiveContainer,
  XAxis,
  YAxis,
} from 'recharts'
import * as api from '@/api/endpoints'
import { ApiError } from '@/api/client'
import type { QueryRow } from '@/api/types'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { CodeEditor } from '@/components/code/code-editor'

const DEFAULT_SQL = 'SELECT slug, display_name, kind FROM points LIMIT 20'

// Recent queries persist locally; the server has no saved-query surface and the
// brief scopes this to client-side.
const RECENT_KEY = 'rubix.sql.recent'

function loadRecent(): string[] {
  try {
    const raw = localStorage.getItem(RECENT_KEY)
    return raw ? (JSON.parse(raw) as string[]) : []
  } catch {
    return []
  }
}

function saveRecent(list: string[]) {
  try {
    localStorage.setItem(RECENT_KEY, JSON.stringify(list.slice(0, 8)))
  } catch {
    // localStorage may be unavailable (private mode); recents are best-effort.
  }
}

function columnsOf(rows: QueryRow[]): string[] {
  return rows.length ? Object.keys(rows[0]!) : []
}

function formatCell(value: QueryRow[string] | undefined): string {
  return value === null || value === undefined ? '—' : String(value)
}

/** The chartable shape of a result: a `ts`-ish column + a numeric column. */
function timeseriesOf(
  rows: QueryRow[]
): { data: { t: string; value: number }[]; valueCol: string } | null {
  if (rows.length < 2) return null
  const cols = columnsOf(rows)
  const tsCol = cols.find((c) => /^(ts|time|timestamp|t)$/i.test(c))
  const valueCol = cols.find(
    (c) => c !== tsCol && rows.every((r) => typeof r[c] === 'number')
  )
  if (!tsCol || !valueCol) return null
  const data = rows.map((r) => ({
    t: String(r[tsCol]),
    value: r[valueCol] as number,
  }))
  return { data, valueCol }
}

type SortState = { col: string; dir: 'asc' | 'desc' } | null

/**
 * SQL / query workbench: a CodeMirror SQL console over `POST /api/v1/query`,
 * a sortable result table, and a chart toggle when the result is a timeseries.
 * Supersedes the legacy `/history` console (which now redirects here). The
 * "use rows as rule input" connective tissue runs the query and hands a point
 * keyexpr suggestion to the debugger via `onUseKeyexpr` when present.
 */
export function SqlWorkbench({
  onUseKeyexpr,
}: {
  onUseKeyexpr?: (keyexpr: string) => void
}) {
  const [sql, setSql] = useState(DEFAULT_SQL)
  const [recent, setRecent] = useState<string[]>(loadRecent)
  const [view, setView] = useState<'table' | 'chart'>('table')
  const [sort, setSort] = useState<SortState>(null)
  const run = useMutation({
    mutationFn: () => api.query.run(sql),
    onSuccess: () => {
      const next = [sql, ...recent.filter((q) => q !== sql)]
      setRecent(next)
      saveRecent(next)
    },
  })

  const rows = useMemo(() => run.data?.rows ?? [], [run.data])
  const cols = columnsOf(rows)
  const series = useMemo(() => timeseriesOf(rows), [rows])
  const sorted = useMemo(() => {
    if (!sort) return rows
    const { col, dir } = sort
    return [...rows].sort((a, b) => {
      const av = a[col]
      const bv = b[col]
      if (av === bv) return 0
      if (av === null || av === undefined) return 1
      if (bv === null || bv === undefined) return -1
      const cmp = av < bv ? -1 : 1
      return dir === 'asc' ? cmp : -cmp
    })
  }, [rows, sort])

  const toggleSort = (col: string) =>
    setSort((s) =>
      s?.col === col
        ? s.dir === 'asc'
          ? { col, dir: 'desc' }
          : null
        : { col, dir: 'asc' }
    )

  // A query selecting a `keyexpr` column lets the debugger reuse a row as input.
  const keyexprValue =
    onUseKeyexpr && rows.length && typeof rows[0]!['keyexpr'] === 'string'
      ? (rows[0]!['keyexpr'] as string)
      : undefined

  return (
    <div className='flex min-h-0 flex-col gap-3'>
      <Card className='gap-3 p-3'>
        <CodeEditor
          value={sql}
          onChange={setSql}
          language='sql'
          ariaLabel='SQL query'
          placeholder='SELECT … FROM points'
          minHeight={120}
        />
        <div className='flex flex-wrap items-center gap-2'>
          <Button
            size='sm'
            onClick={() => run.mutate()}
            disabled={run.isPending}
            className='gap-1.5'
          >
            <Play className='size-3.5' /> Run query
          </Button>
          {keyexprValue && onUseKeyexpr && (
            <Button
              size='sm'
              variant='outline'
              className='gap-1.5'
              onClick={() => onUseKeyexpr(keyexprValue)}
            >
              Use first keyexpr as rule input
            </Button>
          )}
          {run.isError && (
            <span className='text-sev-fault font-mono text-[11px]'>
              {run.error instanceof ApiError ? run.error.message : 'Query failed'}
            </span>
          )}
          {run.data && !run.isError && (
            <span className='text-muted-foreground ms-auto text-[11px]'>
              {rows.length} {rows.length === 1 ? 'row' : 'rows'}
            </span>
          )}
        </div>
        {recent.length > 1 && (
          <div className='flex flex-wrap items-center gap-1.5'>
            <Clock className='text-muted-foreground size-3' />
            {recent.slice(0, 5).map((q) => (
              <button
                key={q}
                type='button'
                onClick={() => setSql(q)}
                className='text-muted-foreground hover:bg-muted hover:text-foreground max-w-[220px] truncate rounded border border-border px-1.5 py-0.5 font-mono text-[10px]'
                title={q}
              >
                {q}
              </button>
            ))}
          </div>
        )}
      </Card>

      {run.data && (
        <Card className='scroll min-h-0 flex-1 overflow-auto p-0'>
          {rows.length === 0 ? (
            <p className='text-muted-foreground py-12 text-center text-sm'>
              No rows.
            </p>
          ) : (
            <>
              {series && (
                <div className='flex items-center gap-1 border-b border-border p-2'>
                  <ViewToggle
                    active={view === 'table'}
                    onClick={() => setView('table')}
                    icon={<Table2 className='size-3.5' />}
                    label='Table'
                  />
                  <ViewToggle
                    active={view === 'chart'}
                    onClick={() => setView('chart')}
                    icon={<ChartLine className='size-3.5' />}
                    label='Chart'
                  />
                </div>
              )}
              {series && view === 'chart' ? (
                <div className='p-3'>
                  <ResponsiveContainer width='100%' height={260}>
                    <AreaChart
                      data={series.data}
                      margin={{ top: 6, right: 8, left: -12, bottom: 0 }}
                    >
                      <defs>
                        <linearGradient id='sqlFill' x1='0' y1='0' x2='0' y2='1'>
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
                        minTickGap={42}
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
                        fill='url(#sqlFill)'
                        isAnimationActive={false}
                      />
                    </AreaChart>
                  </ResponsiveContainer>
                </div>
              ) : (
                <table className='w-full text-[12.5px]'>
                  <thead className='bg-card sticky top-0'>
                    <tr className='border-border border-b'>
                      {cols.map((c) => (
                        <th
                          key={c}
                          className='text-muted-foreground px-3 py-2 text-left font-medium'
                        >
                          <button
                            type='button'
                            onClick={() => toggleSort(c)}
                            className='hover:text-foreground inline-flex items-center gap-1'
                          >
                            {c}
                            <ArrowUpDown className='size-3 opacity-50' />
                          </button>
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {sorted.map((row, i) => (
                      <tr
                        key={i}
                        className='border-border/60 hover:bg-muted/40 border-b last:border-0'
                      >
                        {cols.map((c) => (
                          <td key={c} className='tabular px-3 py-1.5 font-mono'>
                            {formatCell(row[c])}
                          </td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </>
          )}
        </Card>
      )}
    </div>
  )
}

function ViewToggle({
  active,
  onClick,
  icon,
  label,
}: {
  active: boolean
  onClick: () => void
  icon: React.ReactNode
  label: string
}) {
  return (
    <button
      type='button'
      onClick={onClick}
      aria-pressed={active}
      className={`inline-flex items-center gap-1.5 rounded px-2 py-1 text-[11.5px] ${
        active
          ? 'bg-muted text-foreground'
          : 'text-muted-foreground hover:text-foreground'
      }`}
    >
      {icon}
      {label}
    </button>
  )
}
