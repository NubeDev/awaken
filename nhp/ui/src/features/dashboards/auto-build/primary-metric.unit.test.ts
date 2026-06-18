import { describe, expect, it } from 'vitest'
import type { RegisterRec } from '@/api/records'
import { meterTag, quantityTag } from '@/enums/tags'
import type { HistorySample } from '../query/batch'
import { resolveWindow } from '../query/time-window'
import { primaryMetric } from './primary-metric'

const now = Date.now()
const iso = (hoursAgo: number) => new Date(now - hoursAgo * 3600_000).toISOString()
const W = resolveWindow('now-24h', now)
const METER = 'm1'

// A register tagged onto meter `m1`, plus its quantity tag (primaryMetric filters
// by the meter tag exactly as the energy rollup does).
function reg(key: string, over: Partial<RegisterRec['content']> = {}): RegisterRec {
  const quantity = (over.quantity as string) ?? 'temperature'
  return {
    id: `id-${key}`,
    namespace: 'acme',
    tags: [],
    created: '',
    updated: '',
    content: {
      key: `${METER}--${key}`,
      name: key,
      address: 1,
      fn_code: 'lora_uplink',
      datatype: 'float32',
      word_count: 2,
      byte_order: 'big',
      scale: 1,
      offset: 0,
      signed: false,
      unit: '°C',
      quantity,
      history: true,
      chart_type: 'line',
      chart_group: quantity,
      precision: 1,
      meter: 'meter-1',
      tags: [meterTag(METER), quantityTag(quantity)],
      ...over,
    },
  } as RegisterRec
}

describe('primaryMetric', () => {
  it('reads a non-energy device by its own unit + latest value', () => {
    const registers = [reg('temperature', { unit: '°C', precision: 1 })]
    const history: HistorySample[] = [
      { series: 'id-temperature', at: iso(2), value: 21 },
      { series: 'id-temperature', at: iso(1), value: 22.5 },
    ]
    const m = primaryMetric(METER, registers, history, W)
    expect(m.latest).toBe(22.5)
    expect(m.unit).toBe('°C')
    expect(m.severity).toBe('ok')
    expect(m.points).toHaveLength(2)
  })

  it('prefers a register in alarm so the row surfaces the problem', () => {
    const registers = [
      reg('energy', { unit: 'kWh', quantity: 'energy', precision: 0 }),
      reg('temperature', {
        unit: '°C',
        quantity: 'temperature',
        alarm: {
          thresholds: [
            { value: null, severity: 'ok' },
            { value: 35, severity: 'warning' },
            { value: 40, severity: 'critical' },
          ],
        },
      }),
    ]
    const history: HistorySample[] = [
      { series: 'id-energy', at: iso(1), value: 1000 },
      { series: 'id-temperature', at: iso(1), value: 42 }, // critical
    ]
    const m = primaryMetric(METER, registers, history, W)
    expect(m.unit).toBe('°C')
    expect(m.latest).toBe(42)
    expect(m.severity).toBe('critical')
  })

  it('honours a below-direction ramp (low battery)', () => {
    const registers = [
      reg('battery', {
        unit: '%',
        quantity: 'battery',
        history: false,
        alarm: {
          direction: 'below',
          thresholds: [
            { value: null, severity: 'ok' },
            { value: 30, severity: 'warning' },
            { value: 15, severity: 'critical' },
          ],
        },
      }),
    ]
    const history: HistorySample[] = [{ series: 'id-battery', at: iso(0.1), value: 12 }]
    const m = primaryMetric(METER, registers, history, W)
    expect(m.latest).toBe(12)
    expect(m.severity).toBe('critical')
    expect(m.points).toHaveLength(1) // a no-trend register still reports its latest
  })

  it('falls back to energy for a classic power meter', () => {
    const registers = [
      reg('voltage_l1', { unit: 'V', quantity: 'voltage' }),
      reg('energy', { unit: 'kWh', quantity: 'energy', precision: 0 }),
    ]
    const history: HistorySample[] = [
      { series: 'id-voltage_l1', at: iso(1), value: 230 },
      { series: 'id-energy', at: iso(1), value: 1500 },
    ]
    const m = primaryMetric(METER, registers, history, W)
    expect(m.unit).toBe('kWh')
    expect(m.latest).toBe(1500)
  })

  it('returns an empty metric when the meter has no registers', () => {
    const m = primaryMetric('nobody', [reg('temperature')], [], W)
    expect(m.latest).toBeNull()
    expect(m.points).toEqual([])
  })
})
