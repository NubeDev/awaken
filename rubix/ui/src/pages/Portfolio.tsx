// Portfolio — the entry screen and tenant/site picker (PRODUCT-UI: "the
// Sites/portfolio screen is really a tenant picker"). Reads the tenant's real
// records, derives sites, and renders the demo's portfolio grid. Opening a card
// enters that site's scope.

import { Link, useNavigate } from '@tanstack/react-router'
import { Building2, LogOut, Search } from 'lucide-react'
import { useConnection } from '../api/ConnectionContext'
import { useRecords } from '../hooks/useRecords'
import { toSites } from '../utils/derive'
import { fmtNum } from '../utils/format'
import { siteGradient } from '../utils/gradient'
import { Orb } from '../components/ui/Orb'
import { ErrorView, LoadingView, EmptyView } from '../components/ui/StateView'
import type { Site } from '../types/Domain'

export function Portfolio() {
  const { connection, disconnect } = useConnection()
  const tenant = connection!.tenant
  const { data: records, isLoading, error } = useRecords(tenant)
  const sites = records ? toSites(records) : []

  return (
    <div className="h-full flex flex-col">
      <header className="h-14 shrink-0 flex items-center gap-3 px-6">
        <div className="flex items-center gap-2.5">
          <Orb size={28} blur />
          <span className="font-semibold tracking-tight">Rubix</span>
        </div>
        <div className="ml-auto flex items-center gap-2">
          <button className="flex items-center gap-2 h-8 px-2.5 rounded-lg border border-border text-[12px] text-muted hover:text-fg hover:bg-panel2 transition">
            <Search size={14} />
            Search<kbd>⌘K</kbd>
          </button>
          <button
            onClick={disconnect}
            title="Disconnect"
            className="flex items-center gap-2 h-8 px-2.5 rounded-lg border border-border text-[12px] text-muted hover:text-fg hover:bg-panel2 transition"
          >
            <LogOut size={14} />
            {connection!.subject}
          </button>
        </div>
      </header>

      <div className="flex-1 overflow-auto px-10 py-8">
        <div className="max-w-[1080px] mx-auto">
          <div className="text-[13px] text-muted">Tenant · {tenant}</div>
          <h1 className="serif text-[34px] font-semibold tracking-tight mt-1">Your portfolio</h1>
          <p className="text-[14px] text-muted mt-1.5">
            Open a site to manage it. Rubix is watching {sites.length || 'your'} {sites.length === 1 ? 'site' : 'sites'}.
          </p>

          {isLoading && <LoadingView label="Reading the portfolio…" />}
          {error && <ErrorView error={error} />}
          {records && sites.length === 0 && (
            <div className="mt-7">
              <EmptyView
                title="No sites in this tenant yet"
                hint="Boot the backend with SEED=1 to populate the demo portfolio, or create a kind:'site' record."
              />
            </div>
          )}

          {sites.length > 0 && (
            <div className="grid grid-cols-3 gap-4 mt-7">
              {sites.map((s, i) => (
                <SiteCard key={s.id} site={s} tenant={tenant} grad={siteGradient(i)} />
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

function SiteCard({ site, tenant, grad }: { site: Site; tenant: string; grad: string }) {
  const navigate = useNavigate()
  return (
    <button
      onClick={() => navigate({ to: '/t/$tenant', params: { tenant }, search: { site: site.key } })}
      className="qitem text-left rounded-2xl border border-border bg-panel2 hover:bg-panel3 overflow-hidden"
    >
      <div className="h-24 relative" style={{ background: grad }}>
        <div
          className="absolute inset-0"
          style={{ background: 'radial-gradient(120px 80px at 80% 20%,rgba(255,255,255,.18),transparent)' }}
        />
        <div className="absolute bottom-2.5 left-3.5 text-white">
          <div className="text-[15px] font-semibold leading-none">{site.name}</div>
          <div className="text-[11.5px] opacity-85 mt-1">{site.equips} equipment · tenant {tenant}</div>
        </div>
        {site.alerts > 0 && (
          <span className="absolute top-2.5 right-2.5 inline-flex items-center gap-1 rounded-full bg-black/35 backdrop-blur px-2 py-0.5 text-[11px] text-white font-medium">
            <span className="size-1.5 rounded-full bg-crit" />
            {site.alerts}
          </span>
        )}
      </div>
      <div className="p-3.5">
        <div className="flex items-center gap-2 text-[12px] text-muted">
          <Building2 size={14} />
          Site
        </div>
        <div className="flex items-center gap-4 mt-3">
          <div>
            <div className="mono text-[17px] font-semibold">{fmtNum(site.points)}</div>
            <div className="text-[10.5px] text-muted">points</div>
          </div>
          <div>
            <div className="mono text-[17px] font-semibold">{fmtNum(site.equips)}</div>
            <div className="text-[10.5px] text-muted">equipment</div>
          </div>
          <div
            className={`ml-auto flex items-center gap-1.5 text-[11.5px] ${
              site.severity === 'green' ? 'text-green' : site.severity === 'muted' ? 'text-muted' : 'text-amber'
            }`}
          >
            <span
              className={`size-1.5 rounded-full ${
                site.severity === 'green' ? 'bg-green' : site.severity === 'muted' ? 'bg-muted' : 'bg-amber'
              }`}
            />
            {site.severity === 'green' ? 'Healthy' : site.severity === 'muted' ? 'Partial' : 'Attention'}
          </div>
        </div>
      </div>
    </button>
  )
}
