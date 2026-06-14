/**
 * Pure sibling-reorder computation for the navigation builder (docs/design/page-
 * context-and-nav.md §4). Moving a node up/down among its siblings is a patch to
 * the two affected nodes' `sort_order`; the builder applies the patches via the
 * nav PATCH route. Kept pure so the index math is unit-testable apart from React.
 */
import type { NavNode } from '@/api/types'

/** A single reorder patch: a node and its new `sort_order`. */
export type ReorderPatch = { id: string; sort_order: number }

/** The siblings of `parentId` (NULL = root) in `nodes`, ordered by current
 *  `sort_order` then title. */
export function siblings(
  nodes: NavNode[],
  parentId: string | null
): NavNode[] {
  return nodes
    .filter((n) => (n.parent_id ?? null) === parentId)
    .sort((a, b) => a.sort_order - b.sort_order || a.title.localeCompare(b.title))
}

/**
 * The patches to move `nodeId` by `delta` (-1 up, +1 down) among its siblings.
 * Returns `[]` when the move is a no-op (already at an edge, or the node is
 * unknown). Adjacent siblings swap `sort_order`, normalised to their array index
 * so repeated moves stay stable even if the stored orders had gaps or ties.
 */
export function movePatches(
  nodes: NavNode[],
  nodeId: string,
  delta: -1 | 1
): ReorderPatch[] {
  const node = nodes.find((n) => n.id === nodeId)
  if (!node) return []
  const sibs = siblings(nodes, node.parent_id ?? null)
  const i = sibs.findIndex((n) => n.id === nodeId)
  const j = i + delta
  if (i < 0 || j < 0 || j >= sibs.length) return []
  // Swap the two nodes' positions, then re-number the whole sibling run to its
  // index so the result is a clean 0..n-1 ordering with no ties.
  const reordered = sibs.slice()
  ;[reordered[i], reordered[j]] = [reordered[j], reordered[i]]
  const patches: ReorderPatch[] = []
  reordered.forEach((n, idx) => {
    if (n.sort_order !== idx) patches.push({ id: n.id, sort_order: idx })
  })
  return patches
}
