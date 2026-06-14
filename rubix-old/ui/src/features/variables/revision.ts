/**
 * `varRevision`: a compact, stable token for a set of resolved variable values,
 * folded into a widget's data query key so a selection change re-fetches exactly
 * the dependent widgets (docs/design/variables-and-templating.md §6). Two equal
 * value-sets always produce the same token; any change produces a different one.
 *
 * Only the variables a widget's SQL references contribute to its revision, so a
 * widget with no variable reference gets a constant token and is never
 * invalidated by an unrelated selection (back-compat).
 */
import type { Variable, VariableValue } from '@/api/types'

/** A stable string for a single resolved value (arrays are order-significant,
 *  matching how a multi-select expands into an ordered `IN (...)`). */
function valueToken(value: VariableValue | undefined): string {
  if (value === undefined) return 'undef'
  return JSON.stringify(value)
}

/**
 * The revision token for `names`' current values among `variables`. Names are
 * sorted so the token is independent of declaration order; each contributes its
 * name and current value. An empty `names` yields a constant `'none'`.
 */
export function varRevision(
  variables: Variable[],
  names: string[]
): string {
  if (names.length === 0) return 'none'
  const byName = new Map(variables.map((v) => [v.name, v]))
  return [...names]
    .sort()
    .map((name) => `${name}=${valueToken(byName.get(name)?.current)}`)
    .join('&')
}
