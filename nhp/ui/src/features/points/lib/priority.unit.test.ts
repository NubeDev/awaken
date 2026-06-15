import { describe, expect, it } from 'vitest'
import type { PriorityArray } from '@/api/types'
import { effectiveValue, winningSlotIndex } from './priority'

const pa = (
  slots: PriorityArray['slots'],
  relinquish_default: PriorityArray['relinquish_default'] = null
): PriorityArray => ({ slots, relinquish_default })

const sixteen = (overrides: Record<number, number>) =>
  Array.from({ length: 16 }, (_, i) => (i in overrides ? overrides[i] : null))

describe('winningSlotIndex', () => {
  it('picks the lowest non-null level', () => {
    // level 8 and level 13 set → level 8 (index 7) wins
    expect(winningSlotIndex(pa(sixteen({ 7: 82, 12: 70 })))).toBe(7)
  })

  it('returns -1 when all slots are null', () => {
    expect(winningSlotIndex(pa(sixteen({})))).toBe(-1)
  })
})

describe('effectiveValue', () => {
  it('returns the winning slot value', () => {
    expect(effectiveValue(pa(sixteen({ 12: 96 })))).toBe(96)
  })

  it('falls back to the relinquish default when empty', () => {
    expect(effectiveValue(pa(sixteen({}), 14))).toBe(14)
  })
})
