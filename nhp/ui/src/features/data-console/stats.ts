/**
 * Pure summary statistics for one readings series — the numbers an external
 * engineer scans when debugging power spikes / energy use. No fetching, no React:
 * a list of `{ at, value }` samples in, a `SeriesStats` out, so it is trivially
 * testable (stats.unit.test.ts) and reusable by both the on-screen panel and the
 * printable report.
 *
 * `peakJump` is the headline debug signal: the largest absolute change between two
 * CONSECUTIVE samples and the instant it landed on — a sudden current/power step
 * (a "spike") shows up here even when min/max look unremarkable over the window.
 */
import type { Reading } from '@/api/readings'

export interface SeriesStats {
  count: number
  /** Null when the series has no samples in the window. */
  min: number | null
  max: number | null
  avg: number | null
  first: number | null
  last: number | null
  /** Measurement instant (RFC3339) of the min / max / last sample. */
  minAt: string | null
  maxAt: string | null
  lastAt: string | null
  /** Largest absolute step between consecutive samples (spike magnitude). */
  peakJump: number | null
  /** Instant the largest step landed on (the second of the two samples). */
  peakJumpAt: string | null
}

const EMPTY: SeriesStats = {
  count: 0,
  min: null,
  max: null,
  avg: null,
  first: null,
  last: null,
  minAt: null,
  maxAt: null,
  lastAt: null,
  peakJump: null,
  peakJumpAt: null,
}

/** Reduce ascending-by-`at` samples to a single stat row. Input need not be
 *  sorted; callers usually pass the windowed/sorted series. */
export function computeStats(samples: Reading[]): SeriesStats {
  if (samples.length === 0) return { ...EMPTY }

  const rows = [...samples].sort((a, b) => Date.parse(a.at) - Date.parse(b.at))

  let min = rows[0].value
  let max = rows[0].value
  let minAt = rows[0].at
  let maxAt = rows[0].at
  let sum = 0
  let peakJump: number | null = null
  let peakJumpAt: string | null = null

  for (let i = 0; i < rows.length; i++) {
    const v = rows[i].value
    sum += v
    if (v < min) {
      min = v
      minAt = rows[i].at
    }
    if (v > max) {
      max = v
      maxAt = rows[i].at
    }
    if (i > 0) {
      const jump = Math.abs(v - rows[i - 1].value)
      if (peakJump === null || jump > peakJump) {
        peakJump = jump
        peakJumpAt = rows[i].at
      }
    }
  }

  const lastRow = rows[rows.length - 1]
  return {
    count: rows.length,
    min,
    max,
    avg: sum / rows.length,
    first: rows[0].value,
    last: lastRow.value,
    minAt,
    maxAt,
    lastAt: lastRow.at,
    peakJump,
    peakJumpAt,
  }
}
