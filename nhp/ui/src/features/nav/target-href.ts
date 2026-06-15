/**
 * Resolve a nav node's `target` to a concrete in-scope href (docs/design/page-
 * context-and-nav.md §4). A `dashboard` target opens its board with the node id
 * threaded as `?nav=` so the page can re-assemble the node's context; a `route`
 * target opens the built-in static page; a `group` is a non-clickable header and
 * has no href. The board slug is resolved from the org's dashboards (the target
 * stores the global `dashboard_id`, not the slug).
 */
import type { Dashboard, NavRoute, NavTarget } from '@/api/types'

/** The static-route segment for each `NavRoute`, mapped to rubix's URL table.
 *  A site-scoped route uses the active site slug; an org-scoped route does not. */
type RouteHref = { scope: 'org' | 'site'; path: string }

const ROUTE_HREFS: Record<NavRoute, RouteHref> = {
  sites: { scope: 'org', path: '/settings/orgs' },
  equips: { scope: 'site', path: '/points' },
  points: { scope: 'site', path: '/points' },
  dashboards: { scope: 'org', path: '/dashboards' },
  datasources: { scope: 'site', path: '/history' },
  rules: { scope: 'site', path: '/rules' },
  boards: { scope: 'site', path: '/flows' },
  sparks: { scope: 'site', path: '/sparks' },
  runs: { scope: 'site', path: '/runs' },
  audit: { scope: 'org', path: '/settings/access' },
  access: { scope: 'org', path: '/settings/access' },
}

/**
 * The href a node opens, or `null` for a group (non-clickable) or an
 * unresolvable dashboard (board not in the caller's org list). `org` and the
 * active `siteSlug` scope the URL; `dashboards` resolves a dashboard target's
 * slug. The node id is threaded as `?nav=` on a dashboard so the open page
 * re-assembles the node's context.
 */
export function targetHref(args: {
  target: NavTarget
  nodeId: string
  org: string | undefined
  siteSlug: string | undefined
  dashboards: Dashboard[]
}): string | null {
  const { target, nodeId, org, siteSlug, dashboards } = args
  if (!org) return null

  if (target.kind === 'group') return null

  if (target.kind === 'dashboard') {
    const board = dashboards.find((d) => d.id === target.dashboard_id)
    if (!board) return null
    return `/o/${org}/dashboards/${board.slug}?nav=${nodeId}`
  }

  // A static route.
  const r = ROUTE_HREFS[target.route]
  if (r.scope === 'org' || !siteSlug) return `/o/${org}${r.path}`
  return `/o/${org}/s/${siteSlug}${r.path}`
}
