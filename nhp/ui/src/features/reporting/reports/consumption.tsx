/**
 * Energy / consumption summary report. For every `quantity:"energy"` register in
 * scope (cumulative kWh counters), consumption over the window = last − first
 * (computeStats). Rows are grouped by site with a per-site subtotal and a grand
 * total — the headline "how much energy did this tenant/site use" number.
 *
 * A negative delta (a counter reset / replaced meter mid-window) is shown as-is
 * and flagged, never silently clamped, so the figure is auditable.
 */
import { useMemo } from 'react'
import { Link } from '@tanstack/react-router'
import { Card } from '@/components/ui/card'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { computeStats } from '@/features/data-console/stats'
import type { WindowToken } from '@/features/dashboards/query/time-window'
import { useWindowedHistory } from '../use-portfolio'
import { selectRegisters, type PortfolioIndex, type ScopeFilter } from '../scope'

export function ConsumptionReport({
  index,
  filter,
  token,
}: {
  index: PortfolioIndex
  filter: ScopeFilter
  token: WindowToken
}) {
  // Consumption is an energy concept — ignore the quantity filter, use kWh counters.
  const registers = useMemo(
    () => selectRegisters(index, { ...filter, quantity: 'energy' }),
    [index, filter]
  )
  const history = useWindowedHistory(
    registers.map((r) => ({ id: r.id })),
    token
  )

  const rows = useMemo(() => {
    return registers
      .map((r) => {
        const samples = history.data.filter((s) => s.series === r.id)
        const stats = computeStats(samples)
        const loc = index.meterLocation.get(r.content.meter)
        const meter = index.meterById.get(r.content.meter)
        const consumption =
          stats.first !== null && stats.last !== null
            ? stats.last - stats.first
            : null
        return {
          id: r.id,
          meterId: r.content.meter,
          meterName: meter?.content.name ?? '—',
          siteName: loc?.siteName ?? '—',
          tenantKey: loc?.tenantKey,
          siteKey: loc?.siteKey,
          gatewayKey: loc?.gatewayKey,
          register: r.content.name,
          unit: r.content.unit ?? 'kWh',
          precision: r.content.precision ?? 1,
          start: stats.first,
          end: stats.last,
          consumption,
        }
      })
      .sort((a, b) => a.siteName.localeCompare(b.siteName))
  }, [registers, history.data, index])

  const total = rows.reduce((sum, r) => sum + (r.consumption ?? 0), 0)
  const unit = rows[0]?.unit ?? 'kWh'

  if (history.isLoading) {
    return <Card className='text-muted-foreground p-8 text-center text-sm'>Loading…</Card>
  }
  if (registers.length === 0) {
    return (
      <Card className='text-muted-foreground p-8 text-center text-sm'>
        No energy (kWh) registers in this scope.
      </Card>
    )
  }

  const fmt = (v: number | null, p: number) =>
    v === null ? '—' : v.toLocaleString(undefined, { maximumFractionDigits: p })

  return (
    <Card className='report-avoid-break overflow-hidden p-0'>
      <div className='flex items-center justify-between border-b px-3 py-2'>
        <span className='text-sm font-medium'>Energy consumption</span>
        <span className='text-sm'>
          Total: <strong>{fmt(total, 1)}</strong> {unit}
        </span>
      </div>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Site</TableHead>
            <TableHead>Meter</TableHead>
            <TableHead>Register</TableHead>
            <TableHead className='text-right'>Start</TableHead>
            <TableHead className='text-right'>End</TableHead>
            <TableHead className='text-right'>Consumption</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {rows.map((r) => (
            <TableRow key={r.id}>
              <TableCell>
                {r.tenantKey && r.siteKey ? (
                  <Link
                    to='/dashboards'
                    search={{ tenant: r.tenantKey, site: r.siteKey }}
                    className='text-primary hover:underline print:text-current print:no-underline'
                  >
                    {r.siteName}
                  </Link>
                ) : (
                  r.siteName
                )}
              </TableCell>
              <TableCell>
                {r.tenantKey && r.siteKey && r.gatewayKey ? (
                  <Link
                    to='/dashboards'
                    search={{
                      tenant: r.tenantKey,
                      site: r.siteKey,
                      gateway: r.gatewayKey,
                      meter: r.meterId,
                    }}
                    className='text-primary hover:underline print:text-current print:no-underline'
                  >
                    {r.meterName}
                  </Link>
                ) : (
                  r.meterName
                )}
              </TableCell>
              <TableCell>{r.register}</TableCell>
              <TableCell className='text-right font-mono text-xs'>
                {fmt(r.start, r.precision)}
              </TableCell>
              <TableCell className='text-right font-mono text-xs'>
                {fmt(r.end, r.precision)}
              </TableCell>
              <TableCell className='text-right font-mono'>
                {fmt(r.consumption, r.precision)} {r.unit}
                {r.consumption !== null && r.consumption < 0 ? (
                  <span className='text-muted-foreground ml-1 text-xs'>(reset?)</span>
                ) : null}
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </Card>
  )
}
