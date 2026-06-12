import { Activity, Thermometer, Zap } from 'lucide-react'
import { usePointHistory, usePoints, useSparks } from '@/api/hooks'
import type { HisSample, Point, Uuid } from '@/api/types'
import { hasTag } from '@/api/tags'
import { formatValue } from '@/lib/format'
import { KpiCard } from './kpi-card'

function isDemandPoint(p: Point): boolean {
  return hasTag(p.tags, 'meter') || (hasTag(p.tags, 'kw') && p.unit === 'kW')
}

/** % change of the last sample vs 24h earlier. */
function dayDelta(samples: HisSample[]): { delta: string; dir: 'up' | 'down' } | undefined {
  const nums = samples.filter((s) => typeof s.value === 'number')
  if (nums.length < 50) return undefined
  const last = nums[nums.length - 1]!.value as number
  const prior = nums[nums.length - 49]!.value as number
  if (prior === 0) return undefined
  const pct = ((last - prior) / prior) * 100
  return { delta: `${Math.abs(pct).toFixed(1)}%`, dir: pct >= 0 ? 'up' : 'down' }
}

/** Integrate the last 24h of kW samples over their timestamps → MWh. */
function energyTodayMWh(samples: HisSample[]): number {
  const nums = samples.filter((s) => typeof s.value === 'number')
  const dayAgo = nums.length ? Date.parse(nums[nums.length - 1]!.ts) - 24 * 3_600_000 : 0
  let wh = 0
  for (let i = 1; i < nums.length; i++) {
    if (Date.parse(nums[i]!.ts) < dayAgo) continue
    const hours = (Date.parse(nums[i]!.ts) - Date.parse(nums[i - 1]!.ts)) / 3_600_000
    wh += (nums[i]!.value as number) * hours
  }
  return wh / 1000
}

const sparkOf = (samples: HisSample[], n = 20) =>
  samples
    .filter((s) => typeof s.value === 'number')
    .slice(-n)
    .map((s) => s.value as number)

/** Top KPI strip — figures counted from live API data; trends from real history. */
export function KpiRow({ siteId }: { siteId: Uuid | undefined }) {
  const { data: points = [] } = usePoints({ siteId })
  const { data: sparks = [] } = useSparks(siteId)
  const demand = points.find((p) => isDemandPoint(p) && p.slug.includes('total'))
    ?? points.find(isDemandPoint)
  const comfort = points.find((p) => hasTag(p.tags, 'comfort'))
  const { data: demandHis = [] } = usePointHistory(demand?.id)
  const { data: comfortHis = [] } = usePointHistory(comfort?.id)

  const open = sparks.filter((s) => !s.acknowledged)
  const faults = open.filter((s) => s.severity === 'fault').length
  const warnings = open.filter((s) => s.severity === 'warning').length
  const fresh = open.filter((s) => Date.now() - Date.parse(s.ts) < 60 * 60_000).length

  const demandTrend = dayDelta(demandHis)
  const comfortTrend = dayDelta(comfortHis)
  const mwh = energyTodayMWh(demandHis)

  return (
    <div className='grid gap-4 sm:grid-cols-2 xl:grid-cols-4'>
      <KpiCard
        label='Current Demand'
        value={demand ? formatValue(demand.cur_value) : '—'}
        unit={demand?.unit ?? 'kW'}
        icon={Zap}
        delta={demandTrend?.delta}
        deltaDir={demandTrend?.dir}
        spark={sparkOf(demandHis)}
        sparkColor='var(--chart-1)'
        sub='vs 24h earlier'
      />
      <KpiCard
        label='Energy Today'
        value={mwh > 0 ? mwh.toFixed(1) : '—'}
        unit='MWh'
        icon={Activity}
        spark={sparkOf(demandHis, 40)}
        sparkColor='var(--chart-3)'
        sub='integrated from meter history'
      />
      <KpiCard
        label='Comfort Index'
        value={comfort ? formatValue(comfort.cur_value) : '—'}
        unit='%'
        icon={Thermometer}
        delta={comfortTrend?.delta}
        deltaDir={comfortTrend?.dir}
        spark={sparkOf(comfortHis)}
        sparkColor='var(--chart-2)'
        sub={comfort ? comfort.display_name : 'no comfort sensor'}
      />
      <KpiCard
        label='Open Sparks'
        value={String(open.length)}
        unit='active'
        icon={Zap}
        delta={fresh > 0 ? `${fresh} new` : undefined}
        deltaDir={fresh > 0 ? 'up' : undefined}
        sub={`${faults} fault · ${warnings} warning${warnings === 1 ? '' : 's'}`}
      />
    </div>
  )
}
