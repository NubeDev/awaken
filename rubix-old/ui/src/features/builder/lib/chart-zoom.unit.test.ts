import { describe, expect, it } from 'vitest'
import type { ChartRow } from './charts'
import { zoomSelection } from './use-chart-zoom'

const rows: ChartRow[] = [
  { t: '10:00', value: 1, ms: 1000 },
  { t: '10:05', value: 2, ms: 2000 },
  { t: '10:10', value: 3, ms: 3000 },
]

describe('zoomSelection', () => {
  it('orders a right-to-left drag into an ascending span', () => {
    expect(zoomSelection(rows, '10:10', '10:00')).toEqual({
      fromMs: 1000,
      toMs: 3000,
    })
  })

  it('returns a left-to-right span as-is', () => {
    expect(zoomSelection(rows, '10:00', '10:05')).toEqual({
      fromMs: 1000,
      toMs: 2000,
    })
  })

  it('ignores a click with no drag (equal bounds)', () => {
    expect(zoomSelection(rows, '10:05', '10:05')).toBeUndefined()
  })

  it('ignores a missing bound', () => {
    expect(zoomSelection(rows, '10:00', undefined)).toBeUndefined()
    expect(zoomSelection(rows, undefined, '10:05')).toBeUndefined()
  })

  it('ignores labels not present in the rows', () => {
    expect(zoomSelection(rows, '10:00', '99:99')).toBeUndefined()
  })
})
