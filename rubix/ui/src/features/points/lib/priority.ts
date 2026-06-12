import type { PriorityArray, PointValue } from '@/api/types'

/**
 * Index (0-based) of the winning slot in a BACnet priority array: the first
 * non-null level, lowest number wins. Returns -1 when every slot is null.
 */
export function winningSlotIndex(pa: PriorityArray): number {
  return pa.slots.findIndex((s) => s !== null)
}

/**
 * Effective command of a priority array: the winning slot's value, or the
 * relinquish default when all slots are null.
 */
export function effectiveValue(pa: PriorityArray): PointValue | null {
  const i = winningSlotIndex(pa)
  return i === -1 ? pa.relinquish_default : pa.slots[i]
}
