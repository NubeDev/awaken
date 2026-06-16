// Pure value formatter over a resolved FieldDisplay (DASHBOARDS-SCOPE.md §7).
// Adopted from nexus's `_shared/formatValue.ts`: value mappings first (a matched
// text mapping wins outright), then null/non-finite → noValue, then decimals +
// unit symbol. Pure — same inputs, same string — so the §8 renderers and the
// table/stat cells share one formatter and stay testable. The number reaching
// here is already the caller's unit system (the backend converted it, §2); the
// unit symbol is presentation only.

import type { FieldDisplay, ValueMapping } from './field-config'
import { unitDef } from './units'

/** The formatted display text plus an optional colour a matched value-mapping
 *  requested (an hsl string), which the caller may paint the value with. */
export interface FormattedValue {
  text: string
  color?: string
}

/** Format `value` for display under `display` (mappings/decimals/unit). A null,
 *  undefined, or non-finite value renders `display.noValue` (default em dash) so a
 *  panel never shows "NaN". */
export function formatFieldValue(
  value: number | string | null | undefined,
  display: FieldDisplay = {},
): FormattedValue {
  const mapped = matchMapping(value, display.mappings)
  if (mapped && mapped.text != null) {
    // A mapping that supplies text wins outright; a colour-only mapping falls
    // through to numeric formatting but still tints the result.
    return { text: mapped.text, color: mapped.color }
  }

  if (value == null || (typeof value === 'number' && !Number.isFinite(value))) {
    return { text: display.noValue ?? '—', color: mapped?.color }
  }

  const num = typeof value === 'number' ? value : Number(value)
  if (!Number.isFinite(num)) {
    return { text: String(value), color: mapped?.color }
  }

  return { text: formatNumber(num, display), color: mapped?.color }
}

// Apply decimals + the unit symbol to a finite number.
function formatNumber(num: number, display: FieldDisplay): string {
  const u = unitDef(display.unit)
  // `percentunit` stores 0–1 but shows 0–100.
  const scaled = display.unit === 'percentunit' ? num * 100 : num
  const fixed = display.decimals == null ? trimAuto(scaled) : scaled.toFixed(display.decimals)
  if (!u || !u.symbol) return fixed
  if (u.prefix) return `${u.symbol}${fixed}`
  return u.space ? `${fixed} ${u.symbol}` : `${fixed}${u.symbol}`
}

// Auto precision: integers print whole; fractions keep up to 2 places with no
// trailing zeros — a sane default when no `decimals` is set.
function trimAuto(n: number): string {
  if (Number.isInteger(n)) return String(n)
  return String(Math.round(n * 100) / 100)
}

// First matching value mapping (by declaration order), or undefined.
function matchMapping(
  value: number | string | null | undefined,
  mappings: ReadonlyArray<ValueMapping> | undefined,
): ValueMapping | undefined {
  if (!mappings || mappings.length === 0) return undefined
  const str = value == null ? '' : String(value)
  const num = typeof value === 'number' ? value : Number(value)
  return mappings.find((m) => {
    if (m.type === 'value') return m.match != null && m.match === str
    if (m.type === 'regex') {
      if (m.match == null) return false
      try {
        return new RegExp(m.match).test(str)
      } catch {
        return false
      }
    }
    // range
    if (!Number.isFinite(num)) return false
    if (m.from != null && num < m.from) return false
    if (m.to != null && num > m.to) return false
    return m.from != null || m.to != null
  })
}
