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
import { useQuery } from '@tanstack/react-query'
import {
  listRecords,
  type GatewayRecord,
  type MeterRecord,
  type NetworkRecord,
  type RegisterRec,
  type SiteRecord,
} from '@/api/records'

/** A `kind:"history"` sample (seed/history.mjs shape). No `content.tags`; it is
 *  tied to its series by `meter` + `register`. */
export interface HistorySample {
  kind: 'history'
  meter: string
  register: string
  quantity?: string
  unit?: string
  ts: string
  value: number
}

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

/**
 * Every `kind:"history"` sample. The seed is small (≈4k rows) so one whole-kind
 * read serves the alarm rollup on the tenant/site cards (alarm-eval.ts evaluates
 * the latest value per register) AND the meter trend charts — fetched ONCE and
 * shared (the §7 "one place fetches" discipline). A real backend would push the
 * `meter`/`ts` filter into the query (the §3 batch path we fall back from).
 */
export function useAllHistory() {
  return useQuery({
    queryKey: ['dash', 'history'],
    staleTime: STALE,
    queryFn: async () => {
      const all = await listRecords<HistorySample>('history')
      return all.map((r) => r.content)
    },
  })
}

/** This meter's samples, sliced from the shared whole-history read. */
export function useMeterHistory(meterId: string | undefined) {
  const all = useAllHistory()
  const samples = (all.data ?? []).filter((h) => h.meter === meterId)
  return { data: meterId ? samples : [], isLoading: all.isLoading }
}
