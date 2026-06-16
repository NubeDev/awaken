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
import { AdminTeams } from './pages/admin/AdminTeams'
import { AdminAgents } from './pages/admin/AdminAgents'
import { AdminQuery } from './pages/admin/AdminQuery'
import { AdminAudit } from './pages/admin/AdminAudit'
import { AdminRules } from './pages/admin/AdminRules'
import { AdminDashboards, AdminDashboardBuilder } from './pages/admin/AdminDashboards'
import { GenericPage } from './pages/GenericPage'
import { AppShell } from './components/shell/AppShell'

interface SiteSearch {
  site?: string
}

const validateSite = (search: Record<string, unknown>): SiteSearch => ({
  site: typeof search.site === 'string' ? search.site : undefined,
})

// The board builder additionally carries dashboard state in the URL so a
// parameterised board deep-links: `?nav=<node id>` binds the board to a navigation
// node's context, and each `?var-<name>=…` is an explicit variable selection
// (VARIABLES-AND-TEMPLATING §7, PAGE-CONTEXT-AND-NAV §1). Variable keys are dynamic,
// so they pass through untouched alongside the typed `nav`.
interface BoardSearch {
  nav?: string
  [key: string]: string | string[] | undefined
}

const validateBoardSearch = (search: Record<string, unknown>): BoardSearch => {
  const out: BoardSearch = {}
  if (typeof search.nav === 'string') out.nav = search.nav
  for (const [key, value] of Object.entries(search)) {
    if (!key.startsWith('var-')) continue
    if (typeof value === 'string') out[key] = value
    else if (Array.isArray(value))
      out[key] = value.filter((v): v is string => typeof v === 'string')
  }
  return out
}

const rootRoute = createRootRoute({ component: () => <Outlet /> })

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  component: Portfolio,
})

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
    throw redirect({
      to: '/t/$tenant/admin/schema',
      params: { tenant: params.tenant },
    })
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

const adminTeamsRoute = createRoute({
  getParentRoute: () => adminRoute,
  path: 'teams',
  component: AdminTeams,
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

const adminAuditRoute = createRoute({
  getParentRoute: () => adminRoute,
  path: 'audit',
  component: AdminAudit,
})

const adminRulesRoute = createRoute({
  getParentRoute: () => adminRoute,
  path: 'rules',
  component: AdminRules,
})

// The dashboards directory (table of boards) and the per-board builder are two
// real routes so a board deep-links, refreshes, and works with back/forward:
//   /admin/dashboards            → the list
//   /admin/dashboards/$boardId   → that board's builder
const adminDashboardsRoute = createRoute({
  getParentRoute: () => adminRoute,
  path: 'dashboards',
  component: AdminDashboards,
})

const adminDashboardBuilderRoute = createRoute({
  getParentRoute: () => adminRoute,
  path: 'dashboards/$boardId',
  validateSearch: validateBoardSearch,
  component: AdminDashboardBuilder,
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
      adminTeamsRoute,
      adminAgentsRoute,
      adminQueryRoute,
      adminAuditRoute,
      adminRulesRoute,
      adminDashboardsRoute,
      adminDashboardBuilderRoute,
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
