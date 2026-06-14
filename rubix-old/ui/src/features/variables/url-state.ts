/**
 * URL round-trip for variable selections (docs/design/variables-and-templating.md
 * §7): `?var-site=Site-A&var-site=Site-B` (repeatable for multi-select). This is
 * rubix's first query-param sync; the `var-` prefix is reserved for explicit
 * variable state (bare params belong to the page-context source, WS-06). These
 * are pure functions over a `URLSearchParams`; the React hook wires them to the
 * live location.
 */
import type { VariableValue } from '@/api/types'

const PREFIX = 'var-'

/**
 * Read every `var-<name>` param into a name -> value(s) map. A single occurrence
 * yields a scalar string; repeated occurrences yield a string array (a
 * multi-select). Empty values are dropped so `?var-site=` is not a selection.
 */
export function readVarParams(
  params: URLSearchParams
): Record<string, VariableValue> {
  const grouped = new Map<string, string[]>()
  for (const [key, value] of params.entries()) {
    if (!key.startsWith(PREFIX) || value === '') continue
    const name = key.slice(PREFIX.length)
    const list = grouped.get(name) ?? []
    list.push(value)
    grouped.set(name, list)
  }
  const out: Record<string, VariableValue> = {}
  for (const [name, list] of grouped) {
    out[name] = list.length === 1 ? list[0] : list
  }
  return out
}

/**
 * Write a name -> value(s) selection back as `var-<name>` params, returning a new
 * `URLSearchParams` with every prior `var-*` entry replaced (non-`var-` params
 * are preserved untouched). A `null`/empty selection clears that variable's
 * params, keeping shared links clean.
 */
export function writeVarParams(
  base: URLSearchParams,
  selection: Record<string, VariableValue>
): URLSearchParams {
  const next = new URLSearchParams()
  // Preserve non-variable params in their original order.
  for (const [key, value] of base.entries()) {
    if (!key.startsWith(PREFIX)) next.append(key, value)
  }
  for (const [name, value] of Object.entries(selection)) {
    const values = Array.isArray(value) ? value : [value]
    for (const v of values) {
      if (v === null || v === undefined || v === '') continue
      next.append(`${PREFIX}${name}`, String(v))
    }
  }
  return next
}
