import { useSparks } from '@/api/hooks'
import { useActiveSite } from '@/hooks/use-active-site'
import { useLayout } from '@/context/layout-provider'
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarHeader,
  SidebarRail,
} from '@/components/ui/sidebar'
import { sidebarData } from './data/sidebar-data'
import { NavGroup } from './nav-group'
import { NavUser } from './nav-user'
import { SiteSwitcher } from './site-switcher'

export function AppSidebar() {
  const { collapsible, variant } = useLayout()
  const { site } = useActiveSite()
  const { data: sparks = [] } = useSparks(site?.id)
  const openSparks = sparks.filter((s) => !s.acknowledged).length

  // Inject the live unacked-sparks count onto the Sparks nav item.
  const navGroups = sidebarData.navGroups.map((group) => ({
    ...group,
    items: group.items.map((item) =>
      item.title === 'Sparks' && openSparks > 0
        ? { ...item, badge: String(openSparks) }
        : item
    ),
  }))

  return (
    <Sidebar collapsible={collapsible} variant={variant}>
      <SidebarHeader>
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
