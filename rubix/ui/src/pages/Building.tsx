// Building & Zones — the live floor plan. Reads the tenant's real records,
// derives HVAC zones for the active site, and renders the heat-mapped rows +
// comfort donut. Ported from screens.js `building()`.

import { getRouteApi } from '@tanstack/react-router'
import { useRecords } from '../hooks/useRecords'
import { toPoints, toSites, toZones } from '../utils/derive'
import { TopBar } from '../components/ui/TopBar'
import { ZoneRow } from '../components/data/ZoneRow'
import { Donut } from '../components/viz/Donut'
import { ErrorView, LoadingView, EmptyView } from '../components/ui/StateView'

const route = getRouteApi('/t/$tenant/building')

export function Building() {
  const { tenant } = route.useParams()
  const { site } = route.useSearch()
  const { data: records, isLoading, error } = useRecords(tenant)

  const sites = records ? toSites(records) : []
  const activeSite = site ?? sites[0]?.key
  const siteName = sites.find((s) => s.key === activeSite)?.name
  const zones = records ? toZones(records, activeSite) : []
  const livePoints = records ? toPoints(records).filter((p) => !activeSite || p.site === activeSite).length : undefined
  const maxLoad = Math.max(1, ...zones.map((z) => z.load))

  const counts = {
    green: zones.filter((z) => z.severity === 'green').length,
    amber: zones.filter((z) => z.severity === 'amber').length,
    crit: zones.filter((z) => z.severity === 'crit').length,
  }
  const inBand = counts.green
  const total = zones.length || 1
  const segments = [
    { pct: (counts.green / total) * 100, color: 'green' },
    { pct: (counts.amber / total) * 100, color: 'amber' },
    { pct: (counts.crit / total) * 100, color: 'crit' },
  ]

  return (
    <div className="h-full flex flex-col">
      <TopBar tenant={tenant} site={activeSite} siteName={siteName} crumbs={['Building & Zones']} livePoints={livePoints} />
      <div className="flex-1 overflow-auto p-6">
        {isLoading && <LoadingView label="Reading zones…" />}
        {error && <ErrorView error={error} />}
        {records && (
          <div className="max-w-[1080px] mx-auto grid grid-cols-[1fr_300px] gap-5">
            <div className="rounded-2xl border border-border bg-panel2 overflow-hidden">
              <div className="px-5 py-3.5 border-b border-border flex items-center justify-between">
                <div className="font-semibold text-[15px]">Live floor plan</div>
                <div className="text-[12px] text-muted">
                  {zones.length} zones · temperature
                </div>
              </div>
              {zones.length === 0 ? (
                <div className="p-6">
                  <EmptyView title="No HVAC zones for this site" hint="Zones derive from kind:'equip' records in the hvac domain." />
                </div>
              ) : (
                <div className="divide-y divide-border">
                  {zones.map((z) => (
                    <ZoneRow key={z.id} zone={z} maxLoad={maxLoad} />
                  ))}
                </div>
              )}
            </div>

            <div className="space-y-4">
              <div className="rounded-2xl border border-border bg-panel2 p-5">
                <div className="font-semibold text-[15px] mb-3">Comfort</div>
                <div className="flex items-center gap-4">
                  <div className="relative">
                    <Donut segments={segments} size={96} />
                    <div className="absolute inset-0 grid place-content-center text-center">
                      <div className="mono text-[18px] font-semibold">
                        {inBand}/{zones.length}
                      </div>
                      <div className="text-[10px] text-muted">in band</div>
                    </div>
                  </div>
                  <div className="space-y-1.5 text-[12.5px] flex-1">
                    <Legend swatch="bg-green" label="Optimal" value={counts.green} />
                    <Legend swatch="bg-amber" label="Warm" value={counts.amber} />
                    <Legend swatch="bg-crit" label="Fault" value={counts.crit} />
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}

function Legend({ swatch, label, value }: { swatch: string; label: string; value: number }) {
  return (
    <div className="flex items-center gap-2">
      <span className={`size-2 rounded ${swatch}`} />
      {label}
      <b className="ml-auto mono">{value}</b>
    </div>
  )
}
