// SQL editor parameters store — ported from Laminar's `sql-editor-store.ts`
// (§1a, LAMINAR-BORROW.md), trimmed to the part that fits Rubix today: the
// template-parameter set (`start_time` / `end_time` / `interval_unit`) and the
// formatter that turns it into substitutable string values. Laminar's
// template/projectId machinery is dropped — saved queries are `kind:"query"`
// records here (savedQueries.ts), not Postgres templates.

import { format, startOfToday, subDays } from 'date-fns'
import { create } from 'zustand'

export type SQLParameter = { name: string } & (
  | { value?: Date; type: 'date' }
  | { value?: string; type: 'string' }
  | { value?: number; type: 'number' }
)

export interface SqlEditorState {
  parameters: SQLParameter[]
  setParameterValue: (name: string, value: SQLParameter['value']) => void
  getFormattedParameters: () => Record<string, string | number>
}

// Defaults mirror Laminar's: a trailing-7-day window, hourly buckets. These map
// onto the time-window bucketing `POST /query` already does (§1a).
const initialParameters: SQLParameter[] = [
  { name: 'start_time', value: subDays(startOfToday(), 7), type: 'date' },
  { name: 'end_time', value: startOfToday(), type: 'date' },
  { name: 'interval_unit', value: 'hour', type: 'string' },
]

export const useSqlEditorStore = create<SqlEditorState>()((set, get) => ({
  parameters: initialParameters,

  setParameterValue: (name, value) =>
    set((state) => ({
      parameters: state.parameters.map((p) =>
        p.name === name ? ({ ...p, value } as SQLParameter) : p,
      ),
    })),

  getFormattedParameters: () =>
    get().parameters.reduce(
      (acc, p) => {
        if (p.value == null) return acc
        if (p.value instanceof Date) acc[p.name] = format(p.value, 'yyyy-MM-dd HH:mm:ss.SSS')
        else acc[p.name] = p.value
        return acc
      },
      {} as Record<string, string | number>,
    ),
}))

// Substitute `{{param}}` placeholders in a query with the formatted values.
// Date/datetime values are quoted so they drop into SQL as string literals;
// numeric/identifier values (e.g. interval_unit) are spliced raw. Mirrors the
// effect of Laminar's parameter substitution without the server round-trip.
export function applyParameters(
  sql: string,
  params: Record<string, string | number>,
): string {
  return sql.replace(/\{\{\s*(\w+)\s*\}\}/g, (whole, name: string) => {
    if (!(name in params)) return whole
    const value = params[name]
    if (typeof value === 'number') return String(value)
    // Datetime-shaped strings become quoted literals; bare words (interval_unit)
    // splice in unquoted so `date_trunc('hour', …)` style usage works.
    return /[-:\s]/.test(value) ? `'${value.replace(/'/g, "''")}'` : value
  })
}
