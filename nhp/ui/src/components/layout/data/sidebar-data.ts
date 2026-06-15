import { Building2 } from 'lucide-react'
import { type SidebarData } from '../types'

/**
 * Static sidebar chrome: the user footer and the brand "team" slot. The nav
 * groups are NOT here — they are built per-render from the active URL scope by
 * `scopedNavGroups` (see `./scoped-nav`), so every link stays inside the current
 * org/site. `navGroups` is left empty to satisfy the type.
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
  navGroups: [],
}
