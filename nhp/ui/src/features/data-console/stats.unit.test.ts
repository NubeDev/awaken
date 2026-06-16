import { describe, expect, it } from 'vitest'
import type { Reading } from '@/api/readings'
import { computeStats } from './stats'

const r = (at: string, value: number): Reading => ({ series: 's', at, value })

describe('computeStats', () => {
  it('returns an empty row for no samples', () => {
    const s = computeStats([])
    expect(s.count).toBe(0)
    expect(s.min).toBeNull()
    expect(s.peakJump).toBeNull()
  })

  it('computes min/max/avg/first/last with their instants', () => {
    const s = computeStats([
      r('2026-01-01T00:00:00Z', 10),
      r('2026-01-01T01:00:00Z', 30),
      r('2026-01-01T02:00:00Z', 20),
    ])
    expect(s.count).toBe(3)
    expect(s.min).toBe(10)
    expect(s.max).toBe(30)
    expect(s.avg).toBe(20)
    expect(s.first).toBe(10)
    expect(s.last).toBe(20)
    expect(s.maxAt).toBe('2026-01-01T01:00:00Z')
    expect(s.lastAt).toBe('2026-01-01T02:00:00Z')
  })

  it('flags the largest consecutive step as the peak jump (spike)', () => {
    const s = computeStats([
      r('2026-01-01T00:00:00Z', 100),
      r('2026-01-01T01:00:00Z', 105),
      r('2026-01-01T02:00:00Z', 240), // +135 spike
      r('2026-01-01T03:00:00Z', 238),
    ])
    expect(s.peakJump).toBe(135)
    expect(s.peakJumpAt).toBe('2026-01-01T02:00:00Z')
  })

  it('sorts unordered input before reducing', () => {
    const s = computeStats([
      r('2026-01-01T02:00:00Z', 20),
      r('2026-01-01T00:00:00Z', 10),
    ])
    expect(s.first).toBe(10)
    expect(s.last).toBe(20)
  })
})
