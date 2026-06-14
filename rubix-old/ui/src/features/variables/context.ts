/**
 * `PageContext` assembly (docs/design/page-context-and-nav.md §1). A page reads
 * its URL, its nav-tree position, and its tags, and feeds those into variable
 * resolution so one board mounted at two nav nodes resolves two sites' data —
 * no second board authored.
 *
 * The sources are kept **separate** (not pre-flattened) so a `context` variable
 * can address exactly one, and so precedence is explicit and testable. Context
 * is read-only input to resolution — never a fourth store, never a `varOverrides`
 * channel: a `context` variable's resolved value becomes its `current` and from
 * there binds as a SQL parameter via the same injection-safe engine as any other
 * variable.
 */
import type {
  ContextSource,
  NavContext,
  NavNode,
  ScalarValue,
  VariableValue,
} from '@/api/types'

/** The page context assembled at view time. Sources are kept separate; a
 *  `context` variable selects exactly one via its `source`. */
export type PageContext = {
  /** The nav node the page opened under, if any. */
  nav?: { node_id: string; slug: string; name: string; path: string[] }
  /** Bare URL query params (`?building=…`); never the `var-*` namespace. */
  url: Record<string, string | string[]>
  /** The board's tags merged under `NavNode.context.tags`. */
  tags: Record<string, string | null>
  /** Rubix's existing URL scope. */
  scope: { org: string | undefined; site_id?: string }
  /** `NavNode.context.values` overrides, keyed by variable name. */
  values: Record<string, VariableValue>
}

/** A nav node resolved for context assembly: its identity, the slug it opened
 *  the board under, and its ancestor titles (root-first) for `nav` path keys. */
export type NavMount = {
  node: NavNode
  /** The board slug the node mounts (the route segment), for `nav`/`slug`. */
  slug: string
  /** Ancestor titles root-first, including this node's title last. */
  path: string[]
}

/** Read bare (non-`var-`) URL params into a plain record. A repeated param
 *  becomes an array; the `var-` prefix is reserved for explicit variable state
 *  and is excluded here so the two never collide. */
export function readBareParams(
  params: URLSearchParams
): Record<string, string | string[]> {
  const out: Record<string, string | string[]> = {}
  for (const key of new Set(params.keys())) {
    if (key.startsWith('var-')) continue
    // The router owns these scope/time params; they are not page-context input.
    if (key === 'from' || key === 'to' || key === 'refresh' || key === 'nav') {
      continue
    }
    const all = params.getAll(key)
    out[key] = all.length > 1 ? all : (all[0] ?? '')
  }
  return out
}

/**
 * Assemble the `PageContext` from its separate sources. `boardTags` are the
 * dashboard's own tags; a mounted node's `context.tags` merge **over** them, and
 * its `context.values` populate `values`. Nothing is flattened across sources —
 * precedence between them is applied later, per variable, in resolution.
 */
export function assemblePageContext(args: {
  org: string | undefined
  siteId?: string
  bareParams: Record<string, string | string[]>
  boardTags: Record<string, string | null>
  mount?: NavMount
}): PageContext {
  const { org, siteId, bareParams, boardTags, mount } = args
  const nodeCtx: NavContext = mount?.node.context ?? {}

  // Board tags first, then node tag pins win for this mount (design §1).
  const tags: Record<string, string | null> = { ...boardTags }
  for (const [k, v] of Object.entries(nodeCtx.tags ?? {})) tags[k] = v

  const values: Record<string, VariableValue> = {}
  for (const [k, v] of Object.entries(nodeCtx.values ?? {})) {
    values[k] = v as VariableValue
  }

  return {
    nav: mount
      ? {
          node_id: mount.node.id,
          slug: mount.slug,
          name: mount.node.title,
          path: mount.path,
        }
      : undefined,
    url: bareParams,
    tags,
    scope: { org, site_id: siteId },
    values,
  }
}

/** A `nav` source `key`: `slug`, `name`, or `path[n]` (zero-based ancestor). */
const NAV_PATH_KEY = /^path\[(\d+)\]$/

/**
 * Resolve a `context` variable's value from the assembled `PageContext`. Returns
 * `undefined` when the source has no value for `key` (the variable then falls
 * back like any other — to its node default, then null). The returned value is
 * plain data: it binds as a parameter downstream, so a `key` or value containing
 * SQL metacharacters is quoted, never executed.
 */
export function resolveContextValue(
  ctx: PageContext,
  source: ContextSource,
  key: string
): VariableValue | undefined {
  switch (source) {
    case 'nav': {
      if (!ctx.nav) return undefined
      if (key === 'slug') return ctx.nav.slug
      if (key === 'name') return ctx.nav.name
      const m = NAV_PATH_KEY.exec(key)
      if (m) return ctx.nav.path[Number(m[1])] ?? undefined
      return undefined
    }
    case 'url': {
      const v = ctx.url[key]
      return v === undefined ? undefined : (v as VariableValue)
    }
    case 'tag': {
      const v = ctx.tags[key]
      return v === undefined ? undefined : (v as ScalarValue)
    }
    case 'values': {
      const v = ctx.values[key]
      return v === undefined ? undefined : v
    }
    default:
      return undefined
  }
}

/**
 * The built-in page-context tokens that need no authoring (design §2):
 * `$__nav_slug`, `$__nav_name`, and `$__tag(key)`. Returns the seed values to
 * feed into resolution alongside authored `context` variables, so widget SQL can
 * reference them directly. `$__tag(key)` is expanded per distinct tag key present
 * on the page.
 */
export function builtinContextSeeds(
  ctx: PageContext
): Record<string, VariableValue> {
  const seeds: Record<string, VariableValue> = {}
  if (ctx.nav) {
    seeds.__nav_slug = ctx.nav.slug
    seeds.__nav_name = ctx.nav.name
  }
  for (const [k, v] of Object.entries(ctx.tags)) {
    // `$__tag(building)` is referenced as the name `__tag(building)` by the
    // engine's token grammar; seed every present tag so a reference resolves.
    if (v !== null) seeds[`__tag(${k})`] = v
  }
  return seeds
}
