/**
 * Site dashboard page — gateway cards + a cross-meter quantity:power panel
 * (DASHBOARDS.md site row). Pure builder (site-board.ts) over the fetched records.
 * Clicking a gateway card drills into the gateway page (onOpenGateway).
 */
import { Card } from '@/components/ui/card'
import { useGateways, useMeters, useRegisters, useSites } from '../query/batch'
import type { HistorySample } from '../query/batch'
import { buildSiteBoard } from '../auto-build/site-board'
import type { WindowToken } from '../query/time-window'
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

  return (
    <div className='space-y-4'>
      <div className='grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3'>
        {board.gateways.length === 0 ? (
          <Empty message='No gateways at this site' />
        ) : (
          board.gateways.map((g) => (
            <Card
              key={g.key}
              className='hover:border-primary cursor-pointer p-4 transition-colors'
              onClick={() => onOpenGateway(g.key, g.name)}
            >
              <div className='flex items-center justify-between'>
                <div className='font-medium'>{g.name}</div>
                <StatusPill status={g.status} lastSeen={g.lastSeen} />
              </div>
              <div className='text-muted-foreground mt-3 flex gap-4 text-sm'>
                <span>{g.meterCount} meters</span>
                <span className={g.alarmCount > 0 ? 'text-destructive font-medium' : undefined}>
                  {g.alarmCount} alarms
                </span>
              </div>
            </Card>
          ))
        )}
      </div>
      {board.powerPanel && <TrendPanel widget={board.powerPanel} />}
    </div>
  )
}
