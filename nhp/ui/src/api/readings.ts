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
 * Cap on concurrent `/readings` requests in flight.
 *
 * The dashboard fans one historian read out PER register (batch.ts
 * `useRegistersHistory` over `useQueries`). A full portfolio has ~100+ registers,
 * so an unbounded fan-out fires 100+ fetches at once — past the browser's
 * per-host connection ceiling AND the vite dev proxy's, which fails the overflow
 * with `net::ERR_INSUFFICIENT_RESOURCES` / `Failed to fetch`. We gate every read
 * through a small semaphore so at most `MAX_CONCURRENT` are open at a time; the
 * rest queue and drain as slots free. 6 mirrors the classic per-host HTTP/1.1
 * connection limit — enough to stay busy, low enough not to swamp the proxy.
 */
const MAX_CONCURRENT = 6
let active = 0
const waiters: (() => void)[] = []

async function acquire(): Promise<void> {
  if (active < MAX_CONCURRENT) {
    active += 1
    return
  }
  await new Promise<void>((resolve) => waiters.push(resolve))
  active += 1
}

function release(): void {
  active -= 1
  waiters.shift()?.()
}

/**
 * Windowed, series-scoped historian read: the readings for ONE series between
 * `from` and `to` (both RFC3339), ordered by `at`. This is the windowed query that
 * replaces the UI's old "fetch the entire history collection and filter in JS" path
 * (READINGS-TIMESERIES.md §"UI changes"). The server scopes by namespace.
 *
 * Throttled through a shared semaphore (see `MAX_CONCURRENT`) so a per-register
 * fan-out can't exhaust the browser/proxy connection pool.
 */
export async function getReadings(
  series: string,
  from: string,
  to: string
): Promise<Reading[]> {
  await acquire()
  try {
    return await request<Reading[]>('/readings', { query: { series, from, to } })
  } finally {
    release()
  }
}
