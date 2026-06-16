/**
 * Column model for the register-map table. The table has many fields (DOMAIN-MODEL
 * §register); showing them all at once is unreadable, so columns are toggleable and
 * the protocol-heavy ones are hidden by default (edit them in the detail sheet).
 * `key` is pinned (always shown) so a row is never anonymous.
 */
export type RegisterColumnId =
  | 'key'
  | 'name'
  | 'address'
  | 'fn_code'
  | 'datatype'
  | 'word_count'
  | 'byte_order'
  | 'scale'
  | 'offset'
  | 'signed'
  | 'unit'
  | 'quantity'
  | 'history'
  | 'chart_type'
  | 'chart_group'
  | 'precision'

export type RegisterColumn = {
  id: RegisterColumnId
  label: string
  /** Visible on first load. Protocol internals default off. */
  default: boolean
  /** Can't be hidden (keeps every row identifiable). */
  pinned?: boolean
}

export const REGISTER_COLUMNS: RegisterColumn[] = [
  { id: 'key', label: 'Key', default: true, pinned: true },
  { id: 'name', label: 'Name', default: true },
  { id: 'address', label: 'Addr', default: true },
  { id: 'fn_code', label: 'Fn code', default: false },
  { id: 'datatype', label: 'Datatype', default: true },
  { id: 'word_count', label: 'Words', default: false },
  { id: 'byte_order', label: 'Byte order', default: false },
  { id: 'scale', label: 'Scale', default: false },
  { id: 'offset', label: 'Offset', default: false },
  { id: 'signed', label: 'Signed', default: false },
  { id: 'unit', label: 'Unit', default: true },
  { id: 'quantity', label: 'Quantity', default: false },
  { id: 'history', label: 'History', default: false },
  { id: 'chart_type', label: 'Chart', default: false },
  { id: 'chart_group', label: 'Group', default: true },
  { id: 'precision', label: 'Prec.', default: false },
]

export const DEFAULT_VISIBLE_COLUMNS = (): Set<RegisterColumnId> =>
  new Set(REGISTER_COLUMNS.filter((c) => c.default).map((c) => c.id))

export const ALARM_SUMMARY = (count: number) =>
  count === 0 ? 'No alarm' : `${count} threshold${count === 1 ? '' : 's'}`
