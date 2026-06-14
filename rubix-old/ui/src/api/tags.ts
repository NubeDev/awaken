/**
 * Reads over a `TagSet` (the wire map of tag name -> JSON value). The UI treats
 * tags with Haystack marker semantics — presence of a name, not its value — so
 * these helpers project the map down to the name list the chips and filters use.
 * See `crates/rubix-core/src/tags.rs` for the wire shape.
 */
import type { TagSet } from './types'

/** Tag names present on the set, in stable (sorted) order to match the store. */
export function tagNames(tags: TagSet): string[] {
  return Object.keys(tags).sort()
}

/** True when every named marker tag is present (value is irrelevant). */
export function hasTag(tags: TagSet, name: string): boolean {
  return Object.prototype.hasOwnProperty.call(tags, name)
}

/**
 * Parse a comma/space-separated marker list (e.g. `"ahu, vav, submeter"`) into a
 * `TagSet` of `name -> true` markers — the inverse of `tagNames` for the CRUD
 * editors. Empty input yields an empty set. Names are trimmed; blanks dropped.
 */
export function parseTags(input: string): TagSet {
  const tags: TagSet = {}
  for (const raw of input.split(/[,\s]+/)) {
    const name = raw.trim()
    if (name) tags[name] = true
  }
  return tags
}

/** Render a `TagSet` back to the comma-separated marker string the editors show. */
export function tagsToInput(tags: TagSet): string {
  return tagNames(tags).join(', ')
}
