import { describe, expect, it } from 'vitest'
import { rollupStatus, statusCounts } from './rollup'

describe('rollupStatus', () => {
  it('all online → online, all offline → offline', () => {
    expect(rollupStatus([{ status: 'online' }, { status: 'online' }])).toBe('online')
    expect(rollupStatus([{ status: 'offline' }, { status: 'offline' }])).toBe('offline')
  })
  it('any offline among others → degraded (DASHBOARDS.md rollup rule)', () => {
    expect(rollupStatus([{ status: 'online' }, { status: 'offline' }])).toBe('degraded')
  })
  it('empty set → unknown', () => {
    expect(rollupStatus([])).toBe('unknown')
  })
})

describe('statusCounts', () => {
  it('tallies online/offline and total', () => {
    const c = statusCounts([{ status: 'online' }, { status: 'offline' }, { status: 'unknown' }])
    expect(c).toEqual({ online: 1, offline: 1, total: 3 })
  })
})
