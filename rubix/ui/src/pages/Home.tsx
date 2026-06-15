// Home hub — the launcher you land on after opening a site. Real vitals derived
// from the tenant's records, the Rubix attention banner, and the management
// menu. Ported from screens.js `home()`.

import { getRouteApi, Link, useNavigate } from '@tanstack/react-router'
import {
  Building2,
  Cpu,
  Database,
  FileBarChart,
  GitBranch,
  LayoutGrid,
  Settings,
  Sparkles,
  Table2,
  ArrowRight,
} from 'lucide-react'
import { useRecords } from '../hooks/useRecords'
import { toPoints, toSites, toZones } from '../utils/derive'
import { usePageHeader } from '../components/shell/page-header'
import { StatCard, type Stat } from '../components/ui/StatCard'
import { Orb } from '../components/ui/Orb'
import { ErrorView, LoadingView } from '../components/ui/StateView'
import { siteGradient } from '../utils/gradient'

const route = getRouteApi('/t/$tenant/')

type Dest =
  | { to: '/t/$tenant/building' }
  | { to: '/t/$tenant/copilot' }
  | { to: '/t/$tenant/admin' }
  | { to: '/t/$tenant/$page'; page: string }

interface MenuItem {
  label: string
  sub: string
  icon: typeof Building2
  dest: Dest
  accent?: boolean
}

export function Home() {
  const { tenant } = route.useParams()
  const { site } = route.useSearch()
  const navigate = useNavigate()
  const { data: records, isLoading, error } = useRecords(tenant)

  const sites = records ? toSites(records) : []
  const activeKey = site ?? sites[0]?.key
  const active = sites.find((s) => s.key === activeKey)
  const siteIdx = sites.findIndex((s) => s.key === activeKey)
  const zones = records ? toZones(records, activeKey) : []
  const points = records ? toPoints(records).filter((p) => !activeKey || p.site === activeKey) : []
  const inBand = zones.filter((z) => z.severity === 'green').length
  const alerts = zones.filter((z) => z.severity === 'crit' || z.severity === 'amber').length

  const vitals: Stat[] = [
    { label: 'Live points', value: String(points.length) },
    { label: 'Equipment', value: String(active?.equips ?? 0) },
    { label: 'Zones in band', value: `${inBand}/${zones.length}`, tone: alerts ? 'amber' : 'green' },
    { label: 'Need attention', value: String(alerts), tone: alerts ? 'amber' : 'green' },
  ]

  usePageHeader({ site: activeKey, siteName: active?.name, livePoints: points.length || undefined })

  const menu: MenuItem[] = [
    { label: 'Building & Zones', sub: `${zones.length} zones`, icon: Building2, dest: { to: '/t/$tenant/building' } },
    { label: 'Ask Rubix', sub: alerts ? `${alerts} need you` : 'all calm', icon: Sparkles, dest: { to: '/t/$tenant/copilot' }, accent: true },
    { label: 'Admin Console', sub: `${records?.length ?? 0} records`, icon: Table2, dest: { to: '/t/$tenant/admin' } },
    { label: 'Devices', sub: `${active?.equips ?? 0} equipment`, icon: Cpu, dest: { to: '/t/$tenant/$page', page: 'devices' } },
    { label: 'Data Sources', sub: 'connectors', icon: Database, dest: { to: '/t/$tenant/$page', page: 'data' } },
    { label: 'Rules', sub: 'automations', icon: GitBranch, dest: { to: '/t/$tenant/$page', page: 'rules' } },
    { label: 'Reports', sub: 'scheduled', icon: FileBarChart, dest: { to: '/t/$tenant/$page', page: 'reports' } },
    { label: 'Settings', sub: 'Site & team', icon: Settings, dest: { to: '/t/$tenant/$page', page: 'settings' } },
  ]

  return (
    <div className="px-10 py-8">
      {isLoading && <LoadingView label="Opening site…" />}
      {error && <ErrorView error={error} />}
      {records && (
        <div className="max-w-[1080px] mx-auto">
          <div className="flex items-start gap-5">
            <div
              className="size-14 rounded-2xl grid place-items-center text-white shrink-0"
              style={{ background: siteGradient(siteIdx < 0 ? 0 : siteIdx) }}
            >
              <Building2 size={28} />
            </div>
            <div className="flex-1">
              <h1 className="serif text-[30px] font-semibold tracking-tight leading-none">
                {active?.name ?? 'Select a site'}
              </h1>
              <div className="text-[13px] text-muted mt-1.5">
                Tenant {tenant}
                {active ? ` · ${active.equips} equipment · ${points.length} points` : ''}
              </div>
            </div>
            <Link to="/" className="text-[13px] text-muted hover:text-fg transition mt-2">
              Switch site
            </Link>
          </div>

          <button
            onClick={() => navigate({ to: '/t/$tenant/copilot', params: { tenant }, search: { site: activeKey } })}
            className="w-full text-left mt-6 rounded-2xl border border-r1/25 bg-gradient-to-r from-r1/10 to-transparent p-4 flex items-center gap-4 hover:from-r1/15 transition"
          >
            <Orb size={40} sparkle />
            <div className="flex-1">
              <div className="serif text-[16px] text-fg/95">
                {alerts
                  ? `${alerts} ${alerts === 1 ? 'zone needs' : 'zones need'} you at ${active?.name ?? 'this site'}.`
                  : `Everything at ${active?.name ?? 'this site'} is calm — ask Rubix anything.`}
              </div>
            </div>
            <span className="inline-flex items-center gap-1.5 rounded-lg bg-r1/15 text-r1 px-3 py-2 text-[13px] font-semibold shrink-0">
              Ask Rubix
              <ArrowRight size={16} />
            </span>
          </button>

          <div className="grid grid-cols-4 gap-3 mt-4">
            {vitals.map((v) => (
              <StatCard key={v.label} stat={v} />
            ))}
          </div>

          <div className="text-[12px] uppercase tracking-[.12em] text-muted font-medium mt-8 mb-3">
            Manage {active?.name ?? 'this site'}
          </div>
          <div className="grid grid-cols-4 gap-3">
            {menu.map((m) => (
              <MenuCard key={m.label} item={m} tenant={tenant} site={activeKey} />
            ))}
          </div>
        </div>
      )}
    </div>
  )
}

function MenuCard({ item, tenant, site }: { item: MenuItem; tenant: string; site?: string }) {
  const Icon = item.icon
  const params = item.dest.to === '/t/$tenant/$page' ? { tenant, page: item.dest.page } : { tenant }
  return (
    <Link
      to={item.dest.to}
      params={params as never}
      search={{ site }}
      className={`qitem text-left rounded-2xl border ${
        item.accent ? 'border-r1/30 bg-r1/[.06]' : 'border-border bg-panel2'
      } hover:bg-panel3 p-4 block`}
    >
      <div
        className={`size-10 rounded-xl grid place-items-center ${item.accent ? '' : 'bg-panel3'}`}
        style={item.accent ? { background: 'linear-gradient(135deg,hsl(258 84% 64%),hsl(174 70% 50%))' } : undefined}
      >
        <Icon size={20} className={item.accent ? 'text-white' : 'text-fg'} />
      </div>
      <div className="text-[14.5px] font-semibold mt-3">{item.label}</div>
      <div className="text-[12px] text-muted mt-0.5">{item.sub}</div>
    </Link>
  )
}
