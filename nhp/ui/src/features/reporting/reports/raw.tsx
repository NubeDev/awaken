/**
 * Raw readings export — the pivoted reading table across every register in scope
 * (one column per meter·register, one row per instant), capped so a wide/dense
 * scope can't blow up the DOM/PDF. For handing the underlying numbers to an
 * external engineer alongside the charts.
 */
import { useMemo } from 'react'
import { Card } from '@/components/ui/card'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { ScrollArea } from '@/components/ui/scroll-area'
import type { WindowToken } from '@/features/dashboards/query/time-window'
import { useWindowedHistory } from '../use-portfolio'
import { selectRegisters, type PortfolioIndex, type ScopeFilter } from '../scope'

/** Cap on columns (series) and rows so a broad scope stays a usable export. */
const MAX_COLS = 16
const MAX_ROWS = 1500

export function RawReport({
  index,
  filter,
  token,
}: {
  index: PortfolioIndex
  filter: ScopeFilter
  token: WindowToken
}) {
  const allRegisters = useMemo(
    () => selectRegisters(index, filter),
    [index, filter]
  )
  const registers = allRegisters.slice(0, MAX_COLS)
  const droppedCols = allRegisters.length - registers.length

  const history = useWindowedHistory(
    registers.map((r) => ({ id: r.id })),
    token
  )

  const columns = useMemo(
    () =>
      registers.map((r) => {
        const meter = index.meterById.get(r.content.meter)
        return {
          id: r.id,
          label: `${meter?.content.name ?? '—'} · ${r.content.name}`,
          unit: r.content.unit,
          precision: r.content.precision ?? 2,
        }
      }),
    [registers, index]
  )

  const rows = useMemo(() => {
    const byT = new Map<number, Record<string, number> & { t: number }>()
    for (const s of history.data) {
      const t = Date.parse(s.at)
      let row = byT.get(t)
      if (!row) {
        row = { t }
        byT.set(t, row)
      }
      row[s.series] = s.value
    }
    return [...byT.values()].sort((a, b) => a.t - b.t)
  }, [history.data])

  const shown = rows.slice(-MAX_ROWS)
  const droppedRows = rows.length - shown.length

  if (history.isLoading) {
    return <Card className='text-muted-foreground p-8 text-center text-sm'>Loading…</Card>
  }
  if (registers.length === 0) {
    return (
      <Card className='text-muted-foreground p-8 text-center text-sm'>
        No history-bearing registers in this scope.
      </Card>
    )
  }

  return (
    <Card className='overflow-hidden p-0'>
      <div className='text-muted-foreground flex items-center justify-between border-b px-3 py-2 text-xs'>
        <span className='text-foreground text-sm font-medium'>Raw readings</span>
        <span>
          {rows.length} rows × {registers.length} series
          {droppedCols > 0 ? ` (+${droppedCols} more series omitted)` : ''}
          {droppedRows > 0 ? ` · showing latest ${shown.length}` : ''}
        </span>
      </div>
      <ScrollArea className='max-h-[520px] print:max-h-none'>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Time</TableHead>
              {columns.map((c) => (
                <TableHead key={c.id} className='text-right whitespace-nowrap'>
                  {c.label}
                  {c.unit ? ` (${c.unit})` : ''}
                </TableHead>
              ))}
            </TableRow>
          </TableHeader>
          <TableBody>
            {shown.map((row) => (
              <TableRow key={row.t}>
                <TableCell className='whitespace-nowrap font-mono text-xs'>
                  {new Date(row.t).toLocaleString()}
                </TableCell>
                {columns.map((c) => (
                  <TableCell key={c.id} className='text-right font-mono text-xs'>
                    {typeof row[c.id] === 'number'
                      ? row[c.id].toFixed(c.precision)
                      : '—'}
                  </TableCell>
                ))}
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </ScrollArea>
    </Card>
  )
}
