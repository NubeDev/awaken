import { describe, expect, it } from 'vitest'
import type { RegisterRec } from '@/api/records'
import type { HistorySample } from '../query/batch'
import { buildMeterBoard } from './meter-board'

const now = Date.now()
const iso = (hoursAgo: number) => new Date(now - hoursAgo * 3600_000).toISOString()

// Two voltage registers (one chart_group) + one no-history register.
function reg(key: string, over: Partial<RegisterRec['content']> = {}): RegisterRec {
  return {
    id: `id-${key}`,
    namespace: 'acme',
    tags: [],
    created: '',
    updated: '',
    content: {
      key: `m1--${key}`,
      name: key,
      address: 1,
      fn_code: 'read_holding',
      datatype: 'float32',
      word_count: 2,
      byte_order: 'big',
      scale: 1,
      offset: 0,
      signed: false,
      unit: 'V',
      quantity: 'voltage',
      history: true,
      chart_type: 'line',
      chart_group: 'voltage',
      precision: 1,
      meter: 'meter-1',
      ...over,
    },
  } as RegisterRec
}

const history: HistorySample[] = [
  { kind: 'history', meter: 'meter-1', register: 'voltage_l1', ts: iso(2), value: 230 },
  { kind: 'history', meter: 'meter-1', register: 'voltage_l1', ts: iso(1), value: 231 },
  { kind: 'history', meter: 'meter-1', register: 'voltage_l2', ts: iso(1), value: 254 }, // critical
]

describe('buildMeterBoard', () => {
  it('groups registers sharing a chart_group into ONE multi-series trend', () => {
    const board = buildMeterBoard(
      [reg('voltage_l1'), reg('voltage_l2')],
      history,
      'now-24h',
      undefined
    )
    expect(board.trends).toHaveLength(1)
    expect(board.trends[0].title).toBe('Voltage')
    expect(board.trends[0].series.map((s) => s.label).sort()).toEqual(['voltage_l1', 'voltage_l2'])
  })

  it('renders a no-history register as a stat tile, not a trend', () => {
    const board = buildMeterBoard(
      [reg('power_factor', { history: false, chart_type: 'stat', chart_group: '' })],
      history,
      'now-24h',
      undefined
    )
    expect(board.trends).toHaveLength(0)
    expect(board.stats).toHaveLength(1)
  })

  it('flags an alarm when the latest value crosses the ramp', () => {
    const l2 = reg('voltage_l2', {
      alarm: { thresholds: [{ value: null, severity: 'ok' }, { value: 253, severity: 'critical' }] },
    })
    const board = buildMeterBoard([l2], history, 'now-24h', undefined)
    expect(board.alarms).toHaveLength(1)
    expect(board.alarms[0].severity).toBe('critical')
    expect(board.alarms[0].value).toBe(254)
  })
})
