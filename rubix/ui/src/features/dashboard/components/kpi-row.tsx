import { Activity, CircuitBoard, Network, Zap } from 'lucide-react'
import { usePointHistory, usePoints, useSparks } from '@/api/hooks'
import type { Point, Uuid } from '@/api/types'
import { formatValue } from '@/lib/format'
import { KpiCard } from './kpi-card'

function isDemandPoint(p: Point): boolean {
  const t = new Set(p.tags)
  return (t.has('elec') || t.has('energy')) && (t.has('kw') || t.has('meter')) && p.unit === 'kW'
}

/** Top KPI strip — figures counted from live API data; trends from real history. */
export function KpiRow({ siteId }: { siteId: Uuid | undefined }) {
  const { data: points = [] } = usePoints({ siteId })
  const { data: sparks = [] } = useSparks(siteId)
  const demand = points.find(isDemandPoint)
  const { data: demandHis = [] } = usePointHistory(demand?.id)

  const open = sparks.filter((s) => !s.acknowledged)
  const faults = open.filter((s) => s.severity === 'fault').length
  const warnings = open.filter((s) => s.severity === 'warning').length

  const demandSpark = demandHis
    .filter((s) => typeof s.value === 'number')
    .map((s) => s.value as number)
    .slice(-20)
  const cmd = points.filter((p) => p.kind !== 'sensor').length

  return (
    <div className='grid gap-4 sm:grid-cols-2 lg:grid-cols-4'>
      <KpiCard
        label='Current Demand'
        value={demand ? formatValue(demand.cur_value) : '—'}
        unit={demand?.unit ?? 'kW'}
        icon={Zap}
        spark={demandSpark}
        sparkColor='var(--chart-1)'
      />
      <KpiCard
        label='Points'
        value={points.length.toLocaleString()}
        unit='total'
        icon={Network}
        sub={`${cmd} writable`}
      />
      <KpiCard
        label='Command Points'
        value={String(cmd)}
        unit='writable'
        icon={Activity}
      />
      <KpiCard
        label='Open Sparks'
        value={String(open.length)}
        unit='active'
        icon={CircuitBoard}
        sub={`${faults} fault · ${warnings} warning`}
      />
    </div>
  )
}
