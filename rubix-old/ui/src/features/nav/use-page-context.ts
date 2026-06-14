/**
 * Assemble the live `PageContext` for the open dashboard (docs/design/page-
 * context-and-nav.md §§1,5): read the `?nav=` node and bare URL params, the
 * board's own tags, and the active org/site scope, and merge per the §1
 * precedence. The result feeds variable resolution so one board mounted at two
 * nav nodes resolves two sites' data.
 *
 * The nav param and bare params come from `window.location` (the same channel
 * the variable bar uses for `?var-*`), so a deep link restores the exact view;
 * the `var-`/`from`/`to`/`refresh`/`nav` keys are reserved and excluded from the
 * bare `url` source so the namespaces never collide.
 */
import { useEffect, useMemo, useState } from 'react'
import type { Uuid } from '@/api/types'
import {
  assemblePageContext,
  readBareParams,
  type PageContext,
} from '@/features/variables/context'
import { useNavTree } from './use-nav'
import { useEntityTags } from './use-entity-tags'
import { navPath } from './tree'

/** A same-tab event so context readers re-assemble when the URL changes without
 *  a full navigation (mirrors the variable bar's `?var-*` sync). */
const PAGE_URL_EVENT = 'rubix:page-url-change'

function locationSearch(): URLSearchParams {
  return new URLSearchParams(window.location.search)
}

/**
 * The assembled page context for a dashboard. `org`/`siteId` come from the route
 * scope; `dashboardId` keys the board's tag read. Returns a stable object that
 * changes identity only when a context input changes, so it is safe to fold into
 * a resolution key.
 */
export function usePageContext(args: {
  org: string | undefined
  siteId?: string
  dashboardId: Uuid | undefined
  /** The open board's route slug — the `nav`/`slug` source value (design §2). */
  boardSlug?: string
}): PageContext {
  const { org, siteId, dashboardId, boardSlug } = args

  const [search, setSearch] = useState<string>(() => window.location.search)
  useEffect(() => {
    const sync = () => setSearch(window.location.search)
    window.addEventListener('popstate', sync)
    window.addEventListener(PAGE_URL_EVENT, sync)
    return () => {
      window.removeEventListener('popstate', sync)
      window.removeEventListener(PAGE_URL_EVENT, sync)
    }
  }, [])

  const params = useMemo(() => new URLSearchParams(search), [search])
  const navId = params.get('nav') ?? undefined
  const bareParams = useMemo(() => readBareParams(params), [params])

  const { data: tree = [] } = useNavTree(org)
  const { data: boardTags = {} } = useEntityTags('dashboard', dashboardId)

  const mount = useMemo(() => {
    if (!navId) return undefined
    const node = tree.find((n) => n.id === navId)
    if (!node) return undefined
    const path = navPath(tree, navId)
    // `slug` is the board's route slug (the `nav`/`slug` source); the node's
    // title is its display `name`. Fall back to the title when no board slug is
    // threaded (e.g. a group node, which carries no board).
    return { node, slug: boardSlug ?? node.title, path }
  }, [navId, tree, boardSlug])

  return useMemo(
    () =>
      assemblePageContext({ org, siteId, bareParams, boardTags, mount }),
    [org, siteId, bareParams, boardTags, mount]
  )
}

/** Notify same-tab page-context readers that the URL changed (e.g. after a
 *  programmatic nav-param change). The variable-bar's own `?var-*` writer fires
 *  its own event; callers that change the `nav`/bare params call this. */
export function notifyPageUrlChange() {
  window.dispatchEvent(new Event(PAGE_URL_EVENT))
}

export { locationSearch }
