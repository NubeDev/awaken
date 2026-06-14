import { describe, expect, it } from 'vitest'
import type { Dashboard } from '@/api/types'
import { targetHref } from './target-href'

const board: Dashboard = {
  id: 'd1',
  org: 'kfc',
  slug: 'energy-overview',
  title: 'Energy',
  created_at: '2026-01-01T00:00:00Z',
}

describe('targetHref', () => {
  it('a dashboard target opens the board with ?nav threaded', () => {
    expect(
      targetHref({
        target: { kind: 'dashboard', dashboard_id: 'd1' },
        nodeId: 'n1',
        org: 'kfc',
        siteSlug: undefined,
        dashboards: [board],
      })
    ).toBe('/o/kfc/dashboards/energy-overview?nav=n1')
  })

  it('an unresolvable dashboard (not in org list) yields null', () => {
    expect(
      targetHref({
        target: { kind: 'dashboard', dashboard_id: 'missing' },
        nodeId: 'n1',
        org: 'kfc',
        siteSlug: undefined,
        dashboards: [board],
      })
    ).toBeNull()
  })

  it('a group has no href', () => {
    expect(
      targetHref({
        target: { kind: 'group' },
        nodeId: 'n1',
        org: 'kfc',
        siteSlug: 's1',
        dashboards: [],
      })
    ).toBeNull()
  })

  it('an org-scoped route ignores the site slug', () => {
    expect(
      targetHref({
        target: { kind: 'route', route: 'dashboards' },
        nodeId: 'n1',
        org: 'kfc',
        siteSlug: 's1',
        dashboards: [],
      })
    ).toBe('/o/kfc/dashboards')
  })

  it('a site-scoped route uses the active site, else falls back to org', () => {
    expect(
      targetHref({
        target: { kind: 'route', route: 'rules' },
        nodeId: 'n1',
        org: 'kfc',
        siteSlug: 's1',
        dashboards: [],
      })
    ).toBe('/o/kfc/s/s1/rules')
    expect(
      targetHref({
        target: { kind: 'route', route: 'rules' },
        nodeId: 'n1',
        org: 'kfc',
        siteSlug: undefined,
        dashboards: [],
      })
    ).toBe('/o/kfc/rules')
  })

  it('no org yields null', () => {
    expect(
      targetHref({
        target: { kind: 'route', route: 'dashboards' },
        nodeId: 'n1',
        org: undefined,
        siteSlug: undefined,
        dashboards: [],
      })
    ).toBeNull()
  })
})
