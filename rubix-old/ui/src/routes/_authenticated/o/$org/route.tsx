import { createFileRoute, Outlet } from '@tanstack/react-router'

/**
 * Org-scoped layout. The `$org` param is the tenant namespace; `ScopeProvider`
 * (in the authenticated layout) reads it to resolve the active org and its
 * sites. Pages directly under here are org-level (e.g. the overview dashboard);
 * site-level pages nest further under `s/$siteSlug`.
 */
export const Route = createFileRoute('/_authenticated/o/$org')({
  component: Outlet,
})
