/**
 * Pure operations over an entity's tag set (docs/design/page-context-and-nav.md
 * §3), kept separate from the editor component so the set mutations are unit-
 * testable. Tags are a `key → value` map where a `null` value is a present-but-
 * unset key; these return a new map, never mutate in place. The `PUT` replaces
 * the whole set, so the editor edits a working copy then saves it wholesale.
 */
import type { EntityTags } from '@/api/types'

/** A draft tag row in the editor (an ordered list is easier to render/edit than
 *  a map; it serialises back to the map on save). */
export type TagRow = { key: string; value: string }

/** The set as an ordered row list, sorted by key for a stable render. A `null`
 *  value renders as an empty string (the present-but-unset case). */
export function toRows(tags: EntityTags): TagRow[] {
  return Object.entries(tags)
    .map(([key, value]) => ({ key, value: value ?? '' }))
    .sort((a, b) => a.key.localeCompare(b.key))
}

/**
 * Serialise rows back to the wire map for `PUT`. Blank-keyed rows are dropped
 * (an unnamed tag is meaningless); a duplicate key keeps the last row's value so
 * the operator's latest edit wins. An empty value persists as `null` (a
 * present-but-unset key) so it round-trips and `$__tag(key)` resolves to NULL.
 */
export function toTags(rows: TagRow[]): EntityTags {
  const out: EntityTags = {}
  for (const { key, value } of rows) {
    const k = key.trim()
    if (k === '') continue
    out[k] = value === '' ? null : value
  }
  return out
}

export function setRow(rows: TagRow[], index: number, patch: Partial<TagRow>): TagRow[] {
  return rows.map((r, i) => (i === index ? { ...r, ...patch } : r))
}

export function addRow(rows: TagRow[]): TagRow[] {
  return [...rows, { key: '', value: '' }]
}

export function removeRow(rows: TagRow[], index: number): TagRow[] {
  return rows.filter((_, i) => i !== index)
}
