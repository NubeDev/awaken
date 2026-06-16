import { useState } from 'react'
import { Link, useLocation } from '@tanstack/react-router'
import { ChevronRight, Building2, MapPin, Router, Gauge } from 'lucide-react'
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@/components/ui/collapsible'
import {
  SidebarGroup,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSub,
  SidebarMenuSubButton,
  SidebarMenuSubItem,
} from '@/components/ui/sidebar'
import {
  useGateways,
  useMeters,
  useSites,
  useTenants,
} from '@/features/dashboards/query/batch'
import { withTags } from '@/features/dashboards/auto-build/scope'
import type { RecordDto, Site, Gateway, Meter } from '@/api/records'
import {
  gatewayTag,
  siteTag,
  tenantTag,
} from '@/enums/tags'

/**
 * The live portfolio tree in the sidebar: tenant → site → gateway → meter, built
 * from the same records + tag intersection the dashboards use (one shared React
 * Query cache, no extra fetch). Each node deep-links to `/dashboards?tenant=…&
 * site=…&gateway=…&meter=…`; the dashboard route reads those params and seeds its
 * drill stack (see routes/_authenticated/dashboards.tsx + dashboard-page.tsx).
 *
 * Membership is the WS-03/WS-06 `content.tags` convention (tenantTag/siteTag/…),
 * read via `withTags` — the SAME tags the seed and wizards write, so the tree can
 * never silently mismatch the data.
 */
/** The dashboard scope the user is currently on, read from the URL. */
type ActiveScope = {
  tenant?: string
  site?: string
  gateway?: string
  meter?: string
}

/**
 * Read the active dashboard scope from the URL so the tree can expand to and
 * highlight where the user is. Only `/dashboards` carries scope params; on any
 * other route the scope is empty (nothing highlighted). `useLocation` re-runs on
 * every navigation, so breadcrumb/table drills sync the tree too.
 */
function useActiveScope(): ActiveScope {
  return useLocation({
    select: (l) => {
      if (!l.pathname.startsWith('/dashboards')) return {}
      const s = l.search as Record<string, unknown>
      return {
        tenant: typeof s.tenant === 'string' ? s.tenant : undefined,
        site: typeof s.site === 'string' ? s.site : undefined,
        gateway: typeof s.gateway === 'string' ? s.gateway : undefined,
        meter: typeof s.meter === 'string' ? s.meter : undefined,
      }
    },
  })
}

export function NhpPortfolioTree() {
  const tenants = useTenants()
  const sites = useSites()
  const gateways = useGateways()
  const meters = useMeters()
  const active = useActiveScope()

  const tenantList = tenants.data ?? []

  return (
    <SidebarGroup>
      <SidebarGroupLabel>Portfolio</SidebarGroupLabel>
      <SidebarMenu>
        {tenants.isLoading && (
          <SidebarMenuItem>
            <SidebarMenuButton disabled>Loading…</SidebarMenuButton>
          </SidebarMenuItem>
        )}
        {!tenants.isLoading && tenantList.length === 0 && (
          <SidebarMenuItem>
            <SidebarMenuButton disabled>No tenants — seed first</SidebarMenuButton>
          </SidebarMenuItem>
        )}
        {tenantList.map((t) => (
          <TenantNode
            key={t.content.key}
            tenantKey={t.content.key}
            name={t.content.name}
            sites={withTags(sites.data ?? [], [tenantTag(t.content.key)])}
            gateways={gateways.data ?? []}
            meters={meters.data ?? []}
            active={active}
          />
        ))}
      </SidebarMenu>
    </SidebarGroup>
  )
}

/**
 * Sync a node's open state to the URL without an effect: the node is open when
 * it's on the active path OR the user has manually opened it. When the active
 * path changes (a deep link / breadcrumb drill), we reset the manual override so
 * the URL wins again — done during render via the store-previous-value pattern
 * (React's recommended alternative to a setState-in-effect). Returns [open, setOpen].
 */
function useActiveOpen(onActivePath: boolean): [boolean, (v: boolean) => void] {
  const [manual, setManual] = useState<boolean | null>(null)
  const [prevOnPath, setPrevOnPath] = useState(onActivePath)
  // When the active-path flag flips, drop the manual override so the URL decides.
  if (onActivePath !== prevOnPath) {
    setPrevOnPath(onActivePath)
    setManual(null)
  }
  const open = manual ?? onActivePath
  return [open, setManual]
}

type SiteRec = RecordDto<Site>
type GatewayRec = RecordDto<Gateway>
type MeterRec = RecordDto<Meter>

function TenantNode({
  tenantKey,
  name,
  sites,
  gateways,
  meters,
  active,
}: {
  tenantKey: string
  name: string
  sites: SiteRec[]
  gateways: GatewayRec[]
  meters: MeterRec[]
  active: ActiveScope
}) {
  const onPath = active.tenant === tenantKey
  // Tenants default open (the tree's top level); also force-open when active.
  const [open, setOpen] = useActiveOpen(onPath || true)
  // The tenant row is "active" only when it's the deepest selected level.
  const isActive = onPath && !active.site
  return (
    <Collapsible open={open} onOpenChange={setOpen} className='group/tenant'>
      <SidebarMenuItem>
        <CollapsibleTrigger asChild>
          <SidebarMenuButton tooltip={name} isActive={isActive}>
            <Building2 />
            <span className='truncate'>{name}</span>
            <ChevronRight className='ms-auto transition-transform group-data-[state=open]/tenant:rotate-90' />
          </SidebarMenuButton>
        </CollapsibleTrigger>
        <CollapsibleContent>
          <SidebarMenuSub>
            {sites.length === 0 && (
              <SidebarMenuSubItem>
                <span className='text-muted-foreground px-2 py-1 text-xs'>No sites</span>
              </SidebarMenuSubItem>
            )}
            {sites.map((s) => (
              <SiteNode
                key={s.content.key}
                tenantKey={tenantKey}
                siteKey={s.content.key}
                name={s.content.name}
                gateways={withTags(gateways, [siteTag(s.content.key)])}
                meters={meters}
                active={active}
              />
            ))}
          </SidebarMenuSub>
        </CollapsibleContent>
      </SidebarMenuItem>
    </Collapsible>
  )
}

function SiteNode({
  tenantKey,
  siteKey,
  name,
  gateways,
  meters,
  active,
}: {
  tenantKey: string
  siteKey: string
  name: string
  gateways: GatewayRec[]
  meters: MeterRec[]
  active: ActiveScope
}) {
  const onPath = active.tenant === tenantKey && active.site === siteKey
  const [open, setOpen] = useActiveOpen(onPath)
  const isActive = onPath && !active.gateway
  return (
    <Collapsible open={open} onOpenChange={setOpen} className='group/site'>
      <SidebarMenuSubItem>
        <div className='flex items-center'>
          <CollapsibleTrigger asChild>
            <button
              type='button'
              className='hover:bg-sidebar-accent text-sidebar-foreground/70 me-1 rounded p-0.5'
              aria-label={`Toggle ${name}`}
            >
              <ChevronRight className='size-3.5 transition-transform group-data-[state=open]/site:rotate-90' />
            </button>
          </CollapsibleTrigger>
          <SidebarMenuSubButton asChild isActive={isActive}>
            <Link to='/dashboards' search={{ tenant: tenantKey, site: siteKey }}>
              <MapPin className='size-3.5' />
              <span className='truncate'>{name}</span>
            </Link>
          </SidebarMenuSubButton>
        </div>
        <CollapsibleContent>
          <SidebarMenuSub>
            {gateways.length === 0 && (
              <SidebarMenuSubItem>
                <span className='text-muted-foreground px-2 py-1 text-xs'>No gateways</span>
              </SidebarMenuSubItem>
            )}
            {gateways.map((g) => (
              <GatewayNode
                key={g.content.key}
                tenantKey={tenantKey}
                siteKey={siteKey}
                gatewayKey={g.content.key}
                name={g.content.name}
                meters={withTags(meters, [gatewayTag(g.content.key)])}
                active={active}
              />
            ))}
          </SidebarMenuSub>
        </CollapsibleContent>
      </SidebarMenuSubItem>
    </Collapsible>
  )
}

function GatewayNode({
  tenantKey,
  siteKey,
  gatewayKey,
  name,
  meters,
  active,
}: {
  tenantKey: string
  siteKey: string
  gatewayKey: string
  name: string
  meters: MeterRec[]
  active: ActiveScope
}) {
  const onPath =
    active.tenant === tenantKey && active.site === siteKey && active.gateway === gatewayKey
  const [open, setOpen] = useActiveOpen(onPath)
  const isActive = onPath && !active.meter
  return (
    <Collapsible open={open} onOpenChange={setOpen} className='group/gateway'>
      <SidebarMenuSubItem>
        <div className='flex items-center'>
          <CollapsibleTrigger asChild>
            <button
              type='button'
              className='hover:bg-sidebar-accent text-sidebar-foreground/70 me-1 rounded p-0.5'
              aria-label={`Toggle ${name}`}
            >
              <ChevronRight className='size-3.5 transition-transform group-data-[state=open]/gateway:rotate-90' />
            </button>
          </CollapsibleTrigger>
          <SidebarMenuSubButton asChild isActive={isActive}>
            <Link
              to='/dashboards'
              search={{ tenant: tenantKey, site: siteKey, gateway: gatewayKey }}
            >
              <Router className='size-3.5' />
              <span className='truncate'>{name}</span>
            </Link>
          </SidebarMenuSubButton>
        </div>
        <CollapsibleContent>
          <SidebarMenuSub>
            {meters.length === 0 && (
              <SidebarMenuSubItem>
                <span className='text-muted-foreground px-2 py-1 text-xs'>No meters</span>
              </SidebarMenuSubItem>
            )}
            {meters.map((m) => (
              <SidebarMenuSubItem key={m.id}>
                <SidebarMenuSubButton asChild isActive={active.meter === m.id}>
                  <Link
                    to='/dashboards'
                    search={{
                      tenant: tenantKey,
                      site: siteKey,
                      gateway: gatewayKey,
                      meter: m.id,
                    }}
                  >
                    <Gauge className='size-3.5' />
                    <span className='truncate'>{m.content.name}</span>
                  </Link>
                </SidebarMenuSubButton>
              </SidebarMenuSubItem>
            ))}
          </SidebarMenuSub>
        </CollapsibleContent>
      </SidebarMenuSubItem>
    </Collapsible>
  )
}
