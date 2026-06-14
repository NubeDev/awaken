/**
 * Map an undo/redo result to the React Query key prefixes that must be
 * invalidated so the UI reflects the replayed change without a reload
 * (docs/design/audit-and-undo.md "Undo/Redo": the response carries the touched
 * resource ids so the canvas refreshes exactly what moved).
 *
 * The server returns touched resource *ids* but not their kinds (one undo group
 * can span widgets + a dashboard layout). Rather than guess a kind per id, we
 * invalidate the config-entity key prefixes an undo can affect, scoped to the
 * touched-id set being non-empty — a refetch of a handful of list queries is far
 * cheaper than a full reload, and TanStack dedupes overlapping invalidations.
 *
 * Returns the key prefixes (not the individual ids) so the caller pairs each with
 * `queryClient.invalidateQueries({ queryKey })`. Kept pure and data-only so it is
 * unit-testable without a running query client.
 */
import type { UndoResult } from '@/api/types'

/**
 * The config-entity key roots an undo/redo can touch. These match the leading
 * segment of the `qk.*` factories in `api/keys.ts`; invalidating a root refetches
 * every list/detail query nested under it (the canvas tiles, the dashboard list,
 * the rules table, …).
 */
const TOUCHABLE_ROOTS: readonly string[] = [
  'widgets',
  'widget-data',
  'dashboards',
  'sites',
  'equips',
  'points',
  'sparks',
  'rules',
  'boards',
  'nav',
  'tags',
  'grants',
  'users',
  'teams',
]

/**
 * The query-key prefixes to invalidate after an undo/redo. Empty when nothing
 * moved (`touched` is empty), so a no-op undo never churns the cache.
 */
export function invalidationKeys(result: UndoResult): string[][] {
  if (result.touched.length === 0) return []
  return TOUCHABLE_ROOTS.map((root) => [root])
}
