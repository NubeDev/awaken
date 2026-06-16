// Pure list operations for the variable editor (VARIABLES-AND-TEMPLATING.md §4),
// kept apart from the React dialog so add/update/delete/reorder are testable. Each
// returns a new array; the list is the board record's `content.variables`, saved
// in one `PATCH /records/{board}` (the variables ride the board snapshot).

import type { BoardVariable, VariableKind } from '../../api/boards'

// A blank per-kind config, so switching a variable's kind yields a valid config
// for the new kind rather than carrying a stale one.
export function blankConfig(kind: VariableKind): BoardVariable['config'] {
  switch (kind) {
    case 'custom':
      return { options: [] }
    case 'query':
      return { query: '' }
    default:
      return {}
  }
}

// A new variable of `kind` with a name unique within `existing`.
export function newVariable(kind: VariableKind, existing: BoardVariable[]): BoardVariable {
  const taken = new Set(existing.map((v) => v.name))
  let name: string = kind
  let n = 1
  while (taken.has(name)) name = `${kind}${++n}`
  return { name, kind, config: blankConfig(kind), multi: false, include_all: false, hidden: false }
}

// Replace the variable at `index`, resetting the config when the kind changes (and
// the caller did not supply a new config).
export function updateVariable(
  variables: BoardVariable[],
  index: number,
  patch: Partial<BoardVariable>,
): BoardVariable[] {
  return variables.map((v, i) => {
    if (i !== index) return v
    const next = { ...v, ...patch }
    if (patch.kind && patch.kind !== v.kind && !patch.config) {
      next.config = blankConfig(patch.kind)
    }
    return next
  })
}

export function deleteVariable(variables: BoardVariable[], index: number): BoardVariable[] {
  return variables.filter((_, i) => i !== index)
}

// Move the variable at `from` to `to` (clamped), preserving the others' order.
export function moveVariable(
  variables: BoardVariable[],
  from: number,
  to: number,
): BoardVariable[] {
  if (from === to || from < 0 || from >= variables.length) return variables
  const clamped = Math.max(0, Math.min(to, variables.length - 1))
  const next = variables.slice()
  const [moved] = next.splice(from, 1)
  next.splice(clamped, 0, moved)
  return next
}
