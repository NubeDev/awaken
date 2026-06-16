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
import { nhpNavGroups } from './data/nhp-nav'
import { NavGroup } from './nav-group'
import { NavUser } from './nav-user'
import { NhpPortfolioTree } from './nhp-portfolio-tree'

/**
 * NHP app-shell sidebar: the live portfolio tree (tenant → site → gateway →
 * meter, deep-linking into the dashboards) on top, then the static feature nav
 * (Dashboards / Admin / Wizards). See WS-01.md + nhp-portfolio-tree.tsx.
 */
export function AppSidebar() {
  const { collapsible, variant } = useLayout()

  return (
    <Sidebar collapsible={collapsible} variant={variant}>
      <SidebarHeader>
        <AppTitle />
      </SidebarHeader>
      <SidebarContent>
        {nhpNavGroups.map((props) => (
          <NavGroup key={props.title} {...props} />
        ))}
        <NhpPortfolioTree />
      </SidebarContent>
      <SidebarFooter>
        <NavUser user={sidebarData.user} />
      </SidebarFooter>
      <SidebarRail />
    </Sidebar>
  )
}
