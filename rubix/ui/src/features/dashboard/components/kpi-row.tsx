import { Activity, Network, Zap } from 'lucide-react'
import { usePointHistory, usePoints, useSparks } from '@/api/hooks'
import type { HisSample, Point, Uuid } from '@/api/types'
import { formatValue } from '@/lib/format'
import { KpiCard } from './kpi-card'

function isDemandPoint(p: Point): boolean {
  const t = new Set(p.tags)
  return (t.has('elec') || t.has('energy')) && (t.has('kw') || t.has('meter')) && p.unit === 'kW'
}

/** % change of the last sample vs the sample half a window ago. */
function trendDelta(nums: number[]): { delta: string; dir: 'up' | 'down' } | undefined {
  if (nums.length < 4) return undefined
  const last = nums[nums.length - 1]!
  const prior = nums[Math.floor(nums.length / 2)]!
  if (prior === 0) return undefined
  const pct = ((last - prior) / prior) * 100
  return { delta: `${Math.abs(pct).toFixed(1)}%`, dir: pct >= 0 ? 'up' : 'down' }
}

/** Integrate kW samples over their real timestamps → MWh. */
function energyMWh(samples: HisSample[]): number {
  let wh = 0
  const nums = samples.filter((s) => typeof s.value === 'number')
  for (let i = 1; i < nums.length; i++) {
    const hours = (Date.parse(nums[i]!.ts) - Date.parse(nums[i - 1]!.ts)) / 3_600_000
    wh += (nums[i]!.value as number) * hours
  }
  return wh / 1000
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

  const demandNums = demandHis
    .filter((s) => typeof s.value === 'number')
    .map((s) => s.value as number)
  const demandTrend = trendDelta(demandNums)
  const mwh = energyMWh(demandHis)
  const cmd = points.filter((p) => p.kind !== 'sensor').length

  return (
    <div className='grid gap-4 sm:grid-cols-2 lg:grid-cols-4'>
      <KpiCard
        label='Current Demand'
        value={demand ? formatValue(demand.cur_value) : '—'}
        unit={demand?.unit ?? 'kW'}
        icon={Zap}
        delta={demandTrend?.delta}
        deltaDir={demandTrend?.dir}
        spark={demandNums.slice(-20)}
        sparkColor='var(--chart-1)'
        sub={demand ? `last ${demandHis.length} samples` : 'no demand meter'}
      />
      <KpiCard
        label='Energy · 24h'
        value={mwh > 0 ? mwh.toFixed(1) : '—'}
        unit='MWh'
        icon={Activity}
        spark={demandNums.slice(0, 20)}
        sparkColor='var(--chart-3)'
        sub='integrated from meter history'
      />
      <KpiCard
        label='Points'
        value={points.length.toLocaleString()}
        unit='total'
        icon={Network}
        sub={`${cmd} writable · ${points.length - cmd} sensors`}
      />
      <KpiCard
        label='Open Sparks'
        value={String(open.length)}
        unit='active'
        icon={Zap}
        delta={faults > 0 ? `${faults} fault` : undefined}
        deltaDir={faults > 0 ? 'up' : undefined}
        sub={`${faults} fault · ${warnings} warning`}
      />
    </div>
  )
}
