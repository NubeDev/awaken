/**
 * Gateway dashboard page — status + last_seen, headline KPIs (networks / device
 * utilisation / meters online), a networks panel with per-network capacity bars,
 * and the meter list (drill into a meter page). Pure builder (gateway-board.ts);
 * every number is an honest rollup of the fetched records (no fabricated values).
 */
import { Cable, ChevronRight, Cpu, Gauge, Network as NetworkIcon, Router } from 'lucide-react'
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
  useNetworks,
  useRegisters,
  type HistorySample,
} from '../query/batch'
import type { WindowToken } from '../query/time-window'
import {
  buildGatewayBoard,
  type MeterRow,
  type NetworkRow,
} from '../auto-build/gateway-board'
import { formatValue } from '../_shared/format-value'
import { KpiTile } from '../widgets/kpi-tile'
import { StatusPill } from '../widgets/status-tile'
import { Empty } from '../widgets/empty'

export function GatewayPage({
  gatewayKey,
  window,
  history,
  onOpenMeter,
}: {
  gatewayKey: string
  window: WindowToken
  history: HistorySample[]
  onOpenMeter: (meterId: string, name: string) => void
}) {
  const gateways = useGateways()
  const networks = useNetworks()
  const meters = useMeters()
  const registers = useRegisters()

  if (gateways.isLoading || networks.isLoading || meters.isLoading || registers.isLoading) {
    return <Empty message='Loading…' />
  }

  const board = buildGatewayBoard(
    gatewayKey,
    gateways.data ?? [],
    networks.data ?? [],
    meters.data ?? [],
    registers.data ?? [],
    history,
    window
  )
  if (!board) return <Empty message='Gateway not found' />

  const energyUnit = board.meters.find((m) => m.energy.unit)?.energy.unit

  const { kpis } = board

  return (
    <div className='space-y-4'>
      {/* Status header with gateway metadata (model / host) when present. */}
      <Card className='gap-0 p-4'>
        <div className='flex items-center justify-between gap-3'>
          <div className='flex items-center gap-3'>
            <span className='bg-primary/10 text-primary flex size-10 items-center justify-center rounded-lg'>
              <Router className='size-5' />
            </span>
            <div>
              <div className='text-sm font-medium'>Gateway status</div>
              {(board.model || board.host) && (
                <div className='text-muted-foreground text-xs'>
                  {[board.model, board.host].filter(Boolean).join(' · ')}
                </div>
              )}
            </div>
          </div>
          <StatusPill status={board.status} lastSeen={board.lastSeen} />
        </div>
      </Card>

      {/* KPI strip — honest counts rolled up from the records. */}
      <div className='grid grid-cols-2 gap-3 lg:grid-cols-3'>
        <KpiTile
          icon={NetworkIcon}
          color='var(--chart-2)'
          label='Networks'
          value={kpis.networks}
        />
        <KpiTile
          icon={Cpu}
          color='var(--chart-1)'
          label='Devices'
          value={kpis.devices}
          sub={`of ${kpis.capacity} capacity`}
          bar={{ value: kpis.devices, max: kpis.capacity }}
        />
        <KpiTile
          icon={Gauge}
          color={kpis.metersOnline === kpis.meters ? 'var(--chart-3)' : 'var(--chart-4)'}
          label='Meters online'
          value={`${kpis.metersOnline} / ${kpis.meters}`}
        />
      </div>

      {/* Networks with a per-network capacity bar (count vs max_devices). */}
      <Card className='gap-0 p-4'>
        <div className='mb-3 text-sm font-medium'>Networks</div>
        {board.networks.length === 0 ? (
          <Empty message='No networks' />
        ) : (
          <ul className='divide-y'>
            {board.networks.map((n) => (
              <NetworkRowItem key={n.key} net={n} />
            ))}
          </ul>
        )}
      </Card>

      {/* Meters — status + energy (kWh) sparkline; drill into a meter page. */}
      <Card className='gap-0 overflow-hidden p-0'>
        <div className='border-b px-4 py-3 text-sm font-medium'>Meters</div>
        {board.meters.length === 0 ? (
          <Empty message='No meters' />
        ) : (
          <Table>
            <TableHeader>
              <TableRow className='hover:bg-transparent'>
                <TableHead>Meter</TableHead>
                <TableHead className='w-48'>Energy ({energyUnit ?? 'kWh'})</TableHead>
                <TableHead className='text-right'>Status</TableHead>
                <TableHead className='w-8' />
              </TableRow>
            </TableHeader>
            <TableBody>
              {board.meters.map((m) => (
                <MeterRowItem key={m.id} meter={m} onOpen={() => onOpenMeter(m.id, m.name)} />
              ))}
            </TableBody>
          </Table>
        )}
      </Card>
    </div>
  )
}

function MeterRowItem({ meter, onOpen }: { meter: MeterRow; onOpen: () => void }) {
  return (
    <TableRow className='group cursor-pointer' onClick={onOpen}>
      <TableCell>
        <div className='flex items-center gap-2.5'>
          <span className='bg-muted text-muted-foreground flex size-7 items-center justify-center rounded-md'>
            <Cable className='size-3.5' />
          </span>
          <span className='font-medium'>{meter.name}</span>
        </div>
      </TableCell>
      <TableCell>
        <div className='flex items-center gap-3'>
          <span className='w-20 text-sm font-medium tabular-nums'>
            {formatValue(meter.energy.latest, { precision: 0 })}
          </span>
          <div className='h-8 flex-1'>
            {meter.energy.points.length > 1 ? (
              <Sparkline points={meter.energy.points} color='var(--chart-5)' height={32} />
            ) : (
              <span className='text-muted-foreground text-xs'>—</span>
            )}
          </div>
        </div>
      </TableCell>
      <TableCell className='text-right'>
        <StatusPill status={(meter.status as never) ?? 'unknown'} lastSeen={meter.lastSeen} />
      </TableCell>
      <TableCell>
        <ChevronRight className='text-muted-foreground/40 group-hover:text-muted-foreground size-4 transition-colors' />
      </TableCell>
    </TableRow>
  )
}

function NetworkRowItem({ net }: { net: NetworkRow }) {
  const pct = net.max > 0 ? Math.min(100, Math.round((net.count / net.max) * 100)) : 0
  // Capacity colour ramp: green under 75%, amber 75–99%, red at/over cap.
  const barColor =
    net.count >= net.max ? 'var(--chart-4)' : pct >= 75 ? '#ca8a04' : 'var(--chart-3)'
  return (
    <li className='flex items-center gap-4 py-3'>
      <span className='bg-muted text-muted-foreground flex size-8 shrink-0 items-center justify-center rounded-md'>
        <Cable className='size-4' />
      </span>
      <div className='min-w-0 flex-1'>
        <div className='flex items-center gap-2'>
          <span className='truncate text-sm font-medium'>{net.name}</span>
          <span className='bg-muted text-muted-foreground rounded px-1.5 py-0.5 text-[10px] uppercase'>
            {net.protocol}
          </span>
          <span className='text-muted-foreground text-[10px] uppercase'>{net.type}</span>
        </div>
        {net.detail && <div className='text-muted-foreground text-xs'>{net.detail}</div>}
      </div>
      <div className='w-32 shrink-0'>
        <div className='mb-1 flex items-center justify-between text-xs tabular-nums'>
          <span className='text-muted-foreground'>Devices</span>
          <span className={cn(net.count >= net.max && 'text-destructive font-medium')}>
            {net.count} / {net.max}
          </span>
        </div>
        <div className='bg-muted h-1.5 overflow-hidden rounded-full'>
          <div className='h-full rounded-full' style={{ width: `${pct}%`, backgroundColor: barColor }} />
        </div>
      </div>
    </li>
  )
}
