import { describe, expect, it } from 'vitest'

import { ChartType } from './types'
import {
  WIDGETS,
  WIDGET_CATALOG,
  allowsBreakdown,
  descriptor,
  missingFields,
  needsX,
  needsY,
} from './catalog'

describe('widget catalog', () => {
  it('has a descriptor for every ChartType (no widget can be unlisted)', () => {
    for (const type of Object.values(ChartType)) {
      expect(WIDGET_CATALOG[type]).toBeDefined()
      expect(WIDGET_CATALOG[type].type).toBe(type)
    }
  })

  it('lists every catalog entry in the picker order', () => {
    expect(WIDGETS).toHaveLength(Object.keys(WIDGET_CATALOG).length)
  })

  it('defaults a stale/undefined type to a valid descriptor', () => {
    expect(descriptor(undefined).type).toBe(ChartType.LineChart)
    expect(descriptor('bogus' as ChartType).type).toBe(ChartType.LineChart)
  })
})

describe('role predicates', () => {
  it('cartesian widgets need X and Y and allow a breakdown', () => {
    expect(needsX(ChartType.LineChart)).toBe(true)
    expect(needsY(ChartType.LineChart)).toBe(true)
    expect(allowsBreakdown(ChartType.LineChart)).toBe(true)
  })

  it('pie and horizontal bar are single-series (no breakdown)', () => {
    expect(allowsBreakdown(ChartType.PieChart)).toBe(false)
    expect(allowsBreakdown(ChartType.HorizontalBarChart)).toBe(false)
    // ...but they still need X (label) and Y (value).
    expect(needsX(ChartType.PieChart)).toBe(true)
    expect(needsY(ChartType.PieChart)).toBe(true)
  })

  it('table needs no axis columns', () => {
    expect(needsX(ChartType.Table)).toBe(false)
    expect(needsY(ChartType.Table)).toBe(false)
  })
})

describe('missingFields', () => {
  it('reports the X/Y a cartesian widget is missing', () => {
    expect(missingFields(ChartType.LineChart, {})).toEqual(['X-axis column', 'Y-axis column'])
    expect(missingFields(ChartType.LineChart, { x: 'day', y: 'n' })).toEqual([])
  })

  it('reports nothing for a table (no axes required)', () => {
    expect(missingFields(ChartType.Table, {})).toEqual([])
  })

  it('flags a missing type', () => {
    expect(missingFields(undefined, { x: 'a', y: 'b' })).toContain('chart type')
  })
})
