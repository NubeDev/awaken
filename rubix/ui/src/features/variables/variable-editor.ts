/**
 * Pure editor operations over a dashboard's variable list (add / update /
 * delete / reorder), kept separate from the React editor component so the list
 * mutations are unit-testable. The list is the dashboard snapshot's `variables`
 * array (docs/design/variables-and-templating.md §4); these return a new array,
 * never mutate in place.
 */
import type { Variable, VariableConfig, VariableKind } from '@/api/types'

/** A blank config for a kind, so switching kind in the editor yields a valid,
 *  tagged config matching the new kind. */
export function defaultConfig(kind: VariableKind): VariableConfig {
  switch (kind) {
    case 'constant':
      return { kind: 'constant', value: '' }
    case 'custom':
      return { kind: 'custom', options: [] }
    case 'query':
      return { kind: 'query', sql: '' }
    case 'datasource':
      return { kind: 'datasource' }
    case 'site':
      return { kind: 'site' }
    case 'interval':
      return { kind: 'interval', options: ['1m', '5m', '1h'] }
    case 'textbox':
      return { kind: 'textbox' }
  }
}

/** A new variable of `kind` with a unique default name within `existing`. */
export function newVariable(
  kind: VariableKind,
  existing: Variable[]
): Variable {
  const taken = new Set(existing.map((v) => v.name))
  let name: string = kind
  let n = 1
  while (taken.has(name)) name = `${kind}${++n}`
  return {
    name,
    kind,
    config: defaultConfig(kind),
    multi: false,
    include_all: false,
    hidden: false,
  }
}

/** Replace the variable at `index`, keeping `kind` and `config` in agreement
 *  (a kind change resets the config to the new kind's default). */
export function updateVariable(
  variables: Variable[],
  index: number,
  patch: Partial<Variable>
): Variable[] {
  return variables.map((v, i) => {
    if (i !== index) return v
    const next = { ...v, ...patch }
    if (patch.kind && patch.kind !== v.kind && !patch.config) {
      next.config = defaultConfig(patch.kind)
    }
    return next
  })
}

export function deleteVariable(
  variables: Variable[],
  index: number
): Variable[] {
  return variables.filter((_, i) => i !== index)
}

/** Move the variable at `from` to `to` (clamped), preserving the others' order.
 *  Reordering is purely presentational — resolution order is computed from the
 *  dependency graph, not array order. */
export function moveVariable(
  variables: Variable[],
  from: number,
  to: number
): Variable[] {
  if (from === to || from < 0 || from >= variables.length) return variables
  const clamped = Math.max(0, Math.min(to, variables.length - 1))
  const next = variables.slice()
  const [moved] = next.splice(from, 1)
  next.splice(clamped, 0, moved)
  return next
}
