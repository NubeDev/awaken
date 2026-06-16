/**
 * The dashboard scope model + the tag-filter primitive every level-builder uses
 * (DASHBOARDS.md §"Auto-build: tags → pages"). A page is built for a hierarchy
 * level keyed by a record's `key`; the builder walks `content.tags` to find the
 * records that belong to that scope.
 *
 * Tags are read from `content.tags` (WS-03/WS-06 — rubix has no HTTP tag-attach
 * route, so NHP's standard tags live in content, NOT the graph edge the server
 * `?tag=` filter sees). The exact tag strings come from enums/tags.ts — the SAME
 * builders the seed and wizards WRITE, so a page keys off precisely the tags the
 * data carries. Re-deriving the strings inline here would risk the silent-empty
 * drift WS-07 warns about, so we import the builders.
 */
import type { RecordDto } from '@/api/records'

export type ScopeLevel = 'tenant' | 'site' | 'gateway' | 'meter'

export interface Scope {
  level: ScopeLevel
  /** The record `key` the page is built for. */
  key: string
  /** Display name for the header/breadcrumb. */
  name: string
}

/** A record's standard tags, read from `content.tags` (the WS-03 convention). */
export function recordTags(rec: RecordDto<{ tags?: string[] }>): string[] {
  return rec.content.tags ?? []
}

/** Records that carry EVERY tag in `required` (Haystack-style intersection). */
export function withTags<C extends { tags?: string[] }>(
  records: RecordDto<C>[],
  required: string[]
): RecordDto<C>[] {
  return records.filter((r) => {
    const tags = r.content.tags ?? []
    return required.every((t) => tags.includes(t))
  })
}
