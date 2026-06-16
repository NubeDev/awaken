import { createFileRoute } from '@tanstack/react-router'
import { DashboardPage } from '@/features/dashboards'

/**
 * The dashboard scope is carried in the URL search params so the sidebar
 * portfolio tree (nhp-portfolio-tree.tsx) can deep-link straight to a level:
 * `/dashboards?tenant=acme&site=acme-plant&gateway=gw-01`. DashboardPage seeds
 * its drill stack from these. All optional → `/dashboards` alone opens the
 * first tenant's overview.
 */
export type DashboardSearch = {
  tenant?: string
  site?: string
  gateway?: string
  meter?: string
}

export const Route = createFileRoute('/_authenticated/dashboards')({
  validateSearch: (search: Record<string, unknown>): DashboardSearch => ({
    tenant: typeof search.tenant === 'string' ? search.tenant : undefined,
    site: typeof search.site === 'string' ? search.site : undefined,
    gateway: typeof search.gateway === 'string' ? search.gateway : undefined,
    meter: typeof search.meter === 'string' ? search.meter : undefined,
  }),
  component: DashboardPage,
})
