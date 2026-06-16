// Navigation nodes as records — `kind:"nav_node"` over the generic record surface
// (PAGE-CONTEXT-AND-NAV.md §4). A node mounts a board (or a static route) and
// carries a context payload; the UI builds the nested sidebar tree from the flat
// node list via `parent`/`sort_order`. Here we only need the read side that the
// board builder uses: load the node behind `?nav=<id>` and read its
// `context.values` to parameterise the board (the fleet story: one board, many
// mounts).

import type { ApiClient } from './client'
import type { Record, RecordContent } from '../types/Record'
import { getRecord, listRecords } from './records'

export const NAV_NODE_KIND = 'nav_node'

// What a node points at: a non-clickable group header, a reusable board mount, or
// a static built-in page (the `route` allow-list).
export type NavTarget =
  | { kind: 'group' }
  | { kind: 'board'; board: string }
  | { kind: 'route'; route: string }

// A node's context payload (board targets only): variable-value overrides merged
// into the page context, and tags pinned over the board's own (§1). Plain index
// signatures, since the `Record` import here is the wire record, not TS's utility.
export interface NavContext {
  values?: { [name: string]: string | string[] }
  tags?: { [key: string]: string }
}

export interface NavNode {
  id: string
  parent: string | null
  title: string
  sort_order: number
  target: NavTarget
  context?: NavContext
}

interface NavNodeContent extends RecordContent {
  parent?: string | null
  title?: string
  sort_order?: number
  target?: NavTarget
  context?: NavContext
}

function toNavNode(record: Record): NavNode {
  const c = record.content as NavNodeContent
  return {
    id: record.id,
    parent: typeof c.parent === 'string' ? c.parent : null,
    title: typeof c.title === 'string' ? c.title : '(untitled)',
    sort_order: typeof c.sort_order === 'number' ? c.sort_order : 0,
    target: c.target ?? { kind: 'group' },
    context: c.context,
  }
}

export async function listNavNodes(client: ApiClient): Promise<NavNode[]> {
  const records = await listRecords(client, { kind: NAV_NODE_KIND })
  return records.map(toNavNode).sort((a, b) => a.sort_order - b.sort_order)
}

// Load one node by id (the `?nav=` deep-link target). Returns null if it is not a
// nav node the caller can see (a stale link) so the board just opens unbound.
export async function getNavNode(client: ApiClient, id: string): Promise<NavNode | null> {
  try {
    const record = await getRecord(client, id)
    if ((record.content as RecordContent).kind !== NAV_NODE_KIND) return null
    return toNavNode(record)
  } catch {
    return null
  }
}
