import { useRuns, useSparks, useWhoami } from '@/api/hooks'
import { useScope } from '@/context/scope-provider'
import { useLayout } from '@/context/layout-provider'
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarHeader,
  SidebarRail,
} from '@/components/ui/sidebar'
import { AppTitle } from './app-title'
import { sidebarData } from './data/sidebar-data'
import { ADMIN_NAV_TITLES, scopedNavGroups } from './data/scoped-nav'
import { NavGroup } from './nav-group'
import { NavUser } from './nav-user'
import { SiteSwitcher } from './site-switcher'
import { NavTree } from '@/features/nav/nav-tree'

export function AppSidebar() {
  const { collapsible, variant } = useLayout()
  const { org, site, sites } = useScope()
  // The org's first site backstops site-level nav while on an org-level page,
  // so those links always resolve to a real site instead of a dead fallback.
  const fallbackSiteSlug = sites.find((s) => s.org === org)?.slug
  const { data: sparks = [] } = useSparks(site?.id)
  const { data: runs = [] } = useRuns()
  const { data: whoami } = useWhoami()
  const canAdmin = whoami?.can_admin ?? false
  const openSparks = sparks.filter((s) => !s.acknowledged).length
  const awaitingRuns = runs.filter((r) => r.status === 'suspended').length

  // Inject live counts: unacked sparks and runs awaiting approval (suspended).
  const navBadges: Record<string, number> = {
    Sparks: openSparks,
    'Agent Runs': awaitingRuns,
  }
  // Nav URLs are scope-aware: built from the active org (and site when one is
  // selected) so a click stays within the current tenant.
  const navGroups = scopedNavGroups(org, site?.slug, fallbackSiteSlug).map((group) => ({
    ...group,
    items: group.items
      // Hide the RBAC management items unless the caller is an admin.
      .filter(
        (item) =>
          canAdmin ||
          !ADMIN_NAV_TITLES.includes(
            item.title as (typeof ADMIN_NAV_TITLES)[number]
          )
      )
      .map((item) =>
        navBadges[item.title]
          ? { ...item, badge: String(navBadges[item.title]) }
          : item
      ),
  }))

  return (
    <Sidebar collapsible={collapsible} variant={variant}>
      <SidebarHeader>
        <AppTitle />
        <SiteSwitcher />
      </SidebarHeader>
      <SidebarContent>
        {/* The user-built nav tree (docs/design/page-context-and-nav.md §4)
            renders above the scope-derived groups and is empty until nodes
            exist, so the flat nav stays the default. */}
        <NavTree />
        {navGroups.map((props) => (
          <NavGroup key={props.title} {...props} />
        ))}
      </SidebarContent>
      <SidebarFooter>
        <NavUser user={sidebarData.user} />
      </SidebarFooter>
      <SidebarRail />
    </Sidebar>
  )
}
