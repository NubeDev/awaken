import { Activity, CircuitBoard, Network, Zap } from 'lucide-react'
import { useEquips, usePoints, useSparks } from '@/api/hooks'
import type { Uuid } from '@/api/types'
import { KpiCard } from './kpi-card'

/** Top KPI strip — every figure is counted from live API data for the site. */
export function KpiRow({ siteId }: { siteId: Uuid | undefined }) {
  const { data: equips = [] } = useEquips(siteId)
  const { data: points = [] } = usePoints({ siteId })
  const { data: sparks = [] } = useSparks(siteId)

  const open = sparks.filter((s) => !s.acknowledged)
  const faults = open.filter((s) => s.severity === 'fault').length
  const warnings = open.filter((s) => s.severity === 'warning').length

  return (
    <div className='grid gap-4 sm:grid-cols-2 lg:grid-cols-4'>
      <KpiCard label='Equipment' value={String(equips.length)} unit='units' icon={CircuitBoard} />
      <KpiCard label='Points' value={points.length.toLocaleString()} unit='total' icon={Network} />
      <KpiCard
        label='Command Points'
        value={String(points.filter((p) => p.kind !== 'sensor').length)}
        unit='writable'
        icon={Activity}
      />
      <KpiCard
        label='Open Sparks'
        value={String(open.length)}
        unit='active'
        icon={Zap}
        sub={`${faults} fault · ${warnings} warning`}
      />
    </div>
  )
}
