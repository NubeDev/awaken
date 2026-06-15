// Ask Rubix — the copilot surface: a thread on the left, an impact-ranked
// attention queue on the right, an ask bar below (PRODUCT-UI "The agent is the
// front door"). The queue is built from REAL derived attention (zones out of
// band), not invented moments. The agent runtime isn't wired yet, so the ask bar
// answers by grounding in the tenant's actual records rather than faking an LLM;
// actuate-class actions are shown disabled per the documented backend gap.

import { useMemo, useState } from 'react'
import { getRouteApi, useNavigate } from '@tanstack/react-router'
import { ArrowUp } from 'lucide-react'
import { useRecords } from '../hooks/useRecords'
import { pointIdFor, readingSeries, toSites, toZones } from '../utils/derive'
import { TopBar } from '../components/ui/TopBar'
import { Orb } from '../components/ui/Orb'
import { Line } from '../components/viz/Line'
import { sevMap } from '../components/ui/severity'
import { ErrorView, LoadingView } from '../components/ui/StateView'
import { fmtDeviation, fmtTemp } from '../utils/format'
import type { Record } from '../types/Record'
import type { Zone } from '../types/Domain'

const route = getRouteApi('/t/$tenant/copilot')

interface Turn {
  role: 'user' | 'rubix'
  text: string
  zone?: Zone
  series?: number[]
}

export function Copilot() {
  const { tenant } = route.useParams()
  const { site } = route.useSearch()
  const navigate = useNavigate()
  const { data: records, isLoading, error } = useRecords(tenant)

  const sites = records ? toSites(records) : []
  const activeKey = site ?? sites[0]?.key
  const siteName = sites.find((s) => s.key === activeKey)?.name
  const zones = useMemo(() => (records ? toZones(records, activeKey) : []), [records, activeKey])
  const queue = useMemo(
    () => zones.filter((z) => z.severity === 'crit' || z.severity === 'amber'),
    [zones],
  )

  const [thread, setThread] = useState<Turn[]>([])
  const [input, setInput] = useState('')

  const seed: Turn = {
    role: 'rubix',
    text: queue.length
      ? `${queue.length} ${queue.length === 1 ? 'zone needs' : 'zones need'} you at ${siteName ?? 'this site'}. Pick one from the queue and I’ll show the trend, or ask me about any record.`
      : `${siteName ?? 'This site'} is calm — every zone is in band. Ask me about any device, point or record.`,
  }
  const turns = [seed, ...thread]

  function focusZone(zone: Zone) {
    if (!records) return
    const pid = pointIdFor(records, zone.key, zone.site, 'temp')
    const series = pid ? readingSeries(records, pid) : []
    setThread((t) => [
      ...t,
      { role: 'user', text: `What’s happening at ${zone.name}?` },
      {
        role: 'rubix',
        text: `${zone.name} is at ${fmtTemp(zone.temp)} against a ${fmtTemp(zone.sp)} setpoint — ${fmtDeviation(
          zone.temp,
          zone.sp,
        )}° off. Here’s its trailing temperature.`,
        zone,
        series,
      },
    ])
  }

  function ask() {
    const q = input.trim()
    if (!q || !records) return
    setInput('')
    setThread((t) => [...t, { role: 'user', text: q }, { role: 'rubix', text: groundedAnswer(q, records) }])
  }

  return (
    <div className="h-full flex flex-col">
      <TopBar tenant={tenant} site={activeKey} siteName={siteName} crumbs={['Ask Rubix']} livePoints={records?.length} />
      {isLoading && <LoadingView label="Reading the site…" />}
      {error && <ErrorView error={error} />}
      {records && (
        <>
          <div className="flex-1 grid grid-cols-[1fr_372px] gap-5 px-8 pb-2 min-h-0">
            <main className="min-h-0 overflow-auto space-y-5 pt-4 pr-2">
              {turns.map((t, i) => (
                <ThreadTurn key={i} turn={t} onOpenBuilding={() => navigate({ to: '/t/$tenant/building', params: { tenant }, search: { site: activeKey } })} />
              ))}
            </main>
            <aside className="min-h-0 flex flex-col pt-4">
              <div className="flex items-center justify-between mb-2.5">
                <div className="text-[12px] uppercase tracking-[.12em] text-muted font-medium">
                  {queue.length ? 'Rubix lined up · by impact' : 'Nothing needs you'}
                </div>
                <span className="text-[11px] text-muted mono">{queue.length ? `${queue.length} open` : 'all clear'}</span>
              </div>
              <div className="space-y-2.5 overflow-auto flex-1 pr-1">
                {queue.length === 0 ? (
                  <div className="rounded-2xl border border-green/25 bg-green/[.05] p-6 text-center">
                    <div className="text-[14px] font-semibold mt-1">{siteName ?? 'This site'} is calm</div>
                    <div className="text-[12.5px] text-muted mt-1 leading-snug">Every zone in band.</div>
                  </div>
                ) : (
                  queue.map((z) => <QueueItem key={z.id} zone={z} onClick={() => focusZone(z)} />)
                )}
              </div>
            </aside>
          </div>

          <div className="shrink-0 px-8 pb-6 pt-2">
            <div className="relative">
              <div className="absolute left-4 top-1/2 -translate-y-1/2">
                <Orb size={24} />
              </div>
              <input
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && ask()}
                placeholder="Ask Rubix about any zone, device or record…"
                className="w-full h-[52px] rounded-2xl border border-border bg-panel2 pl-12 pr-28 text-[15px] outline-none placeholder:text-muted focus:border-r1/50 focus:ring-4 focus:ring-r1/10 transition"
              />
              <button
                onClick={ask}
                className="absolute right-3 top-1/2 -translate-y-1/2 h-9 px-3.5 rounded-xl bg-fg text-bg text-[13px] font-semibold flex items-center gap-1.5 hover:opacity-90 transition"
              >
                Ask
                <ArrowUp size={16} />
              </button>
            </div>
          </div>
        </>
      )}
    </div>
  )
}

function QueueItem({ zone, onClick }: { zone: Zone; onClick: () => void }) {
  const s = sevMap[zone.severity]
  return (
    <button
      onClick={onClick}
      className="qitem w-full text-left rounded-2xl border border-border bg-panel2 hover:bg-panel3 p-4 flex items-start gap-3.5"
    >
      <div className="size-10 rounded-xl grid place-items-center shrink-0" style={{ background: `hsl(var(--${s.c}) / .12)` }}>
        <span className="size-2.5 rounded-full" style={{ background: `hsl(var(--${s.c}))` }} />
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="size-1.5 rounded-full shrink-0" style={{ background: `hsl(var(--${s.c}))` }} />
          <span className="text-[11px] font-medium" style={{ color: `hsl(var(--${s.c}))` }}>
            {s.label}
          </span>
          <span className="text-[11px] text-muted mono ml-auto">{fmtDeviation(zone.temp, zone.sp)}°</span>
        </div>
        <div className="text-[14px] font-semibold mt-1 leading-tight">{zone.name}</div>
        <div className="text-[12.5px] text-muted mt-1 leading-snug">
          {fmtTemp(zone.temp)} vs {fmtTemp(zone.sp)} setpoint.
        </div>
      </div>
    </button>
  )
}

function ThreadTurn({ turn, onOpenBuilding }: { turn: Turn; onOpenBuilding: () => void }) {
  if (turn.role === 'user') {
    return (
      <div className="flex justify-end fade">
        <div className="max-w-[72%] rounded-2xl rounded-br-sm bg-panel3 border border-border px-4 py-2.5 text-[14px] text-fg/90">
          {turn.text}
        </div>
      </div>
    )
  }
  return (
    <div className="flex gap-3 fade">
      <Orb size={32} sparkle />
      <div className="flex-1 min-w-0 space-y-3">
        <p className="serif text-[17px] leading-relaxed text-fg/88">{turn.text}</p>
        {turn.series && turn.series.length > 1 && (
          <div className="rounded-2xl border border-border bg-bg/40 p-4">
            <Line series={[{ data: turn.series, color: turn.zone?.severity ?? 'r1', fill: true }]} height={150} />
          </div>
        )}
        {turn.zone && (
          <div className="flex flex-wrap items-center gap-2 pt-1">
            <button onClick={onOpenBuilding} className="h-10 px-4 rounded-xl text-[13.5px] font-medium border border-border hover:bg-panel3 transition">
              Open Building & Zones
            </button>
            <button
              disabled
              title="Actuation has no backend plane yet (device-actuate + egress) — PRODUCT-UI backend gap"
              className="h-10 px-4 rounded-xl text-[13.5px] font-medium border border-border text-muted opacity-50 cursor-not-allowed"
            >
              Adjust setpoint · coming
            </button>
          </div>
        )}
      </div>
    </div>
  )
}

// A grounded answer over the actual records: counts what matches the query and
// names a few hits. No fabricated metrics — this is honest until the agent
// runtime lands.
function groundedAnswer(q: string, records: Record[]): string {
  const ql = q.toLowerCase()
  const hits = records.filter(
    (r) => r.id.toLowerCase().includes(ql) || JSON.stringify(r.content).toLowerCase().includes(ql),
  )
  if (hits.length === 0) {
    return `I couldn’t find anything matching “${q}” in this tenant’s ${records.length} records. Try a device name, a measure like “temp”, or a site key.`
  }
  const names = hits
    .slice(0, 4)
    .map((r) => (typeof r.content.name === 'string' ? r.content.name : r.id))
    .join(', ')
  return `Found ${hits.length} record${hits.length === 1 ? '' : 's'} matching “${q}” — including ${names}. Open Admin · Records to inspect them, or ask about a specific zone.`
}
