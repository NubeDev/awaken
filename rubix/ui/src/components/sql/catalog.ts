// The Rubix SQL catalog that feeds editor autocomplete — the "swap the schema"
// payload (§1a, LAMINAR-BORROW.md). Laminar's equivalent is a 360-line typed
// ClickHouse `tableSchemas`; ours is the small, honest truth of the DataFusion
// surface: a handful of canonical tables (rubix-query/src/provider/schema.rs),
// each carrying the structural columns every record has plus `content` as a
// JSON *string*. Per-`content.kind` typed columns appear here only once the
// virtual-table/catalog layer lands (the deferred §1 prerequisite); until then
// fields are reached with the json_get UDF, which is why it's a function below.

import {
  type CompletionContext,
  type CompletionResult,
  type CompletionSource,
} from '@codemirror/autocomplete'

export interface CatalogColumn {
  name: string
  type: string
  description: string
}

export interface CatalogTable {
  name: string
  description: string
  columns: CatalogColumn[]
}

export interface CatalogFunction {
  name: string
  detail: string
  info: string
}

export interface RubixCatalog {
  tables: CatalogTable[]
  functions: CatalogFunction[]
}

// Every canonical table shares these five columns; `content` is JSON text the
// json_get UDF reaches into (see schema.rs `arrow_schema`).
const STRUCTURAL: CatalogColumn[] = [
  { name: 'id', type: 'String', description: 'Record id (table:ulid)' },
  { name: 'namespace', type: 'String', description: 'Owning namespace / tenant' },
  { name: 'created', type: 'Timestamp', description: 'Creation time' },
  { name: 'updated', type: 'Timestamp', description: 'Last-update time' },
  {
    name: 'content',
    type: 'JSON',
    description: 'Document payload as JSON text — reach in with json_get(content, key)',
  },
]

// The canonical tables the read-only query surface scans. Names match
// CanonicalTable::register_name in rubix-query (== the SurrealDB table name).
export const defaultCatalog: RubixCatalog = {
  tables: [
    { name: 'record', description: 'Generic records — every kind:"…" artifact', columns: STRUCTURAL },
    { name: 'tag', description: 'Tags applied to records', columns: STRUCTURAL },
    { name: 'audit', description: 'Immutable gate audit log', columns: STRUCTURAL },
    { name: 'insight', description: 'Rule-decision insights (WS-11)', columns: STRUCTURAL },
    {
      name: 'trace_summary',
      description: 'Per-correlation-id trace rollup (§5b) — status, tokens, cost',
      columns: STRUCTURAL,
    },
  ],
  functions: [
    { name: 'json_get', detail: '(json, key)', info: 'Reach into a JSON-text column. Composable: json_get(json_get(content,\'content\'),\'kind\').' },
    { name: 'count', detail: '(expr)', info: 'Row / non-null count.' },
    { name: 'count_distinct', detail: '(expr)', info: 'Distinct count.' },
    { name: 'sum', detail: '(expr)', info: 'Sum aggregate.' },
    { name: 'avg', detail: '(expr)', info: 'Average aggregate.' },
    { name: 'min', detail: '(expr)', info: 'Minimum aggregate.' },
    { name: 'max', detail: '(expr)', info: 'Maximum aggregate.' },
    { name: 'approx_percentile_cont', detail: '(expr, q)', info: 'Approximate quantile — DataFusion\'s stand-in for ClickHouse quantile(q)(col).' },
    { name: 'date_trunc', detail: "(unit, ts)", info: "Truncate a timestamp to a unit ('day', 'hour', …)." },
    { name: 'date_bin', detail: '(stride, ts, origin)', info: 'Bucket a timestamp into fixed-width intervals — epoch-aligned time buckets.' },
    { name: 'cast', detail: '(expr AS type)', info: 'Type cast, e.g. CAST(json_get(content,\'total_tokens\') AS BIGINT).' },
    { name: 'coalesce', detail: '(a, b, …)', info: 'First non-null argument.' },
  ],
}

// A completion source that offers the catalog's functions anywhere a word is
// being typed (lang-sql's schema source covers tables/columns; functions are
// ours). Mirrors the spirit of Laminar's custom completions, much trimmed.
export function functionCompletions(catalog: RubixCatalog): CompletionSource {
  return (context: CompletionContext): CompletionResult | null => {
    const word = context.matchBefore(/\w*/)
    if (!word || (word.from === word.to && !context.explicit)) return null
    return {
      from: word.from,
      options: catalog.functions.map((fn) => ({
        label: fn.name,
        type: 'function',
        detail: fn.detail,
        info: fn.info,
        apply: `${fn.name}(`,
      })),
      validFor: /^\w*$/,
    }
  }
}
