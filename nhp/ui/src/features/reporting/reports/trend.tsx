/**
 * Trend report — one chart per register quantity (power, voltage, …), each with a
 * line per meter, so an engineer can eyeball a spike across a whole site or tenant
 * at once. Respects the quantity filter (one chart when set, otherwise a chart per
 * quantity present in scope). Series per chart are capped so a large portfolio
 * stays legible; the cap is disclosed, never silent.
 */
import { useMemo } from 'react'
import { Card } from '@/components/ui/card'
import { seriesColor } from '@/features/dashboards/_shared/palette'
import type { WindowToken } from '@/features/dashboards/query/time-window'
import { useWindowedHistory } from '../use-portfolio'
import { selectRegisters, type PortfolioIndex, type ScopeFilter } from '../scope'
import { SeriesChart, type ChartSeries } from '../series-chart'

/** Max lines drawn on one quantity chart before the rest are dropped (disclosed). */
const MAX_SERIES = 12

export function TrendReport({
  index,
  filter,
  token,
}: {
  index: PortfolioIndex
  filter: ScopeFilter
  token: WindowToken
}) {
  const registers = useMemo(
    () => selectRegisters(index, filter),
    [index, filter]
  )
  const history = useWindowedHistory(
    registers.map((r) => ({ id: r.id })),
    token
  )

  // Group registers by quantity → { quantity, unit, series[] }.
  const charts = useMemo(() => {
    const byQuantity = new Map<string, typeof registers>()
    for (const r of registers) {
      const q = r.content.quantity ?? 'other'
      const list = byQuantity.get(q) ?? []
      list.push(r)
      byQuantity.set(q, list)
    }
    return [...byQuantity.entries()]
      .sort((a, b) => a[0].localeCompare(b[0]))
      .map(([quantity, regs]) => {
        const capped = regs.slice(0, MAX_SERIES)
        const series: ChartSeries[] = capped.map((r, i) => {
          const meter = index.meterById.get(r.content.meter)
          const points = history.data
            .filter((s) => s.series === r.id)
            .map((s) => ({ t: Date.parse(s.at), v: s.value }))
            .sort((a, b) => a.t - b.t)
          return {
            label: `${meter?.content.name ?? '—'} · ${r.content.name}`,
            color: seriesColor(i),
            points,
          }
        })
        return {
          quantity,
          unit: regs[0]?.content.unit,
          series,
          dropped: regs.length - capped.length,
        }
      })
  }, [registers, history.data, index])

  if (history.isLoading) {
    return <Card className='text-muted-foreground p-8 text-center text-sm'>Loading…</Card>
  }
  if (charts.length === 0) {
    return (
      <Card className='text-muted-foreground p-8 text-center text-sm'>
        No history-bearing registers in this scope.
      </Card>
    )
  }

  return (
    <div className='space-y-4'>
      {charts.map((c) => (
        <Card key={c.quantity} className='report-avoid-break space-y-2 p-3'>
          <div className='flex items-center justify-between'>
            <span className='text-sm font-medium capitalize'>
              {c.quantity.replace(/_/g, ' ')}
              {c.unit ? <span className='text-muted-foreground'> ({c.unit})</span> : null}
            </span>
            {c.dropped > 0 ? (
              <span className='text-muted-foreground text-xs'>
                showing {MAX_SERIES} of {MAX_SERIES + c.dropped} series
              </span>
            ) : null}
          </div>
          <SeriesChart series={c.series} />
        </Card>
      ))}
    </div>
  )
}
