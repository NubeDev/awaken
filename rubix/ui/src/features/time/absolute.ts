/**
 * Helpers for the absolute (calendar) tab of the picker
 * (docs/design/time-range-and-refresh.md §2): convert a chosen `Date` to the ISO
 * token the store stores, and validate a typed relative token before it is
 * committed (so a bad token never silently yields a wrong range). Pure, shared
 * with the picker tests.
 */
import { resolveBound } from './resolve'

/** A `Date` → the RFC 3339 instant token the store carries for an absolute bound. */
export function dateToToken(d: Date): string {
  return d.toISOString()
}

/**
 * Whether a typed token (relative or absolute) is parseable. Used to gate the
 * relative-input "apply" so an unparseable token is rejected, not guessed.
 */
export function isValidToken(token: string): boolean {
  try {
    resolveBound(token, Date.now())
    return true
  } catch {
    return false
  }
}
