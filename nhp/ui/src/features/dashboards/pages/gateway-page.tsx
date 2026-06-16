/**
 * Gateway dashboard page — status + last_seen, headline KPIs (networks / device
 * utilisation / meters online), a networks panel with per-network capacity bars,
 * and the meter list (drill into a meter page). Pure builder (gateway-board.ts);
 * every number is an honest rollup of the fetched records (no fabricated values).
 */
import { Cable, Cpu, Gauge, Network as NetworkIcon, Router } from 'lucide-react'
import { Card } from '@/components/ui/card'
import { cn } from '@/lib/utils'
import { useGateways, useMeters, useNetworks } from '../query/batch'
import { gatewayTag } from '@/enums/tags'
import { buildGatewayBoard, type NetworkRow } from '../auto-build/gateway-board'
import { KpiTile } from '../widgets/kpi-tile'
import { StatusPill } from '../widgets/status-tile'
import { Empty } from '../widgets/empty'

export function GatewayPage({
  gatewayKey,
  onOpenMeter,
}: {
  gatewayKey: string
  onOpenMeter: (meterId: string, name: string) => void
}) {
  const gateways = useGateways()
  const networks = useNetworks()
  const meters = useMeters()

  if (gateways.isLoading || networks.isLoading || meters.isLoading) {
    return <Empty message='Loading…' />
  }

  const board = buildGatewayBoard(
    gatewayKey,
    gateways.data ?? [],
    networks.data ?? [],
    meters.data ?? []
  )
  if (!board) return <Empty message='Gateway not found' />

  const gTag = gatewayTag(gatewayKey)
  const gwMeters = (meters.data ?? [])
    .filter((m) => (m.content.tags ?? []).includes(gTag))
    .sort((a, b) => a.content.name.localeCompare(b.content.name))

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

      {/* Meters — drill into a meter page. */}
      <Card className='gap-0 p-4'>
        <div className='mb-3 text-sm font-medium'>Meters</div>
        {gwMeters.length === 0 ? (
          <Empty message='No meters' />
        ) : (
          <ul className='divide-y'>
            {gwMeters.map((m) => (
              <li
                key={m.id}
                className='hover:bg-muted/50 -mx-2 flex cursor-pointer items-center justify-between rounded-md px-2 py-2.5 text-sm transition-colors'
                onClick={() => onOpenMeter(m.id, m.content.name)}
              >
                <span className='flex items-center gap-2.5'>
                  <span className='bg-muted text-muted-foreground flex size-7 items-center justify-center rounded-md'>
                    <Cable className='size-3.5' />
                  </span>
                  <span className='font-medium'>{m.content.name}</span>
                </span>
                <StatusPill
                  status={(m.content.status as never) ?? 'unknown'}
                  lastSeen={m.content.last_seen}
                />
              </li>
            ))}
          </ul>
        )}
      </Card>
    </div>
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
