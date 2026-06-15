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

export function runQuery(client: ApiClient, sql: string, time?: TimeScope): Promise<QueryResponse> {
  return client.post<QueryResponse>('query', time ? { sql, time } : { sql })
}

/** One keyed statement in a batch (typically keyed by chart id). */
export interface BatchQueryItem {
  key: string
  sql: string
  time?: TimeScope
}

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
