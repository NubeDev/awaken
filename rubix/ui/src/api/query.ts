// Query surface — POST /query (DataFusion). Read-only, gated on external-query.
//
// Time is a structured, UTC backend concern (DASHBOARDS-SCOPE.md §5): a board
// sends an absolute epoch-ms window (or a relative token) plus a grain or target
// point count, and the backend injects the window/bucket by expanding the chart's
// `$__timeFilter` / `$__timeBucket` / `$__interval` macros. The client never
// formats a locale datetime into SQL (the timezone bug this replaces).

import type { ApiClient } from './client'

export type Grain = 'minute' | 'hour' | 'day' | 'week'

/** One window bound: an absolute UTC epoch-ms instant, or a relative token. */
export type TimeBound = number | string

/** A structured, UTC time scope the backend injects into a query. */
export interface TimeScope {
  from: TimeBound
  to: TimeBound
  /** An explicit bucket grain. */
  grain?: Grain
  /** A desired bucket count the backend snaps a grain to (ignored if `grain`). */
  target_points?: number
}

/** A result column's name and coarse type, as returned by the backend. */
export interface QueryColumn {
  name: string
  type: 'number' | 'string' | 'boolean' | 'timestamp' | 'other'
}

export interface QueryResponse {
  rows: Record<string, unknown>[]
  columns: QueryColumn[]
}

/** Optional extras a single query may carry alongside the SQL. */
export interface QueryExtras {
  time?: TimeScope
  /** `column → physical quantity` for post-read unit conversion (§2). */
  quantities?: Record<string, string>
  /** Transform spec; only aggregate ops run server-side (§1). */
  transforms?: WireTransform[]
}

export function runQuery(client: ApiClient, sql: string, extras: QueryExtras = {}): Promise<QueryResponse> {
  return client.post<QueryResponse>('query', { sql, ...extras })
}

/** One keyed statement in a batch (typically keyed by chart id). */
export interface BatchQueryItem {
  key: string
  sql: string
  /** A saved-query id resolved to SQL server-side on the caller's scope (§4b);
   *  when set, `sql` is ignored. */
  query_id?: string
  time?: TimeScope
  /** Optional `column → physical quantity` map (§2/§7). The backend converts each
   *  named column to the caller's unit system after reading (post-cache). */
  quantities?: Record<string, string>
  /** Optional transform spec. Only the aggregate ops (filter/groupBy/reduce) are
   *  executed server-side; cosmetic ops are ignored here and run client-side (§1). */
  transforms?: WireTransform[]
}

/** A transform on the wire — the discriminated union the backend's aggregate tier
 *  reads (§1). The cosmetic-op variants are accepted but run client-side. */
export type WireTransform =
  | { kind: 'rename'; from: string; to: string }
  | { kind: 'calculated'; field: string; left: string; op: string; right: string }
  | { kind: 'filter'; field: string; op: string; value: string }
  | { kind: 'groupBy'; by: string; field: string; agg: string; as: string }
  | { kind: 'reduce'; field: string; calc: string; as: string }
  | { kind: 'organize'; order: ReadonlyArray<string> }

/** One statement's outcome: rows + columns, or an error — never both. */
export interface BatchQueryResult {
  key: string
  rows?: Record<string, unknown>[]
  columns?: QueryColumn[]
  error?: string
}

export interface BatchQueryResponse {
  results: BatchQueryResult[]
}

// Run a whole board in one round trip (§3). One bad panel reports its error while
// the others render; the HTTP status is 200 unless the request itself is bad.
export function runBatchQuery(
  client: ApiClient,
  queries: BatchQueryItem[],
): Promise<BatchQueryResponse> {
  return client.post<BatchQueryResponse>('query/batch', { queries })
}

/** One readable table and its columns, as the principal addresses it in SQL. */
export interface TableSchema {
  /** The schema: the default catalog for native tables, or a datasource id. */
  schema: string
  /** The bare table name. */
  table: string
  /** The columns, with the same coarse type tags as query result columns. */
  columns: QueryColumn[]
}

export interface QuerySchemaResponse {
  tables: TableSchema[]
}

// The tables + columns the principal can read (§4b) — for query autocomplete and
// to stop charts guessing the JSON shape. Row-perm aware and gated on the same
// external-query capability as POST /query.
export function fetchQuerySchema(client: ApiClient): Promise<QuerySchemaResponse> {
  return client.get<QuerySchemaResponse>('query/schema')
}
