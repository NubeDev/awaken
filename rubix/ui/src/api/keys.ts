/** React Query key factory — central so invalidation stays consistent. */
import type { Uuid } from './types'

export const qk = {
  whoami: ['whoami'] as const,
  myPreferences: ['preferences', 'me'] as const,
  orgPreferences: (org: string) => ['preferences', 'org', org] as const,
  units: ['units'] as const,
  sites: ['sites'] as const,
  site: (id: Uuid) => ['sites', id] as const,
  orgs: ['orgs'] as const,
  equips: (siteId?: Uuid) => ['equips', siteId ?? 'all'] as const,
  points: (params: { equipId?: Uuid; siteId?: Uuid; tags?: string }) =>
    [
      'points',
      params.equipId ?? null,
      params.siteId ?? null,
      params.tags ?? null,
    ] as const,
  point: (id: Uuid) => ['points', 'one', id] as const,
  pointHistory: (id: Uuid, timeKey = 'live') =>
    ['points', id, 'history', timeKey] as const,
  sparks: (siteId?: Uuid) => ['sparks', siteId ?? 'all'] as const,
  agentStatus: ['agent', 'status'] as const,
  runs: ['runs'] as const,
  runsByStatus: (status?: string) => ['runs', 'list', status ?? 'all'] as const,
  run: (id: string) => ['runs', id] as const,
  boards: ['boards'] as const,
  board: (slug: string) => ['boards', slug] as const,
  boardComponents: ['boards', 'components'] as const,
  boardOutputs: (slug: string) => ['boards', slug, 'outputs'] as const,
  widgets: (params: { siteId?: Uuid; dashboardId?: Uuid }) =>
    ['widgets', params.siteId ?? null, params.dashboardId ?? null] as const,
  /**
   * A single widget's *data* query (the SQL result a `datasource` tile renders),
   * keyed additionally on `varRevision` — a hash of the resolved values of the
   * variables the widget's SQL references — so a selection change re-fetches
   * exactly the dependent widgets (variables-and-templating.md §6). A widget that
   * references no variable gets a constant `varRevision` (`'none'`), so an
   * unrelated selection change never invalidates it.
   */
  widgetData: (widgetId: Uuid, varRevision: string, timeKey = 'live') =>
    ['widget-data', widgetId, varRevision, timeKey] as const,
  dashboards: (org?: string, siteId?: Uuid) =>
    ['dashboards', org ?? 'all', siteId ?? 'all'] as const,
  rules: (org?: string) => ['rules', org ?? 'all'] as const,
  rule: (org: string, name: string) => ['rules', org, name] as const,
  ruleReferencing: (org: string, name: string) =>
    ['rules', org, name, 'referencing'] as const,
  users: (org?: string) => ['users', org ?? 'all'] as const,
  teams: (org?: string) => ['teams', org ?? 'all'] as const,
  teamMembers: (org: string, id: Uuid) =>
    ['teams', org, id, 'members'] as const,
  grants: (org?: string, resourceRef?: string) =>
    ['grants', org ?? 'all', resourceRef ?? 'all'] as const,
  dashboardGrants: (id: Uuid) => ['grants', 'dashboard', id] as const,
  /** The org's nav tree (server-filtered to `view`-granted nodes). */
  nav: (org?: string) => ['nav', org ?? 'all'] as const,
  navNode: (id: Uuid) => ['nav', 'node', id] as const,
  /** An entity's tag set (key per kind+id). */
  entityTags: (kind: string, id: Uuid) => ['tags', kind, id] as const,
  tagKeys: (kind: string, org: string) => ['tags', 'keys', kind, org] as const,
}
