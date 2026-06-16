/**
 * Typed access to the rubix readings (time-series) plane — the data-plane sibling
 * of the config-plane `/records` API (api/records.ts). Readings are NOT records:
 * they are lean, append-only `{ series, at, value }` rows in a dedicated `reading`
 * table (READINGS-TIMESERIES.md), never crossing the command gate, namespace-scoped
 * server-side. The one interesting read is "this series, this window":
 *   GET /readings?series=<registerRecordId>&from=<rfc3339>&to=<rfc3339>
 *     → Reading[]  (ordered by `at`)
 * Auth is the same `x-rubix-subject`/`-secret` header pair `listRecords` uses
 * (see api/client.ts); `request<T>` handles it.
 */
import { request } from './client'

/**
 * One sample as the historian read returns it. `series` is the bare register
 * RECORD ID the sample belongs to — its display metadata (unit/quantity/precision)
 * lives ONCE on that register, not duplicated per row. `at` is MEASUREMENT time
 * (RFC3339), the instant the world produced the value — not write time.
 */
export interface Reading {
  series: string
  at: string
  value: number
}

/**
 * Windowed, series-scoped historian read: the readings for ONE series between
 * `from` and `to` (both RFC3339), ordered by `at`. This is the windowed query that
 * replaces the UI's old "fetch the entire history collection and filter in JS" path
 * (READINGS-TIMESERIES.md §"UI changes"). The server scopes by namespace.
 */
export async function getReadings(
  series: string,
  from: string,
  to: string
): Promise<Reading[]> {
  return request<Reading[]>('/readings', { query: { series, from, to } })
}
