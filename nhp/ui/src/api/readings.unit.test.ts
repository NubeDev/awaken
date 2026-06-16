import { afterEach, describe, expect, it, vi } from 'vitest'
import { getReadings } from './readings'

/**
 * The readings fan-out (dashboard) fires one fetch per register; with ~100+
 * registers an unbounded burst exhausts the browser/proxy connection pool
 * (ERR_INSUFFICIENT_RESOURCES). getReadings gates every call through a semaphore
 * capped at MAX_CONCURRENT (6). These tests pin that ceiling and prove a slot is
 * always released — even when the underlying request rejects.
 */
const MAX = 6

afterEach(() => vi.restoreAllMocks())

/** A fetch stub that records peak concurrency and resolves after a tick. */
function trackingFetch() {
  let inFlight = 0
  let peak = 0
  const fn = vi.fn(async () => {
    inFlight += 1
    peak = Math.max(peak, inFlight)
    await new Promise((r) => setTimeout(r, 5))
    inFlight -= 1
    return new Response('[]', {
      status: 200,
      headers: { 'content-type': 'application/json' },
    })
  })
  return { fn, peak: () => peak }
}

describe('getReadings concurrency gate', () => {
  it('never runs more than MAX_CONCURRENT requests at once', async () => {
    const { fn, peak } = trackingFetch()
    vi.stubGlobal('fetch', fn)

    // Fan out far more than the cap, like the dashboard does.
    const calls = Array.from({ length: 40 }, (_, i) =>
      getReadings(`series-${i}`, '2026-01-01T00:00:00Z', '2026-01-08T00:00:00Z')
    )
    await Promise.all(calls)

    expect(fn).toHaveBeenCalledTimes(40)
    expect(peak()).toBeLessThanOrEqual(MAX)
  })

  it('releases a slot when a request rejects, so the queue still drains', async () => {
    // First call rejects, the rest resolve; all must settle (no deadlock).
    let n = 0
    const fn = vi.fn(async () => {
      n += 1
      if (n === 1) throw new TypeError('Failed to fetch')
      return new Response('[]', {
        status: 200,
        headers: { 'content-type': 'application/json' },
      })
    })
    vi.stubGlobal('fetch', fn)

    const results = await Promise.allSettled(
      Array.from({ length: 20 }, (_, i) =>
        getReadings(`s-${i}`, '2026-01-01T00:00:00Z', '2026-01-08T00:00:00Z')
      )
    )

    // Exactly one rejected; the rest resolved — proving the failed call freed its slot.
    expect(results.filter((r) => r.status === 'rejected')).toHaveLength(1)
    expect(results.filter((r) => r.status === 'fulfilled')).toHaveLength(19)
  })
})
