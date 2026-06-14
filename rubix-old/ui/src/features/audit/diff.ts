/**
 * Field-level before→after diff of two change snapshots
 * (docs/design/audit-and-undo.md: `before`/`after` are full JSON snapshots of the
 * mutated row). Used by the per-resource History tab and the admin Audit screen to
 * render "what changed" without re-deriving the comparison in each view. Kept pure
 * and JSON-only so it is unit-testable.
 *
 * A `create` has no `before` (every field is an addition); a `delete` has no
 * `after` (every field is a removal); an `update` lists only the fields whose
 * value actually changed, so the diff stays small even on a wide row.
 */
export type DiffStatus = 'added' | 'removed' | 'changed'

export interface FieldDiff {
  field: string
  status: DiffStatus
  before?: unknown
  after?: unknown
}

function asRecord(v: unknown): Record<string, unknown> | null {
  return v && typeof v === 'object' && !Array.isArray(v)
    ? (v as Record<string, unknown>)
    : null
}

/** Stable-ish equality for snapshot values (JSON, so structural compare). */
function equal(a: unknown, b: unknown): boolean {
  return JSON.stringify(a) === JSON.stringify(b)
}

/**
 * The list of changed fields between two snapshots, sorted by field name for a
 * stable render. Non-object snapshots (a scalar `before`/`after`) collapse to a
 * single synthetic `value` field so the renderer always has rows to show.
 */
export function diffSnapshots(before: unknown, after: unknown): FieldDiff[] {
  const b = asRecord(before)
  const a = asRecord(after)

  // One or both snapshots are non-objects: compare them as a single value.
  if (!b && !a) {
    if (equal(before, after)) return []
    return [{ field: 'value', status: 'changed', before, after }]
  }
  // A create has no `before`: every field of `after` is an addition. A delete has
  // no `after`: every field of `before` is a removal. (A scalar present-side
  // collapses to the single synthetic `value` field above.)
  if (!b && a) {
    return Object.keys(a)
      .sort((x, y) => x.localeCompare(y))
      .map((field) => ({ field, status: 'added' as const, after: a[field] }))
  }
  if (b && !a) {
    return Object.keys(b)
      .sort((x, y) => x.localeCompare(y))
      .map((field) => ({ field, status: 'removed' as const, before: b[field] }))
  }
  if (!b || !a) return []

  const fields = new Set([...Object.keys(b), ...Object.keys(a)])
  const out: FieldDiff[] = []
  for (const field of fields) {
    const hasB = field in b
    const hasA = field in a
    if (hasB && !hasA) {
      out.push({ field, status: 'removed', before: b[field] })
    } else if (!hasB && hasA) {
      out.push({ field, status: 'added', after: a[field] })
    } else if (!equal(b[field], a[field])) {
      out.push({ field, status: 'changed', before: b[field], after: a[field] })
    }
  }
  return out.sort((x, y) => x.field.localeCompare(y.field))
}

/** Render a snapshot value compactly for a diff cell. */
export function formatDiffValue(v: unknown): string {
  if (v === undefined) return '∅'
  if (v === null) return 'null'
  if (typeof v === 'string') return v
  return JSON.stringify(v)
}
