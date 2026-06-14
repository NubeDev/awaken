/**
 * Assemble the flat nav-node list the server returns into a nested tree, and
 * derive the ancestor path of a node (docs/design/page-context-and-nav.md §4).
 * The server returns nodes flat (in `parent_id`/`sort_order` order) and already
 * filtered to the caller's `view` grants, so a child whose parent was filtered
 * out is re-parented to root rather than dropped — the operator still reaches it.
 */
import type { NavNode } from '@/api/types'

/** A nav node with its resolved children, nested for rendering. */
export type NavTreeNode = NavNode & { children: NavTreeNode[] }

/**
 * Nest `nodes` into a tree, ordering siblings by `sort_order` then title. A node
 * whose `parent_id` is absent from `nodes` (filtered out by the grant view, or a
 * true root) becomes a root, so no visible node is ever orphaned off-screen.
 */
export function assembleNavTree(nodes: NavNode[]): NavTreeNode[] {
  const byId = new Map<string, NavTreeNode>()
  for (const n of nodes) byId.set(n.id, { ...n, children: [] })

  const roots: NavTreeNode[] = []
  for (const node of byId.values()) {
    const parent =
      node.parent_id != null ? byId.get(node.parent_id) : undefined
    if (parent) parent.children.push(node)
    else roots.push(node)
  }

  const sort = (a: NavTreeNode, b: NavTreeNode) =>
    a.sort_order - b.sort_order || a.title.localeCompare(b.title)
  const sortRec = (list: NavTreeNode[]) => {
    list.sort(sort)
    for (const n of list) sortRec(n.children)
  }
  sortRec(roots)
  return roots
}

/**
 * The ancestor-title path of `nodeId`, root-first, including the node's own
 * title last (so `path[0]` is the topmost ancestor). Returns `[]` for an unknown
 * id. A parent cycle (should not occur server-side) is bounded by the node count.
 */
export function navPath(nodes: NavNode[], nodeId: string): string[] {
  const byId = new Map(nodes.map((n) => [n.id, n]))
  const path: string[] = []
  const seen = new Set<string>()
  let cur = byId.get(nodeId)
  while (cur && !seen.has(cur.id)) {
    seen.add(cur.id)
    path.unshift(cur.title)
    cur = cur.parent_id != null ? byId.get(cur.parent_id) : undefined
  }
  return path
}
