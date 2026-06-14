/**
 * Resolution ordering + cycle detection for dashboard variables
 * (docs/design/variables-and-templating.md §5). A `query` variable's option SQL
 * may reference another variable (`WHERE site_id = '$site'`), so the parent must
 * resolve before the child. This builds the dependency order and rejects cycles
 * with a clear error.
 *
 * Only references to *other dashboard variables* create an edge; built-in tokens
 * (`$__org`, `$__from`, …) are resolved from context up front and never depend on
 * an authored variable, so they are not nodes here.
 */
import type { Variable } from '@/api/types'
import { referencedVariables } from './refs'

/** A dependency cycle among variables, naming the members of the cycle. */
export class VariableCycleError extends Error {
  constructor(public readonly cycle: string[]) {
    super(`variable dependency cycle: ${cycle.join(' -> ')}`)
    this.name = 'VariableCycleError'
  }
}

/** The names a variable depends on: only `query` kinds reference others, via
 *  their option SQL. Self-references and references to non-existent variables
 *  are dropped (a self-reference would otherwise be a trivial cycle, and an
 *  unknown name is the server's error to raise at query time). */
function dependenciesOf(variable: Variable, known: Set<string>): string[] {
  if (variable.config.kind !== 'query') return []
  return referencedVariables(variable.config.sql).filter(
    (name) => name !== variable.name && known.has(name)
  )
}

/**
 * Order `variables` so every variable comes after the ones it depends on
 * (topological sort, stable on input order for independent variables). Throws
 * [`VariableCycleError`] if the dependency graph has a cycle.
 */
export function resolutionOrder(variables: Variable[]): Variable[] {
  const known = new Set(variables.map((v) => v.name))
  const byName = new Map(variables.map((v) => [v.name, v]))
  const deps = new Map(
    variables.map((v) => [v.name, dependenciesOf(v, known)])
  )

  const ordered: Variable[] = []
  const done = new Set<string>()
  // DFS post-order with on-stack tracking for cycle detection.
  const onStack: string[] = []
  const inStack = new Set<string>()

  const visit = (name: string) => {
    if (done.has(name)) return
    if (inStack.has(name)) {
      // Slice the stack from the first occurrence of `name` to name the cycle.
      const start = onStack.indexOf(name)
      throw new VariableCycleError([...onStack.slice(start), name])
    }
    onStack.push(name)
    inStack.add(name)
    for (const dep of deps.get(name) ?? []) visit(dep)
    inStack.delete(name)
    onStack.pop()
    done.add(name)
    const variable = byName.get(name)
    if (variable) ordered.push(variable)
  }

  // Visit in input order so independent variables keep their authored order.
  for (const v of variables) visit(v.name)
  return ordered
}
