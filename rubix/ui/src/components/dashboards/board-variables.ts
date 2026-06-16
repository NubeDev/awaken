// Dashboard-variable resolution helpers (VARIABLES-AND-TEMPLATING.md). A board
// carries `variables` in its record JSON; at view time each resolves to a live
// selection that is (a) rendered in the variable bar, (b) sent to the backend as
// the `variables` array on every panel query (the backend lowers `$name` into an
// escaped literal — the injection boundary), and (c) mirrored in the URL as
// `?var-<name>=…` so a parameterised board deep-links and survives refresh.
//
// Selection precedence (later wins), per PAGE-CONTEXT-AND-NAV.md §1:
//   board default (`variable.current`) → nav node `context.values` → URL `?var-*`.
// The explicit URL selection is the user's live pick; the nav value is the mount's
// binding; the board default is the fallback.

import type { BoardVariable, VariableScalar } from '../../api/boards'
import type { QueryVariable } from '../../api/query'

// One resolved option for a select-style variable.
export interface VariableOption {
  value: VariableScalar
  label: string
}

// A live selection: one value, several (multi), or the "All" sentinel which
// expands to every resolved option at wire time.
export type Selection = VariableScalar | VariableScalar[]

// The sentinel a multi/include-all variable holds when "All" is picked. Kept out
// of the value space (a `$__`-prefixed token never collides with real data, the
// same convention the backend reserves).
export const ALL_SENTINEL = '$__all'

// The URL search key reserved for a variable's explicit selection.
export function varSearchKey(name: string): string {
  return `var-${name}`
}

// Read a variable's selection from a router search object, if present.
export function urlSelection(search: Record<string, unknown>, name: string): Selection | undefined {
  const raw = search[varSearchKey(name)]
  if (raw === undefined || raw === null) return undefined
  if (Array.isArray(raw)) return raw.filter(isScalar)
  return isScalar(raw) ? raw : undefined
}

function isScalar(v: unknown): v is VariableScalar {
  return typeof v === 'string' || typeof v === 'number' || typeof v === 'boolean'
}

// The effective selection for one variable, applying the precedence above. Falls
// back to the first option (or "All" for an include-all multi) so a freshly opened
// board always has a defined selection rather than an empty query.
export function effectiveSelection(
  variable: BoardVariable,
  fromUrl: Selection | undefined,
  fromNav: Selection | undefined,
  options: VariableOption[],
): Selection {
  const explicit = fromUrl ?? fromNav ?? variable.current
  if (explicit !== undefined) return explicit
  if (variable.multi && variable.include_all) return ALL_SENTINEL
  if (variable.kind === 'textbox' || variable.kind === 'constant') return ''
  return options[0]?.value ?? ''
}

// Expand a selection into the concrete value(s) sent on the wire — resolving the
// "All" sentinel to every option, and normalising a single-select array down.
export function wireValue(
  selection: Selection,
  variable: BoardVariable,
  options: VariableOption[],
): QueryVariable['value'] {
  if (selection === ALL_SENTINEL) return options.map((o) => o.value)
  if (Array.isArray(selection)) return variable.multi ? selection : (selection[0] ?? '')
  return selection
}

// Build the wire `variables` array for a board's panel queries from the resolved
// selections. Every variable is sent; the backend only substitutes the ones a
// statement references, so an unused selection is harmless.
export function toQueryVariables(
  variables: BoardVariable[],
  selections: Record<string, Selection>,
  optionsByName: Map<string, VariableOption[]>,
): QueryVariable[] {
  return variables.map((v) => ({
    name: v.name,
    value: wireValue(selections[v.name] ?? '', v, optionsByName.get(v.name) ?? []),
  }))
}

// A stable hash of the resolved wire values — folded into the board-batch query
// key so a selection change re-fetches exactly the panels whose SQL references a
// variable (an unreferenced panel's row stays cached on the backend's snapshot).
export function variableRevision(queryVariables: QueryVariable[]): string {
  const sorted = [...queryVariables].sort((a, b) => a.name.localeCompare(b.name))
  return JSON.stringify(sorted.map((v) => [v.name, v.value]))
}
