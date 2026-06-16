/**
 * The ONE place a dashboard fetches its data (DASHBOARDS-SCOPE §7/§8: "one place
 * fetches; a widget is a pure function of {widget,data}"). React Query hooks that
 * read every record kind the auto-builder walks, plus the history series.
 *
 * ── POC simplification (documented, per WS-07 fallback clause) ───────────────
 * The spec's "one batched query per board via POST /query/batch" is NOT available
 * on the frozen rubix tree: only `POST /query` exists (a single read-only SQL
 * statement gated on the `external-query` capability — rubix http/query/run.rs),
 * there is NO `/query/batch` route despite OVERVIEW claiming one, and `/query`
 * needs a grant the default operator credential may lack. Rather than couple the
 * dashboards to a route that isn't there, the POC reads the seeded data over the
 * plain `/records` API (api/records.ts, already wired + authorised for the
 * operator) and does windowing/grouping/aggregation CLIENT-SIDE. This is the
 * WS-07 "fall back to /records + a simple history query + a refetch timer" path.
 * The pure renderer + auto-build layers are unchanged if a real batch lands later
 * — only this file's fetch swaps.
 *
 * Tags drive everything: records carry NHP's standard tags in `content.tags`
 * (WS-03: rubix has no HTTP tag-attach route, so tags live in content, NOT graph
 * edges — meaning the server-side `?tag=` filter does NOT see them). We therefore
 * fetch each kind whole and filter by `content.tags` in the builders.
 */
import { useQueries, useQuery } from '@tanstack/react-query'
import {
  listRecords,
  type GatewayRecord,
  type MeterRecord,
  type NetworkRecord,
  type RegisterRec,
  type SiteRecord,
} from '@/api/records'
import { getReadings, type Reading } from '@/api/readings'

/**
 * One readings-plane sample (READINGS-TIMESERIES.md). Lean: just the series it
 * belongs to, its MEASUREMENT instant `at` (RFC3339, not write time), and the
 * numeric value. The display metadata that used to be duplicated on every row
 * (`quantity`/`unit`/`meter`/`register`) now lives ONCE on the series/register and
 * is reached via `series` (= the register RECORD ID). Same shape as the API
 * `Reading`; re-exported here because the dashboard builders import the type.
 */
export type HistorySample = Reading

/** A `kind:"tenant"` record. */
export interface Tenant {
  kind: 'tenant'
  key: string
  name: string
  namespace?: string
  tags?: string[]
}

const STALE = 15_000

export function useTenants() {
  return useQuery({ queryKey: ['dash', 'tenant'], queryFn: () => listRecords<Tenant>('tenant'), staleTime: STALE })
}
export function useSites() {
  return useQuery({ queryKey: ['dash', 'site'], queryFn: () => listRecords('site') as Promise<SiteRecord[]>, staleTime: STALE })
}
export function useGateways() {
  return useQuery({ queryKey: ['dash', 'gateway'], queryFn: () => listRecords('gateway') as Promise<GatewayRecord[]>, staleTime: STALE })
}
export function useNetworks() {
  return useQuery({ queryKey: ['dash', 'network'], queryFn: () => listRecords('network') as Promise<NetworkRecord[]>, staleTime: STALE })
}
export function useMeters() {
  return useQuery({ queryKey: ['dash', 'meter'], queryFn: () => listRecords('meter') as Promise<MeterRecord[]>, staleTime: STALE })
}
export function useRegisters() {
  return useQuery({ queryKey: ['dash', 'register'], queryFn: () => listRecords('register') as Promise<RegisterRec[]>, staleTime: STALE })
}

/** Default trailing window for a historian read: 7 days. */
const DEFAULT_WINDOW_MS = 7 * 24 * 3600_000

/**
 * Bucket size for the default trailing-window end (1 minute). The default `to` is
 * "now", but a raw `Date.now()` changes every millisecond — and `to` is part of
 * the readings query key. An unbucketed value re-keys EVERY register's query on
 * EVERY render, so the cache never hits and the dashboard re-fans ~100 `/readings`
 * fetches in a render storm (observed: 2000+ requests in seconds, exhausting the
 * connection pool). Snapping "now" down to a 1-minute boundary keeps the key
 * stable across renders within the minute, so React Query serves cached data and
 * only refetches when the bucket actually advances. A bucket ≤ the refresh
 * interval costs no data freshness.
 */
const NOW_BUCKET_MS = 60_000

/** Resolve an optional `{from,to}` to an RFC3339 pair, defaulting to the trailing
 *  `DEFAULT_WINDOW_MS` ending at the current minute (READINGS-TIMESERIES.md
 *  §"UI changes"). The default end is bucketed to keep the query key STABLE across
 *  renders — see NOW_BUCKET_MS. */
function resolveReadingWindow(opts?: { from?: string; to?: string }): { from: string; to: string } {
  const nowBucketed = Math.floor(Date.now() / NOW_BUCKET_MS) * NOW_BUCKET_MS
  const to = opts?.to ?? new Date(nowBucketed).toISOString()
  const from = opts?.from ?? new Date(nowBucketed - DEFAULT_WINDOW_MS).toISOString()
  return { from, to }
}

/**
 * Windowed, series-scoped historian read for ONE series (READINGS-TIMESERIES.md
 * §"UI changes": replaces the whole-collection `useAllHistory`). Calls
 * `GET /readings?series&from&to` over a trailing window (default: last 7 days).
 * Disabled and empty when `series` is undefined. Returns `{ data, isLoading }`.
 */
export function useSeriesHistory(
  series: string | undefined,
  opts?: { from?: string; to?: string }
): { data: HistorySample[]; isLoading: boolean } {
  const { from, to } = resolveReadingWindow(opts)
  const q = useQuery({
    queryKey: ['dash', 'readings', series, from, to],
    staleTime: STALE,
    enabled: series !== undefined,
    queryFn: () => getReadings(series as string, from, to),
  })
  return { data: q.data ?? [], isLoading: series !== undefined && q.isLoading }
}

/**
 * Windowed historian read for MANY series at once: fans out one
 * `GET /readings?series&from&to` per register id (React Query `useQueries`) over a
 * trailing window (default: last 7 days) and returns a flat `HistorySample[]` —
 * each sample carries its own `series`, so the board builders join by
 * `sample.series === register.id`. This fan-out is the windowed replacement for the
 * whole-collection read READINGS-TIMESERIES.md calls out as the thing that falls
 * over at volume: each series is bounded by the window, nothing pulls the entire
 * `reading` table. `isLoading` is aggregate (true while ANY series is loading).
 */
export function useRegistersHistory(
  registers: { id: string }[],
  opts?: { from?: string; to?: string }
): { data: HistorySample[]; isLoading: boolean } {
  const { from, to } = resolveReadingWindow(opts)
  const results = useQueries({
    queries: registers.map((r) => ({
      queryKey: ['dash', 'readings', r.id, from, to],
      staleTime: STALE,
      queryFn: () => getReadings(r.id, from, to),
    })),
  })
  const data = results.flatMap((res) => res.data ?? [])
  const isLoading = results.some((res) => res.isLoading)
  return { data, isLoading }
}
