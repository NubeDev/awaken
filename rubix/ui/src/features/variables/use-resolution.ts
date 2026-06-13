/**
 * Resolve a dashboard's variables into option lists + current values
 * (docs/design/variables-and-templating.md §5). Resolution order is built-ins →
 * constants/custom → site → query (topological, via [`resolutionOrder`]); a
 * `query` variable's SQL may reference an already-resolved parent, which is how
 * cascading works. Options are cached per `(name, parent-values)` by React Query
 * (the query variable's key folds in the resolved parent values).
 *
 * The hook returns the variables with their `current` value applied from the URL
 * selection (falling back to the first option / "All"), plus the resolved option
 * lists for the bar to render. `datasource`-kind options need a datasources list
 * endpoint the UI does not yet expose (see docs/sessions/june-13th/TODOs.md); its
 * options resolve empty until that lands.
 */
import { useMemo } from 'react'
import { useQueries } from '@tanstack/react-query'
import * as api from '@/api/endpoints'
import { qk } from '@/api/keys'
import { useSites } from '@/api/hooks'
import type { ScalarValue, Variable, VariableValue } from '@/api/types'
import { resolutionOrder } from './order'
import { queryVariablesFor } from './query-variables'

/** A resolved variable: its definition, the options to offer, and the effective
 *  current selection. */
export type ResolvedVariable = {
  variable: Variable
  options: ScalarValue[]
  current: VariableValue
}

/** Static option lists that need no async resolution. */
function staticOptions(variable: Variable): ScalarValue[] | null {
  switch (variable.config.kind) {
    case 'constant':
      return [variable.config.value]
    case 'custom':
    case 'interval':
      return variable.config.options
    case 'textbox':
      return []
    default:
      return null
  }
}

/**
 * Resolve `variables` for `org`, applying `selection` (the URL `?var-*` state).
 * `query`-kind option lists are fetched live, keyed on the resolved values of any
 * parent variables they reference so a parent change re-resolves the child.
 */
export function useVariableResolution(args: {
  org: string | undefined
  variables: Variable[]
  selection: Record<string, VariableValue>
}): { resolved: ResolvedVariable[]; error?: Error } {
  const { org, variables, selection } = args

  // Cycle detection happens here; a cycle is a hard, surfaced error.
  let ordered: Variable[] = variables
  let cycleError: Error | undefined
  try {
    ordered = resolutionOrder(variables)
  } catch (e) {
    cycleError = e as Error
  }

  const { data: sites = [] } = useSites(org)
  const siteOptions = useMemo<ScalarValue[]>(
    () => sites.filter((s) => s.org === org).map((s) => s.id),
    [sites, org]
  )

  // Resolve in order, threading each variable's effective current value forward
  // so a `query` child sees its parent's selection. The query SQL + its bound
  // parent values determine the React Query key (cache per parent-values).
  const queryVars = ordered.filter((v) => v.config.kind === 'query')
  const currentByName = new Map<string, VariableValue>()
  // Seed non-query currents first so query SQL binds against them.
  for (const v of ordered) {
    if (v.config.kind === 'query') continue
    currentByName.set(v.name, effectiveCurrent(v, selection, optionsFor(v)))
  }

  function optionsFor(v: Variable): ScalarValue[] {
    return v.config.kind === 'site' ? siteOptions : (staticOptions(v) ?? [])
  }

  const queries = useQueries({
    queries: queryVars.map((v) => {
      const sql = v.config.kind === 'query' ? v.config.sql : ''
      const boundVars = queryVariablesFor(
        sql,
        ordered.map((o) => ({
          ...o,
          current: currentByName.get(o.name),
        }))
      )
      const rev = JSON.stringify(boundVars)
      return {
        queryKey: qk.widgetData(`var:${v.name}` as never, rev),
        queryFn: () => api.query.run(sql, { variables: boundVars }),
        enabled: Boolean(org) && sql.length > 0 && !cycleError,
      }
    }),
  })

  const queryOptionsByName = new Map<string, ScalarValue[]>()
  queryVars.forEach((v, i) => {
    const rows = queries[i]?.data?.rows ?? []
    // The first column of each row becomes an option (design §1).
    const opts = rows.map((row) => {
      const first = Object.values(row)[0]
      return (first ?? null) as ScalarValue
    })
    queryOptionsByName.set(v.name, opts)
  })

  const resolved = ordered.map<ResolvedVariable>((variable) => {
    const options =
      variable.config.kind === 'query'
        ? (queryOptionsByName.get(variable.name) ?? [])
        : optionsFor(variable)
    return {
      variable,
      options,
      current: effectiveCurrent(variable, selection, options),
    }
  })

  return { resolved, error: cycleError }
}

/** The effective current value: the URL selection if present, else the first
 *  option (or every option for an `include_all` multi), else null. */
function effectiveCurrent(
  variable: Variable,
  selection: Record<string, VariableValue>,
  options: ScalarValue[]
): VariableValue {
  const picked = selection[variable.name]
  if (picked !== undefined) return picked
  if (variable.config.kind === 'constant') return options[0] ?? null
  if (variable.multi && variable.include_all) return options.slice()
  return options[0] ?? null
}
