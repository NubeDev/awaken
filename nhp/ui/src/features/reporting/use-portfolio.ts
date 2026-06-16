/**
 * React-Query hooks that feed the reporting + alarms surfaces. They REUSE the
 * dashboards' record + readings hooks (features/dashboards/query/batch) — the same
 * windowed, throttled `GET /readings` fan-out and the same cached `/records`
 * reads — so reporting never opens a second, divergent fetch path. This module
 * only adds the portfolio INDEX (the hierarchy walk) and a "latest value per
 * series" reduction the alarm console needs.
 */
import { useMemo } from 'react'
import { listRecords, type MeterTypeRecord } from '@/api/records'
import { useQuery } from '@tanstack/react-query'
import {
  useGateways,
  useMeters,
  useNetworks,
  useRegisters,
  useRegistersHistory,
  useSites,
  useTenants,
  type HistorySample,
} from '@/features/dashboards/query/batch'
import {
  WINDOW_TOKENS,
  type WindowToken,
} from '@/features/dashboards/query/time-window'
import { buildIndex, type PortfolioIndex } from './scope'
import type { LatestBySeries } from './alarms'

/** Meter-types, for the meter-type filter + report labels. */
function useMeterTypes() {
  return useQuery({
    queryKey: ['dash', 'meter-type'],
    queryFn: () => listRecords('meter-type') as Promise<MeterTypeRecord[]>,
    staleTime: 15_000,
  })
}

/** The whole portfolio as a resolved index, plus an aggregate loading flag. */
export function usePortfolio(): { index: PortfolioIndex; isLoading: boolean } {
  const tenants = useTenants()
  const sites = useSites()
  const gateways = useGateways()
  const networks = useNetworks()
  const meters = useMeters()
  const registers = useRegisters()
  const meterTypes = useMeterTypes()

  const index = useMemo(
    () =>
      buildIndex({
        tenants: tenants.data ?? [],
        sites: sites.data ?? [],
        gateways: gateways.data ?? [],
        networks: networks.data ?? [],
        meters: meters.data ?? [],
        registers: registers.data ?? [],
        meterTypes: meterTypes.data ?? [],
      }),
    [
      tenants.data,
      sites.data,
      gateways.data,
      networks.data,
      meters.data,
      registers.data,
      meterTypes.data,
    ]
  )

  const isLoading =
    tenants.isLoading ||
    sites.isLoading ||
    gateways.isLoading ||
    networks.isLoading ||
    meters.isLoading ||
    registers.isLoading ||
    meterTypes.isLoading

  return { index, isLoading }
}

/**
 * Windowed history for a set of registers over a relative token (6h / 24h / 7d).
 * The trailing window is anchored to the current minute and frozen until the
 * register set or token changes, so the `/readings` query keys stay stable (no
 * per-render refetch storm — batch.ts NOW_BUCKET_MS rationale). Returns the flat
 * samples plus the resolved `from`/`to` the report header prints.
 */
export function useWindowedHistory(
  registers: { id: string }[],
  token: WindowToken
): { data: HistorySample[]; isLoading: boolean; from: string; to: string } {
  const { from, to } = bucketedRange(WINDOW_TOKENS[token])
  const history = useRegistersHistory(registers, { from, to })
  return { data: history.data, isLoading: history.isLoading, from, to }
}

/**
 * A trailing `[from,to)` of `hours`, with `to` snapped DOWN to the current minute
 * so the value is stable across renders within the minute (batch.ts NOW_BUCKET_MS:
 * an unbucketed `to` re-keys every `/readings` query each render). A plain render
 * helper — not a `useMemo` — mirroring batch.ts `resolveReadingWindow`.
 */
function bucketedRange(hours: number): { from: string; to: string } {
  const anchor = Math.floor(Date.now() / 60_000) * 60_000
  return {
    from: new Date(anchor - hours * 3600_000).toISOString(),
    to: new Date(anchor).toISOString(),
  }
}

/**
 * Latest value per series for a set of registers, over a short trailing window.
 * Fans out the same throttled `/readings` reads as the dashboard and reduces each
 * series to its most-recent sample — the input the alarm evaluation needs.
 */
export function useLatestReadings(
  registers: { id: string }[],
  windowHours = 24
): { latest: LatestBySeries; isLoading: boolean } {
  const { from, to } = bucketedRange(windowHours)
  const history = useRegistersHistory(registers, { from, to })

  const latest = useMemo(() => {
    const map: LatestBySeries = new Map()
    for (const s of history.data) {
      const prev = map.get(s.series)
      if (!prev || Date.parse(s.at) > Date.parse(prev.at)) {
        map.set(s.series, { at: s.at, value: s.value })
      }
    }
    return map
  }, [history.data])

  return { latest, isLoading: history.isLoading }
}
