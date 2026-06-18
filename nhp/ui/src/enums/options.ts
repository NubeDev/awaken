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

export const NET_TYPE = ['485', 'ethernet', 'lora'] as const
export const PROTOCOL = ['modbus', 'lora'] as const

export const FN_CODE = [
  'read_holding',
  'read_input',
  'read_coil',
  'read_discrete',
  'write_holding',
  'write_coil',
  'lora_uplink',
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

/**
 * Register `quantity` values NHP styles in the dashboard (metric-style icons +
 * colours), groups by, and rolls up. Electrical (power-meter) quantities plus the
 * LoRa sensor / Modbus-IO quantities. Mirror of `QUANTITY` in enums.mjs — the
 * drift test asserts the two are identical. A quantity outside this set still
 * writes fine (it's a plain text field); it just falls back to a generic style.
 */
export const QUANTITY = [
  'voltage',
  'current',
  'power',
  'energy',
  'frequency',
  'power_factor',
  'temperature',
  'co2',
  'co',
  'battery',
  'volume',
  'pulse',
  'state',
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

/**
 * NHP roles on the rubix principal surface (ADMIN.md §6). These are the rubix
 * `Role` wire strings (rubix/crates/rubix-server/src/dto/admin.rs `parse_role`),
 * not an NHP collection field — so they are NOT in enums.mjs / the drift test;
 * they live here only as the user-admin dropdown source (WS-05).
 */
export const ROLE = ['viewer', 'operator', 'admin'] as const

export type NetType = (typeof NET_TYPE)[number]
export type Protocol = (typeof PROTOCOL)[number]
export type Status = (typeof STATUS)[number]
export type FnCode = (typeof FN_CODE)[number]
export type Datatype = (typeof DATATYPE)[number]
export type ByteOrder = (typeof BYTE_ORDER)[number]
export type ChartType = (typeof CHART_TYPE)[number]
export type Quantity = (typeof QUANTITY)[number]

/** `{label,value}` list for the shadcn Select / SelectDropdown components. */
export function toOptions(
  values: readonly string[]
): { label: string; value: string }[] {
  return values.map((value) => ({ value, label: value }))
}
