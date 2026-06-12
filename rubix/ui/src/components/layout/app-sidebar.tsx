import { useRuns, useSparks } from '@/api/hooks'
import { useActiveSite } from '@/hooks/use-active-site'
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
import { NavGroup } from './nav-group'
import { NavUser } from './nav-user'
import { SiteSwitcher } from './site-switcher'

export function AppSidebar() {
  const { collapsible, variant } = useLayout()
  const { site } = useActiveSite()
  const { data: sparks = [] } = useSparks(site?.id)
  const { data: runs = [] } = useRuns()
  const openSparks = sparks.filter((s) => !s.acknowledged).length
  const awaitingRuns = runs.filter((r) => r.status === 'suspended').length

  // Inject live counts: unacked sparks and runs awaiting approval (suspended).
  const navBadges: Record<string, number> = {
    Sparks: openSparks,
    'Agent Runs': awaitingRuns,
  }
  const navGroups = sidebarData.navGroups.map((group) => ({
    ...group,
    items: group.items.map((item) =>
      navBadges[item.title] ? { ...item, badge: String(navBadges[item.title]) } : item
    ),
  }))

  return (
    <Sidebar collapsible={collapsible} variant={variant}>
      <SidebarHeader>
        <AppTitle />
        <SiteSwitcher />
      </SidebarHeader>
      <SidebarContent>
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
