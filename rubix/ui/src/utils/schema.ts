// Derive the backend's shape from live records — purely structural, no domain
// knowledge. The schema inspector exists so a developer can see how the seeded
// backend is built: what kinds exist, what fields each carries, what tags are in
// play. It must work against ANY backend (EMS, project management, anything), so
// it never names a domain type — it groups by whatever `content.kind` strings the
// data actually contains and infers field shapes by sampling content.
//
// Where a `kind: "collection"` record registers a real schema, the inspector
// overlays it (declared fields are authoritative); everywhere else the shape is
// inferred. This mirrors the backend's own fail-open ramp (BACKEND-COLLECTIONS).

import type { Record } from '../types/Record'
import type { CollectionDef, FieldType } from '../api/collections'

/** The inferred JSON shape of a single field, by sampling values across records. */
export interface FieldShape {
  name: string
  /** The JSON types observed for this field (more than one ⇒ inconsistent). */
  types: string[]
  /** Fraction of the kind's records that carry this field (1 = always present). */
  presence: number
  /** A short sample value rendered for the developer, from the first record. */
  sample: string
  /** True when this field is declared by a registered collection schema. */
  declared: boolean
  /** The declared field type, when the kind has a registered collection. */
  declaredType?: FieldType
}

/** One kind's derived profile: how many records, and each field's shape. */
export interface KindProfile {
  kind: string
  count: number
  fields: FieldShape[]
  /** True when a `kind: "collection"` record registers a schema for this kind. */
  hasCollection: boolean
}

/** The JSON type name of a value, the way a developer thinks of it. */
export function jsonType(v: unknown): string {
  if (v === null) return 'null'
  if (Array.isArray(v)) return 'array'
  return typeof v // 'string' | 'number' | 'boolean' | 'object' | 'undefined'
}

function sampleOf(v: unknown): string {
  if (v === null || v === undefined) return ''
  if (typeof v === 'string') return v.length > 40 ? `${v.slice(0, 40)}…` : v
  if (typeof v === 'object') {
    const json = JSON.stringify(v)
    return json.length > 40 ? `${json.slice(0, 40)}…` : json
  }
  return String(v)
}

/** Group records by `content.kind`, profiling each kind's field shapes. Records
 *  with no `kind` are grouped under '(unkinded)' so nothing is silently dropped. */
export function profileKinds(records: Record[], collections: CollectionDef[] = []): KindProfile[] {
  const byKind = new Map<string, Record[]>()
  for (const r of records) {
    const kind = typeof r.content?.kind === 'string' && r.content.kind ? r.content.kind : '(unkinded)'
    const list = byKind.get(kind)
    if (list) list.push(r)
    else byKind.set(kind, [r])
  }

  const collectionByName = new Map(collections.map((c) => [c.name, c]))

  const profiles: KindProfile[] = []
  for (const [kind, group] of byKind) {
    const collection = collectionByName.get(kind)
    profiles.push({
      kind,
      count: group.length,
      hasCollection: collection !== undefined,
      fields: profileFields(group, collection),
    })
  }
  // Stable, useful order: most-populated kinds first, then alphabetical.
  return profiles.sort((a, b) => b.count - a.count || a.kind.localeCompare(b.kind))
}

function profileFields(group: Record[], collection?: CollectionDef): FieldShape[] {
  // Every content key seen across the group (excluding `kind`, the discriminator).
  const fieldNames = new Set<string>()
  for (const r of group) {
    for (const key of Object.keys(r.content ?? {})) {
      if (key !== 'kind') fieldNames.add(key)
    }
  }
  // Declared fields appear even when no record populates them yet.
  for (const f of collection?.schema ?? []) fieldNames.add(f.name)

  const declared = new Map((collection?.schema ?? []).map((f) => [f.name, f.type]))

  const shapes: FieldShape[] = []
  for (const name of fieldNames) {
    const types = new Set<string>()
    let present = 0
    let sample = ''
    for (const r of group) {
      const content = r.content as { [k: string]: unknown }
      if (name in content && content[name] !== undefined) {
        present++
        types.add(jsonType(content[name]))
        if (!sample) sample = sampleOf(content[name])
      }
    }
    shapes.push({
      name,
      types: [...types].sort(),
      presence: group.length ? present / group.length : 0,
      sample,
      declared: declared.has(name),
      declaredType: declared.get(name),
    })
  }
  // Declared fields first, then by presence (most consistent fields on top).
  return shapes.sort(
    (a, b) => Number(b.declared) - Number(a.declared) || b.presence - a.presence || a.name.localeCompare(b.name),
  )
}

/** The distinct tag set across all records, with how many records carry each.
 *  Tags are the structure-by-tagging primitive (SCOPE) — a substrate concept,
 *  not a domain one. */
export function tagFrequencies(records: Record[]): { tag: string; count: number }[] {
  const counts = new Map<string, number>()
  for (const r of records) {
    for (const t of r.tags ?? []) {
      counts.set(t, (counts.get(t) ?? 0) + 1)
    }
  }
  return [...counts.entries()]
    .map(([tag, count]) => ({ tag, count }))
    .sort((a, b) => b.count - a.count || a.tag.localeCompare(b.tag))
}
