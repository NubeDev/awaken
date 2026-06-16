// Bridge the §7 FieldConfig model into the recharts value axes (§8). A renderer
// resolves each series' effective display (unit/decimals/mappings/threshold ramp)
// via `resolveField`, then formats numbers with `formatFieldValue` — so authored
// units and precision actually paint. Falls back to a plain compact number when no
// FieldConfig is set, keeping the MVP charts unchanged.

import { resolveField, rampColor, type FieldConfig, type SeriesField } from '../field-config'
import { formatFieldValue } from '../format-field'

// Compact fallback when a column carries no FieldConfig display (matches the
// legacy axis formatter's intent — short numbers on a value axis).
const compact = new Intl.NumberFormat(undefined, { notation: 'compact', maximumFractionDigits: 3 })

/** Build a value-axis tick formatter for the series column `value` under the
 *  chart's `fieldConfig`. Numbers route through `formatFieldValue` (unit +
 *  decimals + mappings); non-numbers print as-is. */
export function fieldValueFormatter(value: string, fieldConfig: FieldConfig | undefined) {
  const display = resolveField({ value } as SeriesField, fieldConfig)
  return (raw: unknown): string => {
    if (typeof raw === 'number') {
      // No explicit display (no unit/decimals/mappings) → compact fallback.
      if (!display.unit && display.decimals == null && !display.mappings) return compact.format(raw)
      return formatFieldValue(raw, display).text
    }
    return String(raw)
  }
}

/** The resolved colour for a series, honouring a byName/byRegex override's colour
 *  but otherwise the supplied palette colour. */
export function seriesColor(
  value: string,
  fallback: string,
  fieldConfig: FieldConfig | undefined,
): string {
  const display = resolveField({ value, color: cssVarToHslArg(fallback) } as SeriesField, fieldConfig)
  return display.color ? toCss(display.color) : fallback
}

/** A per-point colour from the series' threshold ramp, or undefined when no ramp
 *  applies (the caller keeps the series colour). Used by bar/pie cells. */
export function pointRampColor(
  value: string,
  point: number,
  fieldConfig: FieldConfig | undefined,
): string | undefined {
  const display = resolveField({ value } as SeriesField, fieldConfig)
  return display.thresholds && display.thresholds.length > 0
    ? rampColor(point, display.thresholds)
    : undefined
}

// FieldConfig colours are bare hsl args ("152 76% 44%"); the palette uses
// `hsl(var(--chart-n))`. Keep them distinct: a bare-arg colour becomes
// `hsl(...)`, a CSS expression passes through.
function toCss(color: string): string {
  return color.startsWith('hsl(') || color.startsWith('var(') ? color : `hsl(${color})`
}

// The seed colour fed to resolveField as the series' own colour; the palette
// CSS expression is opaque to the resolver, so pass it through unchanged.
function cssVarToHslArg(color: string): string {
  return color
}
