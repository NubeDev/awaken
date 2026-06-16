/**
 * Status rollup + counts up the hierarchy (DASHBOARDS.md §"Online / offline &
 * stats": "a site is 'degraded' if any gateway is offline"). Pure functions over
 * poller-written `status` (READ-ONLY in NHP — DOMAIN-MODEL). Shared by the tenant
 * and site card builders so the rollup rule lives in ONE place.
 */
import type { RollupStatus } from '../widgets/status-tile'

interface HasStatus {
  status?: string
}

/**
 * Roll a set of child statuses into one: offline if ALL offline, online if ALL
 * online, degraded if mixed (any offline among others), else unknown. An empty
 * set is `unknown` (nothing reporting).
 */
export function rollupStatus(children: HasStatus[]): RollupStatus {
  if (children.length === 0) return 'unknown'
  const states = children.map((c) => c.status ?? 'unknown')
  const offline = states.filter((s) => s === 'offline').length
  const online = states.filter((s) => s === 'online').length
  if (offline === states.length) return 'offline'
  if (online === states.length) return 'online'
  if (offline > 0) return 'degraded'
  return online > 0 ? 'degraded' : 'unknown'
}

/** Online/offline tallies for a status tile row. */
export function statusCounts(children: HasStatus[]): {
  online: number
  offline: number
  total: number
} {
  let online = 0
  let offline = 0
  for (const c of children) {
    if (c.status === 'online') online += 1
    else if (c.status === 'offline') offline += 1
  }
  return { online, offline, total: children.length }
}
