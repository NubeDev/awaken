// The route tree, mirroring the backend's tenant-scoped shape one-to-one
// (PRODUCT-UI "Routing"): `/` is the portfolio/tenant picker, everything else
// nests under `/t/$tenant`. The active site travels as a `?site` search param so
// URLs stay shareable. Code-based routes keep the tree in one readable file.

import {
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
} from '@tanstack/react-router'
import { Portfolio } from './pages/Portfolio'
import { Home } from './pages/Home'
import { Building } from './pages/Building'
import { Copilot } from './pages/Copilot'
import { AdminRecords } from './pages/AdminRecords'
import { GenericPage } from './pages/GenericPage'

interface SiteSearch {
  site?: string
}

const validateSite = (search: Record<string, unknown>): SiteSearch => ({
  site: typeof search.site === 'string' ? search.site : undefined,
})

const rootRoute = createRootRoute({ component: () => <Outlet /> })

const indexRoute = createRoute({ getParentRoute: () => rootRoute, path: '/', component: Portfolio })

// /t/$tenant — the tenant layout. Children render directly; the layout is a
// pass-through Outlet so each screen owns its own chrome (TopBar).
const tenantRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: 't/$tenant',
  component: () => <Outlet />,
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

const adminRecordsRoute = createRoute({
  getParentRoute: () => tenantRoute,
  path: 'admin/records',
  validateSearch: validateSite,
  component: AdminRecords,
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
  tenantRoute.addChildren([homeRoute, buildingRoute, copilotRoute, adminRecordsRoute, pageRoute]),
])

export const router = createRouter({ routeTree })

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}
