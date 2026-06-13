import { describe, expect, it } from 'vitest'
import {
  resolveBound,
  resolveRange,
  snapToTick,
  TimeRangeError,
} from './resolve'
import { isRefreshSecs } from './presets'
import { readTimeParams, writeTimeParams } from './url-state'

// A fixed frozen instant: 2026-06-13T14:37:42.500Z.
const NOW = Date.parse('2026-06-13T14:37:42.500Z')

describe('resolveBound', () => {
  it('resolves bare `now` to the frozen instant', () => {
    expect(resolveBound('now', NOW)).toBe(NOW)
  })

  it('resolves negative relative offsets per unit', () => {
    expect(resolveBound('now-5m', NOW)).toBe(NOW - 5 * 60_000)
    expect(resolveBound('now-6h', NOW)).toBe(NOW - 6 * 3_600_000)
    expect(resolveBound('now-7d', NOW)).toBe(NOW - 7 * 86_400_000)
  })

  it('resolves positive offsets', () => {
    expect(resolveBound('now+1h', NOW)).toBe(NOW + 3_600_000)
  })

  it('floors `now/d` to UTC midnight', () => {
    const floored = resolveBound('now/d', NOW)
    expect(new Date(floored).toISOString()).toBe('2026-06-13T00:00:00.000Z')
  })

  it('floors `now/h` to the hour', () => {
    const floored = resolveBound('now/h', NOW)
    expect(new Date(floored).toISOString()).toBe('2026-06-13T14:00:00.000Z')
  })

  it('applies offset then floor for `now-1d/d` (yesterday start)', () => {
    const floored = resolveBound('now-1d/d', NOW)
    expect(new Date(floored).toISOString()).toBe('2026-06-12T00:00:00.000Z')
  })

  it('parses an absolute RFC 3339 instant', () => {
    expect(resolveBound('2026-01-02T03:04:05.000Z', NOW)).toBe(
      Date.parse('2026-01-02T03:04:05.000Z')
    )
  })

  it('throws on a malformed token rather than guessing', () => {
    expect(() => resolveBound('yesterday', NOW)).toThrow(TimeRangeError)
    expect(() => resolveBound('now-5x', NOW)).toThrow(TimeRangeError)
    expect(() => resolveBound('now/y', NOW)).toThrow(TimeRangeError)
  })
})

describe('resolveRange', () => {
  it('resolves both bounds against the one frozen now', () => {
    const r = resolveRange('now-6h', 'now', NOW)
    expect(r.toMs).toBe(NOW)
    expect(r.fromMs).toBe(NOW - 6 * 3_600_000)
  })
})

describe('snapToTick', () => {
  it('floors an instant to the tick boundary so the cache key is stable', () => {
    const tick = 30_000
    const a = snapToTick(NOW, tick)
    const b = snapToTick(NOW + 1234, tick)
    expect(a).toBe(b) // same 30s bucket → same key
    expect(a % tick).toBe(0)
  })

  it('is identity for a non-positive tick', () => {
    expect(snapToTick(NOW, 0)).toBe(NOW)
  })
})

describe('isRefreshSecs', () => {
  it('accepts presets and rejects others', () => {
    expect(isRefreshSecs(30)).toBe(true)
    expect(isRefreshSecs(0)).toBe(true)
    expect(isRefreshSecs(7)).toBe(false)
  })
})

describe('time url-state', () => {
  it('falls back to defaults when params absent', () => {
    const s = readTimeParams(new URLSearchParams(''))
    expect(s).toEqual({ from: 'now-6h', to: 'now', refresh: 5 })
  })

  it('round-trips an explicit non-default selection', () => {
    const written = writeTimeParams(new URLSearchParams(''), {
      from: 'now-24h',
      to: 'now',
      refresh: 30,
    })
    expect(written.get('from')).toBe('now-24h')
    expect(written.get('to')).toBe('now')
    expect(written.get('refresh')).toBe('30')
    expect(readTimeParams(written)).toEqual({
      from: 'now-24h',
      to: 'now',
      refresh: 30,
    })
  })

  it('omits params equal to the defaults to keep links clean', () => {
    const written = writeTimeParams(new URLSearchParams(''), {
      from: 'now-6h',
      to: 'now',
      refresh: 5,
    })
    expect(written.toString()).toBe('')
  })

  it('preserves unrelated params (e.g. var-*)', () => {
    const base = new URLSearchParams('var-site=A&keep=1')
    const written = writeTimeParams(base, {
      from: 'now-1h',
      to: 'now',
      refresh: 0,
    })
    expect(written.get('var-site')).toBe('A')
    expect(written.get('keep')).toBe('1')
    expect(written.get('refresh')).toBe('0')
  })

  it('ignores a lone bound (half-written link)', () => {
    const s = readTimeParams(new URLSearchParams('from=now-1h'))
    expect(s.from).toBe('now-6h') // fell back to default range
  })

  it('ignores a non-preset refresh value', () => {
    const s = readTimeParams(new URLSearchParams('refresh=7'))
    expect(s.refresh).toBe(5)
  })
})
