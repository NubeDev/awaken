/**
 * Reads over a `TagSet` (the wire map of tag name -> JSON value). The UI treats
 * tags with Haystack marker semantics — presence of a name, not its value — so
 * these helpers project the map down to the name list the chips and filters use.
 * See `crates/rubix-core/src/tags.rs` for the wire shape.
 */
import type { TagSet } from './types';

/** Tag names present on the set, in stable (sorted) order to match the store. */
export function tagNames(tags: TagSet): string[] {
  return Object.keys(tags).sort();
}

/** True when every named marker tag is present (value is irrelevant). */
export function hasTag(tags: TagSet, name: string): boolean {
  return Object.prototype.hasOwnProperty.call(tags, name);
}
