/**
 * The auto-built dashboard surface (WS-07; DASHBOARDS.md). NHP boards are NOT
 * hand-authored — they are GENERATED from the tag graph (DASHBOARDS.md "tags →
 * pages"): pick a tenant, drill tenant → site → gateway → meter, each level a page
 * built deterministically from the records carrying that scope's tag.
 *
 * This component owns the SCOPE STACK (breadcrumb drill-down) and the board
 * CONTROLS (time window + visibility-aware refresh) and is the ONE place that
 * triggers fetching — every level page reads from the shared React Query cache
 * (query/batch.ts) and renders pure builders. Drill state is local, not in the
 * URL: NHP nav is flat (WS-01) and a POC board doesn't need deep links.
 *
 * Refresh: a visibility-aware timer (use-refresh.ts), NOT a /ws/records live
 * subscription — the WS-07 POC-blessed simplification (documented there).
 */
import { useMemo, useState } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { Route, type DashboardSearch } from '@/routes/_authenticated/dashboards'
import { Main } from '@/components/layout/main'
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from '@/components/ui/breadcrumb'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useHeaderLeft } from '@/context/header-slot'
import {
  useGateways,
  useMeters,
  useRegisters,
  useRegistersHistory,
  useSites,
  useTenants,
} from './query/batch'
import {
  REFRESH_OPTIONS,
  useRefreshInterval,
  type RefreshMs,
} from './query/use-refresh'
import { WINDOW_TOKENS, type WindowToken } from './query/time-window'
import { useDashboardRefetch } from './use-dashboard-refetch'
import { TenantPage } from './pages/tenant-page'
import { SitePage } from './pages/site-page'
import { GatewayPage } from './pages/gateway-page'
import { MeterPage } from './pages/meter-page'
import type { Scope } from './auto-build/scope'
import { Empty } from './widgets/empty'

const WINDOWS = Object.keys(WINDOW_TOKENS) as WindowToken[]

export function DashboardPage() {
  const search = Route.useSearch()
  const navigate = useNavigate({ from: Route.fullPath })
  const tenants = useTenants()
  const sites = useSites()
  const gateways = useGateways()
  const meters = useMeters()
  const [window, setWindow] = useState<WindowToken>('now-24h')
  const [refresh, setRefresh] = useState<RefreshMs>(0)

  // The shared WINDOWED history read: fan one /readings query out per register over
  // the default trailing window (READINGS-TIMESERIES.md §"UI changes" — this is the
  // windowed replacement for the whole-collection read that falls over at volume).
  // The refresh interval re-runs the dashboard queries on each visible tick (paused
  // while the tab is hidden).
  const interval = useRefreshInterval(refresh)
  const registers = useRegisters()
  const history = useRegistersHistory(registers.data ?? [])
  useDashboardRefetch(interval)

  const tenantList = tenants.data ?? []
  const activeTenant = search.tenant ?? tenantList[0]?.content.key ?? null
  const tenantName = tenantList.find((t) => t.content.key === activeTenant)?.content.name

  // Drill state is the URL — NOT local state. The scope params (tenant/site/
  // gateway/meter) are the single source of truth, so every level is a shareable
  // deep link (/dashboards?tenant=acme&site=acme-hq&gateway=hq-gw1&meter=<id>) and
  // the sidebar tree (which links with the same params) stays in sync for free.
  // The breadcrumb stack is DERIVED from the params, resolving names from records.
  const stack = useMemo<Scope[]>(() => {
    const next: Scope[] = []
    if (search.site) {
      const s = (sites.data ?? []).find((r) => r.content.key === search.site)
      next.push({ level: 'site', key: search.site, name: s?.content.name ?? search.site })
    }
    if (search.gateway) {
      const g = (gateways.data ?? []).find((r) => r.content.key === search.gateway)
      next.push({ level: 'gateway', key: search.gateway, name: g?.content.name ?? search.gateway })
    }
    if (search.meter) {
      const m = (meters.data ?? []).find((r) => r.id === search.meter)
      next.push({ level: 'meter', key: search.meter, name: m?.content.name ?? 'Meter' })
    }
    return next
  }, [search.site, search.gateway, search.meter, sites.data, gateways.data, meters.data])

  const top = stack[stack.length - 1]

  // Navigation writes the scope to the URL (replace: a drill is not a separate
  // history entry per click — Back returns to wherever you came from). `go` always
  // carries the active `tenant` so the portfolio survives a drill; callers pass
  // only the deeper scope (a missing level clears it and everything below).
  const go = (scope: Omit<DashboardSearch, 'tenant'>) =>
    navigate({ search: { tenant: activeTenant ?? undefined, ...scope }, replace: true })
  const openSite = (site: string) => go({ site })
  const openGateway = (site: string, gateway: string) => go({ site, gateway })
  const openMeter = (site: string, gateway: string, meter: string) =>
    go({ site, gateway, meter })
  const reset = () => go({})
  // Pop the breadcrumb to level `i` of the stack by truncating the scope params.
  const popTo = (i: number) => {
    const s = stack.slice(0, i + 1)
    go({
      site: s.find((x) => x.level === 'site')?.key,
      gateway: s.find((x) => x.level === 'gateway')?.key,
      meter: s.find((x) => x.level === 'meter')?.key,
    })
  }

  // Publish the drill-stack breadcrumb into the app header's left slot
  // (shadcn-admin top-bar convention). Interactive: clicking a crumb pops the
  // stack to that level. Memoised so the slot only re-publishes on real change.
  const crumb = useMemo(
    () => (
      <Breadcrumb>
        <BreadcrumbList>
          <BreadcrumbItem>
            {stack.length === 0 ? (
              <BreadcrumbPage>{tenantName}</BreadcrumbPage>
            ) : (
              <BreadcrumbLink asChild>
                <button type='button' onClick={reset}>
                  {tenantName}
                </button>
              </BreadcrumbLink>
            )}
          </BreadcrumbItem>
          {stack.map((s, i) => {
            const isLast = i === stack.length - 1
            return (
              <BreadcrumbItem key={`${s.level}-${s.key}`}>
                <BreadcrumbSeparator />
                {isLast ? (
                  <BreadcrumbPage>{s.name}</BreadcrumbPage>
                ) : (
                  <BreadcrumbLink asChild>
                    <button type='button' onClick={() => popTo(i)}>
                      {s.name}
                    </button>
                  </BreadcrumbLink>
                )}
              </BreadcrumbItem>
            )
          })}
        </BreadcrumbList>
      </Breadcrumb>
    ),
    // popTo/reset close over `stack` via setStack's functional form is not used,
    // so depend on the inputs that change the rendered crumbs.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [stack, tenantName]
  )
  useHeaderLeft(activeTenant ? crumb : null)

  if (tenants.isLoading) return <Main><Empty message='Loading…' /></Main>
  if (!activeTenant) return <Main><Empty message='No tenants. Seed a portfolio first.' /></Main>

  return (
    <Main>
      {/* Title row (shadcn-admin layout): page heading on the left, board controls
          on the right. Tenant/scope selection is the SIDEBAR tree + breadcrumb
          (the in-row tenant selector was removed — redundant with the sidebar).
          The breadcrumb lives in the app header above (header-slot.tsx). */}
      <div className='mb-4 flex flex-wrap items-center justify-between gap-3'>
        <h1 className='text-2xl font-bold tracking-tight'>{top?.name ?? tenantName ?? 'Dashboard'}</h1>
        <div className='flex items-center gap-2'>
          <Select value={window} onValueChange={(v) => setWindow(v as WindowToken)}>
            <SelectTrigger className='h-8 w-28'>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {WINDOWS.map((w) => (
                <SelectItem key={w} value={w}>
                  {w.replace('now-', 'Last ')}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Select value={String(refresh)} onValueChange={(v) => setRefresh(Number(v) as RefreshMs)}>
            <SelectTrigger className='h-8 w-24'>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {REFRESH_OPTIONS.map((o) => (
                <SelectItem key={o.ms} value={String(o.ms)}>
                  {o.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>

      {!top && (
        <TenantPage
          tenantKey={activeTenant}
          window={window}
          history={history.data ?? []}
          onOpenSite={(key) => openSite(key)}
        />
      )}
      {top?.level === 'site' && (
        <SitePage
          siteKey={top.key}
          window={window}
          history={history.data ?? []}
          onOpenGateway={(key) => openGateway(top.key, key)}
        />
      )}
      {top?.level === 'gateway' && (
        <GatewayPage
          gatewayKey={top.key}
          onOpenMeter={(id) =>
            openMeter(search.site ?? '', top.key, id)
          }
        />
      )}
      {top?.level === 'meter' && <MeterPage meterId={top.key} window={window} />}
    </Main>
  )
}
