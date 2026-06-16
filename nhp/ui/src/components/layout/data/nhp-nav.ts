import {
  FileBarChart,
  LayoutDashboard,
  Settings2,
  Siren,
  Wand2,
} from 'lucide-react'
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
    title: 'Report',
    items: [
      // Tenant- and site-scoped reporting + the live alarm console (scope is
      // chosen in-page via the filter bar, not the URL — NHP nav is flat).
      { title: 'Reports', url: '/reports', icon: FileBarChart },
      { title: 'Alarms', url: '/alarms', icon: Siren },
    ],
  },
  {
    title: 'Configure',
    items: [
      {
        title: 'Admin',
        icon: Settings2,
        // Meter-types is WS-04; gateways/networks + users are WS-05.
        items: [
          { title: 'Meter-types', url: '/admin/meter-types' },
          { title: 'Gateways', url: '/admin/gateways' },
          { title: 'Users', url: '/admin/users' },
        ],
      },
      { title: 'Wizards', url: '/wizards', icon: Wand2 },
    ],
  },
]
