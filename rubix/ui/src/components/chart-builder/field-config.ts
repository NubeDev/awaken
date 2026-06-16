// The FieldConfig data model — the Grafana-shaped, library-agnostic chart
// contract adopted from nexus (DASHBOARDS-SCOPE.md §7). Pure data + pure
// resolution: defaults plus per-series byName/byRegex overrides (unit, decimals,
// min/max, multi-step threshold ramps, value mappings). No React, no fetch — so
// the renderers (§8) and the value formatter consume one resolved shape and stay
// testable, and a future client reads the same portable spec.
//
// This is additive to the MVP ChartConfig: a chart with no `fieldConfig`
// serialises to nothing and renders exactly as before. The types mirror nexus's
// `data/types.ts` so the §8 widget port can reuse its `_shared` logic unchanged.

/** One step in a multi-step threshold ramp: at or above `value` (ascending) the
 *  reading takes `color`. The base step is `value: null` (serialises across JSON,
 *  unlike `-Infinity`). Consumers sort defensively rather than assume order. */
export interface ThresholdStep {
  /** Lower bound; `null` is the base step (no lower bound). */
  value: number | null
  /** An hsl string, e.g. "152 76% 44%". */
  color: string
}

/** Maps a raw value (exact / numeric range / regex over text) onto display text
 *  and/or colour — Grafana's "value mappings". First match wins. */
export interface ValueMapping {
  type: 'value' | 'range' | 'regex'
  /** For `value`: the literal; for `regex`: the pattern. */
  match?: string
  /** For `range`: inclusive bounds (either may be omitted for open-ended). */
  from?: number
  to?: number
  /** Replacement display text. */
  text?: string
  /** An hsl string applied when this mapping matches. */
  color?: string
}

/** Per-field display config — unit, precision, bounds, threshold ramp, value
 *  mappings. The `defaults` half of {@link FieldConfig}; an override carries the
 *  same shape to selectively replace it per series. All optional, so an untouched
 *  panel serialises to `{}`. */
export interface FieldDisplay {
  /** Unit id from the unit registry (`./units`), e.g. `"celsius"`, `"percent"`.
   *  Undefined → unitless. */
  unit?: string
  /** Fixed decimal places; undefined → auto. */
  decimals?: number
  min?: number
  max?: number
  /** What to show when there is no value (defaults to an em dash). */
  noValue?: string
  /** Multi-step colour ramp; empty/undefined → no threshold colouring. */
  thresholds?: ReadonlyArray<ThresholdStep>
  mappings?: ReadonlyArray<ValueMapping>
}

/** How an override selects the series it applies to. `byName` matches a series'
 *  value column (or label) exactly; `byRegex` tests the same against a pattern. */
export interface FieldMatcher {
  type: 'byName' | 'byRegex'
  /** The column name/label for `byName`, the pattern for `byRegex`. */
  value: string
}

/** A per-series override: when a series matches `matcher`, its display config is
 *  the defaults with these laid on top. Also allows renaming/hiding/recolouring. */
export interface FieldOverride {
  matcher: FieldMatcher
  display: FieldDisplay & { displayName?: string; hidden?: boolean; color?: string }
}

/** Grafana-style field config: a default display for every series plus targeted
 *  overrides. Additive on the chart record; absence means "render as before". */
export interface FieldConfig {
  defaults?: FieldDisplay
  overrides?: ReadonlyArray<FieldOverride>
}

/** One drawn series, mapped from a result column. */
export interface SeriesField {
  /** Result column holding the value. */
  value: string
  label?: string
  unit?: string
  /** An hsl string; defaults to the chart palette. */
  color?: string
}

/** The effective display for a series after merging defaults + the first matching
 *  override, plus the override-only display extras. */
export interface ResolvedField extends FieldDisplay {
  displayName?: string
  color?: string
  hidden?: boolean
}

/** Compute the effective display for `series` under `config`: defaults, then the
 *  first matching override laid on top. The series' own label/colour seed the
 *  display name/colour when no override sets them. Mirrors nexus's `resolveField`
 *  (without the legacy flat bridge — Rubix's MVP config has no flat field). */
export function resolveField(series: SeriesField, config: FieldConfig | undefined): ResolvedField {
  const defaults = config?.defaults ?? {}
  const override = matchOverride(series, config?.overrides)

  const base: ResolvedField = {
    ...stripUndefined(defaults),
    unit: defaults.unit ?? series.unit,
    displayName: series.label,
    color: series.color,
  }

  if (!override) return base
  return {
    ...base,
    ...stripUndefined(override.display),
    displayName: override.display.displayName ?? base.displayName,
    color: override.display.color ?? base.color,
  }
}

/** First override whose matcher selects this series, or undefined. `byName` tests
 *  the series' value column and label exactly; `byRegex` tests the same pattern. */
export function matchOverride(
  series: SeriesField,
  overrides: ReadonlyArray<FieldOverride> | undefined,
): FieldOverride | undefined {
  if (!overrides || overrides.length === 0) return undefined
  const candidates = [series.value, series.label].filter(
    (s): s is string => typeof s === 'string' && s.length > 0,
  )
  return overrides.find((o) => {
    if (o.matcher.type === 'byName') return candidates.includes(o.matcher.value)
    try {
      const re = new RegExp(o.matcher.value)
      return candidates.some((c) => re.test(c))
    } catch {
      return false
    }
  })
}

/** The colour for `value` from a multi-step ramp: the highest step whose `value`
 *  the reading meets or exceeds. Base step is `value: null` (−∞). Order-invariant;
 *  returns an `hsl(...)` string or undefined when no step applies. (nexus rampColor) */
export function rampColor(value: number, steps: ReadonlyArray<ThresholdStep>): string | undefined {
  if (steps.length === 0) return undefined
  const sorted = [...steps].sort((a, b) => (a.value ?? -Infinity) - (b.value ?? -Infinity))
  let chosen: ThresholdStep | undefined
  for (const step of sorted) {
    const lower = step.value ?? -Infinity
    if (value >= lower) chosen = step
    else break
  }
  return chosen ? `hsl(${chosen.color})` : undefined
}

// Drop undefined keys so a spread doesn't clobber a lower layer's defined value.
function stripUndefined<T extends object>(obj: T): Partial<T> {
  const out: Partial<T> = {}
  for (const [k, v] of Object.entries(obj)) {
    if (v !== undefined) (out as Record<string, unknown>)[k] = v
  }
  return out
}
