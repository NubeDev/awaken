import { useSites } from '@/api/hooks'
import { useSiteStore } from '@/stores/site-store'
import type { Site } from '@/api/types'

type ActiveSite = {
  site: Site | undefined
  sites: Site[]
  isLoading: boolean
  isError: boolean
}

/**
 * Resolve the site pages should render: the operator's stored selection if it
 * still exists, otherwise the first site returned by the API. Centralised so
 * every page agrees on "the active site" without re-deriving the fallback.
 */
export function useActiveSite(): ActiveSite {
  const { data: sites = [], isLoading, isError } = useSites()
  const siteId = useSiteStore((s) => s.siteId)
  const site = sites.find((s) => s.id === siteId) ?? sites[0]
  return { site, sites, isLoading, isError }
}
