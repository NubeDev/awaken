import { create } from 'zustand'
import {
  DEFAULT_RANGE,
  DEFAULT_REFRESH,
  type RefreshSecs,
} from '@/features/time/presets'
/**
 * Dashboard time-range + refresh store (docs/design/time-range-and-refresh.md
 * §§1-3). Holds the (possibly relative) `{from, to}` selection, the auto-refresh
 * interval, and a monotonically-increasing `tick` that bumps on every auto- or
 * manual refresh. Resolving relative tokens to instants lives in
 * `features/time/resolve.ts`; the server resolves `now` authoritatively per query
 * so this store carries the tokens, not the resolved bounds — and freezes one
 * `nowMs` per tick so every widget in a refresh shares an instant.
 *
 * Initial state comes from the URL (`?from/to/refresh`) so a shared link restores
 * before React mounts; the URL-binding hook keeps the two in sync thereafter.
 */
import { readTimeParams } from '@/features/time/url-state'

interface TimeState {
  /** Range `from` bound: relative token (`now-6h`) or absolute instant. */
  from: string
  /** Range `to` bound. */
  to: string
  /** Auto-refresh interval in seconds; `0` = off. */
  refresh: RefreshSecs
  /** Refresh counter — bumps on auto-tick and manual refresh (cache discriminator). */
  tick: number
  /** Frozen `now` for the current tick, epoch ms — one instant per refresh. */
  nowMs: number
  setRange: (from: string, to: string) => void
  setRefresh: (refresh: RefreshSecs) => void
  /** Bump the tick and re-freeze `now` — drives a coordinated refetch. */
  bumpTick: () => void
}

function initial(): { from: string; to: string; refresh: RefreshSecs } {
  if (typeof window === 'undefined') {
    return {
      from: DEFAULT_RANGE.from,
      to: DEFAULT_RANGE.to,
      refresh: DEFAULT_REFRESH,
    }
  }
  return readTimeParams(new URLSearchParams(window.location.search))
}

export const useTimeStore = create<TimeState>()((set) => {
  const init = initial()
  return {
    from: init.from,
    to: init.to,
    refresh: init.refresh,
    tick: 0,
    nowMs: Date.now(),
    setRange: (from, to) =>
      // A range change is itself a refresh: re-freeze `now` and bump the tick so
      // every widget re-resolves against the new bounds at one instant.
      set((s) => ({ from, to, tick: s.tick + 1, nowMs: Date.now() })),
    setRefresh: (refresh) => set({ refresh }),
    bumpTick: () => set((s) => ({ tick: s.tick + 1, nowMs: Date.now() })),
  }
})
