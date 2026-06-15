// The route tree, mirroring the backend's tenant-scoped shape one-to-one
// (PRODUCT-UI "Routing"): `/` is the portfolio/tenant picker, everything else
// nests under `/t/$tenant`. The active site travels as a `?site` search param so
// URLs stay shareable. Code-based routes keep the tree in one readable file.

import {
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  redirect,
} from '@tanstack/react-router'
import { Portfolio } from './pages/Portfolio'
import { Home } from './pages/Home'
import { Building } from './pages/Building'
import { Copilot } from './pages/Copilot'
import { AdminSchema } from './pages/admin/AdminSchema'
import { AdminRecordsPage } from './pages/admin/AdminRecordsPage'
import { AdminPrincipals } from './pages/admin/AdminPrincipals'
import { AdminAgents } from './pages/admin/AdminAgents'
import { AdminQuery } from './pages/admin/AdminQuery'
import { AdminDashboards } from './pages/admin/AdminDashboards'
import { GenericPage } from './pages/GenericPage'
import { AppShell } from './components/shell/AppShell'

interface SiteSearch {
  site?: string
}

const validateSite = (search: Record<string, unknown>): SiteSearch => ({
  site: typeof search.site === 'string' ? search.site : undefined,
})

const rootRoute = createRootRoute({ component: () => <Outlet /> })

const indexRoute = createRoute({ getParentRoute: () => rootRoute, path: '/', component: Portfolio })

// /t/$tenant — the tenant layout. One shared AppShell (floating sidebar + sticky
// header) wraps every operator and admin screen; pages feed their breadcrumbs and
// live-point counts up into the header via PageHeader context instead of each
// rendering its own chrome.
const tenantRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: 't/$tenant',
  component: AppShell,
})

const homeRoute = createRoute({
  getParentRoute: () => tenantRoute,
  path: '/',
  validateSearch: validateSite,
  component: Home,
})

const buildingRoute = createRoute({
  getParentRoute: () => tenantRoute,
  path: 'building',
  validateSearch: validateSite,
  component: Building,
})

const copilotRoute = createRoute({
  getParentRoute: () => tenantRoute,
  path: 'copilot',
  validateSearch: validateSite,
  component: Copilot,
})

// /t/$tenant/admin — the admin console. A pass-through Outlet route nested inside
// the tenant AppShell; each child screen renders its own content (no separate
// admin chrome). The bare /admin path redirects to the schema inspector (the
// developer's natural entry: "how is this backend shaped?").
const adminRoute = createRoute({
  getParentRoute: () => tenantRoute,
  path: 'admin',
  component: () => <Outlet />,
})

const adminIndexRoute = createRoute({
  getParentRoute: () => adminRoute,
  path: '/',
  beforeLoad: ({ params }) => {
    throw redirect({ to: '/t/$tenant/admin/schema', params: { tenant: params.tenant } })
  },
})

const adminSchemaRoute = createRoute({
  getParentRoute: () => adminRoute,
  path: 'schema',
  component: AdminSchema,
})

const adminRecordsRoute = createRoute({
  getParentRoute: () => adminRoute,
  path: 'records',
  component: AdminRecordsPage,
})

const adminPrincipalsRoute = createRoute({
  getParentRoute: () => adminRoute,
  path: 'principals',
  component: AdminPrincipals,
})

const adminAgentsRoute = createRoute({
  getParentRoute: () => adminRoute,
  path: 'agents',
  component: AdminAgents,
})

const adminQueryRoute = createRoute({
  getParentRoute: () => adminRoute,
  path: 'query',
  component: AdminQuery,
})

const adminDashboardsRoute = createRoute({
  getParentRoute: () => adminRoute,
  path: 'dashboards',
  component: AdminDashboards,
})

// Generic native page (devices/data/rules/reports/settings). Last so the static
// children above win the match.
const pageRoute = createRoute({
  getParentRoute: () => tenantRoute,
  path: '$page',
  validateSearch: validateSite,
  component: GenericPage,
})

const routeTree = rootRoute.addChildren([
  indexRoute,
  tenantRoute.addChildren([
    homeRoute,
    buildingRoute,
    copilotRoute,
    adminRoute.addChildren([
      adminIndexRoute,
      adminSchemaRoute,
      adminRecordsRoute,
      adminPrincipalsRoute,
      adminAgentsRoute,
      adminQueryRoute,
      adminDashboardsRoute,
    ]),
    pageRoute,
  ]),
])

export const router = createRouter({ routeTree })

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}
