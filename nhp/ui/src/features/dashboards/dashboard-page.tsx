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
import { useState } from 'react'
import { ChevronRight } from 'lucide-react'
import { Main } from '@/components/layout/main'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useRegisters, useRegistersHistory, useTenants } from './query/batch'
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
  const tenants = useTenants()
  const [tenantKey, setTenantKey] = useState<string | null>(null)
  const [stack, setStack] = useState<Scope[]>([])
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
  const activeTenant = tenantKey ?? tenantList[0]?.content.key ?? null

  if (tenants.isLoading) return <Main><Empty message='Loading…' /></Main>
  if (!activeTenant) return <Main><Empty message='No tenants. Seed a portfolio first.' /></Main>

  const top = stack[stack.length - 1]
  const popTo = (i: number) => setStack(stack.slice(0, i + 1))
  const reset = () => setStack([])

  return (
    <Main>
      <div className='mb-4 flex flex-wrap items-center justify-between gap-3'>
        <div className='flex items-center gap-2'>
          <h2 className='text-xl font-semibold'>Dashboards</h2>
          <Select
            value={activeTenant}
            onValueChange={(v) => {
              setTenantKey(v)
              reset()
            }}
          >
            <SelectTrigger className='h-8 w-48'>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {tenantList.map((t) => (
                <SelectItem key={t.content.key} value={t.content.key}>
                  {t.content.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
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

      {/* Breadcrumb of the drill stack. */}
      <div className='text-muted-foreground mb-4 flex flex-wrap items-center gap-1 text-sm'>
        <Button variant='ghost' size='sm' className='h-7 px-2' onClick={reset}>
          {tenantList.find((t) => t.content.key === activeTenant)?.content.name}
        </Button>
        {stack.map((s, i) => (
          <span key={`${s.level}-${s.key}`} className='flex items-center gap-1'>
            <ChevronRight className='size-3' />
            <Button variant='ghost' size='sm' className='h-7 px-2' onClick={() => popTo(i)}>
              {s.name}
            </Button>
          </span>
        ))}
      </div>

      {!top && (
        <TenantPage
          tenantKey={activeTenant}
          history={history.data ?? []}
          onOpenSite={(key, name) => setStack([{ level: 'site', key, name }])}
        />
      )}
      {top?.level === 'site' && (
        <SitePage
          siteKey={top.key}
          window={window}
          history={history.data ?? []}
          onOpenGateway={(key, name) =>
            setStack([...stack, { level: 'gateway', key, name }])
          }
        />
      )}
      {top?.level === 'gateway' && (
        <GatewayPage
          gatewayKey={top.key}
          onOpenMeter={(id, name) => setStack([...stack, { level: 'meter', key: id, name }])}
        />
      )}
      {top?.level === 'meter' && <MeterPage meterId={top.key} window={window} />}
    </Main>
  )
}
