import { createFileRoute, Outlet } from '@tanstack/react-router'

/**
 * Site-scoped layout. Adds `$siteSlug` to the org scope; `ScopeProvider`
 * resolves it to the active `Site`. Every page under here (points, sparks,
 * flows, rules, history, runs) renders that one site's data, so the URL alone
 * says exactly what is shown and any link is shareable.
 */
export const Route = createFileRoute('/_authenticated/o/$org/s/$siteSlug')({
  component: Outlet,
})
