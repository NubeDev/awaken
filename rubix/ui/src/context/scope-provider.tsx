import { createContext, use, useMemo } from 'react'
import { useParams } from '@tanstack/react-router'
import { useSites } from '@/api/hooks'
import type { Site } from '@/api/types'

/**
 * The tenant scope a page renders within, derived entirely from the URL — the
 * single source of truth, so every page is concrete and every link is shareable
 * (no hidden localStorage "active site"). Routes nest as `/o/$org/...` and
 * `/o/$org/s/$siteSlug/...`; this resolves those slugs to the live `Site` and
 * org, and the switchers navigate to change them.
 *
 * - `org` is always present on a `/o/$org` route.
 * - `site` is present only on a site-scoped route (`/o/$org/s/$siteSlug`); it is
 *   `undefined` on org-level routes (e.g. an org overview dashboard).
 */
export type Scope = {
  /** Org namespace from the `$org` route param. */
  org: string | undefined
  /** The resolved active site, when on a site-scoped route. */
  site: Site | undefined
  /** Every site under `org` the principal can see (for the site switcher). */
  sites: Site[]
  isLoading: boolean
  isError: boolean
}

const ScopeContext = createContext<Scope | null>(null)

export function ScopeProvider({ children }: { children: React.ReactNode }) {
  // `strict: false` reads whichever of these params the matched route exposes;
  // org-level routes have `org` only, site-level routes have both.
  const params = useParams({ strict: false }) as {
    org?: string
    siteSlug?: string
  }
  const org = params.org
  const siteSlug = params.siteSlug

  const { data: allSites = [], isLoading, isError } = useSites(org)
  const sites = useMemo(
    () => (org ? allSites.filter((s) => s.org === org) : allSites),
    [allSites, org]
  )
  const site = useMemo(
    () => (siteSlug ? sites.find((s) => s.slug === siteSlug) : undefined),
    [sites, siteSlug]
  )

  const value = useMemo<Scope>(
    () => ({ org, site, sites, isLoading, isError }),
    [org, site, sites, isLoading, isError]
  )

  return <ScopeContext value={value}>{children}</ScopeContext>
}

/** The active tenant scope (org + optional site), resolved from the URL. */
export function useScope(): Scope {
  const ctx = use(ScopeContext)
  if (!ctx) throw new Error('useScope must be used within a ScopeProvider')
  return ctx
}

/**
 * Build hrefs within the current scope. `org(p)` prefixes an org-level path,
 * `site(p)` a site-level path (using the active site's slug). Returns plain
 * strings so callers can `navigate({ to })` / `<Link to>` without threading
 * params; the strings resolve against the live scope from the URL.
 */
export function useScopedHref() {
  const { org, site } = useScope()
  return {
    org: (path: string) => (org ? `/o/${org}${path}` : '/'),
    site: (path: string) =>
      org && site ? `/o/${org}/s/${site.slug}${path}` : '/',
    hasSite: Boolean(org && site),
  }
}
