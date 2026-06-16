// The transform pipeline — the portable, hybrid-execution spec (§1). Adopted from
// nexus's `Transform` union verbatim, with the executor split by the scope's
// hybrid decision (DASHBOARDS-SCOPE.md §1):
//
//   - AGGREGATE ops (filter/groupBy/reduce) change row cardinality and shrink the
//     wire, so they run SERVER-SIDE (rubix-query's DataFusion stage). The client
//     sends them in the request and does NOT re-run them.
//   - COSMETIC ops (rename/calculated/organize) are row-by-row or column reorders
//     and run CLIENT-SIDE here, so builder edits are instant — no round trip.
//
// The full spec is stored on the chart record (the durable, portable contract);
// `splitTransforms` partitions it, `applyCosmeticTransforms` runs the client tier
// over already-fetched rows. Pure — same rows + same spec → same rows.

export type CompareOp = '=' | '!=' | '>' | '>=' | '<' | '<='
export type CalcOp = '+' | '-' | '*' | '/'
export type Agg = 'sum' | 'avg' | 'min' | 'max' | 'count'
export type ReduceCalc = 'first' | 'last' | 'sum' | 'avg' | 'min' | 'max' | 'count'

/** A client-side transform applied to query rows. The `kind` discriminant selects
 *  the op; each carries only its own config. Runs as an ordered pipeline. */
export type Transform =
  | { kind: 'rename'; from: string; to: string }
  | { kind: 'calculated'; field: string; left: string; op: CalcOp; right: string }
  | { kind: 'filter'; field: string; op: CompareOp; value: string }
  | { kind: 'groupBy'; by: string; field: string; agg: Agg; as: string }
  | { kind: 'reduce'; field: string; calc: ReduceCalc; as: string }
  | { kind: 'organize'; order: ReadonlyArray<string> }

type Row = Record<string, unknown>

/** Whether a transform changes row cardinality (server-side) vs cosmetic (client). */
export function isAggregate(t: Transform): boolean {
  return t.kind === 'filter' || t.kind === 'groupBy' || t.kind === 'reduce'
}

/** Partition a spec into the server-side (aggregate) and client-side (cosmetic)
 *  tiers, preserving order within each. The aggregate tier is sent with the
 *  request; the cosmetic tier runs locally after the rows return. */
export function splitTransforms(transforms: ReadonlyArray<Transform> | undefined): {
  aggregate: Transform[]
  cosmetic: Transform[]
} {
  const aggregate: Transform[] = []
  const cosmetic: Transform[] = []
  for (const t of transforms ?? []) (isAggregate(t) ? aggregate : cosmetic).push(t)
  return { aggregate, cosmetic }
}

/** Run the cosmetic tier over `rows`, in declared order. Aggregate ops are skipped
 *  (the backend ran them). Pure — returns new rows, never mutates the input. */
export function applyCosmeticTransforms(
  rows: ReadonlyArray<Row>,
  transforms: ReadonlyArray<Transform> | undefined,
): Row[] {
  let out: Row[] = [...rows]
  for (const t of transforms ?? []) {
    if (isAggregate(t)) continue
    out = applyOne(out, t)
  }
  return out
}

function applyOne(rows: Row[], t: Transform): Row[] {
  switch (t.kind) {
    case 'rename':
      return applyRename(rows, t.from, t.to)
    case 'calculated':
      return applyCalculated(rows, t.field, t.left, t.op, t.right)
    case 'organize':
      return applyOrganize(rows, t.order)
    default:
      return rows // aggregate ops handled server-side
  }
}

// Copy `from` into `to`, drop `from`. No-op when equal or `from` absent.
function applyRename(rows: Row[], from: string, to: string): Row[] {
  if (!from || !to || from === to) return [...rows]
  return rows.map((row) => {
    if (!(from in row)) return { ...row }
    const next: Row = { ...row, [to]: row[from] }
    delete next[from]
    return next
  })
}

// Add `field = left <op> right`. Non-numeric operands (or /0) → null, not NaN.
function applyCalculated(rows: Row[], field: string, left: string, op: CalcOp, right: string): Row[] {
  if (!field) return [...rows]
  return rows.map((row) => ({ ...row, [field]: compute(numeric(row[left]), numeric(row[right]), op) }))
}

function compute(a: number | null, b: number | null, op: CalcOp): number | null {
  if (a == null || b == null) return null
  switch (op) {
    case '+':
      return a + b
    case '-':
      return a - b
    case '*':
      return a * b
    case '/':
      return b === 0 ? null : a / b
  }
}

function numeric(v: unknown): number | null {
  if (typeof v === 'number') return Number.isFinite(v) ? v : null
  if (typeof v === 'string' && v.trim() !== '') {
    const n = Number(v)
    return Number.isFinite(n) ? n : null
  }
  return null
}

// Reorder each row's columns to follow `order`, appending any unlisted keys in
// their original relative order (controls display order without dropping data).
function applyOrganize(rows: Row[], order: ReadonlyArray<string>): Row[] {
  return rows.map((row) => {
    const next: Row = {}
    for (const key of order) if (key in row) next[key] = row[key]
    for (const key of Object.keys(row)) if (!(key in next)) next[key] = row[key]
    return next
  })
}
