/**
 * Gateway dashboard page — status + last_seen, and a network list table with
 * device counts vs max_devices (DASHBOARDS.md gateway row). Pure builder
 * (gateway-board.ts). The meters under the gateway are listed so an operator can
 * drill into a meter page (onOpenMeter).
 */
import { Card } from '@/components/ui/card'
import { useGateways, useMeters, useNetworks } from '../query/batch'
import { gatewayTag } from '@/enums/tags'
import { buildGatewayBoard } from '../auto-build/gateway-board'
import { StatusPill } from '../widgets/status-tile'
import { TableTile } from '../widgets/table'
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

  return (
    <div className='space-y-4'>
      <Card className='flex items-center justify-between p-4'>
        <div className='text-sm font-medium'>Gateway status</div>
        <StatusPill status={board.status} lastSeen={board.lastSeen} />
      </Card>

      <Card className='p-4'>
        <div className='mb-2 text-sm font-medium'>{board.networkTable.title}</div>
        <TableTile widget={board.networkTable} />
      </Card>

      <Card className='p-4'>
        <div className='mb-2 text-sm font-medium'>Meters</div>
        {gwMeters.length === 0 ? (
          <Empty message='No meters' />
        ) : (
          <ul className='divide-y'>
            {gwMeters.map((m) => (
              <li
                key={m.id}
                className='hover:bg-muted/50 flex cursor-pointer items-center justify-between py-2 text-sm'
                onClick={() => onOpenMeter(m.id, m.content.name)}
              >
                <span>{m.content.name}</span>
                <StatusPill status={(m.content.status as never) ?? 'unknown'} lastSeen={m.content.last_seen} />
              </li>
            ))}
          </ul>
        )}
      </Card>
    </div>
  )
}
