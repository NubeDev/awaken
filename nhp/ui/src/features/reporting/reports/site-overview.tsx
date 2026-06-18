/**
 * Site Overview report — a single printable document for one site: status + KPIs,
 * active alarms, per-gateway breakdown, energy/power, and a device inventory. A new
 * report VARIANT, not new infrastructure: it reuses the dashboard's pure builders
 * (`buildSiteBoard`, `primaryMetric`), the alarm derivation (`activeAlarms`), and
 * the windowed-history hooks. Site-required — without a site it prompts for one.
 *
 * Every number is an honest rollup of records/readings already fetched for the
 * dashboards (frozen-rubix rule): no new queries, no fabricated values.
 */
import { useMemo } from 'react'
import { Router, Cable, TriangleAlert, Zap } from 'lucide-react'
import { Card } from '@/components/ui/card'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { cn } from '@/lib/utils'
import { Sparkline } from '@/components/charts/sparkline'
import { formatValue } from '@/features/dashboards/_shared/format-value'
import { SEVERITY_COLORS } from '@/features/dashboards/_shared/palette'
import { buildSiteBoard } from '@/features/dashboards/auto-build/site-board'
import { rollupStatus } from '@/features/dashboards/auto-build/rollup'
import { resolveWindow, type WindowToken } from '@/features/dashboards/query/time-window'
import { KpiTile } from '@/features/dashboards/widgets/kpi-tile'
import { StatusPill } from '@/features/dashboards/widgets/status-tile'
import { TrendPanel } from '@/features/dashboards/widgets/widget-host'
import { activeAlarms } from '../alarms'
import { AlarmCounts, AlarmTable } from '../alarm-view'
import {
  selectMeters,
  selectRegisters,
  type PortfolioIndex,
  type ScopeFilter,
} from '../scope'
import { useLatestReadings, useWindowedHistory } from '../use-portfolio'
import { siteInventory } from './site-inventory'

export function SiteOverviewReport({
  index,
  filter,
  token,
}: {
  index: PortfolioIndex
  filter: ScopeFilter
  token: WindowToken
}) {
  const site = filter.siteId
    ? index.data.sites.find((s) => s.id === filter.siteId)
    : undefined

  // Meters/registers under the site (history-bearing, for the board + inventory).
  const siteFilter = useMemo<ScopeFilter>(
    () => ({ tenantId: filter.tenantId, siteId: filter.siteId }),
    [filter.tenantId, filter.siteId]
  )
  const meters = useMemo(() => selectMeters(index, siteFilter), [index, siteFilter])
  const registers = useMemo(
    () => selectRegisters(index, siteFilter, { includeNoHistory: true }),
    [index, siteFilter]
  )
  const history = useWindowedHistory(
    registers.map((r) => ({ id: r.id })),
    token
  )

  // Alarms: same selection the console uses (alarmsOnly + includeNoHistory) so
  // low-battery / gauge alarms surface, evaluated against latest values.
  const alarmRegisters = useMemo(
    () => selectRegisters(index, siteFilter, { alarmsOnly: true, includeNoHistory: true }),
    [index, siteFilter]
  )
  const { latest, isLoading: alarmsLoading } = useLatestReadings(
    alarmRegisters.map((r) => ({ id: r.id }))
  )
  const alarms = useMemo(
    () => activeAlarms(index, alarmRegisters, latest),
    [index, alarmRegisters, latest]
  )

  const board = useMemo(() => {
    if (!site) return null
    return buildSiteBoard(
      site.content.key,
      index.data.gateways,
      index.data.meters,
      index.data.registers,
      history.data,
      token,
      site.content.timezone
    )
  }, [site, index, history.data, token])

  const inventory = useMemo(() => {
    if (!site) return []
    return siteInventory(
      index,
      meters,
      index.data.networks,
      index.data.registers,
      history.data,
      resolveWindow(token)
    )
  }, [site, index, meters, history.data, token])

  if (!filter.siteId || !site) {
    return (
      <Card className='text-muted-foreground p-8 text-center text-sm'>
        Pick a site to generate an overview.
      </Card>
    )
  }
  if (history.isLoading || alarmsLoading || !board) {
    return <Card className='text-muted-foreground p-8 text-center text-sm'>Loading…</Card>
  }

  const { kpis } = board
  const siteStatus = rollupStatus(
    board.gateways.map((g) => ({ status: g.status === 'degraded' ? 'offline' : g.status }))
  )

  return (
    <div className='space-y-4'>
      {/* 1. Header band — address / timezone / status pill */}
      <Card className='report-avoid-break space-y-1 p-4'>
        <div className='flex items-center justify-between gap-3'>
          <h2 className='text-lg font-semibold'>{site.content.name}</h2>
          <StatusPill status={siteStatus} />
        </div>
        <div className='text-muted-foreground grid gap-x-8 gap-y-0.5 text-sm sm:grid-cols-2'>
          {site.content.address ? (
            <span><strong>Address:</strong> {site.content.address}</span>
          ) : null}
          {site.content.timezone ? (
            <span><strong>Timezone:</strong> {site.content.timezone}</span>
          ) : null}
        </div>
      </Card>

      {/* 2. Status + KPI strip */}
      <div className='report-avoid-break grid grid-cols-2 gap-3 lg:grid-cols-4'>
        <KpiTile icon={Router} color='var(--chart-2)' label='Gateways' value={kpis.gateways} />
        <KpiTile icon={Cable} color='var(--chart-1)' label='Meters' value={kpis.meters} />
        <KpiTile
          icon={TriangleAlert}
          color={alarms.length > 0 ? '#dc2626' : 'var(--chart-3)'}
          label='Active alarms'
          value={alarms.length}
          alarm={alarms.length > 0}
        />
        <KpiTile
          icon={Zap}
          color='var(--chart-5)'
          label='Energy'
          value={
            kpis.energy
              ? formatValue(kpis.energy, { precision: 0, unit: kpis.energyUnit })
              : '—'
          }
        />
      </div>

      {/* 3. Active alarms */}
      <div className='report-avoid-break space-y-2'>
        <div className='text-sm font-medium'>Active alarms</div>
        {alarms.length === 0 ? (
          <Card className='text-muted-foreground p-4 text-sm'>No active alarms.</Card>
        ) : (
          <>
            <AlarmCounts alarms={alarms} />
            <AlarmTable alarms={alarms} />
          </>
        )}
      </div>

      {/* 4. Gateways breakdown */}
      <Card className='report-avoid-break gap-0 overflow-hidden p-0'>
        <div className='border-b px-4 py-3 text-sm font-medium'>Gateways</div>
        {board.gateways.length === 0 ? (
          <div className='text-muted-foreground p-4 text-sm'>No gateways at this site.</div>
        ) : (
          <Table>
            <TableHeader>
              <TableRow className='hover:bg-transparent'>
                <TableHead>Gateway</TableHead>
                <TableHead>Status</TableHead>
                <TableHead className='text-right'>Meters</TableHead>
                <TableHead className='text-right'>Alarms</TableHead>
                <TableHead className='w-48'>Energy ({kpis.energyUnit ?? 'kWh'})</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {board.gateways.map((g) => (
                <TableRow key={g.key}>
                  <TableCell className='font-medium'>{g.name}</TableCell>
                  <TableCell>
                    <StatusPill status={g.status} lastSeen={g.lastSeen} />
                  </TableCell>
                  <TableCell className='text-right tabular-nums'>{g.meterCount}</TableCell>
                  <TableCell className='text-right tabular-nums'>
                    <span className={cn(g.alarmCount > 0 && 'text-destructive font-medium')}>
                      {g.alarmCount}
                    </span>
                  </TableCell>
                  <TableCell>
                    <div className='flex items-center gap-3'>
                      <span className='w-20 text-sm font-medium tabular-nums'>
                        {formatValue(g.energy.latest, { precision: 0 })}
                      </span>
                      <div className='h-8 flex-1'>
                        {g.energy.points.length > 1 ? (
                          <Sparkline points={g.energy.points} color='var(--chart-5)' height={32} />
                        ) : (
                          <span className='text-muted-foreground text-xs'>—</span>
                        )}
                      </div>
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </Card>

      {/* 5. Energy / power panel — degrades cleanly when there's no power */}
      {board.powerPanel ? (
        <div className='report-avoid-break'>
          <TrendPanel widget={board.powerPanel} />
        </div>
      ) : null}

      {/* 6. Device inventory */}
      <Card className='report-avoid-break gap-0 overflow-hidden p-0'>
        <div className='border-b px-4 py-3 text-sm font-medium'>Device inventory</div>
        {inventory.length === 0 ? (
          <div className='text-muted-foreground p-4 text-sm'>No meters at this site.</div>
        ) : (
          <Table>
            <TableHeader>
              <TableRow className='hover:bg-transparent'>
                <TableHead>Meter</TableHead>
                <TableHead>Gateway</TableHead>
                <TableHead>Protocol</TableHead>
                <TableHead>Type</TableHead>
                <TableHead className='text-right'>Reading</TableHead>
                <TableHead>Status</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {inventory.map((row) => (
                <TableRow key={row.meterId}>
                  <TableCell className='font-medium'>{row.meterName}</TableCell>
                  <TableCell className='text-muted-foreground'>{row.gatewayKey ?? '—'}</TableCell>
                  <TableCell className='uppercase'>{row.protocol}</TableCell>
                  <TableCell>{row.meterType}</TableCell>
                  <TableCell
                    className='text-right font-mono tabular-nums'
                    style={
                      row.metric.severity !== 'ok'
                        ? { color: SEVERITY_COLORS[row.metric.severity] }
                        : undefined
                    }
                  >
                    {row.metric.latest === null
                      ? '—'
                      : formatValue(row.metric.latest, {
                          precision: row.metric.precision,
                          unit: row.metric.unit,
                        })}
                  </TableCell>
                  <TableCell className='capitalize'>{row.status}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </Card>
    </div>
  )
}
