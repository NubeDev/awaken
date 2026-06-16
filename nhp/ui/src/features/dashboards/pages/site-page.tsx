/**
 * Site dashboard page — a portfolio KPI strip, a GATEWAYS TABLE (status, meter/
 * alarm counts, a per-gateway energy sparkline) and the cross-meter power panel
 * (DASHBOARDS.md site row). Pure builder (site-board.ts) over the fetched records;
 * clicking a row drills into the gateway page (onOpenGateway). Honest rollups.
 */
import { ChevronRight, Router, TriangleAlert, Cable, Zap } from 'lucide-react'
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
import {
  useGateways,
  useMeters,
  useRegisters,
  useSites,
  type HistorySample,
} from '../query/batch'
import { buildSiteBoard, type GatewayCard } from '../auto-build/site-board'
import type { WindowToken } from '../query/time-window'
import { formatValue } from '../_shared/format-value'
import { KpiTile } from '../widgets/kpi-tile'
import { StatusPill } from '../widgets/status-tile'
import { TrendPanel } from '../widgets/widget-host'
import { Empty } from '../widgets/empty'

export function SitePage({
  siteKey,
  window,
  history,
  onOpenGateway,
}: {
  siteKey: string
  window: WindowToken
  history: HistorySample[]
  onOpenGateway: (gatewayKey: string, name: string) => void
}) {
  const sites = useSites()
  const gateways = useGateways()
  const meters = useMeters()
  const registers = useRegisters()

  const site = (sites.data ?? []).find((s) => s.content.key === siteKey)
  const timezone = site?.content.timezone as string | undefined

  if (gateways.isLoading || meters.isLoading || registers.isLoading) {
    return <Empty message='Loading…' />
  }

  const board = buildSiteBoard(
    siteKey,
    gateways.data ?? [],
    meters.data ?? [],
    registers.data ?? [],
    history,
    window,
    timezone
  )
  const { kpis } = board

  return (
    <div className='space-y-4'>
      <div className='grid grid-cols-2 gap-3 lg:grid-cols-4'>
        <KpiTile icon={Router} color='var(--chart-2)' label='Gateways' value={kpis.gateways} />
        <KpiTile icon={Cable} color='var(--chart-1)' label='Meters' value={kpis.meters} />
        <KpiTile
          icon={Zap}
          color='var(--chart-5)'
          label='Energy'
          value={kpis.energy ? formatValue(kpis.energy, { precision: 0, unit: kpis.energyUnit }) : '—'}
        />
        <KpiTile
          icon={TriangleAlert}
          color={kpis.alarms > 0 ? '#dc2626' : 'var(--chart-3)'}
          label='Active alarms'
          value={kpis.alarms}
          alarm={kpis.alarms > 0}
        />
      </div>

      <Card className='gap-0 overflow-hidden p-0'>
        <div className='border-b px-4 py-3 text-sm font-medium'>Gateways</div>
        {board.gateways.length === 0 ? (
          <Empty message='No gateways at this site' />
        ) : (
          <Table>
            <TableHeader>
              <TableRow className='hover:bg-transparent'>
                <TableHead>Gateway</TableHead>
                <TableHead>Status</TableHead>
                <TableHead className='text-right'>Meters</TableHead>
                <TableHead className='text-right'>Alarms</TableHead>
                <TableHead className='w-48'>Energy ({kpis.energyUnit ?? 'kWh'})</TableHead>
                <TableHead className='w-8' />
              </TableRow>
            </TableHeader>
            <TableBody>
              {board.gateways.map((g) => (
                <GatewayRow key={g.key} gw={g} onOpen={() => onOpenGateway(g.key, g.name)} />
              ))}
            </TableBody>
          </Table>
        )}
      </Card>

      {board.powerPanel && <TrendPanel widget={board.powerPanel} />}
    </div>
  )
}

function GatewayRow({ gw, onOpen }: { gw: GatewayCard; onOpen: () => void }) {
  return (
    <TableRow className='group cursor-pointer' onClick={onOpen}>
      <TableCell>
        <div className='flex items-center gap-2.5'>
          <span className='bg-primary/10 text-primary flex size-8 items-center justify-center rounded-md'>
            <Router className='size-4' />
          </span>
          <span className='font-medium'>{gw.name}</span>
        </div>
      </TableCell>
      <TableCell>
        <StatusPill status={gw.status} lastSeen={gw.lastSeen} />
      </TableCell>
      <TableCell className='text-right tabular-nums'>{gw.meterCount}</TableCell>
      <TableCell className='text-right tabular-nums'>
        <span className={cn(gw.alarmCount > 0 && 'text-destructive font-medium')}>
          {gw.alarmCount}
        </span>
      </TableCell>
      <TableCell>
        <div className='flex items-center gap-3'>
          <span className='w-20 text-sm font-medium tabular-nums'>
            {formatValue(gw.energy.latest, { precision: 0 })}
          </span>
          <div className='h-8 flex-1'>
            {gw.energy.points.length > 1 ? (
              <Sparkline points={gw.energy.points} color='var(--chart-5)' height={32} />
            ) : (
              <span className='text-muted-foreground text-xs'>—</span>
            )}
          </div>
        </div>
      </TableCell>
      <TableCell>
        <ChevronRight className='text-muted-foreground/40 group-hover:text-muted-foreground size-4 transition-colors' />
      </TableCell>
    </TableRow>
  )
}
