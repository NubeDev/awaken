// Resolve a board's variables to live selections + the wire array (VARIABLES-AND-
// TEMPLATING.md §5/§6). Option lists are fetched per kind (site records, a custom
// list, or a `query` whose first column is the options); selections apply the URL →
// nav → board-default precedence; the result is the `variables` array each panel
// query sends and a `revision` to fold into the board-batch query key.

import { useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import type { ApiClient } from '../../api/client'
import type { BoardVariable } from '../../api/boards'
import { listRecords } from '../../api/records'
import { runQuery, type QueryVariable, type TimeScope } from '../../api/query'
import {
  effectiveSelection,
  toQueryVariables,
  urlSelection,
  variableRevision,
  type Selection,
  type VariableOption,
} from './board-variables'

interface UseBoardVariablesArgs {
  api: ApiClient
  tenant: string
  variables: BoardVariable[]
  /** The board's time scope — a `query` variable resolves against it. */
  time?: TimeScope
  /** Variable values bound by the nav node behind `?nav=` (context.values). */
  navValues: Record<string, Selection>
  /** The router search object, read for `?var-<name>` explicit selections. */
  search: Record<string, unknown>
}

export interface BoardVariablesState {
  /** Visible variables (a hidden/constant variable still resolves but is not shown). */
  visible: BoardVariable[]
  options: Map<string, VariableOption[]>
  selections: Record<string, Selection>
  /** The wire array for every panel query. */
  queryVariables: QueryVariable[]
  /** A stable hash of the resolved values, for the board-batch query key. */
  revision: string
  loading: boolean
}

export function useBoardVariables({
  api,
  tenant,
  variables,
  time,
  navValues,
  search,
}: UseBoardVariablesArgs): BoardVariablesState {
  // URL-explicit selection per variable (the live user pick).
  const fromUrl = useMemo(() => {
    const m: Record<string, Selection | undefined> = {}
    for (const v of variables) m[v.name] = urlSelection(search, v.name)
    return m
  }, [search, variables])

  // A provisional selection ignoring options, so a `query` variable can resolve
  // against its parents' current picks (one-level cascade) before options load.
  const parentVars = useMemo<QueryVariable[]>(() => {
    const out: QueryVariable[] = []
    for (const v of variables) {
      const sel = fromUrl[v.name] ?? navValues[v.name] ?? v.current
      if (sel === undefined || Array.isArray(sel)) {
        if (Array.isArray(sel)) out.push({ name: v.name, value: sel })
        continue
      }
      out.push({ name: v.name, value: sel })
    }
    return out
  }, [variables, fromUrl, navValues])

  const optionsQuery = useQuery({
    queryKey: [
      'variable-options',
      tenant,
      variables.map((v) => ({ name: v.name, kind: v.kind, config: v.config })),
      time,
      parentVars,
    ],
    queryFn: () => resolveOptions(api, variables, time, parentVars),
    enabled: variables.length > 0,
  })

  const options = optionsQuery.data ?? EMPTY_OPTIONS

  const selections = useMemo(() => {
    const m: Record<string, Selection> = {}
    for (const v of variables) {
      m[v.name] = effectiveSelection(
        v,
        fromUrl[v.name],
        navValues[v.name],
        options.get(v.name) ?? [],
      )
    }
    return m
  }, [variables, fromUrl, navValues, options])

  const queryVariables = useMemo(
    () => toQueryVariables(variables, selections, options),
    [variables, selections, options],
  )
  const revision = useMemo(() => variableRevision(queryVariables), [queryVariables])

  return {
    visible: variables.filter((v) => !v.hidden),
    options,
    selections,
    queryVariables,
    revision,
    loading: variables.length > 0 && optionsQuery.isPending,
  }
}

const EMPTY_OPTIONS = new Map<string, VariableOption[]>()

// Fetch every variable's option list. Site options come from the `kind:"site"`
// records (resolved once and shared); a `query` variable runs its SQL with the
// other variables' current values as parents (the cascade).
async function resolveOptions(
  api: ApiClient,
  variables: BoardVariable[],
  time: TimeScope | undefined,
  parentVars: QueryVariable[],
): Promise<Map<string, VariableOption[]>> {
  const out = new Map<string, VariableOption[]>()
  let sites: VariableOption[] | null = null

  for (const v of variables) {
    if (v.kind === 'custom') {
      out.set(
        v.name,
        (v.config?.options ?? []).map((o) => ({ value: o, label: String(o) })),
      )
    } else if (v.kind === 'site') {
      if (!sites) {
        const records = await listRecords(api, { kind: 'site' })
        sites = records.map((r) => {
          const c = r.content as { key?: unknown; name?: unknown }
          const value = typeof c.key === 'string' ? c.key : r.id
          const label = typeof c.name === 'string' ? c.name : value
          return { value, label }
        })
      }
      out.set(v.name, sites)
    } else if (v.kind === 'query' && v.config?.query) {
      const res = await runQuery(api, v.config.query, {
        time,
        variables: parentVars.filter((p) => p.name !== v.name),
      })
      const col = res.columns[0]?.name
      const seen = new Set<string>()
      const opts: VariableOption[] = []
      for (const row of res.rows) {
        const raw = col ? row[col] : undefined
        if (raw === null || raw === undefined) continue
        const s = String(raw)
        if (seen.has(s)) continue
        seen.add(s)
        opts.push({ value: s, label: s })
      }
      out.set(v.name, opts)
    } else {
      out.set(v.name, [])
    }
  }
  return out
}
