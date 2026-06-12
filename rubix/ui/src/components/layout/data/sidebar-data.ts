import {
  LayoutDashboard,
  LayoutGrid,
  Zap,
  Network,
  Workflow,
  Database,
  Sparkles,
  Building2,
} from 'lucide-react'
import { type SidebarData } from '../types'

/**
 * Rubix navigation. The site switcher (the "teams" slot) is populated live from
 * the sites API by `TeamSwitcher`; this static entry is only the boot fallback
 * shown before the first fetch resolves. Nav groups mirror the product surfaces.
 */
export const sidebarData: SidebarData = {
  user: {
    name: 'Operator',
    email: '',
    avatar: '',
  },
  teams: [
    {
      name: 'Rubix',
      logo: Building2,
      plan: 'Building Intelligence',
    },
  ],
  navGroups: [
    {
      title: 'Operate',
      items: [
        { title: 'Dashboard', url: '/', icon: LayoutDashboard },
        { title: 'Dashboard Builder', url: '/builder', icon: LayoutGrid },
        { title: 'Sparks', url: '/sparks', icon: Zap },
        { title: 'Points & Equip', url: '/points', icon: Network },
        { title: 'Flow Boards', url: '/flows', icon: Workflow },
      ],
    },
    {
      title: 'Analyze',
      items: [
        { title: 'History & SQL', url: '/history', icon: Database },
        { title: 'Agent Runs', url: '/runs', icon: Sparkles },
      ],
    },
  ],
}
