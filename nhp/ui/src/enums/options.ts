/**
 * The closed enum allowed-sets for NHP collection fields, as UI dropdown options.
 *
 * rubix has no native Select/enum FieldType and its gate does NOT enforce any
 * allowed-set (WS-02 verified: gate checks `required` + field TYPE only). So the
 * closed enums are enforced client-side: the registrar/seed via
 * `nhp/collections/enums.mjs`, and the admin UI via the dropdowns built from this
 * module. See nhp/docs/DOMAIN-MODEL.md (register fields) and OVERVIEW.md gap #1.
 *
 * SINGLE SOURCE, NO DUPLICATION. `nhp/collections/enums.mjs` is the Node-side
 * source of truth (WS-02). This module is the TS mirror the UI consumes — the
 * `.mjs` lives outside `src` and is untyped, so it cannot be imported into the
 * tsc build directly. `options.unit.test.ts` imports BOTH and asserts they are
 * identical, so any drift fails the test gate. Edit the `.mjs`, then mirror here.
 *
 * Reusable by WS-05 (network net_type/protocol/status) and WS-06 (wizards).
 */

export const NET_TYPE = ['485', 'ethernet'] as const
export const PROTOCOL = ['modbus'] as const

export const FN_CODE = [
  'read_holding',
  'read_input',
  'read_coil',
  'read_discrete',
  'write_holding',
  'write_coil',
] as const

export const DATATYPE = [
  'int16',
  'uint16',
  'int32',
  'uint32',
  'float32',
  'float64',
  'bool',
] as const

export const BYTE_ORDER = ['big', 'little', 'big_swap', 'little_swap'] as const

export const CHART_TYPE = [
  'line',
  'bar',
  'area',
  'stat',
  'gauge',
  'table',
] as const

/** Poller-owned status (DOMAIN-MODEL "Status fields are poller-owned"). */
export const STATUS = ['online', 'offline', 'unknown'] as const

export type FnCode = (typeof FN_CODE)[number]
export type Datatype = (typeof DATATYPE)[number]
export type ByteOrder = (typeof BYTE_ORDER)[number]
export type ChartType = (typeof CHART_TYPE)[number]

/** `{label,value}` list for the shadcn Select / SelectDropdown components. */
export function toOptions(
  values: readonly string[]
): { label: string; value: string }[] {
  return values.map((value) => ({ value, label: value }))
}
