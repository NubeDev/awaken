/**
 * A small badge showing a network's device count against its cap (used/cap), amber
 * when full. Reads the capacity math from capacity.ts (DOMAIN-MODEL "Device
 * limit"). Display only — the block lives in network-form.tsx.
 */
import type { MeterRecord, NetworkRecord } from '@/api/records'
import { Badge } from '@/components/ui/badge'
import { capacityFor } from './capacity'

export function CapacityBadge({
  network,
  meters,
}: {
  network: NetworkRecord
  meters: MeterRecord[]
}) {
  const cap = capacityFor(network, meters)
  return (
    <Badge
      variant='outline'
      className={cap.full ? 'text-amber-600' : 'text-muted-foreground'}
      title={`${cap.used} of ${cap.cap} devices used (${cap.remaining} free)`}
    >
      {cap.used}/{cap.cap}
    </Badge>
  )
}
