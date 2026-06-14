import {
  Building,
  Database,
  History,
  KeyRound,
  LayoutDashboard,
  ListTree,
  Network,
  ScrollText,
  Sparkles,
  Users,
  UserCog,
  Workflow,
  Zap,
} from 'lucide-react'
import { type NavGroup } from '../types'

/** Nav item titles that require `whoami.can_admin`; the sidebar hides these for
 *  non-admins. Kept here so the gate and the items stay in one place. */
export const ADMIN_NAV_TITLES = [
  'Members',
  'Teams',
  'Access',
  'Navigation',
  'Audit log',
] as const

/**
 * Build the sidebar nav with concrete, scope-aware URLs. Org-level entries point
 * at `/o/$org/...`; site-level entries at `/o/$org/s/$siteSlug/...` so a click
 * stays inside the active tenant.
 *
 * `siteSlug` is the active site (a site-scoped route); `fallbackSiteSlug` is the
 * org's first site, used so site-level items resolve to a *real* site even when
 * the operator is on an org-level page (e.g. dashboards) with no site selected.
 * When the org has no sites at all, those items route to Orgs & Sites so the
 * operator can create one — never a dead link.
 */
export function scopedNavGroups(
  org: string | undefined,
  siteSlug: string | undefined,
  fallbackSiteSlug: string | undefined
): NavGroup[] {
  const o = (p: string) => (org ? `/o/${org}${p}` : '/')
  const activeSite = siteSlug ?? fallbackSiteSlug
  const s = (p: string) =>
    org && activeSite ? `/o/${org}/s/${activeSite}${p}` : o('/settings/orgs')

  return [
    {
      title: 'Operate',
      items: [
        { title: 'Dashboards', url: o('/dashboards'), icon: LayoutDashboard },
        { title: 'Sparks', url: s('/sparks'), icon: Zap },
        { title: 'Points & Equip', url: s('/points'), icon: Network },
        { title: 'Flow Boards', url: s('/flows'), icon: Workflow },
        { title: 'Rules Studio', url: s('/rules'), icon: ScrollText },
      ],
    },
    {
      title: 'Analyze',
      items: [
        { title: 'Query & SQL', url: s('/history'), icon: Database },
        { title: 'Agent Runs', url: s('/runs'), icon: Sparkles },
      ],
    },
    {
      title: 'Manage',
      items: [
        { title: 'Orgs & Sites', url: o('/settings/orgs'), icon: Building },
        { title: 'Members', url: o('/settings/members'), icon: UserCog },
        { title: 'Teams', url: o('/settings/teams'), icon: Users },
        { title: 'Access', url: o('/settings/access'), icon: KeyRound },
        { title: 'Navigation', url: o('/settings/navigation'), icon: ListTree },
        { title: 'Audit log', url: o('/settings/audit'), icon: History },
      ],
    },
  ]
}
