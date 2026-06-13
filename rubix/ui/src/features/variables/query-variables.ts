/**
 * Build the `QueryVariable[]` a widget sends with its SQL so the server's
 * interpolation engine binds the selected values (docs/design/variables-and-
 * templating.md §2, §5). Only the variables a widget's SQL references are sent;
 * each carries its current value (a scalar for single-select, an array for
 * multi/"All").
 *
 * The values leave here as plain data; the server binds them as parameters and
 * never splices them into SQL. The "All" expansion is explicit — an "All"
 * selection resolves to the full option list *before* this point, so a multi
 * value reaching here is already the concrete list to bind.
 */
import type { QueryVariable, Variable, VariableValue } from '@/api/types'
import { referencedVariables } from './refs'

/** The current value normalised for sending: an undefined `current` becomes
 *  `null` (a single bound NULL), which the engine treats as a literal. */
function sendValue(value: VariableValue | undefined): VariableValue {
  return value === undefined ? null : value
}

/**
 * The query variables for `sql`: one per referenced dashboard variable that
 * exists in `variables`. A referenced name with no matching variable is omitted
 * here so the server raises its own clear unknown-variable error (rather than
 * the UI inventing a value).
 */
export function queryVariablesFor(
  sql: string,
  variables: Variable[]
): QueryVariable[] {
  const byName = new Map(variables.map((v) => [v.name, v]))
  const out: QueryVariable[] = []
  for (const name of referencedVariables(sql)) {
    const variable = byName.get(name)
    if (!variable) continue
    out.push({ name, value: sendValue(variable.current) })
  }
  return out
}

/** The concrete options an "All" selection expands to for a variable: every
 *  option value. Explicit expansion keeps the resulting `IN (...)` predictable
 *  and pushdown-friendly (design §"All / multi"). */
export function expandAll(options: VariableValue[]): VariableValue[] {
  return options.slice()
}
