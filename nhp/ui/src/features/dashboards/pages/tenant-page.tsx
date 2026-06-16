/**
 * Tenant dashboard page — a SITES TABLE (DASHBOARDS.md tenant row): one row per
 * site with a status pill, gateway/meter/alarm counts, and a site-wide energy
 * (kWh) trend sparkline. A portfolio KPI strip rolls the whole tenant up above
 * the table. Pure builder (tenant-board.ts) over the fetched records; clicking a
 * row drills into the site page (onOpenSite). Every number is an honest rollup.
 */
import { Building2, ChevronRight, MapPin, TriangleAlert, Zap } from 'lucide-react'
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
import { SiteMap } from '@/features/sites'
import {
  useGateways,
  useMeters,
  useRegisters,
  useSites,
  type HistorySample,
} from '../query/batch'
import { buildTenantBoard, type SiteCard } from '../auto-build/tenant-board'
import type { WindowToken } from '../query/time-window'
import { formatValue } from '../_shared/format-value'
import { KpiTile } from '../widgets/kpi-tile'
import { StatusPill } from '../widgets/status-tile'
import { Empty } from '../widgets/empty'

export function TenantPage({
  tenantKey,
  window,
  history,
  onOpenSite,
}: {
  tenantKey: string
  window: WindowToken
  history: HistorySample[]
  onOpenSite: (siteKey: string, name: string) => void
}) {
  const sites = useSites()
  const gateways = useGateways()
  const meters = useMeters()
  const registers = useRegisters()

  if (sites.isLoading || gateways.isLoading || meters.isLoading || registers.isLoading) {
    return <Empty message='Loading…' />
  }

  const cards = buildTenantBoard(
    tenantKey,
    sites.data ?? [],
    gateways.data ?? [],
    meters.data ?? [],
    registers.data ?? [],
    history,
    window
  )

  if (cards.length === 0) return <Empty message='No sites for this tenant' />

  // Portfolio rollups for the KPI strip — sums across the tenant's sites.
  const totalMeters = cards.reduce((s, c) => s + c.meterCount, 0)
  const totalGateways = cards.reduce((s, c) => s + c.gatewayCount, 0)
  const totalAlarms = cards.reduce((s, c) => s + c.alarmCount, 0)
  const totalEnergy = cards.reduce((s, c) => s + (c.energy.latest ?? 0), 0)
  const energyUnit = cards.find((c) => c.energy.unit)?.energy.unit

  return (
    <div className='space-y-4'>
      <div className='grid grid-cols-2 gap-3 lg:grid-cols-4'>
        <KpiTile icon={MapPin} color='var(--chart-2)' label='Sites' value={cards.length} />
        <KpiTile icon={Building2} color='var(--chart-1)' label='Meters' value={totalMeters}
          sub={`${totalGateways} gateways`} />
        <KpiTile
          icon={Zap}
          color='var(--chart-5)'
          label='Energy'
          value={totalEnergy > 0 ? formatValue(totalEnergy, { precision: 0, unit: energyUnit }) : '—'}
        />
        <KpiTile
          icon={TriangleAlert}
          color={totalAlarms > 0 ? '#dc2626' : 'var(--chart-3)'}
          label='Active alarms'
          value={totalAlarms}
          alarm={totalAlarms > 0}
        />
      </div>

      <SiteMap tenantKey={tenantKey} height={320} />

      <Card className='gap-0 overflow-hidden p-0'>
        <div className='border-b px-4 py-3 text-sm font-medium'>Sites</div>
        <Table>
          <TableHeader>
            <TableRow className='hover:bg-transparent'>
              <TableHead>Site</TableHead>
              <TableHead>Status</TableHead>
              <TableHead className='text-right'>Gateways</TableHead>
              <TableHead className='text-right'>Meters</TableHead>
              <TableHead className='text-right'>Alarms</TableHead>
              <TableHead className='w-48'>Energy ({energyUnit ?? 'kWh'})</TableHead>
              <TableHead className='w-8' />
            </TableRow>
          </TableHeader>
          <TableBody>
            {cards.map((c) => (
              <SiteRow key={c.key} site={c} onOpen={() => onOpenSite(c.key, c.name)} />
            ))}
          </TableBody>
        </Table>
      </Card>
    </div>
  )
}

function SiteRow({ site, onOpen }: { site: SiteCard; onOpen: () => void }) {
  return (
    <TableRow className='group cursor-pointer' onClick={onOpen}>
      <TableCell>
        <div className='flex items-center gap-2.5'>
          <span className='bg-primary/10 text-primary flex size-8 items-center justify-center rounded-md'>
            <MapPin className='size-4' />
          </span>
          <span className='font-medium'>{site.name}</span>
        </div>
      </TableCell>
      <TableCell>
        <StatusPill status={site.status} />
      </TableCell>
      <TableCell className='text-right tabular-nums'>{site.gatewayCount}</TableCell>
      <TableCell className='text-right tabular-nums'>{site.meterCount}</TableCell>
      <TableCell className='text-right tabular-nums'>
        <span className={cn(site.alarmCount > 0 && 'text-destructive font-medium')}>
          {site.alarmCount}
        </span>
      </TableCell>
      <TableCell>
        <div className='flex items-center gap-3'>
          <span className='w-20 text-sm font-medium tabular-nums'>
            {formatValue(site.energy.latest, { precision: 0 })}
          </span>
          <div className='h-8 flex-1'>
            {site.energy.points.length > 1 ? (
              <Sparkline points={site.energy.points} color='var(--chart-5)' height={32} />
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
