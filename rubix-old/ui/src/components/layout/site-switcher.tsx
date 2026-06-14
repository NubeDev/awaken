import { Building, Building2, Check, ChevronsUpDown, Cog } from 'lucide-react'
import { useNavigate } from '@tanstack/react-router'
import { useScope } from '@/context/scope-provider'
import { useOrgs } from '@/api/hooks'
import { cn } from '@/lib/utils'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import {
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  useSidebar,
} from '@/components/ui/sidebar'

/**
 * Sidebar header control: the active org and site, both switched by navigating
 * (the scope lives in the URL, not a store). The Organizations section always
 * shows so you can switch tenants; orgs come from `/orgs` (so a freshly
 * provisioned, site-less org still appears). Picking a site routes to its
 * points; picking an org routes to its dashboards.
 */
export function SiteSwitcher() {
  const { isMobile } = useSidebar()
  const { org, site, sites } = useScope()
  const { data: orgSummaries = [] } = useOrgs()
  const navigate = useNavigate()

  // All orgs the principal can see (from /orgs — includes site-less ones), plus
  // the active org in case it has not surfaced in the list yet.
  const orgs = Array.from(
    new Set([...orgSummaries.map((o) => o.org), ...(org ? [org] : [])])
  ).sort()
  const orgSites = sites.filter((s) => s.org === org)

  const goToSite = (siteSlug: string) =>
    navigate({
      to: '/o/$org/s/$siteSlug/points',
      params: { org: org as string, siteSlug },
    })
  const goToOrg = (nextOrg: string) =>
    navigate({ to: '/o/$org/dashboards', params: { org: nextOrg } })

  return (
    <SidebarMenu>
      <SidebarMenuItem>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <SidebarMenuButton
              size='lg'
              className='data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground'
            >
              <div className='bg-sidebar-primary text-sidebar-primary-foreground flex aspect-square size-8 items-center justify-center rounded-lg'>
                <Building2 className='size-4' />
              </div>
              <div className='grid flex-1 text-start text-sm leading-tight'>
                <span className='truncate font-semibold'>
                  {site?.display_name ?? org ?? 'Rubix'}
                </span>
                <span className='text-muted-foreground truncate text-xs'>
                  {org
                    ? site
                      ? `${org} · ${site.slug}`
                      : `${org} · overview`
                    : 'Building Intelligence'}
                </span>
              </div>
              <ChevronsUpDown className='ms-auto' />
            </SidebarMenuButton>
          </DropdownMenuTrigger>
          <DropdownMenuContent
            className='w-(--radix-dropdown-menu-trigger-width) min-w-56 rounded-lg'
            align='start'
            side={isMobile ? 'bottom' : 'right'}
            sideOffset={4}
          >
            {/* Organizations — always shown so the tenant is switchable. */}
            <DropdownMenuLabel className='text-muted-foreground text-xs'>
              Organizations
            </DropdownMenuLabel>
            {orgs.map((o) => (
              <DropdownMenuItem
                key={o}
                onClick={() => goToOrg(o)}
                className='gap-2 p-2'
              >
                <div className='flex size-6 items-center justify-center rounded-sm border'>
                  <Building className='size-4 shrink-0' />
                </div>
                <span className='truncate'>{o}</span>
                {o === org ? <Check className='ms-auto size-4' /> : null}
              </DropdownMenuItem>
            ))}

            {orgSites.length > 0 ? (
              <>
                <DropdownMenuSeparator />
                <DropdownMenuLabel className='text-muted-foreground text-xs'>
                  Sites{org ? ` · ${org}` : ''}
                </DropdownMenuLabel>
                {orgSites.map((s) => (
                  <DropdownMenuItem
                    key={s.id}
                    onClick={() => goToSite(s.slug)}
                    className='gap-2 p-2'
                  >
                    <div className='flex size-6 items-center justify-center rounded-sm border'>
                      <Building2 className='size-4 shrink-0' />
                    </div>
                    <span className='truncate'>{s.display_name}</span>
                    {s.slug === site?.slug ? (
                      <Check className='ms-auto size-4' />
                    ) : null}
                  </DropdownMenuItem>
                ))}
              </>
            ) : null}

            <DropdownMenuSeparator />
            <DropdownMenuItem
              onClick={() =>
                navigate({
                  to: '/o/$org/settings/orgs',
                  params: { org: org as string },
                })
              }
              className={cn('gap-2 p-2', !org && 'pointer-events-none opacity-50')}
            >
              <div className='flex size-6 items-center justify-center rounded-sm border'>
                <Cog className='size-4 shrink-0' />
              </div>
              <span className='text-muted-foreground'>Manage orgs &amp; sites</span>
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </SidebarMenuItem>
    </SidebarMenu>
  )
}
