/**
 * Tenant dashboard page — site cards (DASHBOARDS.md tenant row). Reads the fetched
 * record sets (query/batch.ts is the one fetcher), builds the cards via the pure
 * tenant-board builder, and renders a status pill + meter/alarm counts per site.
 * Clicking a card drills into the site page (onOpenSite).
 */
import { Card } from '@/components/ui/card'
import { useGateways, useMeters, useRegisters, useSites } from '../query/batch'
import { buildTenantBoard } from '../auto-build/tenant-board'
import { StatusPill } from '../widgets/status-tile'
import { Empty } from '../widgets/empty'
import type { HistorySample } from '../query/batch'

export function TenantPage({
  tenantKey,
  history,
  onOpenSite,
}: {
  tenantKey: string
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
    history
  )

  if (cards.length === 0) return <Empty message='No sites for this tenant' />

  return (
    <div className='grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3'>
      {cards.map((c) => (
        <Card
          key={c.key}
          className='hover:border-primary cursor-pointer p-4 transition-colors'
          onClick={() => onOpenSite(c.key, c.name)}
        >
          <div className='flex items-center justify-between'>
            <div className='font-medium'>{c.name}</div>
            <StatusPill status={c.status} />
          </div>
          <div className='text-muted-foreground mt-3 flex gap-4 text-sm'>
            <span>{c.gatewayCount} gateways</span>
            <span>{c.meterCount} meters</span>
            <span className={c.alarmCount > 0 ? 'text-destructive font-medium' : undefined}>
              {c.alarmCount} alarms
            </span>
          </div>
        </Card>
      ))}
    </div>
  )
}
