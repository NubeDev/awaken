import { LayoutDashboard, Settings2, Wand2 } from 'lucide-react'
import { type NavGroup } from '../types'

/**
 * Static NHP app-shell nav. NHP is flat (no org/site URL scope like rubix-old):
 * the domain is tenant→site→…→register records browsed inside feature pages, not
 * encoded in the path. These three groups are placeholders that later WSs fill
 * (Dashboards = WS-07, Admin = WS-04/05, Wizards = WS-06). See
 * nhp/docs/sessions/WS-01.md.
 */
export const nhpNavGroups: NavGroup[] = [
  {
    title: 'Operate',
    items: [
      { title: 'Dashboards', url: '/dashboards', icon: LayoutDashboard },
    ],
  },
  {
    title: 'Configure',
    items: [
      { title: 'Admin', url: '/admin', icon: Settings2 },
      { title: 'Wizards', url: '/wizards', icon: Wand2 },
    ],
  },
]
