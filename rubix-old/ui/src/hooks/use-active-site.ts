import { useScope } from '@/context/scope-provider'
import type { Site } from '@/api/types'

type ActiveSite = {
  site: Site | undefined
  sites: Site[]
  isLoading: boolean
  isError: boolean
}

/**
 * The site a page renders. Now a thin adapter over {@link useScope}: the active
 * site is resolved from the URL (`/o/$org/s/$siteSlug`), not a localStorage
 * selection, so every page is concrete and shareable. Kept as a named hook so
 * existing callers keep working; new code can read `org` too via `useScope`.
 */
export function useActiveSite(): ActiveSite {
  const { site, sites, isLoading, isError } = useScope()
  return { site, sites, isLoading, isError }
}
